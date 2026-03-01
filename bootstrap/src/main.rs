//! Knull Programming Language - Bootstrap Compiler
//!
//! This is the initial compiler written in Rust to bootstrap the Knull language.
//! It handles lexing, parsing, type checking, and code generation.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{error, info};
use std::io::Read;
use std::path::PathBuf;
use std::process::exit;

mod ast;
mod ccodegen;
mod codegen;
mod compiler;
mod lexer;
mod parser;

#[derive(Parser)]
#[command(name = "knull")]
#[command(version = "1.0.0")]
#[command(about = "The Knull Programming Language - The God Language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true)]
    verbose: bool,

    #[arg(short, long, global = true, default_value = "release")]
    build_type: String,

    #[arg(short, long, global = true)]
    target: Option<String>,

    #[arg(short, long, global = true)]
    output: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a Knull file (compiles and executes)
    Run {
        /// The .knull file to run
        file: PathBuf,

        /// Arguments to pass to the program
        args: Vec<String>,
    },

    /// Compile a Knull file to an executable
    Build {
        /// The .knull file to compile
        file: PathBuf,

        /// Generate debug symbols
        #[arg(long)]
        debug: bool,

        /// Enable link-time optimization
        #[arg(long)]
        lto: bool,
    },

    /// Compile to LLVM IR (for debugging)
    Ir {
        /// The .knull file to compile
        file: PathBuf,

        /// Output file for IR (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Start the Language Server Protocol (LSP) server
    Lsp,

    /// Check syntax and types without compiling
    Check {
        /// The .knull file to check
        file: PathBuf,
    },

    /// Generate C code from a Knull file
    Cc {
        /// The .knull file to compile
        file: PathBuf,

        /// Output file for C code (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Format a Knull file
    Fmt {
        /// The .knull file to format
        file: Option<PathBuf>,

        /// Write output in place
        #[arg(short, long)]
        write: bool,
    },

    /// Print version information
    Version,
}

fn main() {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Set up panic hook for better error reporting
    std::panic::set_hook(Box::new(|panic_info| {
        error!("PANIC: {}", panic_info);
        if let Some(location) = panic_info.location() {
            error!(
                "  at {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
    }));

    // Execute command
    let verbose = cli.verbose;
    let build_type = cli.build_type.clone();
    let target = cli.target.clone();
    let output = cli.output.clone();
    let result = match &cli.command {
        Commands::Run { file, args } => run_file(file.clone(), args.clone(), &cli),
        Commands::Build { file, debug, lto } => build_file(file.clone(), *debug, *lto, &cli),
        Commands::Ir { file, output } => generate_ir(file.clone(), output.clone()),
        Commands::Lsp => start_lsp(),
        Commands::Check { file } => check_file(file.clone()),
        Commands::Cc { file, output } => generate_cc(file.clone(), output.clone()),
        Commands::Fmt { file, write } => format_file(file.clone(), *write),
        Commands::Version => {
            println!("knull {}", env!("CARGO_PKG_VERSION"));
            println!("Platform: {}", std::env::consts::OS);
            println!("Architecture: {}", std::env::consts::ARCH);
            Ok(())
        }
    };

    // Handle result
    match result {
        Ok(_) => {
            info!("Completed successfully");
            exit(0);
        }
        Err(e) => {
            error!("Error: {}", e);
            if cli.verbose {
                error!("Backtrace: {:?}", std::backtrace::Backtrace::capture());
            }
            exit(1);
        }
    }
}

/// Run a Knull file using JIT compilation
fn run_file(file: PathBuf, args: Vec<String>, cli: &Cli) -> Result<()> {
    info!("Running file: {:?}", file);

    // Read source file
    let source = std::fs::read_to_string(&file)
        .with_context(|| format!("Failed to read file: {:?}", file))?;

    // Lex
    let tokens = lexer::Lexer::new(&source)
        .lex()
        .with_context(|| "Lexing failed")?;

    if cli.verbose {
        info!("Tokens: {:?}", tokens);
    }

    // Parse
    let ast = parser::Parser::new(tokens)
        .parse()
        .with_context(|| "Parsing failed")?;

    if cli.verbose {
        info!("AST: {:#?}", ast);
    }

    // Compile
    let module = compiler::Compiler::new(&ast)
        .compile()
        .with_context(|| "Compilation failed")?;

    // Execute (JIT)
    info!("Executing...");
    codegen::JitExecutor::new().execute(&module, args)?;

    Ok(())
}

/// Build a Knull file to an executable
fn build_file(file: PathBuf, debug: bool, lto: bool, cli: &Cli) -> Result<()> {
    info!("Building file: {:?}", file);

    // Determine output path
    let output = cli.output.clone().unwrap_or_else(|| {
        let stem = file.file_stem().unwrap_or_default().to_str().unwrap_or("a");
        let mut out = PathBuf::from(stem);
        if cfg!(windows) {
            out.set_extension("exe");
        }
        out
    });

    // Read source file
    let source = std::fs::read_to_string(&file)
        .with_context(|| format!("Failed to read file: {:?}", file))?;

    // Lex
    let tokens = lexer::Lexer::new(&source).lex()?;

    // Parse
    let ast = parser::Parser::new(tokens).parse()?;

    // Compile to object file
    let module = compiler::Compiler::new(&ast)
        .with_debug(debug)
        .with_lto(lto)
        .compile()?;

    // Link to produce executable
    let target_triple = cli
        .target
        .clone()
        .unwrap_or_else(|| std::env::consts::ARCH.to_string());

    codegen::Linker::new()
        .with_target(&target_triple)
        .link(&module, &output)?;

    info!("Built: {:?}", output);

    Ok(())
}

/// Generate LLVM IR from a Knull file
fn generate_ir(file: PathBuf, output: Option<PathBuf>) -> Result<()> {
    info!("Generating IR for: {:?}", file);

    // Read source file
    let source = std::fs::read_to_string(&file)
        .with_context(|| format!("Failed to read file: {:?}", file))?;

    // Lex
    let tokens = lexer::Lexer::new(&source).lex()?;

    // Parse
    let ast = parser::Parser::new(tokens).parse()?;

    // Compile to IR
    let module = compiler::Compiler::new(&ast).compile()?;

    // Output IR
    let ir = module.ir.llvm_ir();

    match output {
        Some(path) => {
            std::fs::write(&path, ir.as_bytes())?;
            info!("IR written to: {:?}", path);
        }
        None => {
            println!("{}", ir);
        }
    }

    Ok(())
}

/// Generate C code from a Knull file
fn generate_cc(file: PathBuf, output: Option<PathBuf>) -> Result<()> {
    info!("Generating C code for: {:?}", file);

    // Read source file
    let source = std::fs::read_to_string(&file)
        .with_context(|| format!("Failed to read file: {:?}", file))?;

    // Lex
    let tokens = lexer::Lexer::new(&source).lex()?;

    // Parse
    let ast = parser::Parser::new(tokens).parse()?;

    // Generate C code
    let mut cc = ccodegen::CCodeGenerator::new();
    let c_code = cc.generate(&ast);

    // Add runtime functions (before the generated code)
    let full_c = format!("{}\n\n{}\n", RUNTIME_FUNCTIONS, c_code);

    match output {
        Some(path) => {
            std::fs::write(&path, full_c.as_bytes())?;
            info!("C code written to: {:?}", path);
        }
        None => {
            println!("{}", full_c);
        }
    }

    Ok(())
}

const RUNTIME_FUNCTIONS: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define knull_println(x) _Generic((x), \
    int: printf("%d\n", (x)), \
    char*: printf("%s\n", (x)), \
    default: printf("%p\n", (x)))

#define knull_print(x) _Generic((x), \
    int: printf("%d", (x)), \
    char*: printf("%s", (x)), \
    default: printf("%p", (x)))

int knull_len(int* arr) {
    return sizeof(arr) / sizeof(int);
}

void* knull_alloc(size_t size) {
    return malloc(size);
}

void knull_free(void* ptr) {
    free(ptr);
}
"#;

/// Start the Language Server
fn start_lsp() -> Result<()> {
    info!("Starting LSP server...");

    #[cfg(feature = "lsp")]
    {
        lsp::run();
        Ok(())
    }

    #[cfg(not(feature = "lsp"))]
    {
        anyhow::bail!("LSP support not compiled. Enable with --features lsp");
    }
}

/// Check a Knull file for errors
fn check_file(file: PathBuf) -> Result<()> {
    info!("Checking file: {:?}", file);

    // Read source file
    let source = std::fs::read_to_string(&file)
        .with_context(|| format!("Failed to read file: {:?}", file))?;

    // Lex
    let tokens = lexer::Lexer::new(&source).lex()?;

    // Parse
    let ast = parser::Parser::new(tokens).parse()?;

    // Type check
    compiler::TypeChecker::check(&ast)?;

    info!("No errors found");

    Ok(())
}

/// Format a Knull file
fn format_file(file: Option<PathBuf>, write: bool) -> Result<()> {
    match file {
        Some(path) => {
            // Read source file
            let source = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read file: {:?}", path))?;

            // Format
            let formatted = formatter::Formatter::new().format(&source)?;

            if write {
                std::fs::write(&path, formatted.as_bytes())?;
                info!("Formatted: {:?}", path);
            } else {
                print!("{}", formatted);
            }

            Ok(())
        }
        None => {
            // Read from stdin
            let mut source = String::new();
            std::io::stdin().read_to_string(&mut source)?;

            let formatted = formatter::Formatter::new().format(&source)?;
            print!("{}", formatted);

            Ok(())
        }
    }
}

// Re-export for convenience
mod lsp {
    pub use crate::lsp_server::*;
}

mod lsp_server {
    use anyhow::Result;

    pub fn run() -> Result<()> {
        // LSP implementation would go here
        // For now, just a placeholder
        anyhow::bail!("LSP not yet implemented");
    }
}

mod formatter {
    pub struct Formatter;

    impl Formatter {
        pub fn new() -> Self {
            Formatter
        }

        pub fn format(&self, source: &str) -> anyhow::Result<String> {
            // Simple formatter - just returns source for now
            Ok(source.to_string())
        }
    }
}
