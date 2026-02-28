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
        }
        _ => {
            run_node(runtime, ast);
        }
    }
}

fn run_node(runtime: &mut Runtime, node: &ASTNode) -> Option<Value> {
    match node {
        ASTNode::Function { name: _, body } => {
            run_block(runtime, body);
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
            "println" => {
                if !args.is_empty() {
                    let arg = &args[0];
                    if let ASTNode::Identifier(name) = arg {
                        if let Some(val) = runtime.variables.get(name) {
                            println!("{}", val.to_string());
                        } else if name == "true" {
                            println!("true");
                        } else if name == "false" {
                            println!("false");
                        } else {
                            eprintln!("Error: undefined variable '{}'", name);
                        }
                    } else if let Some(val) = eval_expr(runtime, arg) {
                        println!("{}", val.to_string());
                    } else {
                        println!("");
                    }
                } else {
                    println!("");
                }
                None
            }
            "print" => {
                if !args.is_empty() {
                    let arg = &args[0];
                    if let ASTNode::Identifier(name) = arg {
                        if let Some(val) = runtime.variables.get(name) {
                            print!("{}", val.to_string());
                        } else if name == "true" {
                            print!("true");
                        } else if name == "false" {
                            print!("false");
                        } else {
                            eprintln!("Error: undefined variable '{}'", name);
                        }
                    } else if let Some(val) = eval_expr(runtime, arg) {
                        print!("{}", val.to_string());
                    }
                }
                None
            }
            _ => None,
        },

        ASTNode::If {
            cond,
            then_body,
            else_body,
        } => {
            let cond_val = eval_expr(runtime, cond);
            if let Some(Value::Bool(true)) = cond_val {
                run_block(runtime, then_body);
            } else if let Some(Value::Bool(false)) = cond_val {
                if let Some(else_block) = else_body {
                    run_block(runtime, else_block);
                }
            }
            None
        }

        ASTNode::While { cond, body } => {
            loop {
                let cond_val = eval_expr(runtime, cond);
                match cond_val {
                    Some(Value::Bool(true)) => run_block(runtime, body),
                    Some(Value::Bool(false)) => break,
                    _ => break,
                }
            }
            None
        }

        ASTNode::Return(expr) => eval_expr(runtime, expr),

        ASTNode::Literal(_) | ASTNode::Identifier(_) | ASTNode::Binary { .. } => {
            eval_expr(runtime, node);
            None
        }

        _ => None,
    }
}

fn run_block(runtime: &mut Runtime, node: &ASTNode) {
    match node {
        ASTNode::Block(stmts) => {
            for stmt in stmts {
                run_node(runtime, stmt);
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
                Some(val.clone())
            } else if name == "true" {
                Some(Value::Bool(true))
            } else if name == "false" {
                Some(Value::Bool(false))
            } else {
                None
            }
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
                (Some(Value::Int(a)), Some(Value::String(b))) => {
                    let result = match op.as_str() {
                        "+" => Value::String(a.to_string() + &b),
                        _ => Value::String(a.to_string()),
                    };
                    Some(result)
                }
                (Some(Value::String(a)), Some(Value::Int(b))) => {
                    let result = match op.as_str() {
                        "+" => Value::String(a + &b.to_string()),
                        _ => Value::String(a),
                    };
                    Some(result)
                }
                _ => None,
            }
        }

        ASTNode::Call { func, args } => match func.as_str() {
            "len" => {
                if !args.is_empty() {
                    if let Some(val) = eval_expr(runtime, &args[0]) {
                        return Some(match val {
                            Value::String(s) => Value::Int(s.len() as i64),
                            _ => Value::Int(0),
                        });
                    }
                }
                Some(Value::Int(0))
            }
            _ => None,
        },

        _ => None,
    }
}

pub fn compile(source: &str) -> Result<String, String> {
    use crate::parser::parse;
    let ast = parse(source)?;
    Ok("Parsed".to_string())
}
