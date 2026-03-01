//! Constant Folding Optimization
//!
//! Evaluates constant expressions at compile time.

use crate::ast::*;
use crate::optimize::{OptLevel, OptimizationPass, OptimizeOptions};

pub struct ConstantFolding {
    folded_count: usize,
}

impl ConstantFolding {
    pub fn new() -> Self {
        ConstantFolding { folded_count: 0 }
    }
}

impl OptimizationPass for ConstantFolding {
    fn name(&self) -> &'static str {
        "Constant Folding"
    }

    fn run(&self, program: &mut Program, options: &OptimizeOptions) -> bool {
        if options.level == OptLevel::None {
            return false;
        }

        let mut changed = false;

        for item in &mut program.items {
            if let Item::Function(func) = item {
                if fold_function(func) {
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

fn fold_function(func: &mut Function) -> bool {
    let mut changed = false;

    for stmt in &mut func.body.stmts {
        if fold_stmt(stmt) {
            changed = true;
        }
    }

    changed
}

fn fold_stmt(stmt: &mut Stmt) -> bool {
    match stmt {
        Stmt::Expr(expr) => fold_expr(expr),
        Stmt::Let { init, .. } | Stmt::Var { init, .. } => {
            if let Some(e) = init {
                fold_expr(e)
            } else {
                false
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            let c = fold_expr(condition);
            let b = fold_stmt(body);
            c || b
        }
        Stmt::For { iter, body, .. } => {
            let i = fold_expr(iter);
            let b = fold_stmt(body);
            i || b
        }
        Stmt::Return { value } => {
            if let Some(e) = value {
                fold_expr(e)
            } else {
                false
            }
        }
        Stmt::Unsafe { block } => fold_stmt(block),
        Stmt::Defer { expr } => fold_expr(expr),
        Stmt::Comptime { block } => fold_stmt(block),
        _ => false,
    }
}

fn fold_expr(expr: &mut Expr) -> bool {
    match expr {
        Expr::Binary { op, left, right } => {
            let mut changed = fold_expr(left);
            if fold_expr(right) {
                changed = true;
            }

            if let Some(lit) = eval_binary_op(op, left, right) {
                *expr = lit;
                return true;
            }

            changed
        }
        Expr::Unary { op, expr: e } => {
            if fold_expr(e) {
                if let Some(lit) = eval_unary_op(op, e) {
                    *expr = lit;
                    return true;
                }
            }
            false
        }
        Expr::Ternary {
            condition,
            then,
            else_,
        } => {
            let c_changed = fold_expr(condition);
            let t_changed = fold_expr(then);
            let e_changed = fold_expr(else_);

            if c_changed || t_changed || e_changed {
                if let Expr::Literal(lit, span) = &**condition {
                    if let Literal::Bool(b) = lit {
                        *expr = if *b { *then.clone() } else { *else_.clone() };
                        return true;
                    }
                }
            }

            c_changed || t_changed || e_changed
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            let c_changed = fold_expr(condition);
            let t_changed = fold_expr(then_branch);
            let e_changed = if let Some(e) = else_branch {
                fold_expr(e)
            } else {
                false
            };

            if c_changed {
                if let Expr::Literal(lit, span) = &**condition {
                    if let Literal::Bool(b) = lit {
                        *expr = if *b {
                            *then_branch.clone()
                        } else {
                            else_branch.as_ref().map(|e| *e.clone()).unwrap_or_else(|| {
                                Expr::Block(Block {
                                    stmts: vec![],
                                    span: *span,
                                })
                            })
                        };
                        return true;
                    }
                }
            }

            c_changed || t_changed || e_changed
        }
        Expr::Block(block) => {
            let mut changed = false;
            for stmt in &mut block.stmts {
                if fold_stmt(stmt) {
                    changed = true;
                }
            }
            changed
        }
        Expr::Array(arr, _) => {
            let mut changed = false;
            for e in arr {
                if fold_expr(e) {
                    changed = true;
                }
            }
            changed
        }
        Expr::Call { func, args, .. } => {
            let mut changed = false;
            for arg in args {
                if fold_expr(arg) {
                    changed = true;
                }
            }

            if let Expr::Ident(name, _) = &**func {
                if let Some(lit) = fold_intrinsic(name, args) {
                    *expr = lit;
                    return true;
                }
            }

            changed
        }
        _ => false,
    }
}

fn eval_binary_op(op: &BinaryOp, left: &Expr, right: &Expr) -> Option<Expr> {
    match (op, left, right) {
        (BinaryOp::Add, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            add_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Sub, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            sub_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Mul, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            mul_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Div, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            div_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Mod, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            mod_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Shl, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            shift_left_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Shr, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            shift_right_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::BitAnd, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            bitand_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::BitOr, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            bitor_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::BitXor, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            bitxor_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::And, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            and_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Or, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            or_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Eq, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            eq_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Ne, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            neq_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Lt, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            lt_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Gt, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            gt_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Lte, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            lte_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        (BinaryOp::Gte, Expr::Literal(l1, s1), Expr::Literal(l2, _)) => {
            gte_literals(l1, l2).map(|l| Expr::Literal(l, *s1))
        }
        _ => None,
    }
}

fn eval_unary_op(op: &UnaryOp, expr: &Expr) -> Option<Expr> {
    if let Expr::Literal(lit, span) = expr {
        match op {
            UnaryOp::Neg => {
                let new_lit = match lit {
                    Literal::Int(i) => Literal::Int(-i),
                    Literal::Float(f) => Literal::Float(-f),
                    _ => return None,
                };
                Some(Expr::Literal(new_lit, *span))
            }
            UnaryOp::Not => {
                let new_lit = match lit {
                    Literal::Bool(b) => Literal::Bool(!b),
                    Literal::Int(i) => Literal::Int(!i),
                    _ => return None,
                };
                Some(Expr::Literal(new_lit, *span))
            }
            UnaryOp::BitNot => {
                let new_lit = match lit {
                    Literal::Int(i) => Literal::Int(!i),
                    _ => return None,
                };
                Some(Expr::Literal(new_lit, *span))
            }
            _ => None,
        }
    } else {
        None
    }
}

fn add_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => Some(Literal::Int(i1 + i2)),
        (Literal::Float(f1), Literal::Float(f2)) => Some(Literal::Float(f1 + f2)),
        (Literal::Int(i), Literal::Float(f)) => Some(Literal::Float(*i as f64 + f)),
        (Literal::Float(f), Literal::Int(i)) => Some(Literal::Float(f + *i as f64)),
        _ => None,
    }
}

fn sub_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => Some(Literal::Int(i1 - i2)),
        (Literal::Float(f1), Literal::Float(f2)) => Some(Literal::Float(f1 - f2)),
        (Literal::Int(i), Literal::Float(f)) => Some(Literal::Float(*i as f64 - f)),
        (Literal::Float(f), Literal::Int(i)) => Some(Literal::Float(f - *i as f64)),
        _ => None,
    }
}

fn mul_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => Some(Literal::Int(i1 * i2)),
        (Literal::Float(f1), Literal::Float(f2)) => Some(Literal::Float(f1 * f2)),
        (Literal::Int(i), Literal::Float(f)) => Some(Literal::Float(*i as f64 * f)),
        (Literal::Float(f), Literal::Int(i)) => Some(Literal::Float(f * *i as f64)),
        _ => None,
    }
}

fn div_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    match (l1, l2) {
        (_, Literal::Int(0)) | (_, Literal::Float(0.0)) => None,
        (Literal::Int(i1), Literal::Int(i2)) => Some(Literal::Int(i1 / i2)),
        (Literal::Float(f1), Literal::Float(f2)) => Some(Literal::Float(f1 / f2)),
        (Literal::Int(i), Literal::Float(f)) => Some(Literal::Float(*i as f64 / f)),
        (Literal::Float(f), Literal::Int(i)) => Some(Literal::Float(f / *i as f64)),
        _ => None,
    }
}

fn mod_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) if *i2 != 0 => Some(Literal::Int(i1 % i2)),
        _ => None,
    }
}

fn shift_left_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => Some(Literal::Int(i1 << i2)),
        _ => None,
    }
}

fn shift_right_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => Some(Literal::Int(i1 >> i2)),
        _ => None,
    }
}

fn bitand_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => Some(Literal::Int(i1 & i2)),
        _ => None,
    }
}

fn bitor_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => Some(Literal::Int(i1 | i2)),
        _ => None,
    }
}

fn bitxor_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => Some(Literal::Int(i1 ^ i2)),
        _ => None,
    }
}

fn and_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    match (l1, l2) {
        (Literal::Bool(b1), Literal::Bool(b2)) => Some(Literal::Bool(*b1 && *b2)),
        (Literal::Int(i1), Literal::Int(i2)) => Some(Literal::Int(i1 & i2)),
        _ => None,
    }
}

fn or_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    match (l1, l2) {
        (Literal::Bool(b1), Literal::Bool(b2)) => Some(Literal::Bool(*b1 || *b2)),
        (Literal::Int(i1), Literal::Int(i2)) => Some(Literal::Int(i1 | i2)),
        _ => None,
    }
}

fn eq_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    Some(Literal::Bool(match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => i1 == i2,
        (Literal::Float(f1), Literal::Float(f2)) => f1 == f2,
        (Literal::Bool(b1), Literal::Bool(b2)) => b1 == b2,
        (Literal::String(s1), Literal::String(s2)) => s1 == s2,
        (Literal::Char(c1), Literal::Char(c2)) => c1 == c2,
        _ => return None,
    }))
}

fn neq_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    Some(Literal::Bool(match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => i1 != i2,
        (Literal::Float(f1), Literal::Float(f2)) => f1 != f2,
        (Literal::Bool(b1), Literal::Bool(b2)) => b1 != b2,
        (Literal::String(s1), Literal::String(s2)) => s1 != s2,
        (Literal::Char(c1), Literal::Char(c2)) => c1 != c2,
        _ => return None,
    }))
}

fn lt_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    Some(Literal::Bool(match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => i1 < i2,
        (Literal::Float(f1), Literal::Float(f2)) => f1 < f2,
        _ => return None,
    }))
}

fn gt_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    Some(Literal::Bool(match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => i1 > i2,
        (Literal::Float(f1), Literal::Float(f2)) => f1 > f2,
        _ => return None,
    }))
}

fn lte_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    Some(Literal::Bool(match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => i1 <= i2,
        (Literal::Float(f1), Literal::Float(f2)) => f1 <= f2,
        _ => return None,
    }))
}

fn gte_literals(l1: &Literal, l2: &Literal) -> Option<Literal> {
    Some(Literal::Bool(match (l1, l2) {
        (Literal::Int(i1), Literal::Int(i2)) => i1 >= i2,
        (Literal::Float(f1), Literal::Float(f2)) => f1 >= f2,
        _ => return None,
    }))
}

fn fold_intrinsic(name: &str, args: &[Expr]) -> Option<Expr> {
    match name {
        "len" | "sizeof" => {
            if args.len() == 1 {
                if let Expr::Array(arr, _) = &args[0] {
                    return Some(Literal::Int(arr.len() as i64).into());
                }
                if let Expr::Literal(Literal::String(s), _) = &args[0] {
                    return Some(Literal::Int(s.len() as i64).into());
                }
            }
            None
        }
        "abs" => {
            if args.len() == 1 {
                if let Expr::Literal(Literal::Int(i), s) = &args[0] {
                    return Some(Literal::Int(i.abs()).into());
                }
                if let Expr::Literal(Literal::Float(f), s) = &args[0] {
                    return Some(Literal::Float(f.abs()).into());
                }
            }
            None
        }
        "min" => {
            if args.len() == 2 {
                if let (Expr::Literal(Literal::Int(i1), _), Expr::Literal(Literal::Int(i2), _)) =
                    (&args[0], &args[1])
                {
                    return Some(Literal::Int(i1.min(*i2)).into());
                }
            }
            None
        }
        "max" => {
            if args.len() == 2 {
                if let (Expr::Literal(Literal::Int(i1), _), Expr::Literal(Literal::Int(i2), _)) =
                    (&args[0], &args[1])
                {
                    return Some(Literal::Int(i1.max(*i2)).into());
                }
            }
            None
        }
        "pow" => {
            if args.len() == 2 {
                if let (Expr::Literal(Literal::Int(base), _), Expr::Literal(Literal::Int(exp), _)) =
                    (&args[0], &args[1])
                {
                    return Some(Literal::Int(base.pow(*exp as u32)).into());
                }
            }
            None
        }
        "sqrt" => {
            if args.len() == 1 {
                if let Expr::Literal(Literal::Int(i), s) = &args[0] {
                    return Some(Literal::Float((i.abs() as f64).sqrt()).into());
                }
                if let Expr::Literal(Literal::Float(f), s) = &args[0] {
                    return Some(Literal::Float(f.sqrt()).into());
                }
            }
            None
        }
        _ => None,
    }
}

impl Default for ConstantFolding {
    fn default() -> Self {
        Self::new()
    }
}
