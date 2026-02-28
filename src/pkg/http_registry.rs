//! HTTP Registry Client Module
//! Handles package download and upload to remote registry

use crate::pkg::manager::PackageManifest;
use crate::pkg::semver;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_REGISTRY: &str = "https://registry.knull-lang.dev";
const CACHE_DIR: &str = ".knull/cache";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryResponse {
    pub name: String,
    pub version: String,
    pub download_url: String,
    pub checksum: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionList {
    pub versions: Vec<String>,
}

fn get_cache_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    Ok(home.join(CACHE_DIR))
}

fn ensure_cache_exists() -> Result<(), String> {
    let cache_dir = get_cache_dir()?;
    fs::create_dir_all(&cache_dir).map_err(|e| format!("Failed to create cache directory: {}", e))
}

pub fn fetch_from_registry(name: &str, version: &str) -> Result<PathBuf, String> {
    ensure_cache_exists()?;

    let cache_dir = get_cache_dir()?;
    let package_cache = cache_dir.join(format!("{}-{}", name, version));

    // Check if already cached
    if package_cache.exists() {
        return Ok(package_cache);
    }

    let client = Client::new();
    let registry = std::env::var("KNULL_REGISTRY").unwrap_or_else(|_| DEFAULT_REGISTRY.to_string());

    let url = format!("{}/packages/{}/{}/download", registry, name, version);

    println!("  Downloading {}@{} from registry...", name, version);

    let response = client
        .get(&url)
        .send()
        .map_err(|e| format!("Failed to download package: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Package {}@{} not found on registry (status: {})",
            name,
            version,
            response.status()
        ));
    }

    // Create package cache directory
    fs::create_dir_all(&package_cache)
        .map_err(|e| format!("Failed to create package cache: {}", e))?;

    // Save the downloaded archive
    let archive_path = package_cache.join("package.tar.gz");
    let bytes = response
        .bytes()
        .map_err(|e| format!("Failed to read response: {}", e))?;
    fs::write(&archive_path, &bytes).map_err(|e| format!("Failed to write archive: {}", e))?;

    // Extract the archive
    extract_archive(&archive_path, &package_cache)?;

    // Remove the archive file
    let _ = fs::remove_file(&archive_path);

    println!("  Downloaded {}@{} to cache", name, version);
    Ok(package_cache)
}

pub fn resolve_version(name: &str, constraint: &str) -> Result<String, String> {
    let client = Client::new();
    let registry = std::env::var("KNULL_REGISTRY").unwrap_or_else(|_| DEFAULT_REGISTRY.to_string());

    let url = format!("{}/packages/{}/versions", registry, name);

    let response = client
        .get(&url)
        .send()
        .map_err(|e| format!("Failed to query registry: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Package {} not found on registry", name));
    }

    let version_list: VersionList = response
        .json()
        .map_err(|e| format!("Failed to parse registry response: {}", e))?;

    // Find best matching version
    let mut matching_versions: Vec<&str> = version_list
        .versions
        .iter()
        .filter(|v: &&String| semver::satisfies(v, constraint))
        .map(|v: &String| v.as_str())
        .collect();

    // Sort by version (highest first)
    matching_versions.sort_by(|a, b| semver::compare_versions(b, a));

    matching_versions
        .first()
        .map(|v| v.to_string())
        .ok_or_else(|| format!("No version of {} satisfies constraint {}", name, constraint))
}

pub fn publish_to_registry(
    manifest: &PackageManifest,
    archive_path: &Path,
    token: &str,
) -> Result<(), String> {
    let client = Client::new();
    let registry = std::env::var("KNULL_REGISTRY").unwrap_or_else(|_| DEFAULT_REGISTRY.to_string());

    let url = format!("{}/packages/{}/publish", registry, manifest.package.name);

    // Read archive
    let archive_data =
        fs::read(archive_path).map_err(|e| format!("Failed to read archive: {}", e))?;

    // Build multipart form
    let form = reqwest::blocking::multipart::Form::new()
        .part(
            "package",
            reqwest::blocking::multipart::Part::bytes(archive_data)
                .file_name("package.tar.gz")
                .mime_str("application/gzip")
                .map_err(|e| format!("Failed to create form part: {}", e))?,
        )
        .text(
            "manifest",
            toml::to_string(manifest)
                .map_err(|e| format!("Failed to serialize manifest: {}", e))?,
        );

    println!(
        "Publishing {}@{} to registry...",
        manifest.package.name, manifest.package.version
    );

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .map_err(|e| format!("Failed to publish package: {}", e))?;

    match response.status() {
        status if status.is_success() => {
            println!(
                "Published {}@{} successfully",
                manifest.package.name, manifest.package.version
            );
            Ok(())
        }
        status => {
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!("Failed to publish: {} - {}", status, error_text))
        }
    }
}

pub fn search_registry(query: &str) -> Result<Vec<String>, String> {
    let client = Client::new();
    let registry = std::env::var("KNULL_REGISTRY").unwrap_or_else(|_| DEFAULT_REGISTRY.to_string());

    let url = format!("{}/search?q={}", registry, query);

    let response = client
        .get(&url)
        .send()
        .map_err(|e| format!("Failed to search registry: {}", e))?;

    if !response.status().is_success() {
        return Err("Search failed".to_string());
    }

    #[derive(Deserialize)]
    struct SearchResponse {
        packages: Vec<String>,
    }

    let result: SearchResponse = response
        .json()
        .map_err(|e| format!("Failed to parse search response: {}", e))?;

    Ok(result.packages)
}

fn extract_archive(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let tar_gz =
        fs::File::open(archive_path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    archive
        .unpack(dest_dir)
        .map_err(|e| format!("Failed to extract archive: {}", e))?;

    Ok(())
}
