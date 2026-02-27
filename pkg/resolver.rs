// =============================================================================
// KNULL PACKAGE MANAGER: RESOLVER
// =============================================================================
// Package resolution, fetching, and dependency management

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

// =============================================================================
// ERROR TYPES
// =============================================================================

#[derive(Debug)]
pub enum ResolveError {
    NotFound(String),
    VersionConflict(String),
    GitError(String),
    IoError(std::io::Error),
    InvalidManifest(String),
}

impl From<std::io::Error> for ResolveError {
    fn from(e: std::io::Error) -> Self {
        ResolveError::IoError(e)
    }
}

pub type ResolveResult<T> = Result<T, ResolveError>;

// =============================================================================
// MANIFEST
// =============================================================================

#[derive(Debug, Clone)]
pub struct Manifest {
    pub package: PackageInfo,
    pub dependencies: HashMap<String, Dependency>,
    pub dev_dependencies: HashMap<String, Dependency>,
    pub features: HashMap<String, Vec<String>>,
    pub workspace: Option<WorkspaceInfo>,
}

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub entry: String,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub readme: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub source: DependencySource,
    pub features: Vec<String>,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub enum DependencySource {
    Registry(String), // Registry name
    Git {
        url: String,
        branch: Option<String>,
        rev: Option<String>,
    },
    Path(String), // Local path
}

#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub members: Vec<String>,
    pub dependencies: HashMap<String, String>,
}

// =============================================================================
// PACKAGE CACHE
// =============================================================================

pub struct PackageCache {
    pub root: PathBuf,
    pub packages: HashMap<String, PackageMeta>,
}

pub struct PackageMeta {
    pub name: String,
    pub version: String,
    pub source: PathBuf,
    pub manifest: Manifest,
}

impl PackageCache {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            packages: HashMap::new(),
        }
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join(".knull").join("cache")
    }

    pub fn package_dir(&self, name: &str, version: &str) -> PathBuf {
        self.cache_dir().join(name).join(version)
    }
}

// =============================================================================
// RESOLVER
// =============================================================================

pub struct Resolver {
    cache: PackageCache,
    registry_url: String,
}

impl Resolver {
    pub fn new(cache_root: PathBuf) -> Self {
        Self {
            cache: PackageCache::new(cache_root),
            registry_url: "https://registry.knull-lang.dev".to_string(),
        }
    }

    // Resolve all dependencies for a manifest
    pub fn resolve(&mut self, manifest: &Manifest) -> ResolveResult<HashMap<String, PackageMeta>> {
        let mut resolved = HashMap::new();
        let mut to_resolve: Vec<(String, Dependency)> = Vec::new();

        // Add all dependencies
        for (name, dep) in &manifest.dependencies {
            to_resolve.push((name.clone(), dep.clone()));
        }

        // Resolve each dependency
        while let Some((name, dep)) = to_resolve.pop() {
            if resolved.contains_key(&name) {
                continue;
            }

            let package = self.fetch_package(&dep)?;
            resolved.insert(name, package.clone());

            // Add transitive dependencies
            for (trans_name, trans_dep) in &package.manifest.dependencies {
                if !resolved.contains_key(trans_name) {
                    to_resolve.push((trans_name.clone(), trans_dep.clone()));
                }
            }
        }

        Ok(resolved)
    }

    // Fetch a package from its source
    fn fetch_package(&mut self, dep: &Dependency) -> ResolveResult<PackageMeta> {
        match &dep.source {
            DependencySource::Registry(name) => self.fetch_registry(dep, name),
            DependencySource::Git { url, branch, rev } => {
                self.fetch_git(dep, url, branch.as_deref(), rev.as_deref())
            }
            DependencySource::Path(path) => self.fetch_path(dep, path),
        }
    }

    // Fetch from registry
    fn fetch_registry(&self, dep: &Dependency, _registry: &str) -> ResolveResult<PackageMeta> {
        let cache_dir = self.cache.package_dir(&dep.name, &dep.version);

        // Check if already cached
        if cache_dir.exists() {
            return self.load_cached_package(&cache_dir, &dep.name, &dep.version);
        }

        // In a real implementation, we would:
        // 1. Query the registry API for available versions
        // 2. Select the best matching version
        // 3. Download the package tarball
        // 4. Extract to cache directory

        // For now, create a placeholder
        std::fs::create_dir_all(&cache_dir)?;

        // Write a stub manifest
        let stub_manifest = format!(
            r#"[package]
name = "{}"
version = "{}"
entry = "lib.knull"
"#,
            dep.name, dep.version
        );
        std::fs::write(cache_dir.join("knull.toml"), stub_manifest)?;

        self.load_cached_package(&cache_dir, &dep.name, &dep.version)
    }

    // Fetch from git
    fn fetch_git(
        &self,
        dep: &Dependency,
        url: &str,
        branch: Option<&str>,
        _rev: Option<&str>,
    ) -> ResolveResult<PackageMeta> {
        let cache_dir = self
            .cache
            .cache_dir()
            .join("git")
            .join(sanitize_for_path(url));

        // Clone if not cached
        if !cache_dir.exists() {
            // Use git to clone
            let mut cmd = Command::new("git");
            cmd.arg("clone").arg("--depth").arg("1");

            if let Some(branch) = branch {
                cmd.arg("-b").arg(branch);
            }

            cmd.arg(url).arg(&cache_dir);

            let output = cmd
                .output()
                .map_err(|e| ResolveError::GitError(e.to_string()))?;

            if !output.status.success() {
                return Err(ResolveError::GitError(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }
        }

        // Load manifest
        self.load_package_from_dir(&cache_dir, &dep.name, &dep.version)
    }

    // Fetch from local path
    fn fetch_path(&self, dep: &Dependency, path: &str) -> ResolveResult<PackageMeta> {
        let dir = Path::new(path);

        if !dir.exists() {
            return Err(ResolveError::NotFound(format!("Path not found: {}", path)));
        }

        self.load_package_from_dir(dir, &dep.name, &dep.version)
    }

    // Load a cached package
    fn load_cached_package(
        &self,
        dir: &Path,
        name: &str,
        version: &str,
    ) -> ResolveResult<PackageMeta> {
        self.load_package_from_dir(dir, name, version)
    }

    // Load package from directory
    fn load_package_from_dir(
        &self,
        dir: &Path,
        name: &str,
        version: &str,
    ) -> ResolveResult<PackageMeta> {
        let manifest_path = dir.join("knull.toml");

        if !manifest_path.exists() {
            return Err(ResolveError::NotFound(format!(
                "No manifest found in {:?}",
                dir
            )));
        }

        let manifest = parse_manifest(&std::fs::read_to_string(&manifest_path)?)?;

        Ok(PackageMeta {
            name: name.to_string(),
            version: version.to_string(),
            source: dir.to_path_buf(),
            manifest,
        })
    }
}

// =============================================================================
// MANIFEST PARSING
// =============================================================================

pub fn parse_manifest(content: &str) -> ResolveResult<Manifest> {
    let mut package = None;
    let mut dependencies = HashMap::new();
    let mut dev_dependencies = HashMap::new();
    let mut features = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') && line.ends_with(']') {
            // Section header - handle later
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');

            if key == "name" {
                package = Some(PackageInfo {
                    name: value.to_string(),
                    version: String::new(),
                    edition: "2024".to_string(),
                    entry: "lib.knull".to_string(),
                    authors: Vec::new(),
                    license: None,
                    description: None,
                    repository: None,
                    readme: None,
                });
            } else if key == "version" {
                if let Some(ref mut pkg) = package {
                    pkg.version = value.to_string();
                }
            }
        }
    }

    let package = package
        .ok_or_else(|| ResolveError::InvalidManifest("Missing [package] section".to_string()))?;

    Ok(Manifest {
        package,
        dependencies,
        dev_dependencies,
        features,
        workspace: None,
    })
}

// =============================================================================
// UTILITIES
// =============================================================================

fn sanitize_for_path(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

// =============================================================================
// PACKAGE INSTALLER
// =============================================================================

pub struct Installer {
    cache: PackageCache,
}

impl Installer {
    pub fn new(cache_root: PathBuf) -> Self {
        Self {
            cache: PackageCache::new(cache_root),
        }
    }

    // Install a package to the local project
    pub fn install(&self, package: &PackageMeta, target_dir: &Path) -> ResolveResult<()> {
        let dest = target_dir.join("deps").join(&package.name);

        std::fs::create_dir_all(&dest)?;

        // Copy package files
        copy_dir(&package.source, &dest)?;

        Ok(())
    }
}

fn copy_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    if !dest.exists() {
        std::fs::create_dir_all(dest)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dest.join(entry.file_name());

        if ty.is_dir() {
            copy_dir(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }

    Ok(())
}

// =============================================================================
// CLI COMMANDS
// =============================================================================

pub fn cmd_install(name: &str, version: Option<&str>) -> ResolveResult<()> {
    let cache_root = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".knull"))
        .join("knull");

    let mut resolver = Resolver::new(cache_root);

    let dep = Dependency {
        name: name.to_string(),
        version: version.unwrap_or("*").to_string(),
        source: DependencySource::Registry("crates.io".to_string()),
        features: Vec::new(),
        optional: false,
    };

    let manifest = Manifest {
        package: PackageInfo {
            name: "install".to_string(),
            version: "0.0.0".to_string(),
            entry: "".to_string(),
            edition: "2024".to_string(),
            authors: Vec::new(),
            license: None,
            description: None,
            repository: None,
            readme: None,
        },
        dependencies: HashMap::from([(name.to_string(), dep)]),
        dev_dependencies: HashMap::new(),
        features: HashMap::new(),
        workspace: None,
    };

    let resolved = resolver.resolve(&manifest)?;

    println!("Installed {} packages:", resolved.len());
    for (name, pkg) in &resolved {
        println!("  {} v{}", name, pkg.version);
    }

    Ok(())
}

pub fn cmd_remove(name: &str) -> ResolveResult<()> {
    let cache_root = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".knull"))
        .join("knull");

    let package_dir = cache_root.join("cache").join(name);

    if package_dir.exists() {
        std::fs::remove_dir_all(package_dir)?;
        println!("Removed {}", name);
    } else {
        println!("Package {} not found", name);
    }

    Ok(())
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest() {
        let content = r#"
[package]
name = "test-package"
version = "1.0.0"
entry = "src/main.knull"
"#;

        let manifest = parse_manifest(content).unwrap();
        assert_eq!(manifest.package.name, "test-package");
        assert_eq!(manifest.package.version, "1.0.0");
    }

    #[test]
    fn test_sanitize_path() {
        assert_eq!(
            sanitize_for_path("https://github.com/user/repo"),
            "httpsgithubcomuserrepo"
        );
    }
}
