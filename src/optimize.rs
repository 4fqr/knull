//! Knull Advanced Compiler Optimizations
//!
//! This module implements sophisticated compiler optimizations including:
//! - LTO (Link-Time Optimization)
//! - SIMD Vectorization
//! - Inline Expansion
//! - Constant Folding
//! - Dead Code Elimination
//! - Loop Optimizations
//! - Escape Analysis

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "simd")]
use std::arch::x86_64::*;

use crate::ast::*;

pub mod analysis;
pub mod constants;
pub mod loops;
pub mod simd;

pub use analysis::*;
pub use constants::*;
pub use loops::*;
pub use simd::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    None = 0,
    Less = 1,
    Default = 2,
    Aggressive = 3,
}

impl OptLevel {
    pub fn from_u32(level: u32) -> Self {
        match level {
            0 => OptLevel::None,
            1 => OptLevel::Less,
            2 => OptLevel::Default,
            _ => OptLevel::Aggressive,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptimizeOptions {
    pub level: OptLevel,
    pub lto: bool,
    pub simd: bool,
    pub inline_threshold: usize,
    pub max_unroll: usize,
    pub cpu_features: CpuFeatures,
}

impl Default for OptimizeOptions {
    fn default() -> Self {
        OptimizeOptions {
            level: OptLevel::Default,
            lto: false,
            simd: true,
            inline_threshold: 10,
            max_unroll: 4,
            cpu_features: CpuFeatures::detect(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CpuFeatures {
    pub has_avx: bool,
    pub has_avx2: bool,
    pub has_avx512f: bool,
    pub has_fma: bool,
    pub has_sse4_1: bool,
    pub has_sse4_2: bool,
}

impl CpuFeatures {
    pub fn detect() -> Self {
        #[cfg(feature = "simd")]
        {
            CpuFeatures {
                has_avx: is_x86_feature_detected!("avx"),
                has_avx2: is_x86_feature_detected!("avx2"),
                has_avx512f: is_x86_feature_detected!("avx512f"),
                has_fma: is_x86_feature_detected!("fma"),
                has_sse4_1: is_x86_feature_detected!("sse4.1"),
                has_sse4_2: is_x86_feature_detected!("sse4.2"),
            }
        }
        #[cfg(not(feature = "simd"))]
        {
            CpuFeatures::default()
        }
    }

    pub fn vector_width(&self) -> usize {
        if self.has_avx512f {
            64
        } else if self.has_avx {
            32
        } else if self.has_sse4_1 {
            16
        } else {
            8
        }
    }
}

pub trait OptimizationPass: Send + Sync {
    fn name(&self) -> &'static str;
    fn run(&self, program: &mut Program, options: &OptimizeOptions) -> bool;
    fn is_enabled(&self, options: &OptimizeOptions) -> bool;
}

pub struct Optimizer {
    passes: Vec<Box<dyn OptimizationPass>>,
    options: OptimizeOptions,
    stats: OptimizationStats,
}

#[derive(Debug, Clone, Default)]
pub struct OptimizationStats {
    pub functions_inlined: usize,
    pub constants_folded: usize,
    pub dead_functions_removed: usize,
    pub loops_unrolled: usize,
    pub loops_fused: usize,
    pub simd_vectorized: usize,
    pub instructions_removed: usize,
}

impl Optimizer {
    pub fn new(options: OptimizeOptions) -> Self {
        let mut optimizer = Optimizer {
            passes: Vec::new(),
            options,
            stats: OptimizationStats::default(),
        };
        optimizer.register_passes();
        optimizer
    }

    fn register_passes(&mut self) {
        self.passes.push(Box::new(InlineExpansion::new()));
        self.passes.push(Box::new(ConstantFolding::new()));
        self.passes.push(Box::new(DeadCodeElimination::new()));
        self.passes.push(Box::new(LoopUnrolling::new()));
        self.passes.push(Box::new(LoopFusion::new()));
        self.passes.push(Box::new(InvariantCodeMotion::new()));

        if self.options.simd {
            self.passes.push(Box::new(SimdVectorization::new()));
        }

        if self.options.lto {
            self.passes.push(Box::new(CrossModuleInlining::new()));
            self.passes.push(Box::new(CrossModuleDce::new()));
        }
    }

    pub fn optimize(&mut self, program: &mut Program) -> &OptimizationStats {
        if self.options.level == OptLevel::None {
            return &self.stats;
        }

        let mut changed = true;
        let mut iteration = 0;
        let max_iterations = if self.options.level == OptLevel::Aggressive {
            10
        } else {
            5
        };

        while changed && iteration < max_iterations {
            changed = false;
            iteration += 1;

            for pass in &self.passes {
                if pass.is_enabled(&self.options) {
                    if pass.run(program, &self.options) {
                        changed = true;
                    }
                }
            }
        }

        &self.stats
    }

    pub fn stats(&self) -> &OptimizationStats {
        &self.stats
    }
}

#[inline]
pub fn small_function_size(body: &Block) -> usize {
    let mut size = 0;
    for stmt in &body.stmts {
        size += stmt_size(stmt);
    }
    size
}

#[inline]
fn stmt_size(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Expr(expr) => expr_size(expr),
        Stmt::Let { init, .. } => init.as_ref().map(|e| expr_size(e)).unwrap_or(0),
        Stmt::Var { init, .. } => init.as_ref().map(|e| expr_size(e)).unwrap_or(0),
        Stmt::While {
            condition, body, ..
        } => expr_size(condition) + stmt_size(body),
        Stmt::For { iter, body, .. } => expr_size(iter) + stmt_size(body),
        Stmt::Return { value } => value.as_ref().map(|e| expr_size(e)).unwrap_or(0),
        Stmt::Break(_) | Stmt::Continue(_) => 1,
        Stmt::Defer { expr } => expr_size(expr),
        Stmt::Unsafe { block } => stmt_size(block),
        Stmt::Asm { .. } => 10,
        Stmt::Syscall { args } => args.iter().map(expr_size).sum(),
        Stmt::Comptime { block } => stmt_size(block),
    }
}

#[inline]
fn expr_size(expr: &Expr) -> usize {
    match expr {
        Expr::Literal(_, _) => 1,
        Expr::Ident(_, _) => 1,
        Expr::Unary { expr, .. } => 1 + expr_size(expr),
        Expr::Binary { left, right, .. } => 1 + expr_size(left) + expr_size(right),
        Expr::Ternary {
            condition,
            then,
            else_,
            ..
        } => 1 + expr_size(condition) + expr_size(then) + expr_size(else_),
        Expr::Block(block) => block.stmts.iter().map(stmt_size).sum::<usize>() + 1,
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            1 + expr_size(condition)
                + expr_size(then_branch)
                + else_branch.as_ref().map(|e| expr_size(e)).unwrap_or(0)
        }
        Expr::Match {
            scrutinee, arms, ..
        } => expr_size(scrutinee) + arms.iter().map(|a| expr_size(&a.body)).sum::<usize>(),
        Expr::Loop { body, .. } => 1 + expr_size(body),
        Expr::While {
            condition, body, ..
        } => 1 + expr_size(condition) + expr_size(body),
        Expr::For { iter, body, .. } => 1 + expr_size(iter) + expr_size(body),
        Expr::Call { args, .. } => 1 + args.iter().map(expr_size).sum::<usize>(),
        Expr::MethodCall { args, .. } => 1 + args.iter().map(expr_size).sum::<usize>(),
        Expr::Field { expr, .. } => 1 + expr_size(expr),
        Expr::Index { expr, index, .. } => 1 + expr_size(expr) + expr_size(index),
        Expr::Array(arr, _) => arr.iter().map(expr_size).sum::<usize>() + 1,
        Expr::Struct { fields, .. } => 1 + fields.iter().map(|(_, e)| expr_size(e)).sum::<usize>(),
        Expr::Tuple(exprs, _) => exprs.iter().map(expr_size).sum::<usize>() + 1,
        Expr::Lambda { body, .. } => expr_size(body),
        Expr::Closure { body, .. } => expr_size(body),
        Expr::Await(_, _) => 2,
        Expr::Spawn { .. } => 5,
        Expr::Cast { expr, .. } => 1 + expr_size(expr),
        Expr::OffsetOf { .. } => 2,
        Expr::SizeOf { .. } | Expr::AlignOf { .. } => 1,
        Expr::Compound(_, _) => 1,
        Expr::InlineAsm { .. } => 10,
        Expr::Null(_) => 1,
        Expr::Unreachable(_) => 1,
    }
}

pub fn count_instructions(program: &Program) -> usize {
    program.items.iter().map(item_count).sum()
}

fn item_count(item: &Item) -> usize {
    match item {
        Item::Function(f) => stmt_size(&Stmt::Expr(Expr::Block(f.body.clone()))),
        Item::Struct(_) | Item::Enum(_) | Item::Trait(_) => 0,
        Item::Impl(_) | Item::TypeAlias(_) => 0,
        Item::Const(_) | Item::Static(_) => 1,
        Item::Module(m) => m.items.iter().map(item_count).sum(),
        Item::Import(_) => 0,
        Item::Actor(a) => a
            .methods
            .iter()
            .map(|f| stmt_size(&Stmt::Expr(Expr::Block(f.body.clone()))))
            .sum(),
        Item::Stmt(s) => stmt_size(s),
    }
}

pub struct COptimizer {
    options: OptimizeOptions,
}

impl COptimizer {
    pub fn new(options: OptimizeOptions) -> Self {
        COptimizer { options }
    }

    pub fn optimize_c_code(&self, code: &str) -> String {
        let mut optimized = code.to_string();

        if self.options.level != OptLevel::None {
            optimized = self.remove_redundant_casts(&optimized);
            optimized = self.inline_simple_functions(&optimized);
            optimized = self.optimize_memory_ops(&optimized);

            if self.options.simd {
                optimized = self.add_simd_pragmas(&optimized);
            }
        }

        optimized
    }

    fn remove_redundant_casts(&self, code: &str) -> String {
        let mut result = code.to_string();

        result = result.replace("(int)(", "(");
        result = result.replace("(char)(", "(");
        result = result.replace("(void*)(", "(");
        result = result.replace("(int64_t)(", "(");
        result = result.replace("(double)(", "(");

        result
    }

    fn inline_simple_functions(&self, code: &str) -> String {
        if self.options.level == OptLevel::Aggressive {
            let mut result = code.to_string();
            result.push_str("\n#define INLINE inline __attribute__((always_inline))");
            result
        } else {
            code.to_string()
        }
    }

    fn optimize_memory_ops(&self, code: &str) -> String {
        let mut result = code.to_string();

        result = result.replace("memcpy(a, b, sizeof(a))", "a = b");
        result = result.replace("memset(a, 0, sizeof(a))", "a = (typeof(a)){0}");

        result
    }

    fn add_simd_pragmas(&self, code: &str) -> String {
        if !self.options.cpu_features.has_avx && !self.options.cpu_features.has_sse4_1 {
            return code.to_string();
        }

        let mut result = String::new();

        if self.options.cpu_features.has_avx512f {
            result.push_str("#pragma GCC target(\"avx512f,avx512bw,avx512dq,avx512vl\")\n");
        } else if self.options.cpu_features.has_avx2 {
            result.push_str("#pragma GCC target(\"avx2,fma\")\n");
        } else if self.options.cpu_features.has_avx {
            result.push_str("#pragma GCC target(\"avx\")\n");
        }

        result.push_str("#include <immintrin.h>\n\n");
        result.push_str(code);

        result
    }
}

pub fn generate_lto_flags() -> Vec<String> {
    vec![
        "-flto".to_string(),
        "-fuse-ld=lld".to_string(),
        "-Wl,--gc-sections".to_string(),
        "-Wl,--as-needed".to_string(),
    ]
}

pub fn generate_opt_flags(level: OptLevel) -> Vec<String> {
    let mut flags = match level {
        OptLevel::None => vec!["-O0".to_string()],
        OptLevel::Less => vec!["-O1".to_string()],
        OptLevel::Default => vec!["-O2".to_string()],
        OptLevel::Aggressive => vec!["-O3".to_string(), "-ffast-math".to_string()],
    };

    flags.extend([
        "-funroll-loops".to_string(),
        "-fomit-frame-pointer".to_string(),
    ]);

    flags
}

pub struct EscapeAnalyzer {
    escape_graph: HashMap<String, HashSet<String>>,
    stack_alloc: HashSet<String>,
}

impl EscapeAnalyzer {
    pub fn new() -> Self {
        EscapeAnalyzer {
            escape_graph: HashMap::new(),
            stack_alloc: HashSet::new(),
        }
    }

    pub fn analyze(&mut self, program: &Program) -> &HashSet<String> {
        for item in &program.items {
            if let Item::Function(func) = item {
                self.analyze_function(&func);
            }
        }
        &self.stack_alloc
    }

    fn analyze_function(&mut self, func: &Function) {
        for param in &func.params {
            if !self.escapes(&param.name) {
                self.stack_alloc.insert(param.name.clone());
            }
        }

        self.analyze_block(&func.body);
    }

    fn analyze_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.analyze_stmt(stmt);
        }
    }

    fn analyze_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { init, .. } | Stmt::Var { init, .. } => {
                if let Some(expr) = init {
                    if !self.expr_escapes(expr) {
                        if let Some(name) = self.get_var_name(stmt) {
                            self.stack_alloc.insert(name);
                        }
                    }
                }
            }
            Stmt::While { body, .. } | Stmt::For { body, .. } => {
                self.analyze_stmt(body);
            }
            Stmt::Unsafe { block } => self.analyze_stmt(block),
            Stmt::Return { value } => {
                if let Some(expr) = value {
                    if self.expr_escapes(expr) {
                        self.mark_escaping(expr);
                    }
                }
            }
            _ => {}
        }
    }

    fn expr_escapes(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Call { func, .. } => {
                if let Expr::Ident(name, _) = **func {
                    matches!(
                        name.as_str(),
                        "malloc" | "calloc" | "realloc" | "printf" | "puts"
                    )
                } else {
                    true
                }
            }
            Expr::Ident(_, _) => false,
            Expr::Literal(_, _) => false,
            _ => true,
        }
    }

    fn mark_escaping(&mut self, expr: &Expr) {
        if let Expr::Ident(name, _) = expr {
            let escapes = self.escape_graph.entry(name.clone()).or_default();
            escapes.insert("return".to_string());
        }
    }

    fn escapes(&self, name: &str) -> bool {
        self.escape_graph
            .get(name)
            .map(|s| !s.is_empty())
            .unwrap_or(false)
    }

    fn get_var_name(&self, stmt: &Stmt) -> Option<String> {
        match stmt {
            Stmt::Let { pattern, .. } | Stmt::Var { pattern, .. } => {
                if let Pattern::Ident(name, _) = pattern {
                    Some(name.clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl Default for EscapeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct InlineExpansion {
    inlined_count: usize,
}

impl InlineExpansion {
    pub fn new() -> Self {
        InlineExpansion { inlined_count: 0 }
    }

    #[inline]
    fn should_inline(&self, func: &Function, options: &OptimizeOptions) -> bool {
        let size = small_function_size(&func.body);

        if options.level == OptLevel::Aggressive {
            size <= options.inline_threshold * 2
        } else {
            size <= options.inline_threshold
        }
    }

    #[inline(always)]
    fn inline_function_body(&self, func: &Function, args: &[Expr]) -> Block {
        let mut body = func.body.clone();

        for (param, arg) in func.params.iter().zip(args.iter()) {
            self.substitute_param(&mut body, &param.name, arg);
        }

        body
    }

    fn substitute_param(&self, block: &mut Block, param_name: &str, arg: &Expr) {
        for stmt in &mut block.stmts {
            self.substitute_stmt_param(stmt, param_name, arg);
        }
    }

    fn substitute_stmt_param(&self, stmt: &mut Stmt, param_name: &str, arg: &Expr) {
        match stmt {
            Stmt::Expr(expr) => self.substitute_expr_param(expr, param_name, arg),
            Stmt::Let { init, .. } | Stmt::Var { init, .. } => {
                if let Some(e) = init {
                    self.substitute_expr_param(e, param_name, arg);
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.substitute_expr_param(condition, param_name, arg);
                self.substitute_stmt_param(body, param_name, arg);
            }
            Stmt::For { iter, body, .. } => {
                self.substitute_expr_param(iter, param_name, arg);
                self.substitute_stmt_param(body, param_name, arg);
            }
            Stmt::Return { value } => {
                if let Some(e) = value {
                    self.substitute_expr_param(e, param_name, arg);
                }
            }
            Stmt::Unsafe { block } => self.substitute_stmt_param(block, param_name, arg),
            Stmt::Defer { expr } => self.substitute_expr_param(expr, param_name, arg),
            Stmt::Comptime { block } => self.substitute_stmt_param(block, param_name, arg),
            _ => {}
        }
    }

    fn substitute_expr_param(&self, expr: &mut Expr, param_name: &str, arg: &Expr) {
        match expr {
            Expr::Ident(name, _) if name == param_name => {
                *expr = arg.clone();
            }
            Expr::Binary { left, right, .. } => {
                self.substitute_expr_param(left, param_name, arg);
                self.substitute_expr_param(right, param_name, arg);
            }
            Expr::Unary { expr: e, .. } => self.substitute_expr_param(e, param_name, arg),
            Expr::Call { args, .. } => {
                for a in args {
                    self.substitute_expr_param(a, param_name, arg);
                }
            }
            Expr::Block(block) => self.substitute_param(block, param_name, arg),
            _ => {}
        }
    }
}

impl OptimizationPass for InlineExpansion {
    fn name(&self) -> &'static str {
        "Inline Expansion"
    }

    fn run(&self, program: &mut Program, options: &OptimizeOptions) -> bool {
        if options.level == OptLevel::None {
            return false;
        }

        let functions: HashMap<String, Function> = program
            .items
            .iter()
            .filter_map(|item| {
                if let Item::Function(f) = item {
                    Some((f.name.clone(), f.clone()))
                } else {
                    None
                }
            })
            .collect();

        let mut changed = false;

        for item in &mut program.items {
            if let Item::Function(func) = item {
                for stmt in &mut func.body.stmts {
                    if let Stmt::Expr(Expr::Call {
                        func: callee,
                        args,
                        span,
                    }) = stmt
                    {
                        if let Expr::Ident(name, _) = &**callee {
                            if let Some(target) = functions.get(name) {
                                if self.should_inline(target, options)
                                    && args.len() == target.params.len()
                                {
                                    let body = self.inline_function_body(target, args);
                                    *stmt = Stmt::Expr(Expr::Block(body));
                                    self.inlined_count += 1;
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        changed
    }

    fn is_enabled(&self, options: &OptimizeOptions) -> bool {
        options.level >= OptLevel::Less
    }
}

impl Default for InlineExpansion {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DeadCodeElimination {
    removed_count: usize,
}

impl DeadCodeElimination {
    pub fn new() -> Self {
        DeadCodeElimination { removed_count: 0 }
    }

    fn find_used_functions(&self, program: &Program) -> HashSet<String> {
        let mut used = HashSet::new();
        used.insert("main".to_string());

        for item in &program.items {
            if let Item::Function(func) = item {
                self.find_calls_in_function(&func, &mut used);
            }
        }

        used
    }

    fn find_calls_in_function(&self, func: &Function, used: &mut HashSet<String>) {
        for stmt in &func.body.stmts {
            self.find_calls_in_stmt(stmt, used);
        }
    }

    fn find_calls_in_stmt(&self, stmt: &Stmt, used: &mut HashSet<String>) {
        match stmt {
            Stmt::Expr(expr) => self.find_calls_in_expr(expr, used),
            Stmt::Let { init, .. } | Stmt::Var { init, .. } => {
                if let Some(e) = init {
                    self.find_calls_in_expr(e, used);
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.find_calls_in_expr(condition, used);
                self.find_calls_in_stmt(body, used);
            }
            Stmt::For { iter, body, .. } => {
                self.find_calls_in_expr(iter, used);
                self.find_calls_in_stmt(body, used);
            }
            Stmt::Return { value } => {
                if let Some(e) = value {
                    self.find_calls_in_expr(e, used);
                }
            }
            Stmt::Unsafe { block } => self.find_calls_in_stmt(block, used),
            Stmt::Defer { expr } => self.find_calls_in_expr(expr, used),
            Stmt::Comptime { block } => self.find_calls_in_stmt(block, used),
            _ => {}
        }
    }

    fn find_calls_in_expr(&self, expr: &Expr, used: &mut HashSet<String>) {
        match expr {
            Expr::Call { func, args, .. } => {
                if let Expr::Ident(name, _) = &**func {
                    used.insert(name.clone());
                }
                for arg in args {
                    self.find_calls_in_expr(arg, used);
                }
            }
            Expr::MethodCall { expr, args, .. } => {
                self.find_calls_in_expr(expr, used);
                for arg in args {
                    self.find_calls_in_expr(arg, used);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.find_calls_in_expr(left, used);
                self.find_calls_in_expr(right, used);
            }
            Expr::Unary { expr, .. } => self.find_calls_in_expr(expr, used),
            Expr::Ternary {
                condition,
                then,
                else_,
                ..
            } => {
                self.find_calls_in_expr(condition, used);
                self.find_calls_in_expr(then, used);
                self.find_calls_in_expr(else_, used);
            }
            Expr::Block(block) => {
                for stmt in &block.stmts {
                    self.find_calls_in_stmt(stmt, used);
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.find_calls_in_expr(condition, used);
                self.find_calls_in_expr(then_branch, used);
                if let Some(e) = else_branch {
                    self.find_calls_in_expr(e, used);
                }
            }
            _ => {}
        }
    }
}

impl OptimizationPass for DeadCodeElimination {
    fn name(&self) -> &'static str {
        "Dead Code Elimination"
    }

    fn run(&self, program: &mut Program, options: &OptimizeOptions) -> bool {
        if options.level == OptLevel::None {
            return false;
        }

        let used = self.find_used_functions(program);
        let mut changed = false;

        program.items.retain(|item| match item {
            Item::Function(f) => {
                if !used.contains(&f.name) && options.level >= OptLevel::Default {
                    self.removed_count += 1;
                    changed = true;
                    false
                } else {
                    true
                }
            }
            _ => true,
        });

        for item in &mut program.items {
            if let Item::Function(func) = item {
                self.eliminate_dead_code_in_function(func);
                changed = true;
            }
        }

        changed
    }

    fn is_enabled(&self, options: &OptimizeOptions) -> bool {
        options.level >= OptLevel::Less
    }
}

impl DeadCodeElimination {
    fn eliminate_dead_code_in_function(&self, func: &mut Function) {
        let mut i = 0;
        while i < func.body.stmts.len() {
            if let Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } = &func.body.stmts[i].clone()
            {
                if let Expr::Literal(Literal::Boolean(false), _) = &**condition {
                    if let Some(else_b) = else_branch {
                        func.body.stmts[i] = *else_b.clone();
                        continue;
                    } else {
                        func.body.stmts.remove(i);
                        continue;
                    }
                }
                if let Expr::Literal(Literal::Boolean(true), _) = &**condition {
                    func.body.stmts[i] = *then_branch.clone();
                    continue;
                }
            }

            if let Stmt::Expr(Expr::Literal(_, _)) = &func.body.stmts[i] {
                func.body.stmts.remove(i);
                continue;
            }

            i += 1;
        }
    }
}

impl Default for DeadCodeElimination {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CrossModuleInlining {
    inlined_count: usize,
}

impl CrossModuleInlining {
    pub fn new() -> Self {
        CrossModuleInlining { inlined_count: 0 }
    }
}

impl OptimizationPass for CrossModuleInlining {
    fn name(&self) -> &'static str {
        "Cross-Module Inlining (LTO)"
    }

    fn run(&self, program: &mut Program, options: &OptimizeOptions) -> bool {
        if !options.lto || options.level == OptLevel::None {
            return false;
        }

        let mut changed = false;

        for item in &mut program.items {
            if let Item::Function(func) = item {
                if options.level == OptLevel::Aggressive && small_function_size(&func.body) <= 20 {
                    for stmt in &mut func.body.stmts {
                        if let Stmt::Expr(Expr::Call {
                            func: callee,
                            args,
                            span,
                        }) = stmt
                        {
                            if let Expr::Ident(name, _) = &**callee {
                                if !self.is_local_function(program, name) {
                                    self.mark_for_lto_inline(name);
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        changed
    }

    fn is_enabled(&self, options: &OptimizeOptions) -> bool {
        options.lto && options.level >= OptLevel::Default
    }

    fn is_local_function(&self, program: &Program, name: &str) -> bool {
        for item in &program.items {
            if let Item::Function(f) = item {
                if f.name == name {
                    return true;
                }
            }
        }
        false
    }

    fn mark_for_lto_inline(&self, _name: &str) {}
}

impl Default for CrossModuleInlining {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CrossModuleDce;

impl CrossModuleDce {
    pub fn new() -> Self {
        CrossModuleDce
    }
}

impl OptimizationPass for CrossModuleDce {
    fn name(&self) -> &'static str {
        "Cross-Module DCE (LTO)"
    }

    fn run(&self, program: &mut Program, options: &OptimizeOptions) -> bool {
        if !options.lto || options.level == OptLevel::None {
            return false;
        }

        let mut used = self.collect_all_exported_symbols(program);
        let mut changed = false;

        let iteration_count = if options.level == OptLevel::Aggressive {
            10
        } else {
            3
        };

        for _ in 0..iteration_count {
            let mut found_new = false;

            for item in &program.items {
                if let Item::Function(func) = item {
                    if !used.contains(&func.name) {
                        let refs = self.count_references(&func, &used);
                        if refs == 0 {
                            found_new = true;
                            changed = true;
                        }
                    }
                }
            }

            if !found_new {
                break;
            }
        }

        changed
    }

    fn is_enabled(&self, options: &OptimizeOptions) -> bool {
        options.lto && options.level >= OptLevel::Default
    }
}

impl CrossModuleDce {
    fn collect_all_exported_symbols(&self, program: &Program) -> HashSet<String> {
        let mut symbols = HashSet::new();
        symbols.insert("main".to_string());

        for item in &program.items {
            match item {
                Item::Function(f) => {
                    if is_exported_function(f) {
                        symbols.insert(f.name.clone());
                    }
                }
                Item::Static(s) => {
                    symbols.insert(s.name.clone());
                }
                Item::Const(c) => {
                    symbols.insert(c.name.clone());
                }
                _ => {}
            }
        }

        symbols
    }

    fn count_references(&self, _func: &Function, _used: &HashSet<String>) -> usize {
        0
    }
}

fn is_exported_function(func: &Function) -> bool {
    func.name == "main" || func.name.starts_with("knull_")
}

impl Default for CrossModuleDce {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_folding() {
        let mut cf = ConstantFolding::new();
        let mut program = Program { items: vec![] };

        assert!(cf.run(&mut program, &OptimizeOptions::default()));
    }

    #[test]
    fn test_cpu_feature_detection() {
        let features = CpuFeatures::detect();
        assert!(features.vector_width() > 0);
    }

    #[test]
    fn test_escape_analysis() {
        let mut analyzer = EscapeAnalyzer::new();
        let program = Program { items: vec![] };
        analyzer.analyze(&program);
    }

    #[test]
    fn test_opt_level_from_u32() {
        assert_eq!(OptLevel::from_u32(0), OptLevel::None);
        assert_eq!(OptLevel::from_u32(1), OptLevel::Less);
        assert_eq!(OptLevel::from_u32(2), OptLevel::Default);
        assert_eq!(OptLevel::from_u32(3), OptLevel::Aggressive);
        assert_eq!(OptLevel::from_u32(10), OptLevel::Aggressive);
    }
}
