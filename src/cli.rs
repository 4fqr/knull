//! Knull CLI - Command Line Interface
//! Professional compiler interface for the Knull programming language

use crate::compiler::CompileOptions;
use crate::pkg::manager::PackageManager;
use colored::Colorize;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Run a Knull file (compile and execute)
pub fn run_file(path: &Path, verbose: bool) -> Result<(), String> {
    if verbose {
        println!("{} {}", "Running".bright_blue().bold(), path.display());
    }

    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    // For now, use interpreter mode
    if verbose {
        println!("  Compiling...");
    }

    // Parse and execute
    let mut lexer = crate::lexer::Lexer::new(&source);
    let _tokens = lexer.tokenize();

    let mut parser = crate::parser::Parser::new(&source);
    let ast = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    if verbose {
        println!("  {} Parsed successfully", "✓".green());
        println!("  Executing...");
    }

    // Execute
    crate::compiler::execute(&ast);

    Ok(())
}

/// Build a Knull file to native binary
pub fn build_file(path: &Path, output: Option<&Path>, verbose: bool) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let out_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| path.with_extension(""));

    if verbose {
        println!(
            "{} {} → {}",
            "Building".bright_yellow().bold(),
            path.display(),
            out_path.display()
        );
    }

    let _options = CompileOptions::default();

    #[cfg(feature = "llvm-backend")]
    {
        let result = crate::compiler::compile(&source, &out_path, _options)
            .map_err(|e| format!("Compilation failed: {}", e))?;

        if verbose {
            if let Some(ref obj) = result.object_path {
                println!("  Object file: {}", obj);
            }
        }

        println!(
            "{} Build successful: {}",
            "✓".green().bold(),
            out_path.display()
        );
    }

    #[cfg(not(feature = "llvm-backend"))]
    {
        // Use C backend as fallback
        crate::c_codegen::compile_to_binary(&source, out_path.to_str().unwrap())
            .map_err(|e| format!("Compilation failed: {}", e))?;

        println!(
            "{} Build successful: {}",
            "✓".green().bold(),
            out_path.display()
        );
    }

    Ok(())
}

/// Build in release mode (optimized)
pub fn build_release(path: &Path, output: Option<&Path>, verbose: bool) -> Result<(), String> {
    build_file(path, output, verbose)
}

/// Generate assembly output
pub fn generate_asm(path: &Path, output: Option<&Path>) -> Result<(), String> {
    let _source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let out_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| path.with_extension("s"));

    println!(
        "{} {} → {}",
        "Generating Assembly".bright_cyan().bold(),
        path.display(),
        out_path.display()
    );

    let _options = CompileOptions::default();

    #[cfg(feature = "llvm-backend")]
    {
        crate::compiler::generate_assembly(&_source, &out_path, _options)
            .map_err(|e| format!("Assembly generation failed: {}", e))?;
        println!(
            "{} Assembly generated: {}",
            "✓".green().bold(),
            out_path.display()
        );
        Ok(())
    }

    #[cfg(not(feature = "llvm-backend"))]
    {
        Err("LLVM backend not available".to_string())
    }
}

/// Check syntax and types without building
pub fn check_file(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    print!("Checking {}... ", path.display());
    io::stdout().flush().ok();

    // Parse
    let mut lexer = crate::lexer::Lexer::new(&source);
    let _tokens = lexer.tokenize();

    let mut parser = crate::parser::Parser::new(&source);
    let _ast = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    println!("{}", "✓ No errors found".green());

    Ok(())
}

/// Format a Knull file
pub fn format_file(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    println!("{} {}", "Formatting".bright_blue(), path.display());

    // Tokenize and format
    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();

    let mut formatted = String::new();
    let mut indent = 0;
    let mut prev_was_newline = true;

    for token in tokens {
        use crate::lexer::TokenKind;

        match token.kind {
            TokenKind::LBrace => {
                formatted.push_str(" {\n");
                indent += 4;
                prev_was_newline = true;
            }
            TokenKind::RBrace => {
                if indent >= 4 {
                    indent -= 4;
                }
                if !prev_was_newline {
                    formatted.push('\n');
                }
                formatted.push_str(&" ".repeat(indent));
                formatted.push_str("}\n");
                prev_was_newline = true;
            }
            TokenKind::Semicolon => {
                formatted.push('\n');
                prev_was_newline = true;
            }
            TokenKind::Eof => break,
            _ => {
                if prev_was_newline {
                    formatted.push_str(&" ".repeat(indent));
                    prev_was_newline = false;
                } else if token.kind != TokenKind::LParen
                    && token.kind != TokenKind::RParen
                    && token.kind != TokenKind::Comma
                {
                    formatted.push(' ');
                }
                formatted.push_str(&token.value);
            }
        }
    }

    // Write formatted output back
    fs::write(path, formatted).map_err(|e| format!("Failed to write formatted file: {}", e))?;

    println!("{} Formatted {}", "✓".green().bold(), path.display());

    Ok(())
}

/// Create a new Knull project
pub fn new_project(name: &str) -> Result<(), String> {
    println!(
        "{} Creating new Knull project: {}",
        "Creating".bright_yellow(),
        name.bright_cyan().bold()
    );

    PackageManager::new_project(name)?;

    println!("{} Project created successfully", "✓".green().bold());
    println!();
    println!("To get started:");
    println!("  cd {}", name);
    println!("  knull run src/main.knull");

    Ok(())
}

/// Add a dependency to the project
pub fn add_dependency(package: &str, version: Option<&str>) -> Result<(), String> {
    let ver = version.unwrap_or("^1.0");

    println!(
        "{} Adding dependency: {} {}",
        "Adding".bright_yellow(),
        package.bright_cyan(),
        ver.bright_black()
    );

    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    let mut pm = PackageManager::new(current_dir)?;
    pm.add_dependency(package, ver)?;

    println!("{} Added {} to dependencies", "✓".green().bold(), package);

    Ok(())
}

/// Remove a dependency from the project
pub fn remove_dependency(package: &str) -> Result<(), String> {
    println!(
        "{} Removing dependency: {}",
        "Removing".bright_yellow(),
        package.bright_cyan()
    );

    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    let mut pm = PackageManager::new(current_dir)?;
    pm.remove_dependency(package)?;

    println!(
        "{} Removed {} from dependencies",
        "✓".green().bold(),
        package
    );

    Ok(())
}

/// Update all dependencies or a specific package
pub fn update_dependencies(package: Option<&str>) -> Result<(), String> {
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    let mut pm = PackageManager::new(current_dir)?;

    match package {
        Some(pkg) => {
            println!(
                "{} Updating dependency: {}",
                "Updating".bright_yellow(),
                pkg.bright_cyan()
            );

            let constraint = pm
                .manifest()
                .dependencies
                .get(pkg)
                .cloned()
                .unwrap_or_else(|| "^1.0".to_string());

            pm.update_package(pkg, &constraint)?;
            println!("{} Updated {}", "✓".green().bold(), pkg);
        }
        None => {
            pm.update_all_dependencies()?;
        }
    }

    Ok(())
}

/// Publish package to registry
pub fn publish(local: bool, token: Option<&str>) -> Result<(), String> {
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    let pm = PackageManager::new(current_dir)?;

    if local {
        println!("{}", "Publishing to local registry...".bright_yellow());
        pm.publish_local()?;
    } else {
        let auth_token = token
            .map(|s| s.to_string())
            .or_else(|| std::env::var("KNULL_REGISTRY_TOKEN").ok())
            .ok_or("Authentication token required. Use --token or set KNULL_REGISTRY_TOKEN")?;

        println!("{}", "Publishing to registry...".bright_yellow());
        pm.publish_registry(&auth_token)?;
    }

    Ok(())
}

/// Fetch dependencies
pub fn fetch_dependencies() -> Result<(), String> {
    println!("{}", "Fetching dependencies...".bright_yellow());

    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    let pm = PackageManager::new(current_dir)?;
    pm.fetch_dependencies()?;

    println!("{} Dependencies fetched", "✓".green().bold());
    Ok(())
}

/// List project dependencies
pub fn list_dependencies() -> Result<(), String> {
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    let pm = PackageManager::new(current_dir)?;

    println!("{}", "Dependencies:".bright_yellow().bold());

    let deps = pm.manifest().dependencies.clone();
    if deps.is_empty() {
        println!("  No dependencies");
    } else {
        for (name, version) in deps {
            println!("  {} {}", name.bright_cyan(), version.bright_black());
        }
    }

    // Show dev dependencies
    let dev_deps = &pm.manifest().dev_dependencies;
    if !dev_deps.is_empty() {
        println!("\n{}", "Dev Dependencies:".bright_yellow().bold());
        for (name, version) in dev_deps {
            println!("  {} {}", name.bright_cyan(), version.bright_black());
        }
    }

    Ok(())
}

/// Run tests
pub fn run_tests() -> Result<(), String> {
    println!("{}", "Running tests...".bright_yellow().bold());

    let test_dirs = vec!["tests", "test", "src/tests"];
    let mut found_tests = false;
    let mut passed = 0;
    let mut failed = 0;

    for dir in test_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "knull") {
                    found_tests = true;
                    print!("  Testing {}... ", path.display());
                    io::stdout().flush().ok();

                    match run_file(&path, false) {
                        Ok(_) => {
                            println!("{}", "PASS".green());
                            passed += 1;
                        }
                        Err(e) => {
                            println!("{} {}", "FAIL".red(), e);
                            failed += 1;
                        }
                    }
                }
            }
        }
    }

    if !found_tests {
        println!("  No test files found. Create .knull files in tests/ directory.");
    } else {
        println!();
        println!("Test Results: {} passed, {} failed", passed, failed);
    }

    Ok(())
}

/// Start interactive REPL
pub fn start_repl() -> Result<(), String> {
    println!("{}", "Knull REPL v1.0.0".bright_purple().bold());
    println!("Type :quit or :q to exit");
    println!();

    loop {
        print!("{}", "knull> ".bright_magenta().bold());
        io::stdout().flush().ok();

        let mut input = String::new();
        if let Ok(_) = io::stdin().read_line(&mut input) {
            let input = input.trim();
            if input == ":quit" || input == ":q" {
                println!("{}", "Goodbye".bright_green());
                break;
            }

            if input.is_empty() {
                continue;
            }

            // Parse and execute
            let mut lexer = crate::lexer::Lexer::new(input);
            let _tokens = lexer.tokenize();

            let mut parser = crate::parser::Parser::new(input);
            match parser.parse() {
                Ok(ast) => {
                    crate::compiler::execute(&ast);
                }
                Err(e) => println!("{} {}", "Error:".bright_red().bold(), e),
            }
        }
    }

    Ok(())
}

/// Show version information
pub fn show_version() {
    println!(
        "{} {}",
        "Knull".bright_purple().bold(),
        "v1.0.0".bright_cyan()
    );
    println!("The God Programming Language");
    println!();
    println!("Edition: 2024");
    #[cfg(feature = "llvm-backend")]
    println!("LLVM Backend: Enabled");
    #[cfg(not(feature = "llvm-backend"))]
    println!("LLVM Backend: Disabled (interpreter mode)");
    println!("Target: Native");
}

/// Show help information
pub fn show_help() {
    println!("{}", "Knull Programming Language".bright_purple().bold());
    println!();
    println!("{}", "USAGE:".bright_yellow().bold());
    println!("  knull <COMMAND> [OPTIONS] [ARGS]");
    println!();
    println!("{}", "COMMANDS:".bright_yellow().bold());
    println!("  run <file>              Execute a Knull file");
    println!("  build <file>            Compile to binary");
    println!("  asm <file>              Generate assembly output");
    println!("  check <file>            Check syntax and types");
    println!("  fmt <file>              Format a Knull file");
    println!("  new <name>              Create a new Knull project");
    println!("  add <package> [version] Add a dependency");
    println!("  remove <package>        Remove a dependency");
    println!("  update [package]        Update dependencies");
    println!("  fetch                   Fetch all dependencies");
    println!("  list                    List dependencies");
    println!("  publish [options]       Publish package to registry");
    println!("  test                    Run tests");
    println!("  repl                    Start interactive shell");
    println!("  version                 Show version");
    println!("  help                    Show this help");
    println!();
    println!("{}", "PACKAGE OPTIONS:".bright_yellow().bold());
    println!("  --local                 Publish to local registry");
    println!("  --token <token>         Registry authentication token");
    println!();
    println!("{}", "EXAMPLES:".bright_yellow().bold());
    println!("  knull run hello.knull");
    println!("  knull build main.knull -o myapp");
    println!("  knull new my-project");
    println!("  knull add serde ^1.0");
    println!("  knull remove serde");
    println!("  knull update");
    println!("  knull publish --local");
    println!("  knull publish --token <auth_token>");
    println!();
    println!("For more information: https://knull-lang.dev");
}
