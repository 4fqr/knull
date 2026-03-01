//! Knull Type System
//!
//! Implements static type checking with inference for the Expert and God modes.
//! Includes linear types, effect inference, and capability types.

use crate::effects::{Effect, EffectChecker, EffectSet};
use crate::linear_check::{LinearChecker, LinearKind};
use crate::parser::ASTNode;
use crate::parser::Literal;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Void,
    Null,
    Unknown,
    Linear(Box<Type>),
    Capability(CapabilityType),
    EffectType(EffectSet),
    Resource(ResourceType),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityType {
    pub name: String,
    pub permissions: Vec<String>,
    pub resource: Option<Box<Type>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceType {
    pub name: String,
    pub drop_fn: Option<String>,
}

pub struct TypeChecker {
    scopes: Vec<HashMap<String, Type>>,
    errors: Vec<String>,
    linear_checker: LinearChecker,
    effect_checker: EffectChecker,
    enable_linear: bool,
    enable_effects: bool,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
            linear_checker: LinearChecker::new(),
            effect_checker: EffectChecker::new(),
            enable_linear: false,
            enable_effects: false,
        }
    }

    pub fn with_linear_types(enable: bool) -> Self {
        TypeChecker {
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
            linear_checker: LinearChecker::new(),
            effect_checker: EffectChecker::new(),
            enable_linear: enable,
            enable_effects: false,
        }
    }

    pub fn with_effects(enable: bool) -> Self {
        TypeChecker {
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
            linear_checker: LinearChecker::new(),
            effect_checker: EffectChecker::new(),
            enable_linear: false,
            enable_effects: enable,
        }
    }

    pub fn with_all_features() -> Self {
        TypeChecker {
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
            linear_checker: LinearChecker::new(),
            effect_checker: EffectChecker::new(),
            enable_linear: true,
            enable_effects: true,
        }
    }

    pub fn check(&mut self, ast: &ASTNode) -> Result<(), String> {
        self.check_node(ast)?;

        if self.enable_linear {
            if let Err(e) = self.linear_checker.check(ast) {
                self.errors.push(e);
            }
        }

        if self.enable_effects {
            if let Err(e) = self.effect_checker.check(ast) {
                self.errors.push(e);
            }
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.join("\n"))
        }
    }

    pub fn get_linear_drop_code(&self) -> &[crate::linear_check::DropInstruction] {
        self.linear_checker.get_drop_code()
    }

    pub fn infer_effects(&mut self, ast: &ASTNode) -> EffectSet {
        self.effect_checker.infer_effects(ast)
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
                ..
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
            ASTNode::Let { name, value, ty } => {
                let val_type = self.check_node(value)?;
                // TODO: Check against declared type if present
                let _ = ty;
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
                let func_name = match func.as_ref() {
                    ASTNode::Identifier(name) => name.as_str(),
                    _ => return Ok(Type::Unknown),
                };
                match func_name {
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
                Literal::Null => Ok(Type::Null),
            },
            ASTNode::Array(_) => Ok(Type::Unknown),
            ASTNode::Index { obj: _, index: _ } => Ok(Type::Unknown),
            ASTNode::Consume(expr) => {
                self.check_node(expr)?;
                Ok(Type::Void)
            }
            ASTNode::LinearExpr(inner, kind) => {
                self.check_node(inner)?;
                match kind {
                    crate::parser::LinearKind::Linear => Ok(Type::Linear(Box::new(Type::Unknown))),
                    _ => Ok(Type::Unknown),
                }
            }
            ASTNode::EffectAnnotation { expr, effects } => {
                self.check_node(expr)?;
                let mut effect_set = EffectSet::new();
                for effect in effects {
                    match effect {
                        crate::parser::Effect::IO => effect_set.add(crate::effects::Effect::IO),
                        crate::parser::Effect::Mutation => {
                            effect_set.add(crate::effects::Effect::Mutation)
                        }
                        crate::parser::Effect::Panic => {
                            effect_set.add(crate::effects::Effect::Panic)
                        }
                        crate::parser::Effect::Async => {
                            effect_set.add(crate::effects::Effect::Async)
                        }
                        crate::parser::Effect::NonDeterminism => {
                            effect_set.add(crate::effects::Effect::NonDeterminism)
                        }
                        crate::parser::Effect::Divergence => {
                            effect_set.add(crate::effects::Effect::Divergence)
                        }
                        crate::parser::Effect::Custom(name) => {
                            effect_set.add(crate::effects::Effect::Custom(name.clone()))
                        }
                    }
                }
                Ok(Type::EffectType(effect_set))
            }
            ASTNode::Capability {
                name,
                resource,
                permissions,
            } => {
                if let Some(res) = resource {
                    self.check_node(res)?;
                }
                let _ = permissions;
                Ok(Type::Capability(CapabilityType {
                    name: name.clone(),
                    permissions: permissions.iter().map(|p| format!("{:?}", p)).collect(),
                    resource: None,
                }))
            }
            _ => Ok(Type::Unknown),
        }
    }
}
