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
        Ok(ast) => {
            println!("Parsed successfully");

            // Interpret/Execute
            crate::compiler::execute(&ast);
        }
        Err(e) => return Err(format!("Parse error: {}", e)),
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

            // Parse and interpret
            let mut parser = crate::parser::Parser::new(input);
            match parser.parse() {
                Ok(ast) => {
                    crate::compiler::execute(&ast);
                }
                Err(e) => println!("Error: {}", e),
            }
        }
    }

    Ok(())
}

pub fn format_file(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

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

    println!("No errors found");
    Ok(())
}
