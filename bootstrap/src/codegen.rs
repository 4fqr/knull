//! Knull Code Generator - JIT Execution and Linking
//!
//! This module handles JIT compilation and executable linking.

use anyhow::Result;
use std::process::Command;

use crate::compiler::CompiledModule;

/// JIT Executor
pub struct JitExecutor;

impl JitExecutor {
    pub fn new() -> Self {
        JitExecutor
    }

    /// Execute a compiled module
    pub fn execute(&self, module: &CompiledModule, _args: Vec<String>) -> Result<()> {
        // For now, we'll just print the IR
        // A full implementation would use JIT compilation
        println!("{}", module.ir.llvm_ir());
        Ok(())
    }
}

/// Linker
pub struct Linker {
    target_triple: String,
}

impl Linker {
    pub fn new() -> Self {
        Linker {
            target_triple: std::env::consts::ARCH.to_string(),
        }
    }

    pub fn with_target(mut self, target: &str) -> Self {
        self.target_triple = target.to_string();
        self
    }

    /// Link an object file into an executable
    pub fn link(&self, module: &CompiledModule, output: &std::path::Path) -> Result<()> {
        // Generate LLVM IR
        let ir = module.ir.llvm_ir();

        // Write IR to temporary file
        let ir_path = std::env::temp_dir().join("knull_temp.ll");
        std::fs::write(&ir_path, &ir)?;

        // Try to use LLVM tools
        // First try clang, then llc
        let result = self
            .try_link_with_clang(&ir_path, output)
            .or_else(|_| self.try_link_with_llc(&ir_path, output));

        // Clean up temp file
        let _ = std::fs::remove_file(&ir_path);

        result
    }

    fn try_link_with_clang(
        &self,
        ir_path: &std::path::Path,
        output: &std::path::Path,
    ) -> Result<()> {
        let output_str = output.to_string_lossy().to_string();

        // Try clang first
        let result = Command::new("clang")
            .args([
                "-x",
                "ir",
                ir_path.to_str().unwrap(),
                "-o",
                &output_str,
                "-O2",
            ])
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "clang failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("clang not found: {}", e)),
        }
    }

    fn try_link_with_llc(&self, ir_path: &std::path::Path, output: &std::path::Path) -> Result<()> {
        // Try llc + gcc
        let obj_path = std::env::temp_dir().join("knull_temp.o");

        // Compile to object file
        let result = Command::new("llc")
            .args([
                "--filetype=obj",
                ir_path.to_str().unwrap(),
                "-o",
                obj_path.to_str().unwrap(),
            ])
            .output();

        match result {
            Ok(output) => {
                if !output.status.success() {
                    return Err(anyhow::anyhow!(
                        "llc failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("llc not found: {}", e));
            }
        }

        // Link with gcc
        let output_str = output.to_string_lossy().to_string();
        let obj_str = obj_path.to_string_lossy().to_string();

        let result = Command::new("gcc")
            .args([&obj_str, "-o", &output_str])
            .output();

        // Clean up
        let _ = std::fs::remove_file(&obj_path);

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "gcc link failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("gcc not found: {}", e)),
        }
    }
}

/// Standalone object file generator
pub struct ObjectGenerator {
    target_triple: String,
}

impl ObjectGenerator {
    pub fn new() -> Self {
        ObjectGenerator {
            target_triple: std::env::consts::ARCH.to_string(),
        }
    }

    /// Generate an object file
    pub fn generate_object(&self, module: &CompiledModule, output: &std::path::Path) -> Result<()> {
        let ir = module.ir.llvm_ir();

        // Write IR to temporary file
        let ir_path = std::env::temp_dir().join("knull_temp.ll");
        std::fs::write(&ir_path, &ir)?;

        // Compile to object file using llc
        let result = Command::new("llc")
            .args([
                "--filetype=obj",
                ir_path.to_str().unwrap(),
                "-o",
                output.to_str().unwrap(),
            ])
            .output();

        // Clean up
        let _ = std::fs::remove_file(&ir_path);

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "llc failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("llc not found: {}", e)),
        }
    }
}

/// Static library generator
pub struct StaticLibraryGenerator;

impl StaticLibraryGenerator {
    pub fn new() -> Self {
        StaticLibraryGenerator
    }

    /// Generate a static library
    pub fn generate_archive(
        &self,
        objects: &[std::path::Path],
        output: &std::path::Path,
    ) -> Result<()> {
        let mut args = vec!["rcs".to_string(), output.to_string_lossy().to_string()];

        for obj in objects {
            args.push(obj.to_string_lossy().to_string());
        }

        let result = Command::new("ar").args(&args).output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "ar failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("ar not found: {}", e)),
        }
    }
}

/// Shared library generator
pub struct SharedLibraryGenerator;

impl SharedLibraryGenerator {
    pub fn new() -> Self {
        SharedLibraryGenerator
    }

    /// Generate a shared library (DLL or .so)
    pub fn generate_dll(&self, module: &CompiledModule, output: &std::path::Path) -> Result<()> {
        let ir = module.ir.llvm_ir();

        // Write IR to temporary file
        let ir_path = std::env::temp_dir().join("knull_temp.ll");
        std::fs::write(&ir_path, &ir)?;

        let output_str = output.to_string_lossy().to_string();

        // Compile to shared library
        let result = Command::new("clang")
            .args([
                "-x",
                "ir",
                "-shared",
                "-fPIC",
                ir_path.to_str().unwrap(),
                "-o",
                &output_str,
                "-O2",
            ])
            .output();

        // Clean up
        let _ = std::fs::remove_file(&ir_path);

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "clang failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("clang not found: {}", e)),
        }
    }
}
