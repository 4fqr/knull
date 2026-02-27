//! Knull Abstract Syntax Tree (AST)
//! 
//! This module defines the AST node types for the Knull language.

use crate::lexer::Span;

/// A Knull program
#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

/// A top-level item in a Knull module
#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
    Struct(StructDef),
    Enum(EnumDef),
    Trait(TraitDef),
    Impl(ImplBlock),
    TypeAlias(TypeAlias),
    Const(ConstItem),
    Static(StaticItem),
    Module(ModuleItem),
    Import(ImportItem),
    Actor(ActorDef),
    Stmt(Stmt),
}

/// Function definition
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Block,
    pub span: Span,
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub default: Option<Expr>,
}

/// Type parameter (generic)
#[derive(Debug, Clone)]
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<Type>,
}

/// Struct definition
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub fields: Vec<Field>,
    pub span: Span,
}

/// Struct field
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub visibility: Visibility,
}

/// Enum definition
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

/// Enum variant
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub data: Option<EnumData>,
}

/// Enum data
#[derive(Debug, Clone)]
pub enum EnumData {
    Tuple(Vec<Type>),
    Struct(Vec<(String, Type)>),
}

/// Trait definition
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub items: Vec<TraitItem>,
    pub span: Span,
}

/// Trait item
#[derive(Debug, Clone)]
pub enum TraitItem {
    Function(FunctionSig),
    Const(ConstSig),
}

/// Function signature
#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
}

/// Const signature
#[derive(Debug, Clone)]
pub struct ConstSig {
    pub name: String,
    pub ty: Type,
}

/// Implementation block
#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub trait_name: Option<String>,
    pub type_name: Type,
    pub items: Vec<ImplItem>,
    pub span: Span,
}

/// Implementation item
#[derive(Debug, Clone)]
pub enum ImplItem {
    Function(Function),
    Const(ConstItem),
}

/// Type alias
#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub ty: Type,
    pub span: Span,
}

/// Constant item
#[derive(Debug, Clone)]
pub struct ConstItem {
    pub name: String,
    pub ty: Type,
    pub value: Expr,
    pub span: Span,
}

/// Static item
#[derive(Debug, Clone)]
pub struct StaticItem {
    pub name: String,
    pub ty: Type,
    pub value: Option<Expr>,
    pub mutable: bool,
    pub span: Span,
}

/// Module item
#[derive(Debug, Clone)]
pub struct ModuleItem {
    pub name: String,
    pub items: Vec<Item>,
    pub span: Span,
}

/// Import item
#[derive(Debug, Clone)]
pub struct ImportItem {
    pub path: String,
    pub alias: Option<String>,
    pub items: Vec<String>,
    pub span: Span,
}

/// Actor definition
#[derive(Debug, Clone)]
pub struct ActorDef {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub state: Option<Type>,
    pub methods: Vec<Function>,
    pub span: Span,
}

/// Visibility modifier
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visibility {
    Public,
    Private,
}

/// Knull types
#[derive(Debug, Clone)]
pub enum Type {
    Name(String),
    Generic(String, Vec<Type>),
    Pointer(Box<Type>, bool),
    Reference(Box<Type>, bool),
    Array(Box<Type>, usize),
    Slice(Box<Type>),
    Tuple(Vec<Type>),
    Function(Vec<Type>, Option<Box<Type>>),
    Option(Box<Type>),
    Sum(Box<Type>, Box<Type>),
    Never,
    Void,
}

impl Type {
    pub fn name(&self) -> String {
        match self {
            Type::Name(s) => s.clone(),
            Type::Generic(s, _) => s.clone(),
            _ => "unknown".to_string(),
        }
    }
}

/// Block expression
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// Statement
#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Let {
        pattern: Pattern,
        ty: Option<Type>,
        init: Option<Expr>,
    },
    Var {
        pattern: Pattern,
        ty: Option<Type>,
        init: Option<Expr>,
    },
    While {
        label: Option<String>,
        condition: Expr,
        body: Box<Stmt>,
    },
    For {
        pattern: Pattern,
        iter: Expr,
        body: Box<Stmt>,
    },
    Return {
        value: Option<Expr>,
    },
    Break {
        label: Option<String>,
        value: Option<Expr>,
    },
    Continue {
        label: Option<String>,
    },
    Defer {
        expr: Box<Expr>,
    },
    Unsafe {
        block: Box<Stmt>,
    },
    Asm {
        syntax: Option<String>,
        code: String,
    },
    Syscall {
        args: Vec<Expr>,
    },
    Comptime {
        block: Box<Stmt>,
    },
}

/// Expression
#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    Literal(Literal, Span),
    
    // Identifiers
    Ident(String, Span),
    
    // Unary operations
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    
    // Binary operations
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    
    // Ternary
    Ternary {
        condition: Box<Expr>,
        then: Box<Expr>,
        else_: Box<Expr>,
        span: Span,
    },
    
    // Block
    Block(Block),
    
    // If expression
    If {
        condition: Box<Expr>,
        then: Box<Expr>,
        else: Option<Box<Expr>>,
        span: Span,
    },
    
    // Match expression
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    
    // Loop
    Loop {
        label: Option<String>,
        body: Box<Expr>,
        span: Span,
    },
    
    // While loop
    While {
        label: Option<String>,
        condition: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    
    // For loop
    For {
        pattern: Pattern,
        iter: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    
    // Function call
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    
    // Method call
    MethodCall {
        expr: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    
    // Field access
    Field {
        expr: Box<Expr>,
        field: String,
        span: Span,
    },
    
    // Index access
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    
    // Array literal
    Array(Vec<Expr>, Span),
    
    // Struct literal
    Struct {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    
    // Tuple
    Tuple(Vec<Expr>, Span),
    
    // Parenthesized expression
    Paren(Box<Expr>),
    
    // Lambda
    Lambda {
        params: Vec<String>,
        return_type: Option<Type>,
        body: Box<Expr>,
        span: Span,
    },
    
    // Cast
    Cast {
        expr: Box<Expr>,
        ty: Type,
        span: Span,
    },
    
    // Range
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        span: Span,
    },
    
    // Unsafe
    Unsafe {
        expr: Box<Expr>,
        span: Span,
    },
    
    // Comptime
    Comptime {
        expr: Box<Expr>,
        span: Span,
    },
    
    // Break with value
    Break {
        label: Option<String>,
        value: Option<Box<Expr>>,
        span: Span,
    },
    
    // Continue
    Continue {
        label: Option<String>,
        span: Span,
    },
    
    // Return
    Return {
        value: Option<Box<Expr>>,
        span: Span,
    },
}

/// Literal values
#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
}

/// Match arm
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Block,
}

/// Pattern
#[derive(Debug, Clone)]
pub enum Pattern {
    Ident(String, Span),
    Wildcard(Span),
    Int(i64, Span),
    String(String, Span),
    Char(char, Span),
    TupleStruct(String, Vec<Pattern>, Span),
    Struct(String, Vec<(String, Pattern)>, Span),
    Or(Vec<Pattern>, Span),
    Range(i64, i64, Span),
}

/// Unary operators
#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,      // -
    Not,      // !
    BitNot,   // ~
    Deref,    // *
    Ref,      // &
}

/// Binary operators
#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,      // +
    Sub,      // -
    Mul,      // *
    Div,      // /
    Rem,      // %
    Eq,       // ==
    Ne,       // !=
    Lt,       // <
    Gt,       // >
    Le,       // <=
    Ge,       // >=
    And,      // &&
    Or,       // ||
    BitAnd,   // &
    BitOr,    // |
    BitXor,   // ^
    Shl,      // <<
    Shr,      // >>
    Pipeline, // |>
}
