//! Knull Compiler - Type Checking and Module Compilation
//!
//! This module handles type checking, semantic analysis, and orchestration
//! of code generation.

use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};

use crate::ast::*;

/// Knull Compiler
///
/// Performs type checking and orchestrates code generation.
pub struct Compiler {
    ast: Program,
    symbols: SymbolTable,
    types: TypeDatabase,
    debug: bool,
    lto: bool,
}

impl Compiler {
    /// Create a new compiler from an AST
    pub fn new(ast: &Program) -> Self {
        Compiler {
            ast: ast.clone(),
            symbols: SymbolTable::new(),
            types: TypeDatabase::new(),
            debug: false,
            lto: false,
        }
    }

    /// Enable debug mode
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Enable LTO
    pub fn with_lto(mut self, lto: bool) -> Self {
        self.lto = lto;
        self
    }

    /// Compile the AST to an executable module
    pub fn compile(&mut self) -> Result<CompiledModule> {
        // Phase 1: Build symbol table
        self.build_symbols()?;

        // Phase 2: Type checking
        self.type_check()?;

        // Phase 3: Generate code
        let ir = self.generate_ir()?;

        Ok(CompiledModule {
            ir,
            debug: self.debug,
            lto: self.lto,
        })
    }

    /// Build the symbol table
    fn build_symbols(&mut self) -> Result<()> {
        for item in &self.ast.items {
            match item {
                Item::Function(f) => {
                    if let Some(ref rt) = f.return_type {
                        self.symbols.declare_function(&f.name, rt)?;
                    }
                }
                Item::Struct(s) => {
                    self.symbols.declare_type(&s.name)?;
                }
                Item::Enum(e) => {
                    self.symbols.declare_type(&e.name)?;
                }
                Item::Const(c) => {
                    self.symbols.declare_const(&c.name)?;
                }
                Item::Static(s) => {
                    self.symbols.declare_static(&s.name)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Type check the AST
    fn type_check(&mut self) -> Result<()> {
        let items = self.ast.items.clone();
        for item in items {
            self.check_item(&item)?;
        }
        Ok(())
    }

    fn check_item(&mut self, item: &Item) -> Result<()> {
        match item {
            Item::Function(f) => self.check_function(f),
            Item::Struct(s) => self.check_struct(s),
            Item::Enum(e) => self.check_enum(e),
            _ => Ok(()),
        }
    }

    fn check_function(&self, f: &Function) -> Result<()> {
        let mut local_vars: HashMap<String, Type> = HashMap::new();

        // Add parameters
        for param in &f.params {
            local_vars.insert(param.name.clone(), param.ty.clone());
        }

        // Check body
        self.check_block(&f.body, &local_vars)?;

        Ok(())
    }

    fn check_block(&self, block: &Block, vars: &HashMap<String, Type>) -> Result<()> {
        let mut local_vars = vars.clone();

        for stmt in &block.stmts {
            self.check_statement(stmt, &mut local_vars)?;
        }

        Ok(())
    }

    fn check_statement(&self, stmt: &Stmt, vars: &mut HashMap<String, Type>) -> Result<()> {
        match stmt {
            Stmt::Let { pattern, ty, init } => {
                if let Some(init) = init {
                    let init_type = self.infer_expr(init, vars)?;

                    // Check type compatibility
                    if let Some(ty) = ty {
                        self.unify_types(ty, &init_type)?;
                    }
                }
                Ok(())
            }
            Stmt::Return { value } => {
                if let Some(value) = value {
                    self.infer_expr(value, vars)?;
                }
                Ok(())
            }
            Stmt::Expr(expr) => {
                self.infer_expr(expr, vars)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn infer_expr(&self, expr: &Expr, vars: &HashMap<String, Type>) -> Result<Type> {
        match expr {
            Expr::Literal(lit, _) => Ok(match lit {
                Literal::Int(_) => Type::Name("i64".to_string()),
                Literal::Float(_) => Type::Name("f64".to_string()),
                Literal::String(_) => Type::Name("String".to_string()),
                Literal::Char(_) => Type::Name("char".to_string()),
                Literal::Bool(_) => Type::Name("bool".to_string()),
            }),
            Expr::Ident(name, _) => vars
                .get(name)
                .cloned()
                .or_else(|| self.symbols.lookup_var(name))
                .ok_or_else(|| anyhow::anyhow!("Unknown identifier: {}", name)),
            Expr::Binary {
                op, left, right, ..
            } => {
                let left_type = self.infer_expr(left, vars)?;
                let right_type = self.infer_expr(right, vars)?;

                // Check type compatibility
                self.unify_types(&left_type, &right_type)?;

                // Infer result type
                Ok(match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem => left_type,
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Gt
                    | BinaryOp::Le
                    | BinaryOp::Ge => Type::Name("bool".to_string()),
                    BinaryOp::And | BinaryOp::Or => Type::Name("bool".to_string()),
                    _ => left_type,
                })
            }
            Expr::Call { func, args, .. } => {
                // Infer argument types
                for arg in args {
                    self.infer_expr(arg, vars)?;
                }

                // Look up function return type
                if let Expr::Ident(name, _) = func.as_ref() {
                    self.symbols
                        .lookup_function(name)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("Unknown function: {}", name))
                } else {
                    bail!("Cannot call non-function")
                }
            }
            _ => Ok(Type::Name("var".to_string())),
        }
    }

    fn unify_types(&self, a: &Type, b: &Type) -> Result<()> {
        match (a, b) {
            (Type::Name(a), Type::Name(b)) if a == b => Ok(()),
            (a, b) if a.name() == "var" || b.name() == "var" => Ok(()),
            _ => {
                // For now, just allow any types
                Ok(())
            }
        }
    }

    fn check_struct(&mut self, s: &StructDef) -> Result<()> {
        // Register struct fields
        for field in &s.fields {
            self.types
                .register_field(&s.name, &field.name, field.ty.clone());
        }
        Ok(())
    }

    fn check_enum(&mut self, e: &EnumDef) -> Result<()> {
        // Register enum variants
        for variant in &e.variants {
            self.types
                .register_variant(&e.name, &variant.name, variant.data.clone());
        }
        Ok(())
    }

    /// Generate intermediate representation
    fn generate_ir(&self) -> Result<IrModule> {
        let mut module = IrModule::new();

        // Generate functions
        for item in &self.ast.items {
            if let Item::Function(f) = item {
                let ir_fn = self.generate_function(f)?;
                module.functions.push(ir_fn);
            }
        }

        Ok(module)
    }

    fn generate_function(&self, f: &Function) -> Result<IrFunction> {
        let mut ir_fn = IrFunction::new(&f.name);

        // Generate basic blocks
        self.generate_block(&f.body, &mut ir_fn)?;

        Ok(ir_fn)
    }

    fn generate_block(&self, block: &Block, ir_fn: &mut IrFunction) -> Result<()> {
        for stmt in &block.stmts {
            self.generate_statement(stmt, ir_fn)?;
        }
        Ok(())
    }

    fn generate_statement(&self, stmt: &Stmt, ir_fn: &mut IrFunction) -> Result<()> {
        match stmt {
            Stmt::Let { pattern, ty, init } => {
                if let Some(init) = init {
                    let dest = self.get_destination(pattern, ir_fn);
                    self.generate_expression(init, ir_fn)?;
                }
                Ok(())
            }
            Stmt::Return { value } => {
                if let Some(value) = value {
                    self.generate_expression(value, ir_fn)?;
                    ir_fn.emit(IrInstruction::Ret { value: None });
                } else {
                    ir_fn.emit(IrInstruction::RetVoid);
                }
                Ok(())
            }
            Stmt::Expr(expr) => {
                self.generate_expression(expr, ir_fn)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn generate_expression(&self, expr: &Expr, ir_fn: &mut IrFunction) -> Result<()> {
        match expr {
            Expr::Literal(lit, _) => {
                let value = match lit {
                    Literal::Int(i) => IrValue::Int(*i),
                    Literal::Float(f) => IrValue::Float(*f),
                    Literal::String(s) => IrValue::String(s.clone()),
                    Literal::Char(c) => IrValue::Int(*c as i64),
                    Literal::Bool(b) => IrValue::Int(if *b { 1 } else { 0 }),
                };
                ir_fn.emit(IrInstruction::Constant {
                    dest: "%tmp".to_string(),
                    value,
                });
                Ok(())
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                self.generate_expression(left, ir_fn)?;
                self.generate_expression(right, ir_fn)?;

                let ir_op = match op {
                    BinaryOp::Add => IrBinaryOp::Add,
                    BinaryOp::Sub => IrBinaryOp::Sub,
                    BinaryOp::Mul => IrBinaryOp::Mul,
                    BinaryOp::Div => IrBinaryOp::Div,
                    BinaryOp::Rem => IrBinaryOp::Rem,
                    BinaryOp::Eq => IrBinaryOp::Eq,
                    BinaryOp::Ne => IrBinaryOp::Ne,
                    BinaryOp::Lt => IrBinaryOp::Lt,
                    BinaryOp::Gt => IrBinaryOp::Gt,
                    BinaryOp::Le => IrBinaryOp::Le,
                    BinaryOp::Ge => IrBinaryOp::Ge,
                    BinaryOp::And => IrBinaryOp::And,
                    BinaryOp::Or => IrBinaryOp::Or,
                    _ => IrBinaryOp::Add,
                };

                ir_fn.emit(IrInstruction::Binary {
                    dest: "%tmp".to_string(),
                    op: ir_op,
                    left: "%tmp".to_string(),
                    right: "%tmp".to_string(),
                });
                Ok(())
            }
            Expr::Call { func, args, .. } => {
                if let Expr::Ident(name, _) = func.as_ref() {
                    for arg in args {
                        self.generate_expression(arg, ir_fn)?;
                    }
                    ir_fn.emit(IrInstruction::Call {
                        dest: "%result".to_string(),
                        func: name.clone(),
                        args: vec![],
                    });
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn get_destination(&self, pattern: &Pattern, ir_fn: &mut IrFunction) -> String {
        if let Pattern::Ident(name, _) = pattern {
            format!("%{}", name)
        } else {
            "%tmp".to_string()
        }
    }
}

/// Type Checker
pub struct TypeChecker {
    symbols: SymbolTable,
    types: TypeDatabase,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            symbols: SymbolTable::new(),
            types: TypeDatabase::new(),
        }
    }

    pub fn check(ast: &Program) -> Result<()> {
        let mut compiler = Compiler::new(ast);
        compiler.type_check()
    }
}

/// Symbol Table
#[derive(Default)]
pub struct SymbolTable {
    functions: HashMap<String, Type>,
    types: HashSet<String>,
    consts: HashSet<String>,
    statics: HashSet<String>,
    vars: HashMap<String, Type>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = SymbolTable::default();

        // Add built-in functions
        table
            .functions
            .insert("print".to_string(), Type::Name("void".to_string()));
        table
            .functions
            .insert("println".to_string(), Type::Name("void".to_string()));
        table
            .functions
            .insert("read_line".to_string(), Type::Name("String".to_string()));

        // Add built-in types
        table.types.insert("i8".to_string());
        table.types.insert("i16".to_string());
        table.types.insert("i32".to_string());
        table.types.insert("i64".to_string());
        table.types.insert("i128".to_string());
        table.types.insert("u8".to_string());
        table.types.insert("u16".to_string());
        table.types.insert("u32".to_string());
        table.types.insert("u64".to_string());
        table.types.insert("u128".to_string());
        table.types.insert("f32".to_string());
        table.types.insert("f64".to_string());
        table.types.insert("bool".to_string());
        table.types.insert("char".to_string());
        table.types.insert("String".to_string());
        table.types.insert("void".to_string());

        table
    }

    pub fn declare_function(&mut self, name: &str, _ty: &Type) -> Result<()> {
        self.functions
            .insert(name.to_string(), Type::Name(name.to_string()));
        Ok(())
    }

    pub fn declare_type(&mut self, name: &str) -> Result<()> {
        self.types.insert(name.to_string());
        Ok(())
    }

    pub fn declare_const(&mut self, name: &str) -> Result<()> {
        self.consts.insert(name.to_string());
        Ok(())
    }

    pub fn declare_static(&mut self, name: &str) -> Result<()> {
        self.statics.insert(name.to_string());
        Ok(())
    }

    pub fn lookup_function(&self, name: &str) -> Option<&Type> {
        self.functions.get(name)
    }

    pub fn lookup_type(&self, name: &str) -> bool {
        self.types.contains(name)
    }

    pub fn lookup_var(&self, name: &str) -> Option<Type> {
        self.vars.get(name).cloned()
    }
}

/// Type Database
#[derive(Default)]
pub struct TypeDatabase {
    fields: HashMap<(String, String), Type>,
    variants: HashMap<String, Vec<(String, Option<EnumData>)>>,
}

impl TypeDatabase {
    pub fn new() -> Self {
        TypeDatabase::default()
    }

    pub fn register_field(&mut self, struct_name: &str, field_name: &str, ty: Type) {
        self.fields
            .insert((struct_name.to_string(), field_name.to_string()), ty);
    }

    pub fn register_variant(
        &mut self,
        enum_name: &str,
        variant_name: &str,
        data: Option<EnumData>,
    ) {
        self.variants
            .entry(enum_name.to_string())
            .or_insert_with(Vec::new)
            .push((variant_name.to_string(), data));
    }
}

/// Intermediate Representation
#[derive(Debug, Clone)]
pub struct IrModule {
    pub functions: Vec<IrFunction>,
    pub globals: Vec<IrGlobal>,
}

impl IrModule {
    pub fn new() -> Self {
        IrModule {
            functions: Vec::new(),
            globals: Vec::new(),
        }
    }

    pub fn llvm_ir(&self) -> String {
        let mut ir = String::new();
        ir.push_str("; Knull IR\n\n");

        for global in &self.globals {
            ir.push_str(&global.to_string());
            ir.push('\n');
        }

        for func in &self.functions {
            ir.push_str(&func.to_string());
            ir.push('\n');
        }

        ir
    }
}

/// IR Function
#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<String>,
    pub blocks: Vec<IrBasicBlock>,
    current_block: usize,
}

impl IrFunction {
    pub fn new(name: &str) -> Self {
        IrFunction {
            name: name.to_string(),
            params: Vec::new(),
            blocks: vec![IrBasicBlock::new("entry")],
            current_block: 0,
        }
    }

    pub fn emit(&mut self, instruction: IrInstruction) {
        self.blocks[self.current_block]
            .instructions
            .push(instruction);
    }

    pub fn to_string(&self) -> String {
        let mut s = format!("define void @{}() {{\n", self.name);

        for block in &self.blocks {
            s.push_str(&block.to_string());
            s.push('\n');
        }

        s.push_str("}\n");
        s
    }
}

/// IR Basic Block
#[derive(Debug, Clone)]
pub struct IrBasicBlock {
    pub name: String,
    pub instructions: Vec<IrInstruction>,
}

impl IrBasicBlock {
    pub fn new(name: &str) -> Self {
        IrBasicBlock {
            name: name.to_string(),
            instructions: Vec::new(),
        }
    }

    pub fn to_string(&self) -> String {
        let mut s = format!("{}:\n", self.name);

        for inst in &self.instructions {
            s.push_str(&format!("  {}\n", inst.to_string()));
        }

        s
    }
}

/// IR Instruction
#[derive(Debug, Clone)]
pub enum IrInstruction {
    Constant {
        dest: String,
        value: IrValue,
    },
    Binary {
        dest: String,
        op: IrBinaryOp,
        left: String,
        right: String,
    },
    Ret {
        value: Option<String>,
    },
    RetVoid,
    Store {
        value: String,
        dest: String,
    },
    Load {
        dest: String,
        src: String,
    },
    Call {
        dest: String,
        func: String,
        args: Vec<String>,
    },
    Br {
        label: String,
    },
    CondBr {
        cond: String,
        true_label: String,
        false_label: String,
    },
    Label {
        name: String,
    },
}

impl IrInstruction {
    pub fn to_string(&self) -> String {
        match self {
            IrInstruction::Constant { dest, value } => format!("{} = {}", dest, value.to_string()),
            IrInstruction::Binary {
                dest,
                op,
                left,
                right,
            } => {
                format!("{} = {} {}, {}", dest, op.to_string(), left, right)
            }
            IrInstruction::Ret { value } => {
                if let Some(v) = value {
                    format!("ret i64 {}", v)
                } else {
                    "ret void".to_string()
                }
            }
            IrInstruction::RetVoid => "ret void".to_string(),
            IrInstruction::Store { value, dest } => format!("store i64 {}, i64* {}", value, dest),
            IrInstruction::Load { dest, src } => format!("{} = load i64, i64* {}", dest, src),
            IrInstruction::Call { dest, func, args } => {
                format!("{} = call i64 @{}({})", dest, func, args.join(", "))
            }
            IrInstruction::Br { label } => format!("br label %{}", label),
            IrInstruction::CondBr {
                cond,
                true_label,
                false_label,
            } => {
                format!(
                    "br i1 {}, label %{}, label %{}",
                    cond, true_label, false_label
                )
            }
            IrInstruction::Label { name } => format!("{}:", name),
        }
    }
}

/// IR Binary Operation
#[derive(Debug, Clone)]
pub enum IrBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Shl,
    Shr,
}

impl IrBinaryOp {
    pub fn to_string(&self) -> &'static str {
        match self {
            IrBinaryOp::Add => "add",
            IrBinaryOp::Sub => "sub",
            IrBinaryOp::Mul => "mul",
            IrBinaryOp::Div => "sdiv",
            IrBinaryOp::Rem => "srem",
            IrBinaryOp::And => "and",
            IrBinaryOp::Or => "or",
            IrBinaryOp::Eq => "icmp eq",
            IrBinaryOp::Ne => "icmp ne",
            IrBinaryOp::Lt => "icmp slt",
            IrBinaryOp::Gt => "icmp sgt",
            IrBinaryOp::Le => "icmp sle",
            IrBinaryOp::Ge => "icmp sge",
            IrBinaryOp::Shl => "shl",
            IrBinaryOp::Shr => "ashr",
        }
    }
}

/// IR Value
#[derive(Debug, Clone)]
pub enum IrValue {
    Int(i64),
    Float(f64),
    String(String),
}

impl IrValue {
    pub fn to_string(&self) -> String {
        match self {
            IrValue::Int(i) => i.to_string(),
            IrValue::Float(f) => f.to_string(),
            IrValue::String(s) => format!("\"{}\"", s),
        }
    }
}

/// IR Global
#[derive(Debug, Clone)]
pub struct IrGlobal {
    pub name: String,
    pub ty: Type,
    pub value: Option<IrValue>,
}

impl IrGlobal {
    pub fn to_string(&self) -> String {
        if let Some(value) = &self.value {
            format!("@{} = constant i64 {}", self.name, value.to_string())
        } else {
            format!("@{} = global i64 0", self.name)
        }
    }
}

/// Compiled Module
pub struct CompiledModule {
    pub ir: IrModule,
    pub debug: bool,
    pub lto: bool,
}
