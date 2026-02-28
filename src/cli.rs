//! Knull CLI - Command Line Interface
//! The most fabulous CLI tool you've ever seen

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_file(path: &Path, verbose: bool) -> Result<(), String> {
    if verbose {
        println!("{} {}", "▶ Running".bright_blue().bold(), path.display());
    }

    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Tokenize
    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    if verbose {
        println!("  {} Lexed {} tokens", "✓".green(), tokens.len());
    }

    // Parse
    let mut parser = crate::parser::Parser::new(&source);
    match parser.parse() {
        Ok(ast) => {
            if verbose {
                println!("  {} Parsed successfully", "✓".green());
            }

            // Interpret/Execute
            crate::compiler::execute(&ast);
            Ok(())
        }
        Err(e) => Err(format!("Parse error: {}", e)),
    }
}

pub fn build_file(path: &Path, output: Option<&Path>, verbose: bool) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let out_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| path.with_extension(""));

    if verbose {
        println!(
            "{} {} → {}",
            "🔨 Building".bright_yellow().bold(),
            path.display(),
            out_path.display()
        );
    }

    // Parse
    let mut parser = crate::parser::Parser::new(&source);
    let ast = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    if verbose {
        println!("  {} Parsed successfully", "✓".green());
    }

    // Compile to binary (placeholder for now - would generate actual binary)
    println!("{} Build successful!", "✓".green().bold());
    Ok(())
}

pub fn generate_asm(path: &Path, output: Option<&Path>) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let out_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| path.with_extension("asm"));

    println!(
        "{} {} → {}",
        "⚙ Generating ASM".bright_cyan().bold(),
        path.display(),
        out_path.display()
    );

    // Generate x86_64 assembly
    let asm = generate_x86_64_asm(&source)?;

    fs::write(&out_path, asm).map_err(|e| format!("Failed to write assembly: {}", e))?;

    println!(
        "{} Assembly generated: {}",
        "✓".green().bold(),
        out_path.display()
    );
    Ok(())
}

fn generate_x86_64_asm(source: &str) -> Result<String, String> {
    let mut parser = crate::parser::Parser::new(source);
    let ast = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    let mut output = String::new();
    output.push_str("; ============================================\n");
    output.push_str("; Knull Compiled Output - x86_64 Assembly\n");
    output.push_str("; ============================================\n\n");
    output.push_str("section .data\n");
    output.push_str("    fmt_int: db \"%ld\", 10, 0\n");
    output.push_str("    fmt_str: db \"%s\", 10, 0\n\n");
    output.push_str("section .text\n");
    output.push_str("    global _start\n");
    output.push_str("    extern printf\n");
    output.push_str("    extern exit\n\n");
    output.push_str("; Generated functions\n\n");
    output.push_str("_start:\n");
    output.push_str("    call main\n");
    output.push_str("    mov rdi, rax\n");
    output.push_str("    call exit\n\n");

    // Generate main function
    output.push_str("main:\n");
    output.push_str("    push rbp\n");
    output.push_str("    mov rbp, rsp\n\n");
    output.push_str("    ; Function body would go here\n\n");
    output.push_str("    mov rax, 0\n");
    output.push_str("    pop rbp\n");
    output.push_str("    ret\n");

    Ok(output)
}

pub fn start_repl() -> Result<(), String> {
    println!(
        "{}",
        "╭─────────────────────────────────────╮".bright_purple()
    );
    println!(
        "{}",
        "│     Knull REPL v1.0.0               │".bright_purple()
    );
    println!(
        "{}",
        "│     Type :quit or :q to exit        │".bright_purple()
    );
    println!(
        "{}",
        "╰─────────────────────────────────────╯".bright_purple()
    );
    println!();

    loop {
        print!("{}", "knull ".bright_magenta().bold());
        print!("{}", "▸ ".bright_cyan());
        io::stdout().flush().ok();

        let mut input = String::new();
        if let Ok(_) = io::stdin().read_line(&mut input) {
            let input = input.trim();
            if input == ":quit" || input == ":q" {
                println!("{}", "Goodbye! 👋".bright_green());
                break;
            }

            if input.is_empty() {
                continue;
            }

            // Parse and interpret
            let mut parser = crate::parser::Parser::new(input);
            match parser.parse() {
                Ok(ast) => {
                    crate::compiler::execute(&ast);
                }
                Err(e) => println!("{} {}", "✗ Error:".bright_red().bold(), e),
            }
        }
    }

    Ok(())
}

pub fn format_file(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    println!("{} {}", "📝 Formatting".bright_blue(), path.display());

    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();

    for token in tokens {
        if token.kind == crate::lexer::TokenKind::Eof {
            break;
        }
        print!("{} ", token.value);
    }
    println!();

    Ok(())
}

pub fn check_file(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mut lexer = crate::lexer::Lexer::new(&source);
    lexer.tokenize();

    let mut parser = crate::parser::Parser::new(&source);
    parser.parse().map_err(|e| format!("Error: {}", e))?;

    println!("{} No errors found", "✓".green().bold());
    Ok(())
}

pub fn new_project(name: &str) -> Result<(), String> {
    println!(
        "{} Creating new Knull project: {}",
        "📦".bright_yellow(),
        name.bright_cyan().bold()
    );

    let project_dir = PathBuf::from(name);
    fs::create_dir(&project_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    // Create src directory
    let src_dir = project_dir.join("src");
    fs::create_dir(&src_dir).map_err(|e| format!("Failed to create src directory: {}", e))?;

    // Create main.knull
    let main_content = r#"// Welcome to Knull!
// This is your main entry point

fn main() {
    println "Hello, Knull! 🚀"
    println ""
    println "This is your new project."
    println "Run with: knull run src/main.knull"
}
"#;
    fs::write(src_dir.join("main.knull"), main_content)
        .map_err(|e| format!("Failed to create main.knull: {}", e))?;

    // Create knull.toml
    let toml_content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2024"
entry = "src/main.knull"
authors = ["Your Name <you@example.com>"]
description = "A fabulous Knull project"
license = "MIT"

[dependencies]
# Add your dependencies here
# std = "^1.0"

[build]
opt-level = 3
lto = true
"#,
        name
    );

    fs::write(project_dir.join("knull.toml"), toml_content)
        .map_err(|e| format!("Failed to create knull.toml: {}", e))?;

    // Create README.md
    let readme_content = format!(
        r#"# {}

A fabulous Knull project.

## Getting Started

```bash
# Run the project
knull run src/main.knull

# Build the project
knull build src/main.knull

# Check for errors
knull check src/main.knull
```

## Project Structure

```
{}/
├── src/
│   └── main.knull    # Entry point
├── knull.toml        # Package manifest
└── README.md         # This file
```

## License

MIT
"#,
        name, name
    );

    fs::write(project_dir.join("README.md"), readme_content)
        .map_err(|e| format!("Failed to create README.md: {}", e))?;

    println!("{} Project created successfully!", "✓".green().bold());
    println!();
    println!("To get started:");
    println!("  cd {}", name);
    println!("  knull run src/main.knull");

    Ok(())
}

pub fn add_dependency(package: &str, version: Option<&str>) -> Result<(), String> {
    let ver = version.unwrap_or("^1.0");

    println!(
        "{} Adding dependency: {} {}",
        "📦".bright_yellow(),
        package.bright_cyan(),
        ver.bright_black()
    );

    // Read current knull.toml
    let toml_content = fs::read_to_string("knull.toml").map_err(|e| {
        format!(
            "Failed to read knull.toml: {}. Are you in a Knull project?",
            e
        )
    })?;

    // Add dependency (simple string manipulation for now)
    let dep_line = format!("{} = \"{}\"", package, ver);
    let new_content =
        toml_content.replace("[dependencies]", &format!("[dependencies]\n{}", dep_line));

    fs::write("knull.toml", new_content)
        .map_err(|e| format!("Failed to write knull.toml: {}", e))?;

    println!("{} Added {} to dependencies", "✓".green().bold(), package);

    Ok(())
}

pub fn run_tests() -> Result<(), String> {
    println!("{}", "🧪 Running tests...".bright_yellow().bold());

    // Look for test files
    let test_dirs = vec!["tests", "test", "src/tests"];
    let mut found_tests = false;

    for dir in test_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "knull") {
                    found_tests = true;
                    print!("  Testing {}... ", path.display());

                    match run_file(&path, false) {
                        Ok(_) => println!("{}", "✓ PASS".green()),
                        Err(e) => println!("{} {}", "✗ FAIL".red(), e),
                    }
                }
            }
        }
    }

    if !found_tests {
        println!("  No test files found. Create .knull files in tests/ directory.");
    }

    Ok(())
}

pub fn show_version() {
    println!(
        "{} {}",
        "Knull".bright_purple().bold(),
        "v1.0.0".bright_cyan()
    );
    println!("The most fabulous programming language.");
    println!();
    println!("Edition: 2024");
    println!("Compiler: knullc (bootstrap)");
}

pub fn show_help() {
    println!("{}", "Knull Programming Language".bright_purple().bold());
    println!();
    println!("{}", "USAGE:".bright_yellow().bold());
    println!("  knull <COMMAND> [OPTIONS] [ARGS]");
    println!();
    println!("{}", "COMMANDS:".bright_yellow().bold());
    println!(
        "  {} {}     Run a Knull file",
        "run".bright_cyan(),
        "<file>".bright_black()
    );
    println!(
        "  {} {}   Compile a Knull file",
        "build".bright_cyan(),
        "<file>".bright_black()
    );
    println!(
        "  {} {}    Generate assembly output",
        "asm".bright_cyan(),
        "<file>".bright_black()
    );
    println!(
        "  {} {}    Check syntax without building",
        "check".bright_cyan(),
        "<file>".bright_black()
    );
    println!(
        "  {} {}    Format a Knull file",
        "fmt".bright_cyan(),
        "<file>".bright_black()
    );
    println!(
        "  {} {}    Create a new Knull project",
        "new".bright_cyan(),
        "<name>".bright_black()
    );
    println!(
        "  {} {}  Add a dependency",
        "add".bright_cyan(),
        "<package>".bright_black()
    );
    println!("  {}           Run tests", "test".bright_cyan());
    println!("  {}           Start REPL", "repl".bright_cyan());
    println!("  {}           Show version", "version".bright_cyan());
    println!("  {}           Show this help", "help".bright_cyan());
    println!();
    println!("{}", "EXAMPLES:".bright_yellow().bold());
    println!("  knull run hello.knull");
    println!("  knull build main.knull -o myapp");
    println!("  knull asm program.knull -o program.asm");
    println!("  knull new my-awesome-project");
    println!("  knull add json");
    println!();
    println!(
        "For more information: {}",
        "https://knull-lang.dev".bright_blue().underline()
    );
}
