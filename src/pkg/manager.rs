//! Knull Package Manager
//!
//! Handles package dependencies, building, and distribution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
            },
        }
    }
}

/// Package manager
pub struct PackageManager {
    root_path: PathBuf,
    manifest: PackageManifest,
    cache_dir: PathBuf,
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

        Ok(PackageManager {
            root_path,
            manifest,
            cache_dir,
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
"#;
        fs::write(project_dir.join(".gitignore"), gitignore)
            .map_err(|e| format!("Failed to create .gitignore: {}", e))?;

        Ok(())
    }

    /// Add dependency
    pub fn add_dependency(&mut self, name: &str, version: &str) -> Result<(), String> {
        self.manifest
            .dependencies
            .insert(name.to_string(), version.to_string());
        self.manifest.save(&self.root_path.join("knull.toml"))
    }

    /// Remove dependency
    pub fn remove_dependency(&mut self, name: &str) -> Result<(), String> {
        self.manifest.dependencies.remove(name);
        self.manifest.save(&self.root_path.join("knull.toml"))
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

    /// Fetch single package
    fn fetch_package(&self, name: &str, version: &str) -> Result<(), String> {
        let package_dir = self.cache_dir.join(format!("{}-{}", name, version));

        if package_dir.exists() {
            println!("  {} {} (cached)", name, version);
            return Ok(());
        }

        println!("  Fetching {} {}...", name, version);

        // For now, packages would be fetched from a registry
        // In a full implementation, this would clone from git or download from a registry
        // For demonstration, we'll create a placeholder

        fs::create_dir_all(&package_dir)
            .map_err(|e| format!("Failed to create package directory: {}", e))?;

        // Placeholder: In real implementation, fetch from registry
        // For now, just create a stub
        let stub = format!("// Package: {}@{}", name, version);
        fs::write(package_dir.join("lib.knull"), stub)
            .map_err(|e| format!("Failed to write package stub: {}", e))?;

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
}
