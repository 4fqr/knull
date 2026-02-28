//! Lockfile Module
//! Handles lock file generation and parsing for reproducible builds

use crate::pkg::manager::PackageManifest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const LOCKFILE_VERSION: i32 = 1;
pub const LOCKFILE_NAME: &str = "knull.lock";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Lockfile {
    pub version: i32,
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub source: String, // "registry+https://..." or "path+/absolute/path"
    pub checksum: Option<String>,
    pub dependencies: Option<Vec<String>>, // Names of dependencies
}

#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub name: String,
    pub version: String,
    pub source: String,
    pub checksum: Option<String>,
    pub dependencies: Vec<String>,
}

impl Lockfile {
    pub fn new() -> Self {
        Lockfile {
            version: LOCKFILE_VERSION,
            packages: Vec::new(),
        }
    }

    pub fn generate(manifest: &PackageManifest, resolved: &[ResolvedDep]) -> Self {
        let packages: Vec<LockedPackage> = resolved
            .iter()
            .map(|dep| LockedPackage {
                name: dep.name.clone(),
                version: dep.version.clone(),
                source: dep.source.clone(),
                checksum: dep.checksum.clone(),
                dependencies: if dep.dependencies.is_empty() {
                    None
                } else {
                    Some(dep.dependencies.clone())
                },
            })
            .collect();

        Lockfile {
            version: LOCKFILE_VERSION,
            packages,
        }
    }

    pub fn parse(path: &Path) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read lockfile: {}", e))?;

        let lockfile: Lockfile =
            toml::from_str(&content).map_err(|e| format!("Failed to parse lockfile: {}", e))?;

        if lockfile.version != LOCKFILE_VERSION {
            return Err(format!(
                "Lockfile version mismatch. Expected {}, got {}",
                LOCKFILE_VERSION, lockfile.version
            ));
        }

        Ok(lockfile)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize lockfile: {}", e))?;
        fs::write(path, content).map_err(|e| format!("Failed to write lockfile: {}", e))
    }

    pub fn save_to_project(&self, project_root: &Path) -> Result<(), String> {
        let lockfile_path = project_root.join(LOCKFILE_NAME);
        self.save(&lockfile_path)
    }

    pub fn load_from_project(project_root: &Path) -> Result<Self, String> {
        let lockfile_path = project_root.join(LOCKFILE_NAME);
        Self::parse(&lockfile_path)
    }

    pub fn get_package(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }

    pub fn add_package(&mut self, package: LockedPackage) {
        // Remove existing entry if present
        self.packages.retain(|p| p.name != package.name);
        self.packages.push(package);
    }

    pub fn remove_package(&mut self, name: &str) {
        self.packages.retain(|p| p.name != name);
    }

    pub fn contains(&self, name: &str) -> bool {
        self.packages.iter().any(|p| p.name == name)
    }

    pub fn get_locked_version(&self, name: &str) -> Option<&str> {
        self.get_package(name).map(|p| p.version.as_str())
    }

    pub fn verify_checksum(&self, name: &str, actual_checksum: &str) -> Result<(), String> {
        match self.get_package(name) {
            Some(pkg) => {
                match &pkg.checksum {
                    Some(expected) if expected == actual_checksum => Ok(()),
                    Some(expected) => Err(format!(
                        "Checksum mismatch for {}: expected {}, got {}",
                        name, expected, actual_checksum
                    )),
                    None => Ok(()), // No checksum to verify against
                }
            }
            None => Err(format!("Package {} not found in lockfile", name)),
        }
    }

    /// Update lockfile with new resolved dependencies
    pub fn update(&mut self, resolved: &[ResolvedDep]) {
        let new_packages: Vec<LockedPackage> = resolved
            .iter()
            .map(|dep| LockedPackage {
                name: dep.name.clone(),
                version: dep.version.clone(),
                source: dep.source.clone(),
                checksum: dep.checksum.clone(),
                dependencies: if dep.dependencies.is_empty() {
                    None
                } else {
                    Some(dep.dependencies.clone())
                },
            })
            .collect();

        self.packages = new_packages;
    }

    /// Get all dependencies for a package (transitive)
    pub fn get_all_dependencies(&self, name: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();

        fn visit<'a>(
            lockfile: &'a Lockfile,
            name: &'a str,
            result: &mut Vec<String>,
            visited: &mut std::collections::HashSet<&'a str>,
        ) {
            if visited.contains(name) {
                return;
            }
            visited.insert(name);

            if let Some(pkg) = lockfile.get_package(name) {
                if let Some(deps) = &pkg.dependencies {
                    for dep in deps {
                        if !visited.contains(dep.as_str()) {
                            result.push(dep.clone());
                            visit(lockfile, dep, result, visited);
                        }
                    }
                }
            }
        }

        visit(self, name, &mut result, &mut visited);
        result
    }
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a checksum for a package directory
pub fn generate_checksum(package_path: &Path) -> Result<String, String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    fn hash_dir(path: &Path, hasher: &mut DefaultHasher) -> std::io::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                hash_dir(&path, hasher)?;
            } else {
                // Hash file contents
                if let Ok(content) = fs::read(&path) {
                    content.hash(hasher);
                }
            }
        }
        Ok(())
    }

    hash_dir(package_path, &mut hasher).map_err(|e| format!("Failed to hash directory: {}", e))?;

    Ok(format!("{:x}", hasher.finish()))
}

/// Build a dependency graph from the lockfile
pub fn build_dependency_graph(lockfile: &Lockfile) -> HashMap<String, Vec<String>> {
    let mut graph = HashMap::new();

    for pkg in &lockfile.packages {
        let deps = pkg.dependencies.clone().unwrap_or_default();
        graph.insert(pkg.name.clone(), deps);
    }

    graph
}

/// Check for circular dependencies
pub fn detect_cycles(graph: &HashMap<String, Vec<String>>) -> Option<Vec<String>> {
    let mut visited = std::collections::HashSet::new();
    let mut rec_stack = std::collections::HashSet::new();
    let mut path = Vec::new();

    fn dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) = dfs(neighbor, graph, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|n| n == neighbor).unwrap();
                    let cycle: Vec<String> = path[cycle_start..].to_vec();
                    return Some(cycle);
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
        None
    }

    for node in graph.keys() {
        if !visited.contains(node) {
            if let Some(cycle) = dfs(node, graph, &mut visited, &mut rec_stack, &mut path) {
                return Some(cycle);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_lockfile_parse_save() {
        let temp_dir = TempDir::new().unwrap();
        let lockfile_path = temp_dir.path().join("test.lock");

        let lockfile = Lockfile {
            version: 1,
            packages: vec![LockedPackage {
                name: "test-pkg".to_string(),
                version: "1.0.0".to_string(),
                source: "registry+https://registry.knull-lang.dev".to_string(),
                checksum: Some("abc123".to_string()),
                dependencies: Some(vec!["dep1".to_string()]),
            }],
        };

        lockfile.save(&lockfile_path).unwrap();
        let loaded = Lockfile::parse(&lockfile_path).unwrap();

        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.packages.len(), 1);
        assert_eq!(loaded.packages[0].name, "test-pkg");
    }

    #[test]
    fn test_get_package() {
        let lockfile = Lockfile {
            version: 1,
            packages: vec![LockedPackage {
                name: "pkg1".to_string(),
                version: "1.0.0".to_string(),
                source: "registry+https://example.com".to_string(),
                checksum: None,
                dependencies: None,
            }],
        };

        assert!(lockfile.get_package("pkg1").is_some());
        assert!(lockfile.get_package("pkg2").is_none());
    }

    #[test]
    fn test_detect_cycles() {
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec!["b".to_string()]);
        graph.insert("b".to_string(), vec!["c".to_string()]);
        graph.insert("c".to_string(), vec!["a".to_string()]);

        let cycle = detect_cycles(&graph);
        assert!(cycle.is_some());

        // Acyclic graph
        let mut graph2 = HashMap::new();
        graph2.insert("a".to_string(), vec!["b".to_string()]);
        graph2.insert("b".to_string(), vec!["c".to_string()]);

        assert!(detect_cycles(&graph2).is_none());
    }
}
