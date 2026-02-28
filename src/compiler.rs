//! Knull Compiler
//!
//! Provides compilation interfaces for different backends.

#[cfg(feature = "llvm-backend")]
pub mod llvm_codegen;

#[cfg(feature = "llvm-backend")]
use inkwell::OptimizationLevel;

use std::path::Path;

/// Compilation mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompileMode {
    /// Novice mode: Dynamic typing with garbage collection
    Novice,
    /// Expert mode: Static typing with ownership system
    Expert,
    /// God mode: Unsafe blocks, direct memory access, inline assembly
    God,
}

/// Compilation options
#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub mode: CompileMode,
    #[cfg(feature = "llvm-backend")]
    pub opt_level: OptimizationLevel,
    #[cfg(not(feature = "llvm-backend"))]
    pub opt_level: u32,
    pub output_ir: bool,
    pub output_asm: bool,
    pub target_triple: Option<String>,
}

#[cfg(not(feature = "llvm-backend"))]
impl CompileOptions {
    pub fn default() -> Self {
        CompileOptions {
            mode: CompileMode::Novice,
            opt_level: 2,
            output_ir: false,
            output_asm: false,
            target_triple: None,
        }
    }
}

#[cfg(feature = "llvm-backend")]
impl Default for CompileOptions {
    fn default() -> Self {
        CompileOptions {
            mode: CompileMode::Novice,
            opt_level: OptimizationLevel::Default,
            output_ir: false,
            output_asm: false,
            target_triple: None,
        }
    }
}

/// Compilation result
#[derive(Debug)]
pub struct CompilationResult {
    pub output_path: String,
    pub object_path: Option<String>,
    pub executable_path: Option<String>,
}

/// Compile Knull source to native code
#[cfg(feature = "llvm-backend")]
pub fn compile(
    source: &str,
    output_path: &Path,
    options: CompileOptions,
) -> Result<CompilationResult, String> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use inkwell::context::Context;
    use llvm_codegen::LLVMCodeGen;

    // Parse the source
    let mut lexer = Lexer::new(source);
    let _tokens = lexer.tokenize();

    let mut parser = Parser::new(source);
    let ast = parser
        .parse()
        .map_err(|e| format!("Parse error: {:?}", e))?;

    // Type check (in Expert/God mode)
    if options.mode != CompileMode::Novice {
        let mut type_checker = crate::type_system::TypeChecker::new();
        type_checker
            .check(&crate::ast::ASTNode::Program(vec![]))
            .map_err(|e| format!("Type error: {}", e))?;
    }

    // Compile with LLVM
    let context = Context::create();
    let mut codegen = LLVMCodeGen::new(&context, "knull_module", options.mode)?;

    // Generate LLVM IR
    codegen.compile(&ast)?;

    // Optimize
    codegen.optimize(options.opt_level);

    // Output LLVM IR if requested
    if options.output_ir {
        let ir_path = output_path.with_extension("ll");
        codegen.write_ir(&ir_path)?;
    }

    // Compile to object file
    let obj_path = output_path.with_extension("o");
    codegen.compile_to_object(&obj_path)?;

    // Link to create executable
    let exe_path = output_path.to_path_buf();
    link_object(&obj_path, &exe_path)?;

    Ok(CompilationResult {
        output_path: output_path.to_string_lossy().to_string(),
        object_path: Some(obj_path.to_string_lossy().to_string()),
        executable_path: Some(exe_path.to_string_lossy().to_string()),
    })
}

/// Fallback compile without LLVM
#[cfg(not(feature = "llvm-backend"))]
pub fn compile(
    _source: &str,
    _output_path: &Path,
    _options: CompileOptions,
) -> Result<CompilationResult, String> {
    Err(
        "LLVM backend not available. Install LLVM and rebuild with --features llvm-backend"
            .to_string(),
    )
}

/// Generate assembly output
#[cfg(feature = "llvm-backend")]
pub fn generate_assembly(
    source: &str,
    output_path: &Path,
    options: CompileOptions,
) -> Result<(), String> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use inkwell::context::Context;
    use inkwell::targets::FileType;
    use llvm_codegen::LLVMCodeGen;

    let context = Context::create();
    let mut codegen = LLVMCodeGen::new(&context, "knull_module", options.mode)?;

    // Parse
    let mut lexer = Lexer::new(source);
    let _tokens = lexer.tokenize();
    let mut parser = Parser::new(source);
    let ast = parser
        .parse()
        .map_err(|e| format!("Parse error: {:?}", e))?;

    // Compile
    codegen.compile(&ast)?;
    codegen.optimize(options.opt_level);

    // Generate assembly
    let target_machine = &codegen.target_machine;
    target_machine
        .write_to_file(codegen.get_module(), FileType::Assembly, output_path)
        .map_err(|e| format!("Failed to generate assembly: {}", e.to_string()))?;

    Ok(())
}

#[cfg(not(feature = "llvm-backend"))]
pub fn generate_assembly(
    _source: &str,
    _output_path: &Path,
    _options: CompileOptions,
) -> Result<(), String> {
    Err("LLVM backend not available".to_string())
}

/// Link object file to create executable
fn link_object(obj_path: &Path, exe_path: &Path) -> Result<(), String> {
    use std::process::Command;

    let status = Command::new("cc")
        .arg("-o")
        .arg(exe_path)
        .arg(obj_path)
        .arg("-lm") // Math library
        .arg("-lpthread") // Threading support
        .status()
        .map_err(|e| format!("Failed to link: {}", e))?;

    if !status.success() {
        return Err("Linking failed".to_string());
    }

    Ok(())
}

/// Legacy interpreter execution (fallback)
pub fn execute(ast: &crate::parser::ASTNode) {
    // This is a placeholder - the real implementation would be in a separate interpreter module
    // For now, just print the AST
    println!("{:?}", ast);
}
