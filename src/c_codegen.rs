//! Knull C Code Generator
//!
//! Compiles Knull AST to C code, then uses system cc to compile to native binary.
//! This is a practical alternative to LLVM when LLVM libraries aren't available.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::parser::{ASTNode, Literal};

/// C Code Generator
pub struct CCodeGen {
    output: String,
    indent: usize,
    temp_count: u32,
    functions: HashMap<String, FunctionInfo>,
    current_function: Option<String>,
}

#[derive(Debug, Clone)]
struct FunctionInfo {
    params: Vec<String>,
    has_return: bool,
}

impl CCodeGen {
    pub fn new() -> Self {
        CCodeGen {
            output: String::new(),
            indent: 0,
            temp_count: 0,
            functions: HashMap::new(),
            current_function: None,
        }
    }

    /// Generate C code from AST
    pub fn compile(&mut self, ast: &ASTNode) -> Result<String, String> {
        // Add C headers
        self.emit_line("#include <stdio.h>");
        self.emit_line("#include <stdlib.h>");
        self.emit_line("#include <string.h>");
        self.emit_line("#include <stdint.h>");
        self.emit_line("#include <stdbool.h>");
        self.emit_line("");

        // Add Knull runtime support
        self.emit_runtime();

        // Forward declare all functions
        if let ASTNode::Program(items) = ast {
            for item in items {
                if let ASTNode::Function { name, params, .. } = item {
                    self.functions.insert(
                        name.clone(),
                        FunctionInfo {
                            params: params.clone(),
                            has_return: false,
                        },
                    );
                    let params_str = if params.is_empty() {
                        "void".to_string()
                    } else {
                        params
                            .iter()
                            .map(|_| "int64_t".to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    self.emit_line(&format!("int64_t {}({});", name, params_str));
                }
            }
        }

        self.emit_line("");

        // Generate code for all items
        self.compile_node(ast)?;

        Ok(self.output.clone())
    }

    /// Compile AST node to C
    fn compile_node(&mut self, node: &ASTNode) -> Result<String, String> {
        match node {
            ASTNode::Program(items) => {
                for item in items {
                    self.compile_node(item)?;
                    self.emit_line("");
                }
                Ok(String::new())
            }
            ASTNode::Function { name, params, body } => {
                self.current_function = Some(name.clone());

                let params_str = if params.is_empty() {
                    "void".to_string()
                } else {
                    params
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("int64_t arg{}", i))
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                self.emit_line(&format!("int64_t {}({}) {{", name, params_str));
                self.indent += 4;

                // Store param mappings
                for (i, param) in params.iter().enumerate() {
                    self.emit_line(&format!("int64_t {} = arg{};", param, i));
                }

                self.compile_node(body)?;

                // Add default return if none exists
                self.emit_line("return 0;");

                self.indent -= 4;
                self.emit_line("}");

                self.current_function = None;
                Ok(String::new())
            }
            ASTNode::Block(nodes) => {
                for node in nodes {
                    self.compile_node(node)?;
                }
                Ok(String::new())
            }
            ASTNode::Let { name, value } => {
                let val_code = self.compile_node(value)?;
                self.emit_line(&format!("int64_t {} = {};", name, val_code));
                Ok(String::new())
            }
            ASTNode::Return(expr) => {
                let val = self.compile_node(expr)?;
                self.emit_line(&format!("return {};", val));
                Ok(String::new())
            }
            ASTNode::If {
                cond,
                then_body,
                else_body,
            } => {
                let cond_val = self.compile_node(cond)?;
                self.emit_line(&format!("if ({}) {{", cond_val));
                self.indent += 4;
                self.compile_node(then_body)?;
                self.indent -= 4;
                if let Some(else_node) = else_body {
                    self.emit_line("} else {");
                    self.indent += 4;
                    self.compile_node(else_node)?;
                    self.indent -= 4;
                }
                self.emit_line("}");
                Ok(String::new())
            }
            ASTNode::While { cond, body } => {
                let cond_val = self.compile_node(cond)?;
                self.emit_line(&format!("while ({}) {{", cond_val));
                self.indent += 4;
                self.compile_node(body)?;
                self.indent -= 4;
                self.emit_line("}");
                Ok(String::new())
            }
            ASTNode::Binary { op, left, right } => {
                let l = self.compile_node(left)?;
                let r = self.compile_node(right)?;

                let c_op = match op.as_str() {
                    "+" => "+",
                    "-" => "-",
                    "*" => "*",
                    "/" => "/",
                    "%" => "%",
                    "==" => "==",
                    "!=" => "!=",
                    "<" => "<",
                    ">" => ">",
                    "<=" => "<=",
                    ">=" => ">=",
                    "&&" => "&&",
                    "||" => "||",
                    _ => return Err(format!("Unknown operator: {}", op)),
                };

                Ok(format!("({} {} {})", l, c_op, r))
            }
            ASTNode::Unary { op, operand } => {
                let val = self.compile_node(operand)?;
                match op.as_str() {
                    "-" => Ok(format!("(-{})", val)),
                    "!" => Ok(format!("(!{})", val)),
                    _ => Err(format!("Unknown unary operator: {}", op)),
                }
            }
            ASTNode::Call { func, args } => {
                let args_code: Result<Vec<String>, String> =
                    args.iter().map(|a| self.compile_node(a)).collect();
                let args_str = args_code?.join(", ");

                // Handle built-in functions
                match func.as_str() {
                    "println" => {
                        if args.len() == 1 {
                            let arg = self.compile_node(&args[0])?;
                            self.emit_line(&format!("printf(\"%ld\\n\", (long int)({}));", arg));
                        }
                        Ok("0".to_string())
                    }
                    "print" => {
                        if args.len() == 1 {
                            let arg = self.compile_node(&args[0])?;
                            self.emit_line(&format!("printf(\"%ld\", (long int)({}));", arg));
                        }
                        Ok("0".to_string())
                    }
                    _ => Ok(format!("{}({})", func, args_str)),
                }
            }
            ASTNode::Literal(lit) => {
                match lit {
                    Literal::Int(n) => Ok(n.to_string()),
                    Literal::Float(f) => Ok(f.to_string()),
                    Literal::String(s) => {
                        // For now, strings become 0 (would need string runtime)
                        Ok("0".to_string())
                    }
                    Literal::Bool(b) => Ok(if *b { "1" } else { "0" }.to_string()),
                }
            }
            ASTNode::Identifier(name) => Ok(name.clone()),
            _ => Ok("0".to_string()),
        }
    }

    /// Emit runtime support functions
    fn emit_runtime(&mut self) {
        self.emit_line("// Knull Runtime Support");
        self.emit_line("");
    }

    /// Emit line with proper indentation
    fn emit_line(&mut self, line: &str) {
        self.output.push_str(&" ".repeat(self.indent));
        self.output.push_str(line);
        self.output.push('\n');
    }

    /// Get next temporary variable name
    fn next_temp(&mut self) -> String {
        let temp = format!("_t{}", self.temp_count);
        self.temp_count += 1;
        temp
    }
}

/// Compile Knull source to native binary
pub fn compile_to_binary(source: &str, output_path: &str) -> Result<(), String> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    // Parse
    let mut lexer = Lexer::new(source);
    let _tokens = lexer.tokenize();

    let mut parser = Parser::new(source);
    let ast = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    // Generate C code
    let mut codegen = CCodeGen::new();
    let c_code = codegen.compile(&ast)?;

    // Write C file
    let c_path = format!("{}.c", output_path);
    fs::write(&c_path, c_code).map_err(|e| format!("Failed to write C file: {}", e))?;

    // Compile with cc
    let status = Command::new("cc")
        .args(&["-O2", "-o", output_path, &c_path, "-lm"])
        .status()
        .map_err(|e| format!("Failed to run cc: {}", e))?;

    if !status.success() {
        return Err("Compilation failed".to_string());
    }

    // Clean up C file
    let _ = fs::remove_file(&c_path);

    Ok(())
}

/// Compile and run
pub fn compile_and_run(source: &str) -> Result<String, String> {
    let output_path = "/tmp/knull_out";

    compile_to_binary(source, output_path)?;

    // Run the binary
    let output = Command::new(output_path)
        .output()
        .map_err(|e| format!("Failed to run binary: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(format!("Runtime error: {}", stderr));
    }

    Ok(stdout.to_string())
}
