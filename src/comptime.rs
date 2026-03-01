//! Knull Compile-Time Execution Module
//!
//! Implements the #run directive for compile-time code execution.

use crate::interpreter::{execute_source, Value};
use crate::parser::{parse, ASTNode};

#[derive(Debug, Clone)]
pub struct CompileTimeResult {
    pub value: Option<Value>,
    pub output: String,
    pub error: Option<String>,
}

pub struct CompileTimeExecutor {
    captured_output: String,
}

impl CompileTimeExecutor {
    pub fn new() -> Self {
        CompileTimeExecutor {
            captured_output: String::new(),
        }
    }

    pub fn execute_block(&mut self, source: &str) -> CompileTimeResult {
        match execute_source(source) {
            Ok(_) => CompileTimeResult {
                value: None,
                output: self.captured_output.clone(),
                error: None,
            },
            Err(e) => CompileTimeResult {
                value: None,
                output: self.captured_output.clone(),
                error: Some(e),
            },
        }
    }

    pub fn execute_and_embed(&mut self, source: &str) -> Result<EmbeddedValue, String> {
        let ast = parse(source).map_err(|e| format!("Parse error: {}", e))?;
        self.evaluate_ast(&ast)
    }

    fn evaluate_ast(&mut self, ast: &ASTNode) -> Result<EmbeddedValue, String> {
        match ast {
            ASTNode::Block(stmts) => {
                for stmt in stmts {
                    self.evaluate_ast(stmt)?;
                }
                Ok(EmbeddedValue::Unit)
            }
            ASTNode::Let { name, value, .. } => {
                let val = self.eval_expr(value)?;
                let embedded = value_to_embedded(&val);
                Ok(EmbeddedValue::Variable(name.clone(), Box::new(embedded)))
            }
            ASTNode::Call { func, args } => {
                let func_name = match func.as_ref() {
                    ASTNode::Identifier(n) => n.clone(),
                    _ => return Err("Only simple function calls supported in #run".to_string()),
                };

                let arg_values: Result<Vec<Value>, String> =
                    args.iter().map(|a| self.eval_expr(a)).collect();

                self.run_builtin_function(&func_name, arg_values?)
            }
            _ => Ok(EmbeddedValue::Unit),
        }
    }

    fn eval_expr(&mut self, expr: &ASTNode) -> Result<Value, String> {
        match expr {
            ASTNode::Literal(lit) => Ok(match lit {
                crate::parser::Literal::Int(i) => Value::Int(*i),
                crate::parser::Literal::Float(f) => Value::Float(*f),
                crate::parser::Literal::String(s) => Value::String(s.clone()),
                crate::parser::Literal::Bool(b) => Value::Bool(*b),
                crate::parser::Literal::Null => Value::Null,
            }),
            ASTNode::Identifier(name) => Ok(Value::String(name.clone())),
            ASTNode::Binary { op, left, right } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                self.apply_binary_op(op, l, r)
            }
            _ => Ok(Value::Null),
        }
    }

    fn apply_binary_op(&self, op: &str, left: Value, right: Value) -> Result<Value, String> {
        match op {
            "+" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l + r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
                (Value::String(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
                _ => Ok(Value::String(format!("{}{}", left, right))),
            },
            "-" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l - r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
                _ => Err("Cannot subtract non-numeric types".to_string()),
            },
            "*" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l * r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
                _ => Err("Cannot multiply non-numeric types".to_string()),
            },
            "/" => match (&left, &right) {
                (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l / r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l / r)),
                _ => Err("Cannot divide non-numeric types".to_string()),
            },
            _ => Err(format!("Unsupported operator: {}", op)),
        }
    }

    fn run_builtin_function(
        &mut self,
        name: &str,
        args: Vec<Value>,
    ) -> Result<EmbeddedValue, String> {
        match name {
            "println" | "print" => {
                if let Some(arg) = args.first() {
                    self.captured_output.push_str(&format!("{}", arg));
                    if name == "println" {
                        self.captured_output.push('\n');
                    }
                }
                Ok(EmbeddedValue::Unit)
            }
            "file_read" => {
                if let Some(arg) = args.first() {
                    let path = format!("{}", arg);
                    match std::fs::read_to_string(&path) {
                        Ok(contents) => Ok(EmbeddedValue::String(contents)),
                        Err(e) => Err(format!("Failed to read file: {}", e)),
                    }
                } else {
                    Err("file_read requires a path argument".to_string())
                }
            }
            "file_exists" => {
                if let Some(arg) = args.first() {
                    let path = format!("{}", arg);
                    Ok(EmbeddedValue::Bool(std::path::Path::new(&path).exists()))
                } else {
                    Err("file_exists requires a path argument".to_string())
                }
            }
            "len" => {
                if let Some(arg) = args.first() {
                    let len = match arg {
                        Value::String(s) => s.len() as i64,
                        Value::Array(arr) => arr.len() as i64,
                        _ => return Err("len() requires a string or array".to_string()),
                    };
                    Ok(EmbeddedValue::Int(len))
                } else {
                    Err("len() requires an argument".to_string())
                }
            }
            _ => Ok(EmbeddedValue::Unit),
        }
    }
}

fn value_to_embedded(val: &Value) -> EmbeddedValue {
    match val {
        Value::Int(i) => EmbeddedValue::Int(*i),
        Value::Float(f) => EmbeddedValue::Float(*f),
        Value::Bool(b) => EmbeddedValue::Bool(*b),
        Value::String(s) => EmbeddedValue::String(s.clone()),
        Value::Array(arr) => EmbeddedValue::String(format!("{:?}", arr)),
        Value::Null => EmbeddedValue::Unit,
    }
}

impl Default for CompileTimeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum EmbeddedValue {
    Unit,
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Variable(String, Box<EmbeddedValue>),
}

impl EmbeddedValue {
    pub fn to_rust_value(&self) -> String {
        match self {
            EmbeddedValue::Unit => "()".to_string(),
            EmbeddedValue::Int(i) => i.to_string(),
            EmbeddedValue::Float(f) => f.to_string(),
            EmbeddedValue::Bool(b) => b.to_string(),
            EmbeddedValue::String(s) => format!("\"{}\"", s),
            EmbeddedValue::Variable(name, val) => {
                format!("let {} = {};", name, val.to_rust_value())
            }
        }
    }

    pub fn is_serializable(&self) -> bool {
        matches!(
            self,
            EmbeddedValue::Unit
                | EmbeddedValue::Int(_)
                | EmbeddedValue::Float(_)
                | EmbeddedValue::Bool(_)
                | EmbeddedValue::String(_)
        )
    }
}

pub fn run_compile_time(source: &str) -> CompileTimeResult {
    let mut executor = CompileTimeExecutor::new();
    executor.execute_block(source)
}

pub fn process_run_blocks(ast: &ASTNode) -> Vec<CompileTimeResult> {
    let mut results = Vec::new();
    collect_run_blocks(ast, &mut results);
    results
}

fn collect_run_blocks(node: &ASTNode, results: &mut Vec<CompileTimeResult>) {
    match node {
        ASTNode::CompileTimeRun(block) => {
            let source = node_to_source(block.as_ref());
            let mut executor = CompileTimeExecutor::new();
            results.push(executor.execute_block(&source));
        }
        ASTNode::Program(items) => {
            for item in items {
                collect_run_blocks(item, results);
            }
        }
        ASTNode::Block(stmts) => {
            for stmt in stmts {
                collect_run_blocks(stmt, results);
            }
        }
        ASTNode::Function { body, .. } => {
            collect_run_blocks(body, results);
        }
        ASTNode::If {
            then_body,
            else_body,
            ..
        } => {
            collect_run_blocks(then_body, results);
            if let Some(else_node) = else_body {
                collect_run_blocks(else_node, results);
            }
        }
        ASTNode::While { body, .. } => {
            collect_run_blocks(body, results);
        }
        ASTNode::For { body, .. } => {
            collect_run_blocks(body, results);
        }
        ASTNode::Loop(body) => {
            collect_run_blocks(body, results);
        }
        ASTNode::AsyncFunction { body, .. } => {
            collect_run_blocks(body, results);
        }
        _ => {}
    }
}

fn node_to_source(node: &ASTNode) -> String {
    match node {
        ASTNode::Block(stmts) => stmts
            .iter()
            .map(|s| node_to_source(s))
            .collect::<Vec<_>>()
            .join("\n"),
        ASTNode::Let { name, value, .. } => {
            format!("let {} = {};", name, node_to_source(value))
        }
        ASTNode::Return(expr) => {
            format!("return {};", node_to_source(expr))
        }
        ASTNode::Call { func, args } => {
            let func_name = match func.as_ref() {
                ASTNode::Identifier(n) => n.clone(),
                _ => "unknown".to_string(),
            };
            let args_str = args
                .iter()
                .map(|a| node_to_source(a))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({})", func_name, args_str)
        }
        ASTNode::Identifier(s) => s.clone(),
        ASTNode::Literal(lit) => match lit {
            crate::parser::Literal::Int(i) => i.to_string(),
            crate::parser::Literal::Float(f) => f.to_string(),
            crate::parser::Literal::String(s) => format!("\"{}\"", s),
            crate::parser::Literal::Bool(b) => b.to_string(),
            crate::parser::Literal::Null => "null".to_string(),
        },
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_block_execution() {
        let source = "println \"Hello from compile-time!\"";
        let mut executor = CompileTimeExecutor::new();
        let result = executor.execute_block(source);
        assert!(result.error.is_none());
        assert!(result.output.contains("Hello from compile-time"));
    }

    #[test]
    fn test_file_read() {
        let source = "file_read(\"test.txt\")";
        let mut executor = CompileTimeExecutor::new();
        let _ = executor.execute_block(source);
    }
}
