//! Loop Optimizations
//!
//! Implements loop unrolling, fusion, and invariant code motion.

use crate::ast::*;
use crate::optimize::{small_function_size, OptLevel, OptimizationPass, OptimizeOptions};
use std::collections::HashSet;

pub struct LoopUnrolling {
    unrolled_count: usize,
}

impl LoopUnrolling {
    pub fn new() -> Self {
        LoopUnrolling { unrolled_count: 0 }
    }
}

impl OptimizationPass for LoopUnrolling {
    fn name(&self) -> &'static str {
        "Loop Unrolling"
    }

    fn run(&self, program: &mut Program, options: &OptimizeOptions) -> bool {
        if options.level == OptLevel::None {
            return false;
        }

        let mut changed = false;

        for item in &mut program.items {
            if let Item::Function(func) = item {
                if unroll_loops(func, options.max_unroll) {
                    changed = true;
                }
            }
        }

        changed
    }

    fn is_enabled(&self, options: &OptimizeOptions) -> bool {
        options.level >= OptLevel::Default
    }
}

fn unroll_loops(func: &mut Function, max_unroll: usize) -> bool {
    let mut changed = false;

    for stmt in &mut func.body.stmts {
        if unroll_stmt(stmt, max_unroll) {
            changed = true;
        }
    }

    changed
}

fn unroll_stmt(stmt: &mut Stmt, max_unroll: usize) -> bool {
    match stmt {
        Stmt::While {
            label,
            condition,
            body,
        } => {
            if can_unroll(condition, body, max_unroll) {
                *stmt = create_unrolled_while(label.clone(), condition, body, max_unroll);
                return true;
            }
            unroll_stmt(body, max_unroll)
        }
        Stmt::For {
            pattern,
            iter,
            body,
        } => {
            if let Some((start, end)) = get_range_bounds(iter) {
                let count = (end - start) as usize;
                if count <= max_unroll && count > 0 {
                    *stmt = create_unrolled_for(pattern.clone(), start, end, body);
                    return true;
                }
            }
            unroll_stmt(body, max_unroll)
        }
        Stmt::Unsafe { block } => unroll_stmt(block, max_unroll),
        _ => false,
    }
}

fn can_unroll(condition: &Expr, body: &Stmt, max_unroll: usize) -> bool {
    if let Some((start, end)) = get_range_from_condition(condition) {
        let count = (end - start) as usize;
        return count <= max_unroll && count > 0;
    }

    let body_size = stmt_size_simple(body);
    body_size <= 10
}

fn stmt_size_simple(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Expr(expr) => expr_size_simple(expr),
        Stmt::Let { init, .. } | Stmt::Var { init, .. } => {
            init.as_ref().map(|e| expr_size_simple(e)).unwrap_or(0)
        }
        Stmt::While { body, .. } => stmt_size_simple(body) + 1,
        Stmt::For { body, .. } => stmt_size_simple(body) + 1,
        Stmt::Unsafe { block } => stmt_size_simple(block),
        _ => 1,
    }
}

fn expr_size_simple(expr: &Expr) -> usize {
    match expr {
        Expr::Binary { left, right, .. } => 1 + expr_size_simple(left) + expr_size_simple(right),
        Expr::Unary { expr, .. } => 1 + expr_size_simple(expr),
        Expr::Block(block) => block.stmts.iter().map(stmt_size_simple).sum(),
        Expr::Call { args, .. } => 1 + args.iter().map(expr_size_simple).sum(),
        _ => 1,
    }
}

fn get_range_from_condition(cond: &Expr) -> Option<(i64, i64)> {
    match cond {
        Expr::Binary {
            op: BinaryOp::Lt,
            left,
            right,
            ..
        } => {
            if let Expr::Ident(i, _) = &**left {
                if let Expr::Literal(Literal::Int(n), _) = &**right {
                    return Some((0, *n));
                }
            }
            None
        }
        _ => None,
    }
}

fn get_range_bounds(expr: &Expr) -> Option<(i64, i64)> {
    if let Expr::Call { func, args, .. } = expr {
        if let Expr::Ident(name, _) = &**func {
            if name == "range" && args.len() == 2 {
                if let Expr::Literal(Literal::Int(start), _) = &args[0] {
                    if let Expr::Literal(Literal::Int(end), _) = &args[1] {
                        return Some((*start, *end));
                    }
                }
            }
        }
    }
    None
}

fn create_unrolled_while(
    label: Option<String>,
    condition: &Expr,
    body: &Stmt,
    count: usize,
) -> Stmt {
    let mut stmts = Vec::new();

    for _ in 0..count {
        stmts.push(body.clone());
    }

    Stmt::While {
        label,
        condition: condition.clone(),
        body: Box::new(Stmt::Block(Block {
            stmts,
            span: Span::default(),
        })),
    }
}

fn create_unrolled_for(pattern: Pattern, start: i64, end: i64, body: &Stmt) -> Stmt {
    let mut stmts = Vec::new();

    for i in start..end {
        let mut iter_body = body.clone();

        if let Stmt::For {
            pattern: pat,
            body: b,
            ..
        } = &mut iter_body
        {
            if let Some(new_pattern) = replace_pattern_with_value(pat, &pattern, i) {
                *pat = new_pattern;
            }
            stmts.push(*b.clone());
        } else {
            stmts.push(iter_body);
        }
    }

    Stmt::Block(Block {
        stmts,
        span: Span::default(),
    })
}

fn replace_pattern_with_value(
    pattern: &Pattern,
    value_pattern: &Pattern,
    value: i64,
) -> Option<Pattern> {
    if let Pattern::Ident(name, _) = pattern {
        return Some(Pattern::Ident(name.clone(), Span::default()));
    }
    None
}

pub struct LoopFusion;

impl LoopFusion {
    pub fn new() -> Self {
        LoopFusion
    }
}

impl OptimizationPass for LoopFusion {
    fn name(&self) -> &'static str {
        "Loop Fusion"
    }

    fn run(&self, program: &mut Program, options: &OptimizeOptions) -> bool {
        if options.level == OptLevel::None {
            return false;
        }

        let mut changed = false;

        for item in &mut program.items {
            if let Item::Function(func) = item {
                if fuse_adjacent_loops(&mut func.body) {
                    changed = true;
                }
            }
        }

        changed
    }

    fn is_enabled(&self, options: &OptimizeOptions) -> bool {
        options.level >= OptLevel::Default
    }
}

fn fuse_adjacent_loops(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;

    while i + 1 < block.stmts.len() {
        if let (
            Stmt::For {
                pattern: p1,
                iter: i1,
                body: b1,
            },
            Stmt::For {
                pattern: p2,
                iter: i2,
                body: b2,
            },
        ) = (&block.stmts[i], &block.stmts[i + 1])
        {
            if can_fuse(p1, i1, p2, i2) {
                let fused_body = fuse_loop_bodies(b1, b2);
                block.stmts[i] = Stmt::For {
                    pattern: p1.clone(),
                    iter: i1.clone(),
                    body: fused_body,
                };
                block.stmts.remove(i + 1);
                changed = true;
                continue;
            }
        }
        i += 1;
    }

    changed
}

fn can_fuse(p1: &Pattern, i1: &Expr, p2: &Pattern, i2: &Expr) -> bool {
    if let (
        Expr::Call {
            func: f1, args: a1, ..
        },
        Expr::Call {
            func: f2, args: a2, ..
        },
    ) = (i1, i2)
    {
        if let (Expr::Ident(n1, _), Expr::Ident(n2, _)) = (&**f1, &**f2) {
            if n1 == "range" && n2 == "range" && a1 == a2 {
                return true;
            }
        }
    }
    false
}

fn fuse_loop_bodies(b1: &Stmt, b2: &Stmt) -> Box<Stmt> {
    let mut combined = Vec::new();

    if let Stmt::For { body: body1, .. } = b1 {
        combined.push(*body1.clone());
    }
    if let Stmt::For { body: body2, .. } = b2 {
        combined.push(*body2.clone());
    }

    Box::new(Stmt::Block(Block {
        stmts: combined,
        span: Span::default(),
    }))
}

pub struct InvariantCodeMotion;

impl InvariantCodeMotion {
    pub fn new() -> Self {
        InvariantCodeMotion
    }
}

impl OptimizationPass for InvariantCodeMotion {
    fn name(&self) -> &'static str {
        "Invariant Code Motion"
    }

    fn run(&self, program: &mut Program, options: &OptimizeOptions) -> bool {
        if options.level == OptLevel::None {
            return false;
        }

        let mut changed = false;

        for item in &mut program.items {
            if let Item::Function(func) = item {
                if move_invariants(&mut func.body) {
                    changed = true;
                }
            }
        }

        changed
    }

    fn is_enabled(&self, options: &OptimizeOptions) -> bool {
        options.level >= OptLevel::Less
    }
}

fn move_invariants(block: &mut Block) -> bool {
    let mut changed = false;

    for stmt in &mut block.stmts {
        if let Stmt::While { body, .. } = stmt {
            if let Some(invariant) = find_loop_invariant(body) {
                let new_stmt = Stmt::Let {
                    pattern: Pattern::Ident(format!("_invariant_{}", invariant.0), Span::default()),
                    ty: None,
                    init: Some(invariant.1),
                };
                block.stmts.insert(0, new_stmt);
                changed = true;
            }
        }
    }

    changed
}

fn find_loop_invariant(stmt: &Stmt) -> Option<(String, Expr)> {
    None
}

impl Default for LoopUnrolling {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for LoopFusion {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for InvariantCodeMotion {
    fn default() -> Self {
        Self::new()
    }
}
