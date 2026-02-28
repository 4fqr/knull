//! Knull Package Manager
//! Handles dependencies, builds, and package resolution

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub mod http_registry;
pub mod local_registry;
pub mod lockfile;
pub mod manager;
pub mod semver;

// Re-export commonly used items
pub use lockfile::{LockedPackage, Lockfile, ResolvedDep};
pub use manager::{
    find_nearest_manifest, BuildConfig, PackageInfo, PackageManager, PackageManifest,
};
pub use semver::{compare_versions, parse_constraint, parse_version, satisfies};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub entry: String,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Dependencies {
    #[serde(flatten)]
    pub deps: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LegacyManifest {
    pub package: Package,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(rename = "dev-dependencies", default)]
    pub dev_dependencies: HashMap<String, String>,
    #[serde(default)]
    pub features: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub build: Option<BuildConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LegacyBuildConfig {
    #[serde(rename = "opt-level")]
    pub opt_level: Option<u32>,
    pub lto: Option<bool>,
    #[serde(rename = "script")]
    pub script: Option<String>,
}

impl LegacyManifest {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read knull.toml: {}", e))?;

        toml::from_str(&content).map_err(|e| format!("Failed to parse knull.toml: {}", e))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

        fs::write(path, content).map_err(|e| format!("Failed to write knull.toml: {}", e))
    }
}

pub struct LegacyPackageManager {
    manifest: LegacyManifest,
    project_root: PathBuf,
}

impl LegacyPackageManager {
    pub fn new(project_root: PathBuf) -> Result<Self, String> {
        let manifest_path = project_root.join("knull.toml");
        let manifest = LegacyManifest::from_file(&manifest_path)?;

        Ok(LegacyPackageManager {
            manifest,
            project_root,
        })
    }

    pub fn add_dependency(&mut self, name: &str, version: &str) {
        self.manifest
            .dependencies
            .insert(name.to_string(), version.to_string());
    }

    pub fn remove_dependency(&mut self, name: &str) {
        self.manifest.dependencies.remove(name);
    }

    pub fn list_dependencies(&self) -> &HashMap<String, String> {
        &self.manifest.dependencies
    }

    pub fn save(&self) -> Result<(), String> {
        let manifest_path = self.project_root.join("knull.toml");
        self.manifest.save(&manifest_path)
    }

    pub fn get_entry_point(&self) -> PathBuf {
        self.project_root.join(&self.manifest.package.entry)
    }

    pub fn build(&self) -> Result<(), String> {
        // Run build script if present
        if let Some(build) = &self.manifest.build {
            if let Some(script) = build.script.as_ref() {
                let script_path = self.project_root.join(script);
                if script_path.exists() {
                    println!("Running build script: {}", script);
                    // Execute build script
                }
            }
        }

        Ok(())
    }
}

pub fn resolve_dependencies(
    deps: &HashMap<String, String>,
) -> Result<Vec<(String, String)>, String> {
    // Placeholder for actual dependency resolution
    // Would download from registry, resolve versions, etc.
    Ok(deps.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
}
