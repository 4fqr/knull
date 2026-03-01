//! Knull Effect System
//!
//! Implements effect tracking for the language, including:
//! - Effect types (IO, Mutation, Panic, Async, etc.)
//! - Effect polymorphism
//! - Effect inference
//! - Effect handlers

use crate::parser::ASTNode;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    IO,
    Mutation,
    Panic,
    Async,
    NonDeterminism,
    Divergence,
    RefRead,
    RefWrite,
    Custom(String),
}

impl Effect {
    pub fn name(&self) -> String {
        match self {
            Effect::IO => "IO".to_string(),
            Effect::Mutation => "Mutation".to_string(),
            Effect::Panic => "Panic".to_string(),
            Effect::Async => "Async".to_string(),
            Effect::NonDeterminism => "NonDeterminism".to_string(),
            Effect::Divergence => "Divergence".to_string(),
            Effect::RefRead => "RefRead".to_string(),
            Effect::RefWrite => "RefWrite".to_string(),
            Effect::Custom(name) => name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectSet {
    effects: Vec<Effect>,
}

impl EffectSet {
    pub fn new() -> Self {
        EffectSet {
            effects: Vec::new(),
        }
    }

    pub fn pure() -> Self {
        EffectSet {
            effects: Vec::new(),
        }
    }

    pub fn io() -> Self {
        EffectSet {
            effects: vec![Effect::IO],
        }
    }

    pub fn mutation() -> Self {
        EffectSet {
            effects: vec![Effect::Mutation],
        }
    }

    pub fn add(&mut self, effect: Effect) {
        if !self.effects.contains(&effect) {
            self.effects.push(effect);
        }
    }

    pub fn union(&mut self, other: &EffectSet) {
        for effect in &other.effects {
            self.add(effect.clone());
        }
    }

    pub fn contains(&self, effect: &Effect) -> bool {
        self.effects.contains(effect)
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }
}

impl Default for EffectSet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub params: Vec<EffectSet>,
    pub return_effects: EffectSet,
}

impl FunctionSignature {
    pub fn new() -> Self {
        FunctionSignature {
            params: Vec::new(),
            return_effects: EffectSet::new(),
        }
    }

    pub fn pure() -> Self {
        FunctionSignature {
            params: Vec::new(),
            return_effects: EffectSet::pure(),
        }
    }
}

impl Default for FunctionSignature {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EffectChecker {
    scopes: Vec<HashMap<String, FunctionSignature>>,
    errors: Vec<String>,
    current_effects: EffectSet,
    effect_vars: HashMap<String, EffectSet>,
}

impl EffectChecker {
    pub fn new() -> Self {
        let mut effect_vars = HashMap::new();
        effect_vars.insert("println".to_string(), EffectSet::io());
        effect_vars.insert("print".to_string(), EffectSet::io());
        effect_vars.insert("read_line".to_string(), EffectSet::io());
        effect_vars.insert("open".to_string(), EffectSet::io());
        effect_vars.insert("read".to_string(), EffectSet::io());
        effect_vars.insert("write".to_string(), EffectSet::io());
        effect_vars.insert("close".to_string(), EffectSet::io());

        EffectChecker {
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
            current_effects: EffectSet::new(),
            effect_vars,
        }
    }

    pub fn check(&mut self, ast: &ASTNode) -> Result<EffectSet, String> {
        let effects = self.check_node(ast)?;

        if self.errors.is_empty() {
            Ok(effects)
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

    fn check_node(&mut self, node: &ASTNode) -> Result<EffectSet, String> {
        match node {
            ASTNode::Program(items) => {
                let mut result = EffectSet::new();
                for item in items {
                    let effects = self.check_node(item)?;
                    result.union(&effects);
                }
                Ok(result)
            }
            ASTNode::Function {
                name,
                params,
                ret_type: _,
                body,
            } => {
                self.push_scope();
                let mut sig = FunctionSignature::new();
                for param in params {
                    sig.params.push(EffectSet::new());
                }

                let body_effects = self.check_node(body)?;
                sig.return_effects = body_effects;

                self.scopes.last_mut().unwrap().insert(name.clone(), sig);

                self.pop_scope();
                Ok(EffectSet::new())
            }
            ASTNode::Block(stmts) => {
                self.push_scope();
                let mut result = EffectSet::new();
                for stmt in stmts {
                    let effects = self.check_node(stmt)?;
                    result.union(&effects);
                }
                self.pop_scope();
                Ok(result)
            }
            ASTNode::Let {
                name: _,
                value,
                ty: _,
            } => self.check_node(value),
            ASTNode::While { cond, body } => {
                let cond_effects = self.check_node(cond)?;
                let body_effects = self.check_node(body)?;
                let mut result = EffectSet::new();
                result.union(&cond_effects);
                result.union(&body_effects);
                Ok(result)
            }
            ASTNode::If {
                cond,
                then_body,
                else_body,
            } => {
                let cond_effects = self.check_node(cond)?;
                let then_effects = self.check_node(then_body)?;
                let mut result = EffectSet::new();
                result.union(&cond_effects);
                result.union(&then_effects);
                if let Some(else_branch) = else_body {
                    let else_effects = self.check_node(else_branch)?;
                    result.union(&else_effects);
                }
                Ok(result)
            }
            ASTNode::Return(expr) => self.check_node(expr),
            ASTNode::Binary { op: _, left, right } => {
                let left_effects = self.check_node(left)?;
                let right_effects = self.check_node(right)?;
                let mut result = EffectSet::new();
                result.union(&left_effects);
                result.union(&right_effects);
                Ok(result)
            }
            ASTNode::Unary { op, operand } => {
                if op == "&" || op == "&mut" {
                    let mut result = EffectSet::new();
                    result.add(Effect::RefWrite);
                    self.check_node(operand)?;
                    Ok(result)
                } else {
                    self.check_node(operand)
                }
            }
            ASTNode::Call { func, args } => {
                let mut result = EffectSet::new();
                self.check_node(func)?;

                for arg in args {
                    let arg_effects = self.check_node(arg)?;
                    result.union(&arg_effects);
                }

                if let ASTNode::Identifier(name) = func.as_ref() {
                    if let Some(fn_effects) = self.effect_vars.get(name) {
                        result.union(fn_effects);
                    } else {
                        for scope in self.scopes.iter().rev() {
                            if let Some(sig) = scope.get(name) {
                                result.union(&sig.return_effects);
                                break;
                            }
                        }
                    }
                }

                Ok(result)
            }
            ASTNode::MethodCall { obj, method, args } => {
                let mut result = EffectSet::new();
                let obj_effects = self.check_node(obj)?;
                result.union(&obj_effects);

                for arg in args {
                    let arg_effects = self.check_node(arg)?;
                    result.union(&arg_effects);
                }

                if *method == "push" || *method == "pop" || *method == "insert" {
                    result.add(Effect::Mutation);
                } else if *method == "read" || *method == "write" {
                    result.add(Effect::IO);
                }

                Ok(result)
            }
            ASTNode::Identifier(_) => Ok(EffectSet::pure()),
            ASTNode::Literal(_) => Ok(EffectSet::pure()),
            ASTNode::EffectAnnotation { expr, effects } => {
                let mut result = EffectSet::new();
                self.check_node(expr)?;
                for effect in effects {
                    let eff = match effect {
                        crate::parser::Effect::IO => Effect::IO,
                        crate::parser::Effect::Mutation => Effect::Mutation,
                        crate::parser::Effect::Panic => Effect::Panic,
                        crate::parser::Effect::Async => Effect::Async,
                        crate::parser::Effect::NonDeterminism => Effect::NonDeterminism,
                        crate::parser::Effect::Divergence => Effect::Divergence,
                        crate::parser::Effect::Custom(name) => Effect::Custom(name.clone()),
                    };
                    result.add(eff);
                }
                Ok(result)
            }
            ASTNode::LinearExpr(inner, _) => self.check_node(inner),
            ASTNode::Consume(expr) => self.check_node(expr),
            _ => Ok(EffectSet::pure()),
        }
    }

    pub fn infer_effects(&mut self, ast: &ASTNode) -> EffectSet {
        self.check_node(ast).unwrap_or(EffectSet::new())
    }
}

impl Default for EffectChecker {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_effect_annotation(effect_str: &str) -> Vec<Effect> {
    let mut effects = Vec::new();
    for part in effect_str.split(',') {
        let part = part.trim();
        let effect = match part.to_lowercase().as_str() {
            "io" => Effect::IO,
            "mutation" | "mut" => Effect::Mutation,
            "panic" => Effect::Panic,
            "async" => Effect::Async,
            "nondet" | "nondeterminism" => Effect::NonDeterminism,
            "divergence" => Effect::Divergence,
            "read" => Effect::RefRead,
            "write" => Effect::RefWrite,
            _ if !part.is_empty() => Effect::Custom(part.to_string()),
            _ => continue,
        };
        effects.push(effect);
    }
    effects
}
