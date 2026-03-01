//! Knull Linear Type Checker
//!
//! Implements linear type checking that enforces exactly-once use of values.
//! Linear types ensure resources are used exactly once, preventing leaks
//! and use-after-free bugs.

use crate::parser::ASTNode;
use crate::parser::Literal;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinearKind {
    Linear,
    Unrestricted,
}

#[derive(Debug, Clone)]
pub struct LinearVar {
    pub name: String,
    pub linear_kind: LinearKind,
    pub usage_count: usize,
    pub consumed: bool,
    pub drop_fn: Option<String>,
}

pub struct LinearChecker {
    scopes: Vec<HashMap<String, LinearVar>>,
    errors: Vec<String>,
    drop_code: Vec<DropInstruction>,
}

#[derive(Debug, Clone)]
pub enum DropInstruction {
    DropValue(String),
    DropField { struct_name: String, field: String },
    Custom(String),
}

impl LinearChecker {
    pub fn new() -> Self {
        LinearChecker {
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
            drop_code: Vec::new(),
        }
    }

    pub fn check(&mut self, ast: &ASTNode) -> Result<(), String> {
        self.check_node(ast)?;
        self.check_unused_values()?;

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
        let scope = self.scopes.pop().expect("No scope available");
        for (_, var) in scope {
            if var.linear_kind == LinearKind::Linear && !var.consumed && var.usage_count > 0 {
                self.errors.push(format!(
                    "Linear value '{}' was not consumed (used {} time(s))",
                    var.name, var.usage_count
                ));
            }
        }
    }

    fn current_scope(&mut self) -> &mut HashMap<String, LinearVar> {
        self.scopes.last_mut().expect("No scope available")
    }

    fn find_variable(&self, name: &str) -> Option<(usize, &LinearVar)> {
        for (i, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(var) = scope.get(name) {
                return Some((i, var));
            }
        }
        None
    }

    fn find_variable_mut(&mut self, name: &str) -> Option<&mut LinearVar> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(name) {
                return Some(var);
            }
        }
        None
    }

    fn declare_linear(&mut self, name: String, linear_kind: LinearKind, drop_fn: Option<String>) {
        let var = LinearVar {
            name: name.clone(),
            linear_kind,
            usage_count: 0,
            consumed: false,
            drop_fn,
        };
        self.current_scope().insert(name, var);
    }

    fn use_variable(&mut self, name: &str) -> bool {
        let should_generate_drop;
        let drop_fn_for_var;

        {
            if let Some(var) = self.find_variable_mut(name) {
                var.usage_count += 1;
                if var.linear_kind == LinearKind::Linear && var.usage_count > 1 {
                    self.errors
                        .push(format!("Linear value '{}' used more than once", name));
                    return false;
                }
                should_generate_drop = var.usage_count == 1;
                drop_fn_for_var = var.drop_fn.clone();
                if should_generate_drop {
                    var.consumed = true;
                }
            } else {
                return false;
            }
        }

        if should_generate_drop {
            if let Some(drop_fn) = drop_fn_for_var {
                self.drop_code.push(DropInstruction::Custom(format!(
                    "call {} for {}",
                    drop_fn, name
                )));
            } else {
                self.drop_code
                    .push(DropInstruction::DropValue(name.to_string()));
            }
        }

        true
    }

    fn generate_drop(&mut self, var: &LinearVar) {
        if let Some(drop_fn) = &var.drop_fn {
            self.drop_code.push(DropInstruction::Custom(format!(
                "call {} for {}",
                drop_fn, var.name
            )));
        } else {
            self.drop_code
                .push(DropInstruction::DropValue(var.name.clone()));
        }
    }

    fn check_unused_values(&self) -> Result<(), String> {
        for scope in &self.scopes {
            for (name, var) in scope {
                if var.linear_kind == LinearKind::Linear && var.usage_count == 0 && !var.consumed {
                    return Err(format!(
                        "Linear variable '{}' declared but never used",
                        name
                    ));
                }
            }
        }
        Ok(())
    }

    fn check_node(&mut self, node: &ASTNode) -> Result<(), String> {
        match node {
            ASTNode::Program(items) => {
                for item in items {
                    self.check_node(item)?;
                }
                Ok(())
            }
            ASTNode::Function {
                name: _,
                params,
                body,
                ret_type: _,
            } => {
                self.push_scope();
                for param in params {
                    self.declare_linear(param.name.clone(), LinearKind::Linear, None);
                }
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
            ASTNode::Let { name, value, ty } => {
                self.check_node(value)?;
                let is_linear = self.is_linear_type(ty.as_ref());
                self.declare_linear(
                    name.clone(),
                    if is_linear {
                        LinearKind::Linear
                    } else {
                        LinearKind::Unrestricted
                    },
                    self.infer_drop_fn(ty.as_ref()),
                );
                Ok(())
            }
            ASTNode::While { cond, body } => {
                self.check_node(cond)?;
                self.check_node(body)?;
                Ok(())
            }
            ASTNode::If {
                cond,
                then_body,
                else_body,
            } => {
                self.check_node(cond)?;
                self.check_node(then_body)?;
                if let Some(else_branch) = else_body {
                    self.check_node(else_branch)?;
                }
                Ok(())
            }
            ASTNode::Return(expr) => {
                self.check_node(expr)?;
                Ok(())
            }
            ASTNode::Binary { op: _, left, right } => {
                self.check_node(left)?;
                self.check_node(right)?;
                Ok(())
            }
            ASTNode::Unary { op: _, operand } => {
                self.check_node(operand)?;
                Ok(())
            }
            ASTNode::Call { func, args } => {
                self.check_node(func)?;
                for arg in args {
                    self.check_node(arg)?;
                }
                Ok(())
            }
            ASTNode::MethodCall {
                obj,
                method: _,
                args,
            } => {
                self.check_node(obj)?;
                for arg in args {
                    self.check_node(arg)?;
                }
                Ok(())
            }
            ASTNode::Index { obj, index } => {
                self.check_node(obj)?;
                self.check_node(index)?;
                Ok(())
            }
            ASTNode::FieldAccess { obj, field: _ } => {
                self.check_node(obj)?;
                Ok(())
            }
            ASTNode::Identifier(name) => {
                if self.use_variable(name) {
                    Ok(())
                } else {
                    Ok(())
                }
            }
            ASTNode::Literal(_) => Ok(()),
            ASTNode::Array(items) => {
                for item in items {
                    self.check_node(item)?;
                }
                Ok(())
            }
            ASTNode::Consume(expr) => {
                self.check_node(expr)?;
                if let ASTNode::Identifier(name) = expr.as_ref() {
                    if let Some(var) = self.find_variable_mut(name) {
                        if var.usage_count > 0 {
                            self.errors
                                .push(format!("Cannot consume '{}': value already used", name));
                        } else {
                            let drop_fn = var.drop_fn.clone();
                            let var_name = var.name.clone();
                            var.consumed = true;
                            drop_fn.map(|f| {
                                self.drop_code.push(DropInstruction::Custom(format!(
                                    "call {} for {}",
                                    f, var_name
                                )))
                            });
                        }
                    }
                }
                Ok(())
            }
            ASTNode::LinearExpr(inner, linear_kind) => {
                self.push_scope();
                self.check_node(inner)?;
                self.pop_scope();
                let _ = linear_kind;
                Ok(())
            }
            ASTNode::EffectAnnotation { expr, effects: _ } => {
                self.check_node(expr)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn is_linear_type(&self, ty: Option<&crate::parser::Type>) -> bool {
        if let Some(t) = ty {
            matches!(
                t,
                crate::parser::Type::Ref(_)
                    | crate::parser::Type::MutRef(_)
                    | crate::parser::Type::RawPtr(_)
                    | crate::parser::Type::MutRawPtr(_)
            )
        } else {
            false
        }
    }

    fn infer_drop_fn(&self, ty: Option<&crate::parser::Type>) -> Option<String> {
        if let Some(t) = ty {
            match t {
                crate::parser::Type::Custom(name) => Some(format!("drop_{}", name)),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn get_drop_code(&self) -> &[DropInstruction] {
        &self.drop_code
    }
}

impl Default for LinearChecker {
    fn default() -> Self {
        Self::new()
    }
}
