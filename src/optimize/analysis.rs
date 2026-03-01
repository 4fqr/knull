//! Program Analysis for Optimization
//!
//! Provides analysis utilities needed for optimization passes.

use crate::ast::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct FunctionStats {
    pub instruction_count: usize,
    pub call_count: usize,
    pub branch_count: usize,
    pub loop_count: usize,
    pub memory_ops: usize,
    pub arithmetic_ops: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CallGraph {
    callers: HashMap<String, HashSet<String>>,
    callees: HashMap<String, HashSet<String>>,
}

impl CallGraph {
    pub fn new() -> Self {
        CallGraph {
            callers: HashMap::new(),
            callees: HashMap::new(),
        }
    }

    pub fn add_edge(&mut self, caller: String, callee: String) {
        self.callers
            .entry(callee.clone())
            .or_default()
            .insert(caller);
        self.callees.entry(caller).or_default().insert(callee);
    }

    pub fn get_callers(&self, func: &str) -> HashSet<String> {
        self.callers.get(func).cloned().unwrap_or_default()
    }

    pub fn get_callees(&self, func: &str) -> HashSet<String> {
        self.callees.get(func).cloned().unwrap_or_default()
    }
}

pub struct ControlFlowAnalysis {
    pub dominators: HashMap<String, HashSet<String>>,
    pub post_dominators: HashMap<String, HashSet<String>>,
    pub immediate_dominators: HashMap<String, String>,
}

impl ControlFlowAnalysis {
    pub fn new() -> Self {
        ControlFlowAnalysis {
            dominators: HashMap::new(),
            post_dominators: HashMap::new(),
            immediate_dominators: HashMap::new(),
        }
    }

    pub fn analyze_function(&mut self, _func: &Function) {}
}

impl Default for ControlFlowAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MemoryAnalysis {
    pub allocs: HashMap<String, AllocInfo>,
    pub aliasing: HashSet<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct AllocInfo {
    pub size: usize,
    pub alignment: usize,
    pub is_stack: bool,
    pub escape_points: Vec<String>,
}

impl Default for MemoryAnalysis {
    fn default() -> Self {
        MemoryAnalysis {
            allocs: HashMap::new(),
            aliasing: HashSet::new(),
        }
    }
}

pub struct TypeAnalysis {
    pub type_map: HashMap<String, Type>,
    pub size_map: HashMap<String, usize>,
}

impl TypeAnalysis {
    pub fn new() -> Self {
        TypeAnalysis {
            type_map: HashMap::new(),
            size_map: HashMap::new(),
        }
    }

    pub fn compute_size(&self, ty: &Type) -> usize {
        match ty {
            Type::Int(_) | Type::Float(_) => 8,
            Type::Bool => 1,
            Type::Char => 4,
            Type::String => 8,
            Type::Pointer(_) => 8,
            Type::Array(ty, size) => self.compute_size(ty) * size,
            Type::Struct(fields) => fields.iter().map(|(_, t)| self.compute_size(t)).sum(),
            _ => 8,
        }
    }
}

impl Default for TypeAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct LoopInfo {
    pub header: String,
    pub latch: String,
    pub blocks: Vec<String>,
    pub trip_count: Option<usize>,
    pub is_invariant: Vec<String>,
}

impl LoopInfo {
    pub fn new() -> Self {
        LoopInfo {
            header: String::new(),
            latch: String::new(),
            blocks: Vec::new(),
            trip_count: None,
            is_invariant: Vec::new(),
        }
    }

    pub fn can_unroll(&self, max_trip: usize) -> bool {
        self.trip_count.map(|c| c <= max_trip).unwrap_or(false)
    }
}

impl Default for LoopInfo {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LivenessAnalysis {
    pub live_in: HashMap<String, HashSet<String>>,
    pub live_out: HashMap<String, HashSet<String>>,
}

impl LivenessAnalysis {
    pub fn new() -> Self {
        LivenessAnalysis {
            live_in: HashMap::new(),
            live_out: HashMap::new(),
        }
    }

    pub fn compute(&mut self, _func: &Function) {}
}

impl Default for LivenessAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

pub fn collect_functions(program: &Program) -> HashMap<String, &Function> {
    let mut functions = HashMap::new();

    for item in &program.items {
        if let Item::Function(func) = item {
            functions.insert(func.name.clone(), func);
        }
    }

    functions
}

pub fn collect_calls(expr: &Expr) -> Vec<String> {
    let mut calls = Vec::new();
    collect_calls_rec(expr, &mut calls);
    calls
}

fn collect_calls_rec(expr: &Expr, calls: &mut Vec<String>) {
    match expr {
        Expr::Call { func, args, .. } => {
            if let Expr::Ident(name, _) = **func {
                calls.push(name);
            }
            for arg in args {
                collect_calls_rec(arg, calls);
            }
        }
        Expr::MethodCall { args, .. } => {
            for arg in args {
                collect_calls_rec(arg, calls);
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_calls_rec(left, calls);
            collect_calls_rec(right, calls);
        }
        Expr::Unary { expr, .. } => collect_calls_rec(expr, calls),
        Expr::Ternary {
            condition,
            then,
            else_,
            ..
        } => {
            collect_calls_rec(condition, calls);
            collect_calls_rec(then, calls);
            collect_calls_rec(else_, calls);
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_calls_rec(condition, calls);
            collect_calls_rec(then_branch, calls);
            if let Some(e) = else_branch {
                collect_calls_rec(e, calls);
            }
        }
        Expr::Block(block) => {
            for stmt in &block.stmts {
                collect_stmt_calls(stmt, calls);
            }
        }
        _ => {}
    }
}

fn collect_stmt_calls(stmt: &Stmt, calls: &mut Vec<String>) {
    match stmt {
        Stmt::Expr(expr) => collect_calls_rec(expr, calls),
        Stmt::Let { init, .. } | Stmt::Var { init, .. } => {
            if let Some(e) = init {
                collect_calls_rec(e, calls);
            }
        }
        Stmt::Return { value } => {
            if let Some(e) = value {
                collect_calls_rec(e, calls);
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            collect_calls_rec(condition, calls);
            collect_stmt_calls(body, calls);
        }
        Stmt::For { iter, body, .. } => {
            collect_calls_rec(iter, calls);
            collect_stmt_calls(body, calls);
        }
        Stmt::Unsafe { block } => collect_stmt_calls(block, calls),
        Stmt::Defer { expr } => collect_calls_rec(expr, calls),
        _ => {}
    }
}

pub fn is_pure_function(name: &str, program: &Program) -> bool {
    let pure_functions = [
        "abs", "min", "max", "len", "size", "pow", "sqrt", "sin", "cos", "tan", "log", "exp",
        "floor", "ceil", "round", "trunc", "fabs", "fmod", "strlen", "strcmp",
    ];

    if pure_functions.contains(&name) {
        return true;
    }

    for item in &program.items {
        if let Item::Function(func) = item {
            if func.name == name {
                return is_function_pure(&func);
            }
        }
    }

    false
}

fn is_function_pure(func: &Function) -> bool {
    for stmt in &func.body.stmts {
        if !stmt_is_pure(stmt) {
            return false;
        }
    }
    true
}

fn stmt_is_pure(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expr(expr) => expr_is_pure(expr),
        Stmt::Let { init, .. } => init.as_ref().map(expr_is_pure).unwrap_or(true),
        Stmt::Var { init, .. } => init.as_ref().map(expr_is_pure).unwrap_or(true),
        Stmt::Return { value } => value.as_ref().map(expr_is_pure).unwrap_or(true),
        Stmt::While {
            condition, body, ..
        } => expr_is_pure(condition) && stmt_is_pure(body),
        Stmt::For { iter, body, .. } => expr_is_pure(iter) && stmt_is_pure(body),
        Stmt::Break(_) | Stmt::Continue(_) => true,
        Stmt::Unsafe { block } => stmt_is_pure(block),
        Stmt::Defer { .. } => false,
        Stmt::Asm { .. } | Stmt::Syscall { .. } => false,
        Stmt::Comptime { block } => stmt_is_pure(block),
    }
}

fn expr_is_pure(expr: &Expr) -> bool {
    match expr {
        Expr::Literal(_, _) | Expr::Ident(_, _) => true,
        Expr::Binary { left, right, .. } => expr_is_pure(left) && expr_is_pure(right),
        Expr::Unary { expr, .. } => expr_is_pure(expr),
        Expr::Ternary {
            condition,
            then,
            else_,
            ..
        } => expr_is_pure(condition) && expr_is_pure(then) && expr_is_pure(else_),
        Expr::Block(block) => block.stmts.iter().all(stmt_is_pure),
        Expr::Call { func, args, .. } => {
            if let Expr::Ident(name, _) = **func {
                matches!(
                    name.as_str(),
                    "abs" | "min" | "max" | "len" | "pow" | "sqrt"
                ) && args.iter().all(expr_is_pure)
            } else {
                false
            }
        }
        _ => false,
    }
}
