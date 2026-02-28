//! Knull Interpreter - Execute Knull code directly

use crate::parser::{ASTNode, Literal};
use std::collections::HashMap;

struct Runtime {
    variables: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

impl Value {
    fn to_string(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
        }
    }
}

pub fn execute(ast: &ASTNode) {
    let mut runtime = Runtime {
        variables: HashMap::new(),
    };

    run_program(&mut runtime, ast);
}

fn run_program(runtime: &mut Runtime, ast: &ASTNode) {
    match ast {
        ASTNode::Program(items) => {
            for item in items {
                if let ASTNode::Function { name, body } = item {
                    if name == "main" {
                        run_block(runtime, body);
                        return;
                    }
                }
            }
            for item in items {
                run_node(runtime, item);
            }
            return;
        }
        _ => {
            run_node(runtime, ast);
        }
    }
}

fn run_node(runtime: &mut Runtime, node: &ASTNode) -> Option<Value> {
    match node {
        ASTNode::Function { name, body } => {
            if name == "main" {
                run_block(runtime, body);
            }
            None
        }

        ASTNode::Block(stmts) => {
            for stmt in stmts {
                run_node(runtime, stmt);
            }
            None
        }

        ASTNode::Let { name, value } => {
            let val = eval_expr(runtime, value);
            if let Some(v) = val {
                runtime.variables.insert(name.clone(), v);
            }
            None
        }

        ASTNode::Call { func, args } => match func.as_str() {
            "println" | "print" => {
                if !args.is_empty() {
                    if let Some(val) = eval_expr(runtime, &args[0]) {
                        println!("{}", val.to_string());
                    } else {
                        println!("");
                    }
                } else {
                    println!("");
                }
                None
            }
            _ => None,
        },

        ASTNode::Literal(lit) => match lit {
            Literal::Int(n) => Some(Value::Int(*n)),
            Literal::Float(f) => Some(Value::Float(*f)),
            Literal::String(s) => Some(Value::String(s.clone())),
        },

        _ => None,
    }
}

fn run_block(runtime: &mut Runtime, node: &ASTNode) {
    match node {
        ASTNode::Block(stmts) => {
            let mut i = 0;
            while i < stmts.len() {
                let stmt = &stmts[i];

                // Handle identifier followed by expression as function call
                if let ASTNode::Identifier(name) = stmt {
                    if i + 1 < stmts.len() {
                        let next = &stmts[i + 1];
                        if let ASTNode::Literal(_) = next {
                            // Treat as function call
                            let mut args = vec![next.clone()];
                            run_node(
                                runtime,
                                &ASTNode::Call {
                                    func: name.clone(),
                                    args,
                                },
                            );
                            i += 2;
                            continue;
                        }
                    }
                }

                run_node(runtime, stmt);
                i += 1;
            }
        }
        _ => {
            run_node(runtime, node);
        }
    }
}

fn eval_expr(runtime: &mut Runtime, expr: &ASTNode) -> Option<Value> {
    match expr {
        ASTNode::Literal(lit) => match lit {
            Literal::Int(n) => Some(Value::Int(*n)),
            Literal::Float(f) => Some(Value::Float(*f)),
            Literal::String(s) => Some(Value::String(s.clone())),
        },

        ASTNode::Identifier(name) => {
            if let Some(val) = runtime.variables.get(name) {
                return Some(val.clone());
            }
            if name == "true" {
                return Some(Value::Bool(true));
            }
            if name == "false" {
                return Some(Value::Bool(false));
            }
            None
        }

        ASTNode::Binary { op, left, right } => {
            let l = eval_expr(runtime, left);
            let r = eval_expr(runtime, right);

            match (l, r) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => {
                    let result = match op.as_str() {
                        "+" => Value::Int(a + b),
                        "-" => Value::Int(a - b),
                        "*" => Value::Int(a * b),
                        "/" => Value::Int(a / b),
                        "%" => Value::Int(a % b),
                        "==" => Value::Bool(a == b),
                        "!=" => Value::Bool(a != b),
                        "<" => Value::Bool(a < b),
                        ">" => Value::Bool(a > b),
                        "<=" => Value::Bool(a <= b),
                        ">=" => Value::Bool(a >= b),
                        _ => Value::Int(0),
                    };
                    Some(result)
                }
                (Some(Value::String(a)), Some(Value::String(b))) => {
                    let result = match op.as_str() {
                        "+" => Value::String(a + &b),
                        "==" => Value::Bool(a == b),
                        "!=" => Value::Bool(a != b),
                        _ => Value::String(a),
                    };
                    Some(result)
                }
                _ => None,
            }
        }

        _ => None,
    }
}

pub fn compile(source: &str) -> Result<String, String> {
    use crate::parser::parse;
    let ast = parse(source)?;
    Ok("Parsed".to_string())
}
