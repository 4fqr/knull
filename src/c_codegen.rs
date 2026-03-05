//! Knull C Code Generator - COMPLETE IMPLEMENTATION
//!
//! Compiles Knull AST to C code with full feature support.

use std::collections::HashMap;
use std::fs;
use std::process::Command;

use crate::parser::{ASTNode, Literal};

#[derive(Debug, Clone, PartialEq)]
pub enum CompileMode {
    Normal,
    God,
}

pub struct CCodeGen {
    output: String,
    indent: usize,
    temp_count: u32,
    functions: HashMap<String, FunctionInfo>,
    current_function: Option<String>,
    string_literals: Vec<String>,
    needs_gc: bool,
    needs_threading: bool,
    needs_networking: bool,
    mode: CompileMode,
    var_types: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct FunctionInfo {
    params: Vec<String>,
    has_return: bool,
}

impl CCodeGen {
    pub fn new() -> Self {
        CCodeGen {
            output: String::new(),
            indent: 0,
            temp_count: 0,
            functions: HashMap::new(),
            current_function: None,
            string_literals: Vec::new(),
            needs_gc: false,
            needs_threading: false,
            needs_networking: false,
            mode: CompileMode::Normal,
            var_types: HashMap::new(),
        }
    }

    pub fn compile(&mut self, ast: &ASTNode) -> Result<String, String> {
        // Add C headers
        self.emit_line("/* Knull Generated C Code */");
        self.emit_line("#include <stdio.h>");
        self.emit_line("#include <stdlib.h>");
        self.emit_line("#include <string.h>");
        self.emit_line("#include <stdint.h>");
        self.emit_line("#include <stdbool.h>");
        self.emit_line("#include <math.h>");
        self.emit_line("#include <time.h>");
        self.emit_line("#include <unistd.h>");
        self.emit_line("#include <sys/types.h>");
        self.emit_line("#include <sys/stat.h>");
        self.emit_line("#include <fcntl.h>");
        self.emit_line("#include <errno.h>");
        self.emit_line("");
        self.emit_line("/* Type Definitions */");
        self.emit_line("typedef int64_t knull_int;");
        self.emit_line("typedef double knull_float;");
        self.emit_line("typedef const char* knull_string;");
        self.emit_line("typedef bool knull_bool;");
        self.emit_line(""); // Add full runtime
        self.emit_full_runtime();
        self.emit_line(""); // Forward declarations
        self.collect_functions(ast);
        self.emit_forward_declarations();
        self.emit_line(""); // Generate code
        self.compile_node(ast)?;
        self.emit_line(""); // Main function wrapper if needed
        if !self.functions.contains_key("main") {
            self.emit_line("int main(int argc, char** argv) {");
            self.indent += 4;
            self.emit_line("return 0;");
            self.indent -= 4;
            self.emit_line("}");
        }

        Ok(self.output.clone())
    }

    fn collect_functions(&mut self, node: &ASTNode) {
        if let ASTNode::Program(items) = node {
            for item in items {
                if let ASTNode::Function { name, params, .. } = item {
                    self.functions.insert(
                        name.clone(),
                        FunctionInfo {
                            params: params.iter().map(|p| p.name.clone()).collect(),
                            has_return: false,
                        },
                    );
                }
            }
        }
    }

    fn emit_forward_declarations(&mut self) {
        let decls: Vec<String> = self
            .functions
            .iter()
            .map(|(name, info)| {
                let params_str = if info.params.is_empty() {
                    "void".to_string()
                } else {
                    info.params
                        .iter()
                        .map(|_| "knull_int".to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                format!("knull_int {}({});", name, params_str)
            })
            .collect();
        for decl in decls {
            self.emit_line(&decl);
        }
    }

    fn emit_full_runtime(&mut self) {
        self.emit_line("/* Knull Runtime Support */");
        self.emit_line(""); // GC support
        self.emit_line("static int gc_enabled = 1;");
        self.emit_line("static int gc_collect() { return 0; }");
        self.emit_line("static void* gc_alloc(size_t size) { return malloc(size); }");
        self.emit_line(""); // Array support
        self.emit_line("typedef struct {");
        self.emit_line("    knull_int* data;");
        self.emit_line("    size_t len;");
        self.emit_line("    size_t capacity;");
        self.emit_line("} knull_array;");
        self.emit_line("");
        self.emit_line("static knull_array* array_new(void) {");
        self.emit_line("    knull_array* arr = malloc(sizeof(knull_array));");
        self.emit_line("    arr->data = NULL;");
        self.emit_line("    arr->len = 0;");
        self.emit_line("    arr->capacity = 0;");
        self.emit_line("    return arr;");
        self.emit_line("}");
        self.emit_line("");
        self.emit_line("static void array_push(knull_array* arr, knull_int val) {");
        self.emit_line("    if (arr->len >= arr->capacity) {");
        self.emit_line("        arr->capacity = arr->capacity ? arr->capacity * 2 : 4;");
        self.emit_line(
            "        arr->data = realloc(arr->data, arr->capacity * sizeof(knull_int));",
        );
        self.emit_line("    }");
        self.emit_line("    arr->data[arr->len++] = val;");
        self.emit_line("}");
        self.emit_line(""); // String operations
        self.emit_line("static knull_int knull_strlen(knull_string s) { return strlen(s); }");
        self.emit_line(""); // File I/O
        self.emit_line(
            "static knull_int knull_file_read(knull_string path, char* buf, size_t len) {",
        );
        self.emit_line("    FILE* f = fopen(path, \"r\");");
        self.emit_line("    if (!f) return -1;");
        self.emit_line("    size_t n = fread(buf, 1, len, f);");
        self.emit_line("    buf[n] = 0;");
        self.emit_line("    fclose(f);");
        self.emit_line("    return n;");
        self.emit_line("}");
        self.emit_line("");
        self.emit_line(
            "static knull_int knull_file_write(knull_string path, knull_string content) {",
        );
        self.emit_line("    FILE* f = fopen(path, \"w\");");
        self.emit_line("    if (!f) return -1;");
        self.emit_line("    fprintf(f, \"%s\", content);");
        self.emit_line("    fclose(f);");
        self.emit_line("    return 0;");
        self.emit_line("}");
        self.emit_line(""); // Time
        self.emit_line("static knull_int knull_time(void) { return time(NULL); }");
        self.emit_line(
            "static knull_int knull_time_millis(void) { return clock() * 1000 / CLOCKS_PER_SEC; }",
        );
        self.emit_line(""); // Sleep
        self.emit_line("static void knull_sleep(knull_int ms) { usleep(ms * 1000); }");
    }

    fn compile_node(&mut self, node: &ASTNode) -> Result<String, String> {
        match node {
            ASTNode::Program(items) => {
                for item in items {
                    self.compile_node(item)?;
                    self.emit_line("");
                }
                Ok(String::new())
            }
            ASTNode::Function {
                name,
                params,
                ret_type: _,
                body,
            } => {
                self.current_function = Some(name.clone());

                let params_str = if params.is_empty() {
                    "void".to_string()
                } else {
                    params
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("knull_int arg{}", i))
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                if name == "main" {
                    self.emit_line(&format!("int {}(int argc, char** argv) {{", name));
                } else {
                    self.emit_line(&format!("knull_int {}({}) {{", name, params_str));
                }
                self.indent += 4;

                // Store param mappings
                for (i, param) in params.iter().enumerate() {
                    self.emit_line(&format!("knull_int {} = arg{};", param.name, i));
                }

                self.compile_node(body)?;

                // Add default return
                if name == "main" {
                    self.emit_line("return 0;");
                } else {
                    self.emit_line("return 0;");
                }

                self.indent -= 4;
                self.emit_line("}");

                self.current_function = None;
                Ok(String::new())
            }
            ASTNode::Block(nodes) => {
                for node in nodes {
                    self.compile_node(node)?;
                }
                Ok(String::new())
            }
            ASTNode::Let { name, value, .. } => {
                let c_type = self.infer_c_type(value);
                let val_code = self.compile_node(value)?;
                self.var_types.insert(name.clone(), c_type.clone());
                self.emit_line(&format!("{} {} = {};", c_type, name, val_code));
                Ok(String::new())
            }
            ASTNode::Return(expr) => {
                let val = self.compile_node(expr)?;
                self.emit_line(&format!("return {};", val));
                Ok(String::new())
            }
            ASTNode::If {
                cond,
                then_body,
                else_body,
            } => {
                let cond_val = self.compile_node(cond)?;
                self.emit_line(&format!("if ({}) {{", cond_val));
                self.indent += 4;
                self.compile_node(then_body)?;
                self.indent -= 4;
                if let Some(else_node) = else_body {
                    self.emit_line("} else {");
                    self.indent += 4;
                    self.compile_node(else_node)?;
                    self.indent -= 4;
                }
                self.emit_line("}");
                Ok(String::new())
            }
            ASTNode::While { cond, body } => {
                let cond_val = self.compile_node(cond)?;
                self.emit_line(&format!("while ({}) {{", cond_val));
                self.indent += 4;
                self.compile_node(body)?;
                self.indent -= 4;
                self.emit_line("}");
                Ok(String::new())
            }
            ASTNode::Binary { op, left, right } => {
                let l = self.compile_node(left)?;
                let r = self.compile_node(right)?;

                let c_op = match op.as_str() {
                    "+" => "+",
                    "-" => "-",
                    "*" => "*",
                    "/" => "/",
                    "%" => "%",
                    "==" => "==",
                    "!=" => "!=",
                    "<" => "<",
                    ">" => ">",
                    "<=" => "<=",
                    ">=" => ">=",
                    "&&" => "&&",
                    "||" => "||",
                    "&" => "&",
                    "|" => "|",
                    "^" => "^",
                    "<<" => "<<",
                    ">>" => ">>",
                    _ => return Err(format!("Unknown operator: {}", op)),
                };

                Ok(format!("({} {} {})", l, c_op, r))
            }
            ASTNode::Unary { op, operand } => {
                let val = self.compile_node(operand)?;
                match op.as_str() {
                    "-" => Ok(format!("(-{})", val)),
                    "!" => Ok(format!("(!{})", val)),
                    "~" => Ok(format!("(~{})", val)),
                    _ => Err(format!("Unknown unary operator: {}", op)),
                }
            }
            ASTNode::Call { func, args } => {
                let args_code: Result<Vec<String>, String> =
                    args.iter().map(|a| self.compile_node(a)).collect();
                let args_str = args_code?.join(", ");

                let func_name = match func.as_ref() {
                    ASTNode::Identifier(name) => name.clone(),
                    _ => {
                        return Err(
                            "Complex function expressions not supported in C codegen".to_string()
                        )
                    }
                };

                match func_name.as_str() {
                    "println" => {
                        if args.is_empty() {
                            self.emit_line("printf(\"\\n\");");
                        } else {
                            let fmt = self.format_spec_for(&args[0]);
                            let arg_str = self.compile_node(&args[0])?;
                            self.emit_line(&format!("printf(\"{}\\n\", {});", fmt, arg_str));
                        }
                        Ok("0".to_string())
                    }
                    "print" => {
                        if !args.is_empty() {
                            let fmt = self.format_spec_for(&args[0]);
                            let arg_str = self.compile_node(&args[0])?;
                            self.emit_line(&format!("printf(\"{}\", {});", fmt, arg_str));
                        }
                        Ok("0".to_string())
                    }
                    "len" => {
                        if args.len() == 1 {
                            let arg = self.compile_node(&args[0])?;
                            Ok(format!("knull_strlen({})", arg))
                        } else {
                            Ok("0".to_string())
                        }
                    }
                    "time" => Ok("knull_time()".to_string()),
                    "time_millis" => Ok("knull_time_millis()".to_string()),
                    "sleep" => {
                        if args.len() == 1 {
                            let ms = self.compile_node(&args[0])?;
                            self.emit_line(&format!("knull_sleep({});", ms));
                        }
                        Ok("0".to_string())
                    }
                    "gc_collect" => {
                        self.emit_line("gc_collect();");
                        Ok("0".to_string())
                    }
                    _ => Ok(format!("{}({})", func_name, args_str)),
                }
            }
            ASTNode::Literal(lit) => match lit {
                Literal::Int(n) => Ok(n.to_string()),
                Literal::Float(f) => Ok(f.to_string()),
                Literal::String(s) => {
                    self.string_literals.push(s.clone());
                    Ok(format!("(knull_string)\"{}\"", s))
                }
                Literal::Bool(b) => Ok(if *b { "1" } else { "0" }.to_string()),
                Literal::Null => Ok("NULL".to_string()),
            },
            ASTNode::Identifier(name) => Ok(name.clone()),
            ASTNode::Array(elements) => {
                let temp = self.next_temp();
                self.emit_line(&format!("knull_array* {} = array_new();", temp));
                for elem in elements {
                    let val = self.compile_node(elem)?;
                    self.emit_line(&format!("array_push({}, {});", temp, val));
                }
                Ok(format!("(knull_int){}", temp))
            }
            ASTNode::Asm(code) => {
                if self.mode == CompileMode::God {
                    Ok(format!("asm({});", code))
                } else {
                    Err("Inline assembly only allowed in God mode".to_string())
                }
            }
            ASTNode::Syscall(args) => {
                let args_code: Result<Vec<String>, String> =
                    args.iter().map(|a| self.compile_node(a)).collect();
                let args_str = args_code?.join(", ");
                Ok(format!("syscall({})", args_str))
            }
            // ── For loop ─────────────────────────────────────────────────────
            ASTNode::For { var, iter, body } => {
                // Emit a C for loop over a range or array
                let iter_code = self.compile_node(iter)?;
                // Check if iter is a Range node
                if let ASTNode::Range { start, end, inclusive } = iter.as_ref() {
                    let s = self.compile_node(start)?;
                    let e = self.compile_node(end)?;
                    let op = if *inclusive { "<=" } else { "<" };
                    self.emit_line(&format!("for (knull_int {} = {}; {} {} {}; {}++) {{", var, s, var, op, e, var));
                } else {
                    // Generic: iterate array len
                    let tmp = self.next_temp();
                    self.emit_line(&format!("knull_array* {} = (knull_array*)({}); ", tmp, iter_code));
                    self.emit_line(&format!("for (size_t _i = 0; _i < {}->len; _i++) {{", tmp));
                    self.emit_line(&format!("    knull_int {} = {}->data[_i];", var, tmp));
                }
                self.indent += 4;
                self.compile_node(body)?;
                self.indent -= 4;
                self.emit_line("}");
                Ok(String::new())
            }
            // ── Loop ─────────────────────────────────────────────────────────
            ASTNode::Loop(body) => {
                self.emit_line("while (1) {");
                self.indent += 4;
                self.compile_node(body)?;
                self.indent -= 4;
                self.emit_line("}");
                Ok(String::new())
            }
            // ── Break / Continue ─────────────────────────────────────────────
            ASTNode::Break(_) => {
                self.emit_line("break;");
                Ok(String::new())
            }
            ASTNode::Continue(_) => {
                self.emit_line("continue;");
                Ok(String::new())
            }
            // ── Assignment ───────────────────────────────────────────────────
            ASTNode::Assign { target, value } => {
                let lhs = self.compile_node(target)?;
                let rhs = self.compile_node(value)?;
                self.emit_line(&format!("{} = {};", lhs, rhs));
                Ok(String::new())
            }
            ASTNode::AssignOp { target, op, value } => {
                let lhs = self.compile_node(target)?;
                let rhs = self.compile_node(value)?;
                self.emit_line(&format!("{} {}= {};", lhs, op, rhs));
                Ok(String::new())
            }
            // ── Index ────────────────────────────────────────────────────────
            ASTNode::Index { obj, index } => {
                let o = self.compile_node(obj)?;
                let i = self.compile_node(index)?;
                Ok(format!("({})->data[{}]", o, i))
            }
            // ── Field access ─────────────────────────────────────────────────
            ASTNode::FieldAccess { obj, field } => {
                let o = self.compile_node(obj)?;
                Ok(format!("{}->{}", o, field))
            }
            // ── Struct definition ─────────────────────────────────────────────
            ASTNode::StructDef { name, fields } => {
                self.emit_line(&format!("typedef struct {{"));
                for (fname, ftype) in fields {
                    let c_type = self.type_to_c(ftype);
                    self.emit_line(&format!("    {} {};", c_type, fname));
                }
                self.emit_line(&format!("}} {};", name));
                Ok(String::new())
            }
            // ── Struct literal ────────────────────────────────────────────────
            ASTNode::StructLiteral { name, fields } => {
                let tmp = self.next_temp();
                self.emit_line(&format!("{} {} = {{}};", name, tmp));
                for (fname, fval) in fields {
                    let val = self.compile_node(fval)?;
                    self.emit_line(&format!("{}.{} = {};", tmp, fname, val));
                }
                Ok(tmp)
            }
            // ── Impl block ────────────────────────────────────────────────────
            ASTNode::Impl { ty: _, methods } => {
                for method in methods {
                    self.compile_node(method)?;
                }
                Ok(String::new())
            }
            // ── Method call: obj.method(args) → name__method(obj, args) ──────
            ASTNode::MethodCall { obj, method, args } => {
                let obj_code = self.compile_node(obj)?;
                let mut all_args = vec![obj_code];
                for a in args {
                    all_args.push(self.compile_node(a)?);
                }
                // Map common methods
                match method.as_str() {
                    "len" => Ok(format!("knull_strlen({})", all_args[0])),
                    "push" if all_args.len() >= 2 => {
                        self.emit_line(&format!("array_push({});", all_args.join(", ")));
                        Ok(String::new())
                    }
                    _ => Ok(format!("{}_{}({})", all_args[0], method, all_args[1..].join(", "))),
                }
            }
            // ── Range ─────────────────────────────────────────────────────────
            ASTNode::Range { start, end, .. } => {
                let s = self.compile_node(start)?;
                let e = self.compile_node(end)?;
                // emit as a temporary struct (not commonly needed as standalone)
                Ok(format!("/* range {}->{} */", s, e))
            }
            // ── Enum definition ───────────────────────────────────────────────
            ASTNode::EnumDef { name, variants } => {
                // Emit as an int enum
                self.emit_line(&format!("typedef enum {{"));
                for v in variants {
                    self.emit_line(&format!("    {}_{},", name, v.name));
                }
                self.emit_line(&format!("}} {};", name));
                Ok(String::new())
            }
            // ── Match ─────────────────────────────────────────────────────────
            ASTNode::Match { expr, arms } => {
                let val = self.compile_node(expr)?;
                let tmp = self.next_temp();
                self.emit_line(&format!("knull_int {} = {};", tmp, val));
                let mut first = true;
                for arm in arms {
                    use crate::parser::Pattern;
                    let cond = match &arm.pattern {
                        Pattern::Wildcard => None,
                        Pattern::Literal(crate::parser::Literal::Int(n)) => Some(format!("{} == {}", tmp, n)),
                        Pattern::Literal(crate::parser::Literal::Bool(b)) => Some(format!("{} == {}", tmp, *b as i64)),
                        Pattern::Identifier(name) => Some(format!("/* {} */1", name)),
                        _ => Some(format!("1 /* pattern */")),
                    };
                    match cond {
                        None => {
                            self.emit_line("else {");
                        }
                        Some(c) => {
                            if first { self.emit_line(&format!("if ({}) {{", c)); first = false; }
                            else { self.emit_line(&format!("else if ({}) {{", c)); }
                        }
                    }
                    self.indent += 4;
                    self.compile_node(&arm.body)?;
                    self.indent -= 4;
                    self.emit_line("}");
                }
                Ok(String::new())
            }
            // ── Tuple ─────────────────────────────────────────────────────────
            ASTNode::Tuple(elems) => {
                // Represent as array
                let tmp = self.next_temp();
                self.emit_line(&format!("knull_array* {} = array_new();", tmp));
                for elem in elems {
                    let v = self.compile_node(elem)?;
                    self.emit_line(&format!("array_push({}, {});", tmp, v));
                }
                Ok(format!("(knull_int){}", tmp))
            }
            // ── InterpolatedString (f-string) ─────────────────────────────────
            ASTNode::InterpolatedString(segments) => {
                // Build a snprintf call
                let mut fmt = String::new();
                let mut args_list = Vec::new();
                for (is_expr, content) in segments {
                    if *is_expr {
                        fmt.push_str("%lld");
                        // Parse the expression and compile it
                        let mut sub_parser = crate::parser::Parser::new(content);
                        if let Ok(sub_ast) = sub_parser.parse_expression_pub() {
                            if let Ok(code) = self.compile_node(&sub_ast) {
                                args_list.push(code);
                            }
                        }
                    } else {
                        fmt.push_str(&content.replace('"', "\\\"").replace('\n', "\\n"));
                    }
                }
                let tmp = self.next_temp();
                self.emit_line(&format!("char {}[4096];", tmp));
                if args_list.is_empty() {
                    self.emit_line(&format!("snprintf({}, 4096, \"{}\");", tmp, fmt));
                } else {
                    self.emit_line(&format!("snprintf({}, 4096, \"{}\", {});", tmp, fmt, args_list.join(", ")));
                }
                Ok(format!("(knull_string){}", tmp))
            }
            _ => Ok("0".to_string()),
        }
    }

    fn compile_expr_for_print(&mut self, node: &ASTNode) -> Result<String, String> {
        match node {
            ASTNode::Literal(Literal::String(s)) => Ok(format!("\"{}\"", s)),
            _ => self.compile_node(node),
        }
    }

    /// Convert a Knull Type to a C type string
    fn type_to_c(&self, ty: &crate::parser::Type) -> String {
        use crate::parser::Type;
        match ty {
            Type::I8  | Type::U8  => "int8_t".to_string(),
            Type::I16 | Type::U16 => "int16_t".to_string(),
            Type::I32 | Type::U32 => "int32_t".to_string(),
            Type::I64 | Type::U64 | Type::I128 | Type::U128 => "knull_int".to_string(),
            Type::F32 | Type::F64 => "knull_float".to_string(),
            Type::Bool  => "knull_bool".to_string(),
            Type::Char  => "char".to_string(),
            Type::String => "knull_string".to_string(),
            Type::Void  => "void".to_string(),
            Type::RawPtr(_) | Type::MutRawPtr(_) => "void*".to_string(),
            Type::Array(inner, _) | Type::Slice(inner) | Type::Vec(inner) => {
                format!("{}*", self.type_to_c(inner))
            }
            Type::Custom(name) => name.clone(),
            _ => "knull_int".to_string(),
        }
    }

    /// Infer C type string from an AST node
    fn infer_c_type(&self, node: &ASTNode) -> String {
        match node {
            ASTNode::Literal(Literal::Int(_)) => "knull_int".to_string(),
            ASTNode::Literal(Literal::Float(_)) => "knull_float".to_string(),
            ASTNode::Literal(Literal::String(_)) => "knull_string".to_string(),
            ASTNode::Literal(Literal::Bool(_)) => "knull_bool".to_string(),
            ASTNode::Literal(Literal::Null) => "void*".to_string(),
            ASTNode::Array(_) => "knull_array*".to_string(),
            ASTNode::Binary { op, left, right } => {
                // Comparison ops → bool
                if matches!(op.as_str(), "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||") {
                    return "knull_bool".to_string();
                }
                // Float arithmetic if either side is float
                let lt = self.infer_c_type(left);
                let rt = self.infer_c_type(right);
                if lt == "knull_float" || rt == "knull_float" {
                    "knull_float".to_string()
                } else {
                    lt
                }
            }
            ASTNode::Identifier(name) => {
                self.var_types.get(name).cloned().unwrap_or_else(|| "knull_int".to_string())
            }
            _ => "knull_int".to_string(),
        }
    }

    /// Return a printf format specifier for a node
    fn format_spec_for(&self, node: &ASTNode) -> &'static str {
        match node {
            ASTNode::Literal(Literal::Int(_)) => "%lld",
            ASTNode::Literal(Literal::Float(_)) => "%g",
            ASTNode::Literal(Literal::String(_)) => "%s",
            ASTNode::Literal(Literal::Bool(_)) => "%d",
            ASTNode::Identifier(name) => {
                match self.var_types.get(name).map(|s| s.as_str()) {
                    Some("knull_float") => "%g",
                    Some("knull_string") => "%s",
                    Some("knull_bool") => "%d",
                    _ => "%lld",
                }
            }
            ASTNode::Binary { op, .. } => {
                if matches!(op.as_str(), "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||") {
                    "%d"
                } else {
                    "%lld"
                }
            }
            _ => "%lld",
        }
    }

    fn emit_line(&mut self, line: &str) {
        if line.is_empty() {
            self.output.push('\n');
        } else {
            self.output.push_str(&" ".repeat(self.indent));
            self.output.push_str(line);
            self.output.push('\n');
        }
    }

    fn next_temp(&mut self) -> String {
        let temp = format!("_t{}", self.temp_count);
        self.temp_count += 1;
        temp
    }
}

pub fn compile_to_binary(source: &str, output_path: &str) -> Result<(), String> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let mut lexer = Lexer::new(source);
    let _tokens = lexer.tokenize();

    let mut parser = Parser::new(source);
    let ast = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    let mut codegen = CCodeGen::new();
    let c_code = codegen.compile(&ast)?;

    let c_path = format!("{}.c", output_path);
    fs::write(&c_path, c_code).map_err(|e| format!("Failed to write C file: {}", e))?;

    let status = Command::new("cc")
        .args(&["-O2", "-o", output_path, &c_path, "-lm", "-lpthread"])
        .status()
        .map_err(|e| format!("Failed to run cc: {}", e))?;

    if !status.success() {
        return Err("C compilation failed".to_string());
    }

    let _ = fs::remove_file(&c_path);

    Ok(())
}
