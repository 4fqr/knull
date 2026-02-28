//! Knull Package Manager
//!
//! Handles package dependencies, building, and distribution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::pkg::lockfile::{Lockfile, ResolvedDep, LOCKFILE_NAME};
use crate::pkg::semver;

/// Package manifest (knull.toml)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageManifest {
    pub package: PackageInfo,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub dev_dependencies: HashMap<String, String>,
    #[serde(default)]
    pub build: BuildConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub entry: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub repository: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BuildConfig {
    #[serde(rename = "opt-level", default)]
    pub opt_level: u32,
    #[serde(default)]
    pub lto: bool,
    #[serde(default)]
    pub script: Option<String>,
}

impl PackageManifest {
    /// Load manifest from file
    pub fn load(path: &Path) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read manifest: {}", e))?;
        toml::from_str(&content).map_err(|e| format!("Failed to parse manifest: {}", e))
    }

    /// Save manifest to file
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        fs::write(path, content).map_err(|e| format!("Failed to write manifest: {}", e))
    }

    /// Create new manifest with defaults
    pub fn new(name: &str) -> Self {
        PackageManifest {
            package: PackageInfo {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                edition: "2024".to_string(),
                entry: "src/main.knull".to_string(),
                authors: vec![],
                description: String::new(),
                license: "MIT".to_string(),
                repository: String::new(),
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            build: BuildConfig {
                opt_level: 2,
                lto: false,
                script: None,
            },
        }
    }
}

/// Package manager
pub struct PackageManager {
    root_path: PathBuf,
    manifest: PackageManifest,
    cache_dir: PathBuf,
    lockfile: Option<Lockfile>,
}

impl PackageManager {
    pub fn new(root_path: PathBuf) -> Result<Self, String> {
        let manifest_path = root_path.join("knull.toml");
        let manifest = if manifest_path.exists() {
            PackageManifest::load(&manifest_path)?
        } else {
            return Err("No knull.toml found in project root".to_string());
        };

        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| root_path.join(".cache"))
            .join("knull")
            .join("packages");

        // Try to load existing lockfile
        let lockfile_path = root_path.join(LOCKFILE_NAME);
        let lockfile = if lockfile_path.exists() {
            Lockfile::parse(&lockfile_path).ok()
        } else {
            None
        };

        Ok(PackageManager {
            root_path,
            manifest,
            cache_dir,
            lockfile,
        })
    }

    /// Create new project
    pub fn new_project(name: &str) -> Result<(), String> {
        let project_dir = PathBuf::from(name);
        if project_dir.exists() {
            return Err(format!("Directory {} already exists", name));
        }

        fs::create_dir(&project_dir)
            .map_err(|e| format!("Failed to create project directory: {}", e))?;

        // Create src directory
        let src_dir = project_dir.join("src");
        fs::create_dir(&src_dir).map_err(|e| format!("Failed to create src directory: {}", e))?;

        // Create main.knull
        let main_content = r#"// Main entry point for your Knull application

fn main() {
    println("Hello, Knull!")
}
"#;
        fs::write(src_dir.join("main.knull"), main_content)
            .map_err(|e| format!("Failed to create main.knull: {}", e))?;

        // Create manifest
        let manifest = PackageManifest::new(name);
        manifest
            .save(&project_dir.join("knull.toml"))
            .map_err(|e| format!("Failed to create manifest: {}", e))?;

        // Create README
        let readme = format!("# {}\n\nA Knull project.\n", name);
        fs::write(project_dir.join("README.md"), readme)
            .map_err(|e| format!("Failed to create README: {}", e))?;

        // Create .gitignore
        let gitignore = r#"/target
*.o
*.so
*.dylib
*.dll
.DS_Store
knull.lock
"#;
        fs::write(project_dir.join(".gitignore"), gitignore)
            .map_err(|e| format!("Failed to create .gitignore: {}", e))?;

        // Create empty lockfile
        let lockfile = Lockfile::new();
        lockfile
            .save(&project_dir.join(LOCKFILE_NAME))
            .map_err(|e| format!("Failed to create lockfile: {}", e))?;

        Ok(())
    }

    /// Add dependency
    pub fn add_dependency(&mut self, name: &str, version: &str) -> Result<(), String> {
        self.manifest
            .dependencies
            .insert(name.to_string(), version.to_string());
        self.manifest.save(&self.root_path.join("knull.toml"))?;

        // Fetch the dependency immediately
        self.fetch_package(name, version)?;

        // Update lockfile
        self.update_lockfile()?;

        Ok(())
    }

    /// Remove dependency
    pub fn remove_dependency(&mut self, name: &str) -> Result<(), String> {
        if self.manifest.dependencies.remove(name).is_none() {
            return Err(format!("Dependency '{}' not found", name));
        }

        self.manifest.save(&self.root_path.join("knull.toml"))?;

        // Update lockfile to remove package
        if let Some(ref mut lockfile) = self.lockfile {
            lockfile.remove_package(name);
            lockfile.save(&self.root_path.join(LOCKFILE_NAME))?;
        }

        println!("Removed dependency: {}", name);
        Ok(())
    }

    /// Update all dependencies
    pub fn update_all_dependencies(&mut self) -> Result<(), String> {
        println!("Updating all dependencies...");

        let deps: Vec<(String, String)> = self
            .manifest
            .dependencies
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (name, constraint) in deps {
            self.update_package(&name, &constraint)?;
        }

        self.update_lockfile()?;
        println!("All dependencies updated");
        Ok(())
    }

    /// Update a specific package
    pub fn update_package(&mut self, name: &str, constraint: &str) -> Result<(), String> {
        println!("Updating {}...", name);

        // Resolve the latest version matching constraint
        let resolved_version = crate::pkg::http_registry::resolve_version(name, constraint)
            .or_else(|_| {
                // If HTTP fails, try local registry
                crate::pkg::local_registry::list_local_versions(name)?
                    .into_iter()
                    .filter(|v| semver::satisfies(v, constraint))
                    .max_by(|a, b| semver::compare_versions(a, b))
                    .ok_or_else(|| format!("No version of {} satisfies {}", name, constraint))
            })?;

        println!("  Resolved {} to {}", name, resolved_version);

        // Fetch the package
        self.fetch_package(name, &resolved_version)?;

        // Update the constraint in manifest if it was an exact version
        if let Some(existing) = self.manifest.dependencies.get(name) {
            if !existing.starts_with('^')
                && !existing.starts_with('~')
                && !existing.starts_with('>')
                && !existing.starts_with('<')
            {
                // Was exact version, update it
                self.manifest
                    .dependencies
                    .insert(name.to_string(), resolved_version.clone());
                self.manifest.save(&self.root_path.join("knull.toml"))?;
            }
        }

        Ok(())
    }

    /// Fetch dependencies
    pub fn fetch_dependencies(&self) -> Result<(), String> {
        // Create cache directory
        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {}", e))?;

        for (name, version) in &self.manifest.dependencies {
            self.fetch_package(name, version)?;
        }

        Ok(())
    }

    /// Fetch single package - tries local first, then HTTP registry
    pub fn fetch_package(&self, name: &str, version: &str) -> Result<PathBuf, String> {
        // Try local registry first
        if let Ok(path) = crate::pkg::local_registry::fetch_from_local(name, version) {
            println!("  {} {} (from local registry)", name, version);
            return Ok(path);
        }

        // Fall back to HTTP registry
        crate::pkg::http_registry::fetch_from_registry(name, version)
    }

    /// Resolve version with semver constraint
    pub fn resolve_version(&self, name: &str, constraint: &str) -> Result<String, String> {
        crate::pkg::http_registry::resolve_version(name, constraint)
    }

    /// Update lockfile with current dependencies
    pub fn update_lockfile(&mut self) -> Result<(), String> {
        let mut resolved = Vec::new();

        for (name, constraint) in &self.manifest.dependencies {
            // Fetch and resolve the package
            let version = self
                .resolve_version(name, constraint)
                .or_else(|_| Ok::<String, String>(constraint.clone()))?;

            let package_path = self.fetch_package(name, &version)?;

            // Generate checksum
            let checksum = crate::pkg::lockfile::generate_checksum(&package_path).ok();

            // Get transitive dependencies
            let deps = Vec::new(); // Simplified for now

            resolved.push(ResolvedDep {
                name: name.clone(),
                version: version.clone(),
                source: format!("registry+https://registry.knull-lang.dev"),
                checksum,
                dependencies: deps,
            });
        }

        let lockfile = Lockfile::generate(&self.manifest, &resolved);
        lockfile.save(&self.root_path.join(LOCKFILE_NAME))?;

        self.lockfile = Some(lockfile);
        Ok(())
    }

    /// Publish package to local registry
    pub fn publish_local(&self) -> Result<(), String> {
        println!(
            "Publishing {}@{} to local registry...",
            self.manifest.package.name, self.manifest.package.version
        );

        crate::pkg::local_registry::publish_to_local(&self.root_path, &self.manifest)
    }

    /// Publish package to HTTP registry
    pub fn publish_registry(&self, token: &str) -> Result<(), String> {
        println!(
            "Publishing {}@{} to registry...",
            self.manifest.package.name, self.manifest.package.version
        );

        // Validate manifest before publishing
        self.validate_for_publish()?;

        // Create package archive
        let archive_path = self.create_package_archive()?;

        // Upload to registry
        crate::pkg::http_registry::publish_to_registry(&self.manifest, &archive_path, token)?;

        // Clean up archive
        let _ = fs::remove_file(&archive_path);

        Ok(())
    }

    /// Validate manifest before publishing
    fn validate_for_publish(&self) -> Result<(), String> {
        // Check required fields
        if self.manifest.package.name.is_empty() {
            return Err("Package name is required".to_string());
        }

        if self.manifest.package.version.is_empty() {
            return Err("Package version is required".to_string());
        }

        // Validate version format
        if semver::parse_version(&self.manifest.package.version).is_err() {
            return Err(format!(
                "Invalid version format: {}",
                self.manifest.package.version
            ));
        }

        if self.manifest.package.description.is_empty() {
            println!("Warning: No description provided");
        }

        if self.manifest.package.license.is_empty() {
            println!("Warning: No license specified");
        }

        // Check that entry file exists
        let entry_path = self.root_path.join(&self.manifest.package.entry);
        if !entry_path.exists() {
            return Err(format!("Entry file not found: {}", entry_path.display()));
        }

        Ok(())
    }

    /// Create package archive for publishing
    fn create_package_archive(&self) -> Result<PathBuf, String> {
        let temp_dir = std::env::temp_dir().join(format!(
            "knull-publish-{}-{}",
            self.manifest.package.name, self.manifest.package.version
        ));

        fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;

        let archive_path = temp_dir.join("package.tar.gz");

        // Create tarball
        self.create_tarball(&archive_path)?;

        Ok(archive_path)
    }

    fn create_tarball(&self, output_path: &Path) -> Result<(), String> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use tar::Builder;

        let tar_gz = fs::File::create(output_path)
            .map_err(|e| format!("Failed to create archive: {}", e))?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = Builder::new(enc);

        // Add src directory
        let src_dir = self.root_path.join("src");
        if src_dir.exists() {
            tar.append_dir_all("src", &src_dir)
                .map_err(|e| format!("Failed to add src directory: {}", e))?;
        }

        // Add knull.toml (renamed to package.toml for registry)
        let manifest_content = toml::to_string_pretty(&self.manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        let mut header = tar::Header::new_gnu();
        header.set_path("package.toml").map_err(|e| e.to_string())?;
        header.set_size(manifest_content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append(&header, manifest_content.as_bytes())
            .map_err(|e| format!("Failed to add manifest: {}", e))?;

        // Add README if exists
        let readme_path = self.root_path.join("README.md");
        if readme_path.exists() {
            tar.append_path_with_name(&readme_path, "README.md")
                .map_err(|e| format!("Failed to add README: {}", e))?;
        }

        // Add LICENSE if exists
        let license_path = self.root_path.join("LICENSE");
        if license_path.exists() {
            tar.append_path_with_name(&license_path, "LICENSE")
                .map_err(|e| format!("Failed to add LICENSE: {}", e))?;
        }

        tar.finish()
            .map_err(|e| format!("Failed to finish archive: {}", e))?;

        Ok(())
    }

    /// Build project
    pub fn build(&self, release: bool) -> Result<(), String> {
        let entry_path = self.root_path.join(&self.manifest.package.entry);

        if !entry_path.exists() {
            return Err(format!("Entry file not found: {}", entry_path.display()));
        }

        let output_name = &self.manifest.package.name;
        let output_path = if release {
            self.root_path
                .join("target")
                .join("release")
                .join(output_name)
        } else {
            self.root_path
                .join("target")
                .join("debug")
                .join(output_name)
        };

        // Create output directory
        fs::create_dir_all(output_path.parent().unwrap())
            .map_err(|e| format!("Failed to create output directory: {}", e))?;

        // Compile
        let source = fs::read_to_string(&entry_path)
            .map_err(|e| format!("Failed to read entry file: {}", e))?;

        // Use knull compiler
        let status = Command::new("knull")
            .arg("build")
            .arg(&entry_path)
            .arg("-o")
            .arg(&output_path)
            .status()
            .map_err(|e| format!("Failed to execute compiler: {}", e))?;

        if !status.success() {
            return Err("Build failed".to_string());
        }

        println!("Built: {}", output_path.display());
        Ok(())
    }

    /// Run project
    pub fn run(&self, args: &[String]) -> Result<(), String> {
        let entry_path = self.root_path.join(&self.manifest.package.entry);

        let mut cmd = Command::new("knull");
        cmd.arg("run").arg(&entry_path);

        for arg in args {
            cmd.arg(arg);
        }

        let status = cmd
            .status()
            .map_err(|e| format!("Failed to execute: {}", e))?;

        if !status.success() {
            return Err("Program exited with error".to_string());
        }

        Ok(())
    }

    /// Run tests
    pub fn test(&self) -> Result<(), String> {
        let test_dir = self.root_path.join("tests");

        if !test_dir.exists() {
            println!("No tests directory found");
            return Ok(());
        }

        let mut passed = 0;
        let mut failed = 0;

        for entry in fs::read_dir(&test_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "knull") {
                print!(
                    "Running {}... ",
                    path.file_name().unwrap().to_string_lossy()
                );

                let status = Command::new("knull").arg("run").arg(&path).status();

                match status {
                    Ok(s) if s.success() => {
                        println!("PASS");
                        passed += 1;
                    }
                    _ => {
                        println!("FAIL");
                        failed += 1;
                    }
                }
            }
        }

        println!("\nTest results: {} passed, {} failed", passed, failed);

        if failed > 0 {
            return Err("Some tests failed".to_string());
        }

        Ok(())
    }

    /// Get manifest reference
    pub fn manifest(&self) -> &PackageManifest {
        &self.manifest
    }

    /// Get mutable manifest reference
    pub fn manifest_mut(&mut self) -> &mut PackageManifest {
        &mut self.manifest
    }
}

/// Find nearest manifest from current directory
pub fn find_nearest_manifest() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;

    loop {
        let manifest = current.join("knull.toml");
        if manifest.exists() {
            return Some(manifest);
        }

        if !current.pop() {
            break;
        }
    }

    None
}
