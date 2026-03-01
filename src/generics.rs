//! Knull Advanced Generics & Traits System
//!
//! Implements:
//! - Generic type parameters parsing and resolution
//! - Trait bounds and where clauses
//! - Type inference with unification
//! - Monomorphization for compile-time code generation
//! - Trait objects and dynamic dispatch
//! - Associated types and generic defaults

use crate::ast::{
    Block, Expr, Function, ImplBlock, Item, Param, Span, Stmt, StructDef, TraitDef, Type, TypeParam,
};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum GenericType {
    Param(TypeParam),
    Concrete(Type),
    Primitive(PrimitiveType),
    Function(Box<GenericType>, Box<GenericType>),
    Tuple(Vec<GenericType>),
    Array(Box<GenericType>, usize),
    Slice(Box<GenericType>),
    Option(Box<GenericType>),
    Result(Box<GenericType>, Box<GenericType>),
    Pointer(Box<GenericType>, bool),
    Reference(Box<GenericType>, bool),
    TraitObject(Vec<TraitRef>),
    Never,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct TraitRef {
    pub name: String,
    pub type_args: Vec<GenericType>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum PrimitiveType {
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    Bool,
    Char,
    String,
    Never,
    Void,
}

impl From<Type> for GenericType {
    fn from(t: Type) -> Self {
        match t {
            Type::Name(s) => GenericType::Concrete(Type::Name(s)),
            Type::Generic(name, args) => {
                if args.is_empty() {
                    GenericType::Param(TypeParam {
                        name,
                        bounds: vec![],
                    })
                } else {
                    GenericType::Concrete(Type::Generic(name, args))
                }
            }
            Type::Pointer(ty, mutable) => GenericType::Pointer(Box::new((*ty).into()), mutable),
            Type::Reference(ty, mutable) => GenericType::Reference(Box::new((*ty).into()), mutable),
            Type::Array(ty, size) => GenericType::Array(Box::new((*ty).into()), size),
            Type::Slice(ty) => GenericType::Slice(Box::new((*ty).into())),
            Type::Tuple(types) => GenericType::Tuple(types.into_iter().map(|t| t.into()).collect()),
            Type::Function(params, ret) => {
                let ret_type = ret.map(|b| Box::new((*b).into()));
                GenericType::Function(
                    Box::new(GenericType::Tuple(
                        params.into_iter().map(|t| t.into()).collect(),
                    )),
                    Box::new(ret_type.map(|b| *b).unwrap_or(GenericType::Unknown)),
                )
            }
            Type::Option(ty) => GenericType::Option(Box::new((*ty).into())),
            Type::Sum(a, b) => GenericType::Result(Box::new((*a).into()), Box::new((*b).into())),
            Type::Never => GenericType::Never,
            Type::Void => GenericType::Void,
        }
    }
}

impl From<GenericType> for Type {
    fn from(t: GenericType) -> Self {
        match t {
            GenericType::Param(p) => Type::Name(p.name),
            GenericType::Concrete(c) => c,
            GenericType::Primitive(p) => match p {
                PrimitiveType::I32 => Type::Name("i32".to_string()),
                PrimitiveType::Bool => Type::Name("bool".to_string()),
                PrimitiveType::String => Type::String,
                _ => Type::Name(format!("{:?}", p).to_lowercase()),
            },
            GenericType::Function(params, ret) => {
                let params: Vec<Type> = match *params {
                    GenericType::Tuple(ts) => ts.into_iter().map(|t| t.into()).collect(),
                    _ => vec![(*params).into()],
                };
                let ret = Box::new((*ret).into());
                Type::Function(params, Some(ret))
            }
            GenericType::Tuple(ts) => Type::Tuple(ts.into_iter().map(|t| t.into()).collect()),
            GenericType::Array(ty, size) => Type::Array(Box::new((*ty).into()), size),
            GenericType::Slice(ty) => Type::Slice(Box::new((*ty).into())),
            GenericType::Option(ty) => Type::Option(Box::new((*ty).into())),
            GenericType::Result(a, b) => Type::Sum(Box::new((*a).into()), Box::new((*b).into())),
            GenericType::Pointer(ty, mutbl) => Type::Pointer(Box::new((*ty).into()), mutbl),
            GenericType::Reference(ty, mutbl) => Type::Reference(Box::new((*ty).into()), mutbl),
            GenericType::TraitObject(_) => Type::Name("dyn Trait".to_string()),
            GenericType::Never => Type::Never,
            GenericType::Unknown => Type::Name("unknown".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WhereClause {
    pub predicates: Vec<WherePredicate>,
}

#[derive(Debug, Clone)]
pub struct WherePredicate {
    pub type_param: String,
    pub bounds: Vec<TraitBound>,
}

#[derive(Debug, Clone)]
pub enum TraitBound {
    Trait(TraitRef),
    Lifetime(String),
    Eq(GenericType),
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct Substitution {
    pub type_param: String,
    pub concrete_type: GenericType,
}

pub struct GenericResolver {
    type_params: HashMap<String, TypeParam>,
    substitutions: Vec<Substitution>,
    trait_bounds: HashMap<String, Vec<TraitBound>>,
    where_clauses: Vec<WhereClause>,
    monomorphizations: HashMap<String, GenericFunction>,
    trait_impls: HashMap<String, TraitImpl>,
    associated_types: HashMap<String, HashMap<String, GenericType>>,
}

#[derive(Debug, Clone)]
pub struct GenericFunction {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub return_type: Option<GenericType>,
    pub body: Block,
    pub where_clause: Option<WhereClause>,
    pub span: Span,
    pub specialized: HashMap<Vec<GenericType>, GenericFunction>,
}

#[derive(Debug, Clone)]
pub struct TraitImpl {
    pub trait_ref: TraitRef,
    pub for_type: GenericType,
    pub items: Vec<ImplItem>,
}

#[derive(Debug, Clone)]
pub enum ImplItem {
    Method(GenericFunction),
    AssociatedType(String, GenericType),
    AssociatedConst(String, GenericType, Option<Expr>),
}

#[derive(Debug, Clone, Default)]
pub struct TypeInference {
    constraints: Vec<TypeConstraint>,
    substitutions: HashMap<String, GenericType>,
}

#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub expected: GenericType,
    pub actual: GenericType,
    pub origin: ConstraintOrigin,
}

#[derive(Debug, Clone)]
pub enum ConstraintOrigin {
    Assignment {
        var: String,
        span: Span,
    },
    FunctionCall {
        func: String,
        arg_index: usize,
        span: Span,
    },
    Return {
        expected: GenericType,
        span: Span,
    },
    BinaryOp {
        op: String,
        span: Span,
    },
    Cast {
        from: GenericType,
        to: GenericType,
        span: Span,
    },
    TraitBound {
        trait_name: String,
        type_param: String,
        span: Span,
    },
}

impl GenericResolver {
    pub fn new() -> Self {
        GenericResolver {
            type_params: HashMap::new(),
            substitutions: Vec::new(),
            trait_bounds: HashMap::new(),
            where_clauses: Vec::new(),
            monomorphizations: HashMap::new(),
            trait_impls: HashMap::new(),
            associated_types: HashMap::new(),
        }
    }

    pub fn with_std_traits() -> Self {
        let mut resolver = Self::new();
        resolver.register_std_traits();
        resolver
    }

    fn register_std_traits(&mut self) {
        self.trait_impls.insert(
            "Clone".to_string(),
            TraitImpl {
                trait_ref: TraitRef {
                    name: "Clone".to_string(),
                    type_args: vec![],
                    span: Span::default(),
                },
                for_type: GenericType::Unknown,
                items: vec![],
            },
        );
        self.trait_impls.insert(
            "Copy".to_string(),
            TraitImpl {
                trait_ref: TraitRef {
                    name: "Copy".to_string(),
                    type_args: vec![],
                    span: Span::default(),
                },
                for_type: GenericType::Unknown,
                items: vec![],
            },
        );
        self.trait_impls.insert(
            "Debug".to_string(),
            TraitImpl {
                trait_ref: TraitRef {
                    name: "Debug".to_string(),
                    type_args: vec![],
                    span: Span::default(),
                },
                for_type: GenericType::Unknown,
                items: vec![],
            },
        );
        self.trait_impls.insert(
            "Display".to_string(),
            TraitImpl {
                trait_ref: TraitRef {
                    name: "Display".to_string(),
                    type_args: vec![],
                    span: Span::default(),
                },
                for_type: GenericType::Unknown,
                items: vec![],
            },
        );
        self.trait_impls.insert(
            "Add".to_string(),
            TraitImpl {
                trait_ref: TraitRef {
                    name: "Add".to_string(),
                    type_args: vec![],
                    span: Span::default(),
                },
                for_type: GenericType::Unknown,
                items: vec![],
            },
        );
    }

    pub fn add_type_param(&mut self, param: TypeParam) {
        self.type_params.insert(param.name.clone(), param);
    }

    pub fn add_trait_bound(&mut self, type_param: &str, bound: TraitBound) {
        self.trait_bounds
            .entry(type_param.to_string())
            .or_insert_with(Vec::new)
            .push(bound);
    }

    pub fn add_where_clause(&mut self, clause: WhereClause) {
        self.where_clauses.push(clause);
    }

    pub fn add_trait_impl(&mut self, impl_block: ImplBlock) -> Result<(), String> {
        let type_name = match &impl_block.type_name {
            Type::Name(n) => n.clone(),
            Type::Generic(n, _) => n.clone(),
            _ => return Err("Invalid type in impl block".to_string()),
        };

        let trait_ref = impl_block.trait_name.map(|name| TraitRef {
            name,
            type_args: vec![],
            span: impl_block.span,
        });

        let items: Vec<ImplItem> = impl_block
            .items
            .into_iter()
            .map(|item| match item {
                crate::ast::ImplItem::Function(f) => ImplItem::Method(GenericFunction {
                    name: f.name,
                    type_params: f.type_params,
                    params: f.params,
                    return_type: f.return_type.map(|t| t.into()),
                    body: f.body,
                    where_clause: None,
                    span: f.span,
                    specialized: HashMap::new(),
                }),
                crate::ast::ImplItem::Const(c) => {
                    ImplItem::AssociatedConst(c.name, c.ty.into(), Some(c.value))
                }
            })
            .collect();

        let key = if let Some(ref tr) = trait_ref {
            format!("{} for {}", tr.name, type_name)
        } else {
            type_name
        };

        self.trait_impls.insert(
            key,
            TraitImpl {
                trait_ref: trait_ref.unwrap_or(TraitRef {
                    name: type_name.clone(),
                    type_args: vec![],
                    span: impl_block.span,
                }),
                for_type: GenericType::Concrete(Type::Name(type_name)),
                items,
            },
        );

        Ok(())
    }

    pub fn resolve_type(&self, ty: &Type) -> GenericType {
        ty.clone().into()
    }

    pub fn substitute_type(&self, ty: GenericType) -> GenericType {
        match ty {
            GenericType::Param(p) => {
                for sub in &self.substitutions {
                    if sub.type_param == p.name {
                        return sub.concrete_type.clone();
                    }
                }
                GenericType::Param(p)
            }
            GenericType::Function(params, ret) => GenericType::Function(
                Box::new(self.substitute_type(*params)),
                Box::new(self.substitute_type(*ret)),
            ),
            GenericType::Tuple(types) => {
                GenericType::Tuple(types.into_iter().map(|t| self.substitute_type(t)).collect())
            }
            GenericType::Array(ty, size) => {
                GenericType::Array(Box::new(self.substitute_type(*ty)), size)
            }
            GenericType::Slice(ty) => GenericType::Slice(Box::new(self.substitute_type(*ty))),
            GenericType::Option(ty) => GenericType::Option(Box::new(self.substitute_type(*ty))),
            GenericType::Result(a, b) => GenericType::Result(
                Box::new(self.substitute_type(*a)),
                Box::new(self.substitute_type(*b)),
            ),
            GenericType::Pointer(ty, m) => {
                GenericType::Pointer(Box::new(self.substitute_type(*ty)), m)
            }
            GenericType::Reference(ty, m) => {
                GenericType::Reference(Box::new(self.substitute_type(*ty)), m)
            }
            other => other,
        }
    }

    pub fn unify(&mut self, t1: GenericType, t2: GenericType) -> Result<(), String> {
        let (t1, t2) = self.flatten(&t1, &t2);

        match (&t1, &t2) {
            (GenericType::Param(p), _) => {
                self.substitutions.push(Substitution {
                    type_param: p.name.clone(),
                    concrete_type: t2,
                });
                Ok(())
            }
            (_, GenericType::Param(p)) => {
                self.substitutions.push(Substitution {
                    type_param: p.name.clone(),
                    concrete_type: t1,
                });
                Ok(())
            }
            (GenericType::Unknown, _) => Ok(()),
            (_, GenericType::Unknown) => Ok(()),
            (GenericType::Never, GenericType::Never) => Ok(()),
            (GenericType::Void, GenericType::Void) => Ok(()),
            (GenericType::Primitive(p1), GenericType::Primitive(p2)) if p1 == p2 => Ok(()),
            (GenericType::Tuple(ts1), GenericType::Tuple(ts2)) if ts1.len() == ts2.len() => {
                for (t1, t2) in ts1.iter().zip(ts2.iter()) {
                    self.unify(t1.clone(), t2.clone())?;
                }
                Ok(())
            }
            (GenericType::Array(ty1, s1), GenericType::Array(ty2, s2)) if s1 == s2 => {
                self.unify(*ty1.clone(), *ty2.clone())
            }
            (GenericType::Slice(ty1), GenericType::Slice(ty2)) => {
                self.unify(*ty1.clone(), *ty2.clone())
            }
            (GenericType::Option(ty1), GenericType::Option(ty2)) => {
                self.unify(*ty1.clone(), *ty2.clone())
            }
            (GenericType::Result(a1, b1), GenericType::Result(a2, b2)) => {
                self.unify(*a1.clone(), *a2.clone())?;
                self.unify(*b1.clone(), *b2.clone())
            }
            (GenericType::Function(p1, r1), GenericType::Function(p2, r2)) => {
                self.unify(*p1.clone(), *p2.clone())?;
                self.unify(*r1.clone(), *r2.clone())
            }
            (GenericType::Concrete(Type::Name(n1)), GenericType::Concrete(Type::Name(n2)))
                if n1 == n2 =>
            {
                Ok(())
            }
            (
                GenericType::Concrete(Type::Generic(n1, a1)),
                GenericType::Concrete(Type::Generic(n2, a2)),
            ) if n1 == n2 && a1.len() == a2.len() => {
                for (a1, a2) in a1.iter().zip(a2.iter()) {
                    self.unify(a1.clone().into(), a2.clone().into())?;
                }
                Ok(())
            }
            _ => Err(format!("Cannot unify {:?} with {:?}", t1, t2)),
        }
    }

    fn flatten(&self, t1: &GenericType, t2: &GenericType) -> (GenericType, GenericType) {
        let t1 = self.substitute_type(t1.clone());
        let t2 = self.substitute_type(t2.clone());
        (t1, t2)
    }

    pub fn check_trait_bound(&self, type_param: &str, trait_name: &str) -> bool {
        self.trait_bounds
            .get(type_param)
            .map(|bounds| {
                bounds.iter().any(|b| match b {
                    TraitBound::Trait(tr) => tr.name == trait_name,
                    _ => false,
                })
            })
            .unwrap_or(false)
    }

    pub fn satisfies_bounds(&self, ty: &GenericType, bounds: &[TraitBound]) -> bool {
        for bound in bounds {
            match bound {
                TraitBound::Trait(tr) => {
                    if !self.type_implements_trait(ty, tr) {
                        return false;
                    }
                }
                TraitBound::Eq(expected) => {
                    if ty != expected {
                        return false;
                    }
                }
                TraitBound::Lifetime(_) => {}
            }
        }
        true
    }

    fn type_implements_trait(&self, ty: &GenericType, trait_ref: &TraitRef) -> bool {
        let ty_key = format!("{:?}", ty);

        if self
            .trait_impls
            .contains_key(&format!("{} for {}", trait_ref.name, ty_key))
        {
            return true;
        }

        match (ty, trait_ref.name.as_str()) {
            (GenericType::Primitive(PrimitiveType::I32), "Add")
            | (GenericType::Primitive(PrimitiveType::I64), "Add")
            | (GenericType::Primitive(PrimitiveType::F32), "Add")
            | (GenericType::Primitive(PrimitiveType::F64), "Add") => true,
            (GenericType::Primitive(PrimitiveType::I32), "Clone")
            | (GenericType::Primitive(PrimitiveType::I64), "Clone")
            | (GenericType::Primitive(PrimitiveType::Bool), "Clone") => true,
            (GenericType::Primitive(PrimitiveType::I32), "Copy")
            | (GenericType::Primitive(PrimitiveType::I64), "Copy")
            | (GenericType::Primitive(PrimitiveType::Bool), "Copy") => true,
            _ => false,
        }
    }

    pub fn monomorphize(
        &mut self,
        func: &GenericFunction,
        args: &[GenericType],
    ) -> Result<GenericFunction, String> {
        if func.specialized.contains_key(args) {
            return Ok(func.specialized.get(args).unwrap().clone());
        }

        let mut specialized = func.clone();
        specialized.type_params.clear();

        for (param, arg) in func.type_params.iter().zip(args.iter()) {
            self.substitutions.push(Substitution {
                type_param: param.name.clone(),
                concrete_type: arg.clone(),
            });
        }

        specialized.body = self.monomorphize_block(&func.body);

        if let Some(ref wc) = func.where_clause {
            specialized.where_clause = Some(self.monomorphize_where_clause(wc));
        }

        self.substitutions.clear();

        let result = specialized.clone();
        func.specialized.insert(args.to_vec(), specialized);

        Ok(result)
    }

    fn monomorphize_block(&self, block: &Block) -> Block {
        let stmts: Vec<Stmt> = block
            .stmts
            .iter()
            .map(|s| self.monomorphize_stmt(s))
            .collect();
        Block {
            stmts,
            span: block.span,
        }
    }

    fn monomorphize_stmt(&self, stmt: &Stmt) -> Stmt {
        match stmt {
            Stmt::Expr(e) => Stmt::Expr(self.monomorphize_expr(e)),
            Stmt::Let { pattern, ty, init } => Stmt::Let {
                pattern: pattern.clone(),
                ty: ty.clone(),
                init: init.as_ref().map(|e| Box::new(self.monomorphize_expr(e))),
            },
            Stmt::Return { value } => Stmt::Return {
                value: value.as_ref().map(|e| Box::new(self.monomorphize_expr(e))),
            },
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => Stmt::If {
                cond: Box::new(self.monomorphize_expr(cond)),
                then_body: Box::new(self.monomorphize_stmt(then_body)),
                else_body: else_body
                    .as_ref()
                    .map(|b| Box::new(self.monomorphize_stmt(b))),
            },
            Stmt::While {
                label,
                condition,
                body,
            } => Stmt::While {
                label: label.clone(),
                condition: Box::new(self.monomorphize_expr(condition)),
                body: Box::new(self.monomorphize_stmt(body)),
            },
            Stmt::For {
                pattern,
                iter,
                body,
            } => Stmt::For {
                pattern: pattern.clone(),
                iter: Box::new(self.monomorphize_expr(iter)),
                body: Box::new(self.monomorphize_stmt(body)),
            },
            Stmt::Break { label, value } => Stmt::Break {
                label: label.clone(),
                value: value.as_ref().map(|e| Box::new(self.monomorphize_expr(e))),
            },
            Stmt::Continue { label } => Stmt::Continue {
                label: label.clone(),
            },
            Stmt::Defer { expr } => Stmt::Defer {
                expr: Box::new(self.monomorphize_expr(expr)),
            },
            Stmt::Match { expr, arms } => Stmt::Match {
                expr: Box::new(self.monomorphize_expr(expr)),
                arms: arms.clone(),
            },
            Stmt::Unsafe { block } => Stmt::Unsafe {
                block: Box::new(self.monomorphize_stmt(block)),
            },
            Stmt::Asm { syntax, code } => Stmt::Asm {
                syntax: syntax.clone(),
                code: code.clone(),
            },
            Stmt::Syscall { args } => Stmt::Syscall {
                args: args.iter().map(|e| self.monomorphize_expr(e)).collect(),
            },
            Stmt::Var { pattern, ty, init } => Stmt::Var {
                pattern: pattern.clone(),
                ty: ty.clone(),
                init: init.as_ref().map(|e| Box::new(self.monomorphize_expr(e))),
            },
            Stmt::Comptime { block } => Stmt::Comptime {
                block: Box::new(self.monomorphize_stmt(block)),
            },
        }
    }

    fn monomorphize_expr(&self, expr: &Expr) -> Expr {
        match expr {
            Expr::Literal(l, span) => Expr::Literal(l.clone(), *span),
            Expr::Ident(s, span) => {
                if let Some(sub) = self.substitutions.iter().find(|s| s.type_param == *s) {
                    Expr::Literal(Literal::Ident(sub.concrete_type.to_string()), *span)
                } else {
                    Expr::Ident(s.clone(), *span)
                }
            }
            Expr::Unary { op, expr, span } => Expr::Unary {
                op: op.clone(),
                expr: Box::new(self.monomorphize_expr(expr)),
                span: *span,
            },
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => Expr::Binary {
                op: op.clone(),
                left: Box::new(self.monomorphize_expr(left)),
                right: Box::new(self.monomorphize_expr(right)),
                span: *span,
            },
            Expr::Ternary {
                condition,
                then,
                else_,
                span,
            } => Expr::Ternary {
                condition: Box::new(self.monomorphize_expr(condition)),
                then: Box::new(self.monomorphize_expr(then)),
                else_: Box::new(self.monomorphize_expr(else_)),
                span: *span,
            },
            Expr::Block(b) => Expr::Block(self.monomorphize_block(b)),
            Expr::Call { func, args, span } => Expr::Call {
                func: Box::new(self.monomorphize_expr(func)),
                args: args.iter().map(|a| self.monomorphize_expr(a)).collect(),
                span: *span,
            },
            Expr::MethodCall {
                expr,
                method,
                args,
                span,
            } => Expr::MethodCall {
                expr: Box::new(self.monomorphize_expr(expr)),
                method: method.clone(),
                args: args.iter().map(|a| self.monomorphize_expr(a)).collect(),
                span: *span,
            },
            Expr::Field { expr, field, span } => Expr::Field {
                expr: Box::new(self.monomorphize_expr(expr)),
                field: field.clone(),
                span: *span,
            },
            Expr::Index { expr, index, span } => Expr::Index {
                expr: Box::new(self.monomorphize_expr(expr)),
                index: Box::new(self.monomorphize_expr(index)),
                span: *span,
            },
            Expr::Array(arr, span) => Expr::Array(
                arr.iter().map(|e| self.monomorphize_expr(e)).collect(),
                *span,
            ),
            Expr::Struct { name, fields, span } => Expr::Struct {
                name: name.clone(),
                fields: fields.clone(),
                span: *span,
            },
            Expr::Tuple(exprs, span) => Expr::Tuple(
                exprs.iter().map(|e| self.monomorphize_expr(e)).collect(),
                *span,
            ),
            Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => Expr::If {
                condition: Box::new(self.monomorphize_expr(condition)),
                then_branch: Box::new(self.monomorphize_expr(then_branch)),
                else_branch: else_branch
                    .as_ref()
                    .map(|e| Box::new(self.monomorphize_expr(e))),
                span: *span,
            },
            Expr::Match {
                scrutinee,
                arms,
                span,
            } => Expr::Match {
                scrutinee: Box::new(self.monomorphize_expr(scrutinee)),
                arms: arms.clone(),
                span: *span,
            },
            Expr::Loop { label, body, span } => Expr::Loop {
                label: label.clone(),
                body: Box::new(self.monomorphize_expr(body)),
                span: *span,
            },
            Expr::While {
                label,
                condition,
                body,
                span,
            } => Expr::While {
                label: label.clone(),
                condition: Box::new(self.monomorphize_expr(condition)),
                body: Box::new(self.monomorphize_expr(body)),
                span: *span,
            },
            Expr::For {
                pattern,
                iter,
                body,
                span,
            } => Expr::For {
                pattern: pattern.clone(),
                iter: Box::new(self.monomorphize_expr(iter)),
                body: Box::new(self.monomorphize_expr(body)),
                span: *span,
            },
            _ => expr.clone(),
        }
    }

    fn monomorphize_where_clause(&self, clause: &WhereClause) -> WhereClause {
        WhereClause {
            predicates: clause
                .predicates
                .iter()
                .map(|p| WherePredicate {
                    type_param: p.type_param.clone(),
                    bounds: p.bounds.clone(),
                })
                .collect(),
        }
    }

    pub fn generate_vtable(&self, trait_ref: &TraitRef, ty: &GenericType) -> VTable {
        let key = format!("{} for {:?}", trait_ref.name, ty);

        if let Some(impl_) = self.trait_impls.get(&key) {
            let mut methods = Vec::new();

            for item in &impl_.items {
                if let ImplItem::Method(method) = item {
                    methods.push(MethodInfo {
                        name: method.name.clone(),
                        signature: method.return_type.clone(),
                    });
                }
            }

            return VTable {
                trait_ref: trait_ref.clone(),
                for_type: ty.clone(),
                methods,
            };
        }

        VTable {
            trait_ref: trait_ref.clone(),
            for_type: ty.clone(),
            methods: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct VTable {
    pub trait_ref: TraitRef,
    pub for_type: GenericType,
    pub methods: Vec<MethodInfo>,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub signature: Option<GenericType>,
}

impl Default for GenericResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for GenericType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenericType::Param(p) => write!(f, "{}", p.name),
            GenericType::Concrete(t) => write!(f, "{:?}", t),
            GenericType::Primitive(p) => write!(f, "{:?}", p),
            GenericType::Function(params, ret) => write!(f, "fn({:?}) -> {:?}", params, ret),
            GenericType::Tuple(types) => {
                write!(f, "(")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            GenericType::Array(ty, size) => write!(f, "[{}; {}]", ty, size),
            GenericType::Slice(ty) => write!(f, "[{}]", ty),
            GenericType::Option(ty) => write!(f, "{}?", ty),
            GenericType::Result(a, b) => write!(f, "Result<{}, {}>", a, b),
            GenericType::Pointer(ty, mutable) => {
                if *mutable {
                    write!(f, "*mut {}", ty)
                } else {
                    write!(f, "*const {}", ty)
                }
            }
            GenericType::Reference(ty, mutable) => {
                if *mutable {
                    write!(f, "&mut {}", ty)
                } else {
                    write!(f, "&{}", ty)
                }
            }
            GenericType::TraitObject(trs) => {
                write!(f, "dyn ")?;
                for (i, tr) in trs.iter().enumerate() {
                    if i > 0 {
                        write!(f, " + ")?;
                    }
                    write!(f, "{}", tr.name)?;
                }
                Ok(())
            }
            GenericType::Never => write!(f, "!"),
            GenericType::Unknown => write!(f, "_"),
        }
    }
}

pub struct TraitChecker {
    resolver: GenericResolver,
    method_table: HashMap<String, Vec<MethodResolution>>,
}

#[derive(Debug, Clone)]
pub struct MethodResolution {
    pub trait_name: String,
    pub method_name: String,
    pub impl_type: GenericType,
    pub function: GenericFunction,
}

impl TraitChecker {
    pub fn new() -> Self {
        TraitChecker {
            resolver: GenericResolver::with_std_traits(),
            method_table: HashMap::new(),
        }
    }

    pub fn check_trait_implementation(
        &mut self,
        impl_block: &ImplBlock,
    ) -> Result<(), Vec<TraitError>> {
        let mut errors = Vec::new();

        let trait_ref = if let Some(ref name) = impl_block.trait_name {
            Some(TraitRef {
                name: name.clone(),
                type_args: vec![],
                span: impl_block.span,
            })
        } else {
            None
        };

        if let Some(ref tr) = trait_ref {
            for item in &impl_block.items {
                if let crate::ast::ImplItem::Function(func) = item {
                    if let Err(e) = self.check_method_implementation(tr, func) {
                        errors.push(e);
                    }
                }
            }
        }

        if errors.is_empty() {
            self.resolver.add_trait_impl(impl_block.clone())?;
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_method_implementation(
        &self,
        trait_ref: &TraitRef,
        func: &Function,
    ) -> Result<(), TraitError> {
        Ok(())
    }

    pub fn resolve_method(&self, ty: &GenericType, method_name: &str) -> Option<MethodResolution> {
        let key = format!("{:?}", ty);

        for (k, impl_) in &self.resolver.trait_impls {
            for item in &impl_.items {
                if let ImplItem::Method(m) = item {
                    if m.name == method_name {
                        return Some(MethodResolution {
                            trait_name: impl_.trait_ref.name.clone(),
                            method_name: method_name.to_string(),
                            impl_type: ty.clone(),
                            function: m.clone(),
                        });
                    }
                }
            }
        }

        None
    }
}

#[derive(Debug, Clone)]
pub struct TraitError {
    pub message: String,
    pub span: Span,
}

impl TypeInference {
    pub fn new() -> Self {
        TypeInference {
            constraints: Vec::new(),
            substitutions: HashMap::new(),
        }
    }

    pub fn infer(
        &mut self,
        resolver: &mut GenericResolver,
    ) -> Result<HashMap<String, GenericType>, String> {
        for constraint in &self.constraints {
            resolver.unify(constraint.expected.clone(), constraint.actual.clone())?;
        }

        Ok(self.substitutions.clone())
    }

    pub fn add_constraint(
        &mut self,
        expected: GenericType,
        actual: GenericType,
        origin: ConstraintOrigin,
    ) {
        self.constraints.push(TypeConstraint {
            expected,
            actual,
            origin,
        });
    }

    pub fn infer_function_call(
        &mut self,
        func_name: &str,
        param_types: &[GenericType],
        arg_types: &[GenericType],
        span: Span,
    ) {
        for (i, (param, arg)) in param_types.iter().zip(arg_types.iter()).enumerate() {
            self.add_constraint(
                param.clone(),
                arg.clone(),
                ConstraintOrigin::FunctionCall {
                    func: func_name.to_string(),
                    arg_index: i,
                    span,
                },
            );
        }
    }

    pub fn infer_return(&mut self, expected: GenericType, actual: GenericType, span: Span) {
        self.add_constraint(
            expected,
            actual,
            ConstraintOrigin::Return { expected, span },
        );
    }

    pub fn generalize_type(&self, ty: GenericType, type_params: &[TypeParam]) -> GenericType {
        let mut general = ty;

        for param in type_params {
            if !self.substitutions.contains_key(&param.name) {
                general = self.replace_with_param(general, &param.name);
            }
        }

        general
    }

    fn replace_with_param(&self, ty: GenericType, param_name: &str) -> GenericType {
        if let Some(sub) = self.substitutions.get(param_name) {
            return sub.clone();
        }
        ty
    }
}

pub struct MonomorphizationPass {
    resolver: GenericResolver,
    generated_functions: Vec<GenericFunction>,
    vtables: HashMap<String, VTable>,
}

impl MonomorphizationPass {
    pub fn new() -> Self {
        MonomorphizationPass {
            resolver: GenericResolver::new(),
            generated_functions: Vec::new(),
            vtables: HashMap::new(),
        }
    }

    pub fn run(&mut self, items: &[Item]) -> Result<Vec<Item>, String> {
        let mut result = Vec::new();

        for item in items {
            match item {
                Item::Function(func) => {
                    if !func.type_params.is_empty() {
                        let generic_func = self.convert_function(func);
                        self.process_generic_function(&generic_func, &mut result);
                    } else {
                        result.push(item.clone());
                    }
                }
                Item::Impl(impl_block) => {
                    let processed = self.process_impl_block(impl_block)?;
                    result.push(Item::Impl(processed));
                }
                other => result.push(other.clone()),
            }
        }

        Ok(result)
    }

    fn convert_function(&self, func: &Function) -> GenericFunction {
        GenericFunction {
            name: func.name.clone(),
            type_params: func.type_params.clone(),
            params: func.params.clone(),
            return_type: func.return_type.clone().map(|t| t.into()),
            body: func.body.clone(),
            where_clause: None,
            span: func.span,
            specialized: HashMap::new(),
        }
    }

    fn process_generic_function(&mut self, func: &GenericFunction, result: &mut Vec<Item>) {
        let type_args: Vec<GenericType> = func
            .type_params
            .iter()
            .map(|p| GenericType::Param(p.clone()))
            .collect();

        if let Ok(specialized) = self.resolver.monomorphize(func, &type_args) {
            result.push(Item::Function(Function {
                name: specialized.name,
                type_params: vec![],
                params: specialized.params,
                return_type: specialized.return_type.map(|t| t.into()),
                body: specialized.body,
                span: specialized.span,
            }));
        }
    }

    fn process_impl_block(&mut self, impl_block: &ImplBlock) -> Result<ImplBlock, String> {
        let mut processed = impl_block.clone();

        for item in &mut processed.items {
            if let crate::ast::ImplItem::Function(ref mut func) = item {
                if !func.type_params.is_empty() {
                    func.type_params.clear();
                }
            }
        }

        Ok(processed)
    }

    pub fn generate_vtable(&mut self, trait_ref: &TraitRef, ty: &GenericType) -> VTable {
        self.resolver.generate_vtable(trait_ref, ty)
    }
}

impl Default for TraitChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TypeInference {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MonomorphizationPass {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Ident(String),
    Interpolated(String),
}

impl From<Literal> for GenericType {
    fn from(l: Literal) -> Self {
        match l {
            Literal::Int(_) => GenericType::Primitive(PrimitiveType::I64),
            Literal::Float(_) => GenericType::Primitive(PrimitiveType::F64),
            Literal::String(_) => GenericType::Primitive(PrimitiveType::String),
            Literal::Char(_) => GenericType::Primitive(PrimitiveType::Char),
            Literal::Bool(_) => GenericType::Primitive(PrimitiveType::Bool),
            Literal::Ident(_) => GenericType::Unknown,
            Literal::Interpolated(_) => GenericType::Primitive(PrimitiveType::String),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_identity() {
        let mut resolver = GenericResolver::new();
        resolver.add_type_param(TypeParam {
            name: "T".to_string(),
            bounds: vec![],
        });

        let param = GenericType::Param(TypeParam {
            name: "T".to_string(),
            bounds: vec![],
        });

        let concrete = GenericType::Primitive(PrimitiveType::I32);

        resolver.substitutions.push(Substitution {
            type_param: "T".to_string(),
            concrete_type: concrete.clone(),
        });

        let result = resolver.substitute_type(param);
        assert_eq!(result, concrete);
    }

    #[test]
    fn test_type_unification() {
        let mut resolver = GenericResolver::new();

        let t1 = GenericType::Param(TypeParam {
            name: "T".to_string(),
            bounds: vec![],
        });
        let t2 = GenericType::Primitive(PrimitiveType::I32);

        assert!(resolver.unify(t1, t2).is_ok());
        assert_eq!(resolver.substitutions.len(), 1);
    }

    #[test]
    fn test_trait_bound_check() {
        let mut resolver = GenericResolver::new();

        resolver.add_type_param(TypeParam {
            name: "T".to_string(),
            bounds: vec![],
        });

        resolver.add_trait_bound(
            "T",
            TraitBound::Trait(TraitRef {
                name: "Clone".to_string(),
                type_args: vec![],
                span: Span::default(),
            }),
        );

        assert!(resolver.check_trait_bound("T", "Clone"));
        assert!(!resolver.check_trait_bound("T", "Debug"));
    }

    #[test]
    fn test_generic_type_display() {
        let ty = GenericType::Param(TypeParam {
            name: "T".to_string(),
            bounds: vec![],
        });

        assert_eq!(format!("{}", ty), "T");

        let arr = GenericType::Array(
            Box::new(GenericType::Param(TypeParam {
                name: "T".to_string(),
                bounds: vec![],
            })),
            100,
        );

        assert_eq!(format!("{}", arr), "[T; 100]");

        let opt = GenericType::Option(Box::new(GenericType::Param(TypeParam {
            name: "T".to_string(),
            bounds: vec![],
        })));

        assert_eq!(format!("{}", opt), "T?");
    }

    #[test]
    fn test_monomorphization() {
        let func = GenericFunction {
            name: "identity".to_string(),
            type_params: vec![TypeParam {
                name: "T".to_string(),
                bounds: vec![],
            }],
            params: vec![Param {
                name: "x".to_string(),
                ty: Some(Type::Name("T".to_string())),
                default: None,
            }],
            return_type: Some(GenericType::Param(TypeParam {
                name: "T".to_string(),
                bounds: vec![],
            })),
            body: Block {
                stmts: vec![],
                span: Span::default(),
            },
            where_clause: None,
            span: Span::default(),
            specialized: HashMap::new(),
        };

        let mut resolver = GenericResolver::new();
        let args = vec![GenericType::Primitive(PrimitiveType::I32)];

        let result = resolver.monomorphize(&func, &args);
        assert!(result.is_ok());
    }
}
