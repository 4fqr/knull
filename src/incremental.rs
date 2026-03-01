//! Knull Incremental Compilation Module
//!
//! Provides caching and incremental compilation support for faster builds.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const CACHE_DIR: &str = ".knull/cache";
const MANIFEST_FILE: &str = ".knull/cache/manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCache {
    pub path: PathBuf,
    pub hash: String,
    pub timestamp: u64,
    pub ast_serialized: Vec<u8>,
    pub dependencies: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompilationManifest {
    pub files: HashMap<String, FileCacheEntry>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCacheEntry {
    pub path: String,
    pub hash: String,
    pub timestamp: u64,
    pub dependencies: Vec<String>,
    pub compiled_at: u64,
}

pub struct IncrementalCompiler {
    cache_dir: PathBuf,
    manifest: CompilationManifest,
    force_rebuild: bool,
}

impl IncrementalCompiler {
    pub fn new() -> Self {
        let cache_dir = PathBuf::from(CACHE_DIR);
        let manifest = Self::load_manifest(&cache_dir);

        IncrementalCompiler {
            cache_dir,
            manifest,
            force_rebuild: false,
        }
    }

    pub fn with_force_rebuild(force: bool) -> Self {
        let mut compiler = Self::new();
        compiler.force_rebuild = force;
        compiler
    }

    fn load_manifest(cache_dir: &Path) -> CompilationManifest {
        let manifest_path = cache_dir.join("manifest.json");
        if manifest_path.exists() {
            if let Ok(mut file) = File::open(&manifest_path) {
                let mut contents = String::new();
                if file.read_to_string(&mut contents).is_ok() {
                    if let Ok(manifest) = serde_json::from_str(&contents) {
                        return manifest;
                    }
                }
            }
        }
        CompilationManifest::default()
    }

    fn save_manifest(&self) -> Result<(), String> {
        if !self.cache_dir.exists() {
            fs::create_dir_all(&self.cache_dir)
                .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        }

        let manifest_path = self.cache_dir.join("manifest.json");
        let json = serde_json::to_string_pretty(&self.manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

        let mut file = File::create(&manifest_path)
            .map_err(|e| format!("Failed to create manifest file: {}", e))?;

        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write manifest: {}", e))?;

        Ok(())
    }

    pub fn needs_compilation(&self, file_path: &Path) -> bool {
        if self.force_rebuild {
            return true;
        }

        if !file_path.exists() {
            return true;
        }

        let file_key = file_path.to_string_lossy().to_string();

        if !self.manifest.files.contains_key(&file_key) {
            return true;
        }

        let entry = &self.manifest.files[&file_key];

        let current_hash = Self::compute_file_hash(file_path);
        if current_hash != entry.hash {
            return true;
        }

        for dep in &entry.dependencies {
            let dep_path = Path::new(dep);
            if dep_path.exists() {
                if let Ok(dep_meta) = dep_path.metadata() {
                    if let Ok(dep_modified) = dep_meta.modified() {
                        let dep_timestamp = dep_modified
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);

                        if dep_timestamp > entry.compiled_at {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    pub fn mark_compiled(
        &mut self,
        file_path: &Path,
        dependencies: &[PathBuf],
    ) -> Result<(), String> {
        let file_key = file_path.to_string_lossy().to_string();

        let hash = Self::compute_file_hash(file_path);
        let timestamp = Self::get_file_timestamp(file_path);

        let dep_strings: Vec<String> = dependencies
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        self.manifest.files.insert(
            file_key,
            FileCacheEntry {
                path: file_path.to_string_lossy().to_string(),
                hash,
                timestamp,
                dependencies: dep_strings,
                compiled_at: Self::current_timestamp(),
            },
        );

        self.save_manifest()
    }

    pub fn invalidate(&mut self, file_path: &Path) {
        let file_key = file_path.to_string_lossy().to_string();
        self.manifest.files.remove(&file_key);

        let files_to_remove: Vec<String> = self
            .manifest
            .files
            .iter()
            .filter(|(_, entry)| entry.dependencies.contains(&file_key))
            .map(|(k, _)| k.clone())
            .collect();

        for f in files_to_remove {
            self.manifest.files.remove(&f);
        }
    }

    pub fn clean_cache(&mut self) -> Result<(), String> {
        self.manifest.files.clear();

        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .map_err(|e| format!("Failed to remove cache: {}", e))?;
        }

        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| format!("Failed to recreate cache directory: {}", e))?;

        self.save_manifest()
    }

    pub fn get_cache_stats(&self) -> CacheStats {
        let total_files = self.manifest.files.len();
        let mut total_size = 0u64;

        for (_, entry) in &self.manifest.files {
            total_size += entry.hash.len() as u64;
            total_size += entry.timestamp.to_le_bytes().len() as u64;
        }

        CacheStats {
            cached_files: total_files,
            total_size_bytes: total_size,
            cache_dir: self.cache_dir.to_string_lossy().to_string(),
        }
    }

    fn compute_file_hash(file_path: &Path) -> String {
        if let Ok(mut file) = File::open(file_path) {
            let mut contents = Vec::new();
            if file.read_to_end(&mut contents).is_ok() {
                return Self::compute_simple_hash(&contents);
            }
        }
        String::new()
    }

    fn compute_simple_hash(data: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn get_file_timestamp(file_path: &Path) -> u64 {
        if let Ok(meta) = file_path.metadata() {
            if let Ok(modified) = meta.modified() {
                return modified
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
            }
        }
        0
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn extract_dependencies(source: &str) -> Vec<PathBuf> {
        let mut deps = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("use ") {
                if let Some(module) = Self::extract_module_path(trimmed) {
                    deps.push(module);
                }
            } else if trimmed.starts_with("mod ") {
                if let Some(module) = Self::extract_module_path(trimmed) {
                    deps.push(module);
                }
            } else if trimmed.starts_with("include ") || trimmed.starts_with("import ") {
                if let Some(path) = Self::extract_import_path(trimmed) {
                    deps.push(path);
                }
            }
        }

        deps
    }

    fn extract_module_path(line: &str) -> Option<PathBuf> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let module = parts[1].trim_end_matches(';');
            Some(PathBuf::from(format!("{}.knull", module)))
        } else {
            None
        }
    }

    fn extract_import_path(line: &str) -> Option<PathBuf> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let path = parts[1].trim_end_matches(';').trim_matches('"');
            Some(PathBuf::from(path))
        } else {
            None
        }
    }
}

impl Default for IncrementalCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub cached_files: usize,
    pub total_size_bytes: u64,
    pub cache_dir: String,
}

pub struct CompilationUnit {
    pub source_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub dependencies: Vec<PathBuf>,
    pub needs_compile: bool,
}

impl CompilationUnit {
    pub fn new(source_path: PathBuf) -> Self {
        let mut unit = CompilationUnit {
            source_path: source_path.clone(),
            output_path: None,
            dependencies: Vec::new(),
            needs_compile: true,
        };

        if let Ok(contents) = fs::read_to_string(&source_path) {
            unit.dependencies = IncrementalCompiler::extract_dependencies(&contents);
        }

        unit
    }

    pub fn with_output(mut self, output: PathBuf) -> Self {
        self.output_path = Some(output);
        self
    }
}

pub struct BuildPlan {
    pub units: Vec<CompilationUnit>,
    pub parallel: bool,
    pub max_jobs: usize,
}

impl BuildPlan {
    pub fn from_sources(sources: Vec<PathBuf>) -> Self {
        let units: Vec<CompilationUnit> = sources.into_iter().map(CompilationUnit::new).collect();

        BuildPlan {
            units,
            parallel: true,
            max_jobs: std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(1),
        }
    }

    pub fn create_order(&self) -> Vec<usize> {
        let mut order = Vec::new();
        let mut processed = vec![false; self.units.len()];

        fn visit(
            units: &[CompilationUnit],
            idx: usize,
            processed: &mut Vec<bool>,
            order: &mut Vec<usize>,
        ) {
            if processed[idx] {
                return;
            }
            processed[idx] = true;

            for dep in &units[idx].dependencies {
                if let Some(dep_idx) = units.iter().position(|u| &u.source_path == dep) {
                    visit(units, dep_idx, processed, order);
                }
            }

            order.push(idx);
        }

        for i in 0..self.units.len() {
            visit(&self.units, i, &mut processed, &mut order);
        }

        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_extraction() {
        let source = r#"
            mod foo;
            use bar;
            include "test.knull"
        "#;

        let deps = IncrementalCompiler::extract_dependencies(source);
        assert!(deps.len() >= 2);
    }

    #[test]
    fn test_build_order() {
        let sources = vec![
            PathBuf::from("main.knull"),
            PathBuf::from("foo.knull"),
            PathBuf::from("bar.knull"),
        ];

        let plan = BuildPlan::from_sources(sources);
        let order = plan.create_order();

        assert_eq!(order.len(), 3);
    }
}
