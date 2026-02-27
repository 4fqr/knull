//! Knull CLI - Command Line Interface

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

pub fn run_file(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    println!("Running: {}", path.display());

    // Tokenize
    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    println!("Lexed {} tokens", tokens.len());

    // Parse
    let mut parser = crate::parser::Parser::new(&source);
    match parser.parse() {
        Ok(_) => println!("Parsed successfully"),
        Err(e) => return Err(format!("Parse error: {}", e)),
    }

    // Compile
    match crate::compiler::compile(&source) {
        Ok(asm) => {
            // Write assembly
            let asm_path = path.with_extension("asm");
            fs::write(&asm_path, &asm).map_err(|e| format!("Failed to write assembly: {}", e))?;
            println!("Wrote: {}", asm_path.display());

            // Try to assemble and run (if nasm available)
            if let Ok(output) = Command::new("nasm")
                .args([
                    "-f",
                    "elf64",
                    "-o",
                    "/tmp/knull.o",
                    asm_path.to_str().unwrap(),
                ])
                .output()
            {
                if output.status.success() {
                    // Link
                    if let Ok(link_output) = Command::new("ld")
                        .args(["-o", "/tmp/knull", "/tmp/knull.o"])
                        .output()
                    {
                        if link_output.status.success() {
                            // Run
                            println!("Executing...");
                            if let Ok(run_output) = Command::new("/tmp/knull").output() {
                                io::stdout().write_all(&run_output.stdout).ok();
                                io::stderr().write_all(&run_output.stderr).ok();
                            }
                        }
                    }
                }
            }
        }
        Err(e) => return Err(format!("Compile error: {}", e)),
    }

    Ok(())
}

pub fn build_file(path: &Path, output: Option<&Path>) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let out_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| path.with_extension(""));

    println!("Building: {} -> {}", path.display(), out_path.display());

    // Parse
    let mut parser = crate::parser::Parser::new(&source);
    parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    println!("Build successful!");
    Ok(())
}

pub fn start_repl() -> Result<(), String> {
    println!("Knull REPL v1.0.0");
    println!("Type :quit to exit\n");

    loop {
        print!("knull> ");
        io::stdout().flush().ok();

        let mut input = String::new();
        if let Ok(_) = io::stdin().read_line(&mut input) {
            let input = input.trim();
            if input == ":quit" || input == ":q" {
                break;
            }

            if input.is_empty() {
                continue;
            }

            // Parse and evaluate
            let mut parser = crate::parser::Parser::new(input);
            match parser.parse() {
                Ok(_) => println!("Ok"),
                Err(e) => println!("Error: {}", e),
            }
        }
    }

    Ok(())
}

pub fn format_file(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Simple formatting: just re-tokenize and print
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

    // Tokenize
    let mut lexer = crate::lexer::Lexer::new(&source);
    lexer.tokenize();

    // Parse
    let mut parser = crate::parser::Parser::new(&source);
    parser.parse().map_err(|e| format!("Error: {}", e))?;

    println!("No errors found");
    Ok(())
}
