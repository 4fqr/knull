//! Knull Ownership System
//!
//! Implements the ownership and borrowing rules for memory safety
//! without garbage collection. This is used in Expert and God modes.

use crate::parser::ASTNode;
use std::collections::HashMap;

/// Ownership status of a value
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ownership {
    Owned,
    Borrowed,
    MutBorrowed,
    Moved,
}

/// Ownership context for a scope
#[derive(Debug, Clone)]
pub struct OwnershipScope {
    variables: HashMap<String, Ownership>,
}

impl OwnershipScope {
    pub fn new() -> Self {
        OwnershipScope {
            variables: HashMap::new(),
        }
    }

    pub fn declare(&mut self, name: &str, ownership: Ownership) {
        self.variables.insert(name.to_string(), ownership);
    }

    pub fn get(&self, name: &str) -> Option<Ownership> {
        self.variables.get(name).copied()
    }

    pub fn set(&mut self, name: &str, ownership: Ownership) {
        self.variables.insert(name.to_string(), ownership);
    }
}

/// Ownership checker
pub struct OwnershipChecker {
    scopes: Vec<OwnershipScope>,
    errors: Vec<String>,
}

impl OwnershipChecker {
    pub fn new() -> Self {
        OwnershipChecker {
            scopes: vec![OwnershipScope::new()],
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
        self.scopes.push(OwnershipScope::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn current_scope(&mut self) -> &mut OwnershipScope {
        self.scopes.last_mut().expect("No scope available")
    }

    fn find_variable(&self, name: &str) -> Option<(usize, Ownership)> {
        for (i, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(ownership) = scope.get(name) {
                return Some((i, ownership));
            }
        }
        None
    }

    fn check_node(&mut self, node: &ASTNode) -> Result<(), String> {
        match node {
            ASTNode::Program(items) => {
                for item in items {
                    self.check_node(item)?;
                }
                Ok(())
            }
            ASTNode::Function { name: _, body } => {
                self.push_scope();
                self.check_node(body)?;
                self.pop_scope();
                Ok(())
            }
            ASTNode::Block(stmts) => {
                self.push_scope();
                for stmt in stmts {
                    self.check_node(stmt)?;
                }
                self.pop_scope();
                Ok(())
            }
            ASTNode::Let { name, value } => {
                let ownership = self.check_expr(value);
                match ownership {
                    Ownership::Moved | Ownership::Owned => {
                        self.current_scope().declare(name, Ownership::Owned);
                    }
                    _ => {
                        self.errors
                            .push(format!("Cannot bind borrowed value to {}", name));
                    }
                }
                Ok(())
            }
            ASTNode::While { cond, body } => {
                self.check_expr(cond);
                self.check_node(body)?;
                Ok(())
            }
            ASTNode::If {
                cond,
                then_body,
                else_body,
            } => {
                self.check_expr(cond);
                self.check_node(then_body)?;
                if let Some(else_branch) = else_body {
                    self.check_node(else_branch)?;
                }
                Ok(())
            }
            ASTNode::Call { func: _, args } => {
                for arg in args {
                    self.check_expr(arg);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn check_expr(&mut self, expr: &ASTNode) -> Ownership {
        match expr {
            ASTNode::Identifier(name) => {
                if let Some((scope_idx, ownership)) = self.find_variable(name) {
                    match ownership {
                        Ownership::Owned => {
                            // Move the value
                            if scope_idx == self.scopes.len() - 1 {
                                self.current_scope().set(name, Ownership::Moved);
                            }
                            Ownership::Moved
                        }
                        Ownership::Borrowed => Ownership::Borrowed,
                        Ownership::MutBorrowed => Ownership::MutBorrowed,
                        Ownership::Moved => {
                            self.errors.push(format!("Use of moved value: {}", name));
                            Ownership::Moved
                        }
                    }
                } else {
                    // Assume it's a function or constant
                    Ownership::Owned
                }
            }
            ASTNode::Literal(_) => Ownership::Owned,
            ASTNode::Binary { left, right, .. } => {
                self.check_expr(left);
                self.check_expr(right);
                Ownership::Owned
            }
            ASTNode::Unary { operand, .. } => self.check_expr(operand),
            ASTNode::Call { args, .. } => {
                for arg in args {
                    self.check_expr(arg);
                }
                Ownership::Owned
            }
            ASTNode::Index { obj, index } => {
                self.check_expr(obj);
                self.check_expr(index);
                Ownership::Owned
            }
            _ => Ownership::Owned,
        }
    }
}
