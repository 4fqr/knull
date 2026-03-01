//! Knull C Code Generator
//!
//! Generates C code from the AST

use crate::ast::*;

pub struct CCodeGenerator {
    output: String,
    indent: usize,
}

impl CCodeGenerator {
    pub fn new() -> Self {
        CCodeGenerator {
            output: String::new(),
            indent: 0,
        }
    }

    pub fn generate(&mut self, ast: &Program) -> String {
        self.output = String::new();
        self.emit("#include <stdio.h>");
        self.emit("#include <stdlib.h>");
        self.emit("#include <string.h>");
        self.emit("");

        for item in &ast.items {
            self.emit_item(item);
        }

        self.output.clone()
    }

    fn emit(&mut self, s: &str) {
        let indent_str = "    ".repeat(self.indent);
        self.output.push_str(&indent_str);
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn emit_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => self.emit_function(f),
            Item::Struct(s) => self.emit_struct(s),
            Item::Enum(e) => self.emit_enum(e),
            _ => {}
        }
    }

    fn emit_function(&mut self, f: &Function) {
        let ret_type = if f.name == "main" {
            "int".to_string()
        } else {
            self.c_type(&f.return_type)
        };
        self.emit(&format!("{} {}(", ret_type, self.mangle_name(&f.name)));
        self.emit_params(&f.params);
        self.emit(" {");
        self.indent += 1;

        for stmt in &f.body.stmts {
            self.emit_stmt(stmt);
        }

        if f.name == "main" {
            if f.body.stmts.is_empty() || !self.last_stmt_returns(&f.body.stmts) {
                self.emit("return 0;");
            }
        } else if f.body.stmts.is_empty() || !self.last_stmt_returns(&f.body.stmts) {
            self.emit("return 0;");
        }

        self.indent -= 1;
        self.emit("}");
        self.emit("");
    }

    fn emit_params(&mut self, params: &[Param]) {
        if params.is_empty() {
            self.emit("void)");
            return;
        }

        for (i, param) in params.iter().enumerate() {
            let ty = self.c_type(&Some(param.ty.clone()));
            if i > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(&format!("{} {}", ty, param.name));
        }
        self.emit(")");
    }

    fn emit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { pattern, ty, init } => {
                let name = self.get_pattern_name(pattern);
                let var_type = if let Some(t) = ty {
                    self.c_type(ty)
                } else if let Some(init_expr) = init {
                    self.infer_type(init_expr)
                } else {
                    "int".to_string()
                };
                if let Some(init) = init {
                    let val = self.emit_expr(init);
                    self.emit(&format!("{} {} = {};", var_type, name, val));
                } else {
                    self.emit(&format!("{} {};", var_type, name));
                }
            }
            Stmt::Return { value } => {
                if let Some(val) = value {
                    self.emit(&format!("return {};", self.emit_expr(val)));
                } else {
                    self.emit("return;");
                }
            }
            Stmt::Expr(expr) => {
                // Handle If expression as statement - emit as if/else statement
                if let Expr::If {
                    condition,
                    then,
                    else_branch,
                    ..
                } = expr
                {
                    self.emit(&format!("if ({}) {{", self.emit_expr(condition)));
                    self.indent += 1;
                    if let Expr::Block(block) = &**then {
                        for stmt in &block.stmts {
                            self.emit_stmt(stmt);
                        }
                    }
                    self.indent -= 1;
                    if let Some(else_blk) = else_branch {
                        self.emit("} else {");
                        self.indent += 1;
                        if let Expr::Block(block) = &**else_blk {
                            for stmt in &block.stmts {
                                self.emit_stmt(stmt);
                            }
                        }
                        self.indent -= 1;
                    }
                    self.emit("}");
                } else {
                    let s = self.emit_expr(expr);
                    if !s.is_empty() {
                        self.emit(&format!("{};", s));
                    }
                }
            }
            Stmt::While {
                label: _,
                condition,
                body,
            } => {
                self.emit(&format!("while ({}) {{", self.emit_expr(condition)));
                self.indent += 1;
                self.emit_stmt(body);
                self.indent -= 1;
                self.emit("}");
            }
            Stmt::For {
                pattern,
                iter,
                body,
            } => {
                let iter_var = self.get_pattern_name(pattern);
                self.emit(&format!(
                    "for (int {} = 0; {} < {}; {}) {{",
                    iter_var,
                    iter_var,
                    self.emit_expr(iter),
                    iter_var
                ));
                self.indent += 1;
                self.emit_stmt(body);
                self.indent -= 1;
                self.emit("}");
            }
            Stmt::Break { .. } => self.emit("break;"),
            Stmt::Continue { .. } => self.emit("continue;"),
            _ => {}
        }
    }

    fn emit_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit, _) => match lit {
                Literal::Int(n) => n.to_string(),
                Literal::Float(f) => f.to_string(),
                Literal::String(s) => {
                    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                    format!("\"{}\"", escaped)
                }
                Literal::Char(c) => format!("'{}'", c),
                Literal::Bool(b) => if *b { "1" } else { "0" }.to_string(),
            },
            Expr::Ident(name, _) => name.clone(),
            Expr::Binary {
                op, left, right, ..
            } => {
                let l = self.emit_expr(left);
                let r = self.emit_expr(right);
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Rem => "%",
                    BinaryOp::Eq => "==",
                    BinaryOp::Ne => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::Gt => ">",
                    BinaryOp::Le => "<=",
                    BinaryOp::Ge => ">=",
                    BinaryOp::And => "&&",
                    BinaryOp::Or => "||",
                    BinaryOp::Shl => "<<",
                    BinaryOp::Shr => ">>",
                    BinaryOp::BitAnd => "&",
                    BinaryOp::BitOr => "|",
                    BinaryOp::BitXor => "^",
                    _ => "??",
                };
                format!("({} {} {})", l, op_str, r)
            }
            Expr::Unary { op, expr, .. } => {
                let e = self.emit_expr(expr);
                let op_str = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                    UnaryOp::BitNot => "~",
                    UnaryOp::Deref => "*",
                    UnaryOp::Ref => "&",
                };
                format!("({}{})", op_str, e)
            }
            Expr::Call { func, args, .. } => {
                let fn_name = match func.as_ref() {
                    Expr::Ident(name, _) => self.mangle_name(name),
                    _ => "unknown".to_string(),
                };
                let mut arg_strs = Vec::new();
                for arg in args {
                    arg_strs.push(self.emit_expr(arg));
                }
                format!("{}({})", fn_name, arg_strs.join(", "))
            }
            Expr::If {
                condition,
                then,
                else_branch,
                ..
            } => {
                let c = self.emit_expr(condition);
                let t = self.emit_expr(then);
                let e = else_branch
                    .as_ref()
                    .map(|e| self.emit_expr(e))
                    .unwrap_or_else(|| "0".to_string());
                format!("({} ? {} : {})", c, t, e)
            }
            Expr::Block(block) => {
                let mut result = "(".to_string();
                for stmt in &block.stmts {
                    if let Stmt::Expr(e) = stmt {
                        result.push_str(&self.emit_expr(e));
                    }
                }
                result.push(')');
                result
            }
            Expr::Array(arr, _) => {
                let mut elems = Vec::new();
                for e in arr {
                    elems.push(self.emit_expr(e));
                }
                format!("((int[]){{{}}})", elems.join(", "))
            }
            Expr::Return { value, .. } => {
                if let Some(v) = value {
                    format!("return {}", self.emit_expr(v))
                } else {
                    "return".to_string()
                }
            }
            _ => "0".to_string(),
        }
    }

    fn c_type(&self, ty: &Option<Type>) -> String {
        match ty {
            Some(t) => self.type_to_c(t),
            None => "void".to_string(),
        }
    }

    fn type_to_c(&self, ty: &Type) -> String {
        match ty {
            Type::Name(name) => match name.as_str() {
                "i8" => "char".to_string(),
                "i16" => "short".to_string(),
                "i32" => "int".to_string(),
                "i64" => "long".to_string(),
                "u8" => "unsigned char".to_string(),
                "u16" => "unsigned short".to_string(),
                "u32" => "unsigned int".to_string(),
                "u64" => "unsigned long".to_string(),
                "f32" => "float".to_string(),
                "f64" => "double".to_string(),
                "bool" => "int".to_string(),
                "char" => "char".to_string(),
                "String" => "char*".to_string(),
                "()" => "void".to_string(),
                _ => name.clone(),
            },
            Type::Pointer(t, _) => format!("{}*", self.type_to_c(t)),
            Type::Reference(t, _) => format!("{}*", self.type_to_c(t)),
            Type::Array(t, size) => format!("{} [{}]", self.type_to_c(t), size),
            Type::Function(params, ret) => {
                let ret_str = ret
                    .as_ref()
                    .map(|t| self.type_to_c(t))
                    .unwrap_or_else(|| "void".to_string());
                let param_strs: Vec<String> = params.iter().map(|t| self.type_to_c(t)).collect();
                format!("{} (*)({})", ret_str, param_strs.join(", "))
            }
            _ => "int".to_string(),
        }
    }

    fn infer_type(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit, _) => match lit {
                Literal::Int(_) => "int".to_string(),
                Literal::Float(_) => "double".to_string(),
                Literal::String(_) => "char*".to_string(),
                Literal::Char(_) => "char".to_string(),
                Literal::Bool(_) => "int".to_string(),
            },
            Expr::Binary { op, .. } => match op {
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                    "int".to_string()
                }
                _ => "int".to_string(),
            },
            _ => "int".to_string(),
        }
    }

    fn mangle_name(&self, name: &str) -> String {
        match name {
            "println" => "knull_println".to_string(),
            "print" => "knull_print".to_string(),
            "len" => "knull_len".to_string(),
            "alloc" => "knull_alloc".to_string(),
            "free" => "knull_free".to_string(),
            _ => name.to_string(),
        }
    }

    fn get_pattern_name(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Ident(name, _) => name.clone(),
            _ => "_".to_string(),
        }
    }

    fn last_stmt_returns(&self, stmts: &[Stmt]) -> bool {
        if let Some(last) = stmts.last() {
            match last {
                Stmt::Return { .. } => true,
                Stmt::Expr(expr) => matches!(expr, Expr::Return { .. }),
                _ => false,
            }
        } else {
            false
        }
    }

    fn emit_struct(&mut self, s: &StructDef) {
        self.emit(&format!("struct {} {{", s.name));
        for field in &s.fields {
            let ty = self.type_to_c(&field.ty);
            self.emit(&format!("    {} {};", ty, field.name));
        }
        self.emit("};");
        self.emit("");
    }

    fn emit_enum(&mut self, e: &EnumDef) {
        self.emit(&format!("enum {} {{", e.name));
        for (i, variant) in e.variants.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(&variant.name);
        }
        self.emit("};");
        self.emit("");
    }
}
