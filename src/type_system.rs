//! Knull Type System
//!
//! Implements static type checking with inference for the Expert and God modes.

use crate::parser::ASTNode;
use crate::parser::Literal;
use std::collections::HashMap;

/// Type information
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Void,
    Unknown,
}

/// Type checker
pub struct TypeChecker {
    scopes: Vec<HashMap<String, Type>>,
    errors: Vec<String>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
        }
    }

    pub fn check(&mut self, ast: &ASTNode) -> Result<(), String> {
        self.check_node(ast)?;

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.join("\n"))
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn current_scope(&mut self) -> &mut HashMap<String, Type> {
        self.scopes.last_mut().expect("No scope available")
    }

    fn find_variable(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn check_node(&mut self, node: &ASTNode) -> Result<Type, String> {
        match node {
            ASTNode::Program(items) => {
                for item in items {
                    self.check_node(item)?;
                }
                Ok(Type::Void)
            }
            ASTNode::Function {
                name: _,
                params: _,
                body,
            } => {
                self.push_scope();
                self.check_node(body)?;
                self.pop_scope();
                Ok(Type::Void)
            }
            ASTNode::Block(stmts) => {
                self.push_scope();
                let mut last_type = Type::Void;
                for stmt in stmts {
                    last_type = self.check_node(stmt)?;
                }
                self.pop_scope();
                Ok(last_type)
            }
            ASTNode::Let { name, value } => {
                let val_type = self.check_node(value)?;
                self.current_scope().insert(name.clone(), val_type);
                Ok(Type::Void)
            }
            ASTNode::While { cond, body } => {
                let cond_type = self.check_node(cond)?;
                if cond_type != Type::Bool && cond_type != Type::Int {
                    self.errors
                        .push("While condition must be boolean".to_string());
                }
                self.check_node(body)?;
                Ok(Type::Void)
            }
            ASTNode::If {
                cond,
                then_body,
                else_body,
            } => {
                let cond_type = self.check_node(cond)?;
                if cond_type != Type::Bool && cond_type != Type::Int {
                    self.errors.push("If condition must be boolean".to_string());
                }
                let then_type = self.check_node(then_body)?;
                if let Some(else_branch) = else_body {
                    let _ = self.check_node(else_branch)?;
                }
                Ok(then_type)
            }
            ASTNode::Return(expr) => self.check_node(expr),
            ASTNode::Binary { op, left, right } => {
                let left_type = self.check_node(left)?;
                let right_type = self.check_node(right)?;

                match op.as_str() {
                    "+" | "-" | "*" | "/" | "%" => {
                        if left_type == Type::Int && right_type == Type::Int {
                            Ok(Type::Int)
                        } else if left_type == Type::Float || right_type == Type::Float {
                            Ok(Type::Float)
                        } else {
                            self.errors.push(format!(
                                "Cannot apply {} to {:?} and {:?}",
                                op, left_type, right_type
                            ));
                            Ok(Type::Unknown)
                        }
                    }
                    "==" | "!=" | "<" | ">" | "<=" | ">=" => Ok(Type::Bool),
                    "&&" | "||" => Ok(Type::Bool),
                    _ => Ok(Type::Unknown),
                }
            }
            ASTNode::Unary { op, operand } => {
                let operand_type = self.check_node(operand)?;
                match op.as_str() {
                    "-" => Ok(operand_type),
                    "!" => Ok(Type::Bool),
                    _ => Ok(Type::Unknown),
                }
            }
            ASTNode::Call { func, args } => {
                for arg in args {
                    self.check_node(arg)?;
                }
                // Return type depends on function
                match func.as_str() {
                    "println" | "print" => Ok(Type::Void),
                    _ => Ok(Type::Unknown),
                }
            }
            ASTNode::Identifier(name) => self
                .find_variable(name)
                .ok_or_else(|| format!("Unknown variable: {}", name)),
            ASTNode::Literal(lit) => match lit {
                Literal::Int(_) => Ok(Type::Int),
                Literal::Float(_) => Ok(Type::Float),
                Literal::String(_) => Ok(Type::String),
                Literal::Bool(_) => Ok(Type::Bool),
            },
            ASTNode::Array(_) => Ok(Type::Unknown),
            ASTNode::Index { obj: _, index: _ } => Ok(Type::Unknown),
            _ => Ok(Type::Unknown),
        }
    }
}
