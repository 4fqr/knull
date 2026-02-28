//! Local Registry Module
//! Handles package storage and retrieval from local filesystem registry

use crate::pkg::manager::PackageManifest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const LOCAL_REGISTRY_PATH: &str = ".knull/registry";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalPackageIndex {
    pub packages: HashMap<String, PackageEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageEntry {
    pub name: String,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionEntry {
    pub version: String,
    pub path: PathBuf,
    pub published_at: String,
}

impl Default for LocalPackageIndex {
    fn default() -> Self {
        LocalPackageIndex {
            packages: HashMap::new(),
        }
    }
}

fn get_home_dir() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())
}

fn get_registry_dir() -> Result<PathBuf, String> {
    let home = get_home_dir()?;
    Ok(home.join(LOCAL_REGISTRY_PATH))
}

fn get_packages_dir() -> Result<PathBuf, String> {
    Ok(get_registry_dir()?.join("packages"))
}

fn get_index_path() -> Result<PathBuf, String> {
    Ok(get_registry_dir()?.join("index").join("packages.json"))
}

fn ensure_registry_exists() -> Result<(), String> {
    let registry_dir = get_registry_dir()?;
    let packages_dir = registry_dir.join("packages");
    let index_dir = registry_dir.join("index");

    fs::create_dir_all(&packages_dir)
        .map_err(|e| format!("Failed to create packages directory: {}", e))?;
    fs::create_dir_all(&index_dir)
        .map_err(|e| format!("Failed to create index directory: {}", e))?;

    let index_path = index_dir.join("packages.json");
    if !index_path.exists() {
        let index = LocalPackageIndex::default();
        let content = serde_json::to_string_pretty(&index)
            .map_err(|e| format!("Failed to serialize index: {}", e))?;
        fs::write(&index_path, content).map_err(|e| format!("Failed to write index: {}", e))?;
    }

    Ok(())
}

fn load_index() -> Result<LocalPackageIndex, String> {
    ensure_registry_exists()?;
    let index_path = get_index_path()?;

    let content =
        fs::read_to_string(&index_path).map_err(|e| format!("Failed to read index: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse index: {}", e))
}

fn save_index(index: &LocalPackageIndex) -> Result<(), String> {
    let index_path = get_index_path()?;
    let content = serde_json::to_string_pretty(index)
        .map_err(|e| format!("Failed to serialize index: {}", e))?;
    fs::write(&index_path, content).map_err(|e| format!("Failed to write index: {}", e))
}

pub fn fetch_from_local(name: &str, version: &str) -> Result<PathBuf, String> {
    ensure_registry_exists()?;
    let packages_dir = get_packages_dir()?;
    let package_path = packages_dir.join(name).join(version);

    if package_path.exists() {
        Ok(package_path)
    } else {
        Err(format!(
            "Package {}@{} not found in local registry",
            name, version
        ))
    }
}

pub fn publish_to_local(project_path: &Path, manifest: &PackageManifest) -> Result<(), String> {
    ensure_registry_exists()?;

    let packages_dir = get_packages_dir()?;
    let package_dir = packages_dir
        .join(&manifest.package.name)
        .join(&manifest.package.version);

    if package_dir.exists() {
        return Err(format!(
            "Package {}@{} already exists in local registry",
            manifest.package.name, manifest.package.version
        ));
    }

    fs::create_dir_all(&package_dir)
        .map_err(|e| format!("Failed to create package directory: {}", e))?;

    // Copy manifest
    let manifest_dest = package_dir.join("package.toml");
    let toml_content = toml::to_string_pretty(manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    fs::write(&manifest_dest, toml_content)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    // Copy source files
    let src_dir = project_path.join("src");
    if src_dir.exists() {
        let dest_src = package_dir.join("src");
        copy_dir_all(&src_dir, &dest_src)
            .map_err(|e| format!("Failed to copy source files: {}", e))?;
    }

    // Copy additional files
    for file in &["README.md", "LICENSE", "CHANGELOG.md"] {
        let src = project_path.join(file);
        if src.exists() {
            fs::copy(&src, package_dir.join(file))
                .map_err(|e| format!("Failed to copy {}: {}", file, e))?;
        }
    }

    // Update index
    let mut index = load_index()?;
    let entry = index
        .packages
        .entry(manifest.package.name.clone())
        .or_insert_with(|| PackageEntry {
            name: manifest.package.name.clone(),
            versions: vec![],
        });

    entry.versions.push(VersionEntry {
        version: manifest.package.version.clone(),
        path: package_dir.clone(),
        published_at: chrono::Utc::now().to_rfc3339(),
    });

    save_index(&index)?;

    println!(
        "Published {}@{} to local registry",
        manifest.package.name, manifest.package.version
    );
    Ok(())
}

pub fn list_local_packages() -> Result<Vec<String>, String> {
    ensure_registry_exists()?;
    let index = load_index()?;

    let mut packages: Vec<String> = index.packages.keys().cloned().collect();
    packages.sort();
    Ok(packages)
}

pub fn list_local_versions(name: &str) -> Result<Vec<String>, String> {
    ensure_registry_exists()?;
    let index = load_index()?;

    match index.packages.get(name) {
        Some(entry) => {
            let versions: Vec<String> = entry.versions.iter().map(|v| v.version.clone()).collect();
            Ok(versions)
        }
        None => Ok(vec![]),
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.as_ref().join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
