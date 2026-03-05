//! Knull Parser - AST Builder

use crate::lexer::{Lexer, Token, TokenKind};

#[derive(Debug, Clone)]
pub enum ASTNode {
    Program(Vec<ASTNode>),
    Mode(String), // novice, expert, god
    Function {
        name: String,
        params: Vec<Param>,
        ret_type: Option<Type>,
        body: Box<ASTNode>,
    },
    Let {
        name: String,
        mutable: bool,
        ty: Option<Type>,
        value: Box<ASTNode>,
    },
    /// Re-assignment to existing variable (or field/index)
    Assign {
        target: Box<ASTNode>,
        value: Box<ASTNode>,
    },
    /// Compound assignment: target += value, etc.
    AssignOp {
        target: Box<ASTNode>,
        op: String,
        value: Box<ASTNode>,
    },
    Break(Option<String>),
    Continue(Option<String>),
    Return(Box<ASTNode>),
    If {
        cond: Box<ASTNode>,
        then_body: Box<ASTNode>,
        else_body: Option<Box<ASTNode>>,
    },
    Match {
        expr: Box<ASTNode>,
        arms: Vec<MatchArm>,
    },
    While {
        cond: Box<ASTNode>,
        body: Box<ASTNode>,
    },
    For {
        var: String,
        iter: Box<ASTNode>,
        body: Box<ASTNode>,
    },
    Loop(Box<ASTNode>),
    Binary {
        op: String,
        left: Box<ASTNode>,
        right: Box<ASTNode>,
    },
    Unary {
        op: String,
        operand: Box<ASTNode>,
    },
    Call {
        func: Box<ASTNode>,
        args: Vec<ASTNode>,
    },
    MethodCall {
        obj: Box<ASTNode>,
        method: String,
        args: Vec<ASTNode>,
    },
    Index {
        obj: Box<ASTNode>,
        index: Box<ASTNode>,
    },
    FieldAccess {
        obj: Box<ASTNode>,
        field: String,
    },
    Block(Vec<ASTNode>),
    Literal(Literal),
    Identifier(String),
    Array(Vec<ASTNode>),
    Range {
        start: Box<ASTNode>,
        end: Box<ASTNode>,
        inclusive: bool,
    },
    // Structs
    StructDef {
        name: String,
        fields: Vec<(String, Type)>,
    },
    StructLiteral {
        name: String,
        fields: Vec<(String, ASTNode)>,
    },
    Impl {
        ty: String,
        methods: Vec<ASTNode>,
    },
    // Enums
    EnumDef {
        name: String,
        variants: Vec<EnumVariant>,
    },
    // Types
    TypeAnnotation(Type),
    // Modules
    Mod(String),
    Use(String),
    // Unsafe
    Unsafe(Box<ASTNode>),
    // Inline assembly
    Asm(String),
    // Syscall
    Syscall(Vec<ASTNode>),
    // Type cast
    AsCast {
        expr: Box<ASTNode>,
        target_type: Type,
    },
    // Linear types
    Consume(Box<ASTNode>),
    LinearExpr(Box<ASTNode>, LinearKind),
    // Effects
    EffectAnnotation {
        expr: Box<ASTNode>,
        effects: Vec<Effect>,
    },
    // Capability types
    Capability {
        name: String,
        resource: Option<Box<ASTNode>>,
        permissions: Vec<Permission>,
    },
    // Compile-time execution (#run)
    CompileTimeRun(Box<ASTNode>),
    // Async function
    AsyncFunction {
        name: String,
        params: Vec<Param>,
        ret_type: Option<Type>,
        body: Box<ASTNode>,
    },
    // Await expression
    Await(Box<ASTNode>),
    // Yield expression
    Yield(Option<Box<ASTNode>>),
    // Interpolated string: f"Hello {name}" or `Hello {name}`
    // Segments: Vec<(is_expr, content)>: is_expr=false → literal text, is_expr=true → source to re-parse
    InterpolatedString(Vec<(bool, String)>),
    // Lambda / closure: |x, y| body   OR   \(x, y) => body
    Lambda {
        params: Vec<String>,
        body: Box<ASTNode>,
    },
    // Map literal: {"key": value, ...}
    Map(Vec<(ASTNode, ASTNode)>),
    // Tuple: (a, b, c)
    Tuple(Vec<ASTNode>),
    // Pipeline: left |> right
    Pipeline {
        left: Box<ASTNode>,
        right: Box<ASTNode>,
    },
    // Null coalesce: a ?? b
    NullCoalesce {
        left: Box<ASTNode>,
        right: Box<ASTNode>,
    },
    // Safe navigation: a?.field or a?.method()
    SafeNav {
        obj: Box<ASTNode>,
        field: String,
    },
    // Try operator: expr?
    TryOp(Box<ASTNode>),
    // Spread: ...expr (inside arrays/args)
    Spread(Box<ASTNode>),
    // List comprehension: [expr for x in iter if cond]
    ListComp {
        expr: Box<ASTNode>,
        var: String,
        iter: Box<ASTNode>,
        filter: Option<Box<ASTNode>>,
    },
    // Do-while loop
    DoWhile {
        body: Box<ASTNode>,
        cond: Box<ASTNode>,
    },
    // If-let binding: if let pattern = expr { ... }
    IfLet {
        pattern: String,
        value: Box<ASTNode>,
        then_body: Box<ASTNode>,
        else_body: Option<Box<ASTNode>>,
    },
    // Named arg: func(name: value) — stored as a wrapping node
    NamedArg {
        name: String,
        value: Box<ASTNode>,
    },
    // Spawn a concurrent task
    Spawn(Box<ASTNode>),
    // Type alias: type X = Y
    TypeAlias {
        name: String,
        ty: Type,
    },
    // Defer statement
    Defer(Box<ASTNode>),
    // Const declaration
    Const {
        name: String,
        ty: Option<Type>,
        value: Box<ASTNode>,
    },
    // Try/catch error handling
    TryCatch {
        try_body: Box<ASTNode>,
        catch_var: String,
        catch_body: Box<ASTNode>,
    },
    // Throw an error
    Throw(Box<ASTNode>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinearKind {
    Linear,
    Unrestricted,
    Shareable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    IO,
    Mutation,
    Panic,
    Async,
    NonDeterminism,
    Divergence,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Own,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<ASTNode>,  // if guard condition
    pub body: ASTNode,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Literal(Literal),
    Identifier(String),
    Or(Vec<Pattern>),
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
    },
    Enum {
        name: String,
        variant: String,
        data: Option<Box<Pattern>>,
    },
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub data: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
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
    Void,
    Never,
    Ref(Box<Type>),
    MutRef(Box<Type>),
    RawPtr(Box<Type>),
    MutRawPtr(Box<Type>),
    Array(Box<Type>, usize),
    Slice(Box<Type>),
    Vec(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Fn(Vec<Type>, Box<Type>),
    Custom(String),
}

impl Type {
    pub fn from_str(s: &str) -> Option<Type> {
        match s {
            "i8" => Some(Type::I8),
            "i16" => Some(Type::I16),
            "i32" => Some(Type::I32),
            "i64" => Some(Type::I64),
            "i128" => Some(Type::I128),
            "u8" => Some(Type::U8),
            "u16" => Some(Type::U16),
            "u32" => Some(Type::U32),
            "u64" => Some(Type::U64),
            "u128" => Some(Type::U128),
            "f32" => Some(Type::F32),
            "f64" => Some(Type::F64),
            "bool" => Some(Type::Bool),
            "char" => Some(Type::Char),
            "String" => Some(Type::String),
            "void" => Some(Type::Void),
            "never" => Some(Type::Never),
            _ => Some(Type::Custom(s.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        Parser { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
    }

    pub fn parse(&mut self) -> Result<ASTNode, String> {
        let mut items = Vec::new();
        while self.current().kind != TokenKind::Eof {
            self.skip_semis();
            if self.current().kind == TokenKind::Eof {
                break;
            }
            match self.current().kind {
                TokenKind::Mode => items.push(self.parse_mode()?),
                TokenKind::Async => items.push(self.parse_async_function()?),
                TokenKind::Fn => items.push(self.parse_function()?),
                TokenKind::Class => items.push(self.parse_class()?),
                TokenKind::Spawn => {
                    self.advance();
                    let body = self.parse_block()?;
                    items.push(ASTNode::Spawn(Box::new(body)));
                }
                TokenKind::Let => items.push(self.parse_let()?),
                TokenKind::If => items.push(self.parse_if()?),
                TokenKind::Match => items.push(self.parse_match()?),
                TokenKind::While => items.push(self.parse_while()?),
                TokenKind::Do => items.push(self.parse_do_while()?),
                TokenKind::For => items.push(self.parse_for()?),
                TokenKind::Loop => items.push(self.parse_loop()?),
                TokenKind::Mod => items.push(self.parse_module_decl()?),
                TokenKind::Struct => items.push(self.parse_struct_def()?),
                TokenKind::Enum => items.push(self.parse_enum_def()?),
                TokenKind::Impl => items.push(self.parse_impl()?),
                TokenKind::Trait => items.push(self.parse_trait()?),
                TokenKind::Use => items.push(self.parse_use_decl()?),
                TokenKind::Pub => {
                    self.advance();
                    continue;
                }
                TokenKind::Const => {
                    self.advance();
                    items.push(self.parse_const()?);
                }
                TokenKind::Type => {
                    self.advance();
                    let alias = self.parse_type_alias()?;
                    items.push(alias);
                }
                TokenKind::Defer => {
                    self.advance();
                    let expr = self.parse_expression()?;
                    items.push(ASTNode::Defer(Box::new(expr)));
                }
                TokenKind::Throw => {
                    self.advance();
                    let expr = self.parse_expression()?;
                    items.push(ASTNode::Throw(Box::new(expr)));
                }
                TokenKind::Unsafe => {
                    self.advance();
                    if self.current().kind == TokenKind::LBrace {
                        let block = self.parse_block()?;
                        items.push(ASTNode::Unsafe(Box::new(block)));
                    } else if self.current().kind == TokenKind::Fn {
                        items.push(self.parse_function()?);
                    }
                }
                TokenKind::Asm => items.push(self.parse_asm()?),
                TokenKind::Syscall => items.push(self.parse_syscall()?),
                TokenKind::Consume => items.push(self.parse_consume()?),
                TokenKind::Linear => items.push(self.parse_linear_expr()?),
                TokenKind::Effect => items.push(self.parse_effect_annotation()?),
                TokenKind::HashRun => items.push(self.parse_run_block()?),
                _ => {
                    let expr = self.parse_expression()?;
                    items.push(expr);
                }
            }
        }
        Ok(ASTNode::Program(items))
    }

    fn skip_semis(&mut self) {
        while self.current().kind == TokenKind::Semicolon {
            self.advance();
        }
    }

    fn parse_function(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Fn)?;
        let name = self.parse_identifier()?;

        // Parse params
        let mut params = Vec::new();
        if self.current().kind == TokenKind::LParen {
            self.advance();
            while self.current().kind != TokenKind::RParen && self.current().kind != TokenKind::Eof
            {
                // Parse parameter name (can be identifier or 'self')
                let param_name = if self.current().kind == TokenKind::SelfValue {
                    self.advance();
                    "self".to_string()
                } else {
                    self.parse_identifier()?
                };

                // Check for type annotation
                let ty = if self.current().kind == TokenKind::Colon {
                    self.advance(); // skip :
                    Some(self.parse_type()?)
                } else {
                    None
                };
                params.push(Param {
                    name: param_name,
                    ty,
                });

                // Check for comma
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            if self.current().kind == TokenKind::RParen {
                self.advance();
            }
        }

        // Parse return type
        let ret_type = if self.current().kind == TokenKind::Arrow {
            self.advance(); // skip ->
            Some(self.parse_type()?)
        } else {
            None
        };

        // Parse body
        let body = self.parse_block()?;
        Ok(ASTNode::Function {
            name,
            params,
            ret_type,
            body: Box::new(body),
        })
    }

    fn parse_let(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Let)?;

        // Check for mut
        let mutable = if self.current().kind == TokenKind::Mut {
            self.advance();
            true
        } else {
            false
        };

        let name = self.parse_identifier()?;

        // Check for type annotation
        let ty = if self.current().kind == TokenKind::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(TokenKind::Eq)?;
        let value = self.parse_expression()?;
        if self.current().kind == TokenKind::Semicolon {
            self.advance();
        }
        Ok(ASTNode::Let {
            name,
            mutable,
            ty,
            value: Box::new(value),
        })
    }

    fn parse_const(&mut self) -> Result<ASTNode, String> {
        let name = self.parse_identifier()?;

        // Check for type annotation
        let ty = if self.current().kind == TokenKind::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(TokenKind::Eq)?;
        let value = self.parse_expression()?;
        if self.current().kind == TokenKind::Semicolon {
            self.advance();
        }
        Ok(ASTNode::Const {
            name,
            ty,
            value: Box::new(value),
        })
    }

    fn parse_if_let(&mut self) -> Result<ASTNode, String> {
        // `if let pattern = expr { } else { }`
        self.expect(TokenKind::Let)?;
        // For now support simple identifier patterns
        let pattern = self.parse_identifier()?;
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expression()?;
        let then_body = self.parse_block()?;
        let else_body = if self.current().kind == TokenKind::Else {
            self.advance();
            if self.current().kind == TokenKind::If {
                self.advance();
                Some(Box::new(self.parse_if()?))
            } else {
                Some(Box::new(self.parse_block()?))
            }
        } else { None };
        Ok(ASTNode::IfLet { pattern, value: Box::new(value), then_body: Box::new(then_body), else_body })
    }

    fn parse_if(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::If)?;
        // if let binding
        if self.current().kind == TokenKind::Let {
            return self.parse_if_let();
        }
        let cond = self.parse_expression()?;
        let then_body = self.parse_block()?;
        let else_body = if self.current().kind == TokenKind::Else {
            self.advance();
            // `else if` chain
            if self.current().kind == TokenKind::If {
                Some(Box::new(self.parse_if()?))
            } else {
                Some(Box::new(self.parse_block()?))
            }
        } else {
            None
        };
        Ok(ASTNode::If {
            cond: Box::new(cond),
            then_body: Box::new(then_body),
            else_body,
        })
    }

    fn parse_do_while(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Do)?;
        let body = self.parse_block()?;
        self.expect(TokenKind::While)?;
        let cond = self.parse_expression()?;
        if self.current().kind == TokenKind::Semicolon { self.advance(); }
        Ok(ASTNode::DoWhile { body: Box::new(body), cond: Box::new(cond) })
    }

    fn parse_while(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::While)?;
        let cond = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(ASTNode::While {
            cond: Box::new(cond),
            body: Box::new(body),
        })
    }

    fn parse_for(&mut self) -> Result<ASTNode, String> {
        // Simplified - treat 'for' as a regular identifier for now
        self.advance();
        let var = self.parse_identifier()?;
        self.expect(TokenKind::In)?;
        let iter = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(ASTNode::For {
            var,
            iter: Box::new(iter),
            body: Box::new(body),
        })
    }

    fn parse_block(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while self.current().kind != TokenKind::RBrace && self.current().kind != TokenKind::Eof {
            self.skip_semis();
            if self.current().kind == TokenKind::RBrace {
                break;
            }
            stmts.push(self.parse_statement()?);
        }
        if self.current().kind == TokenKind::RBrace {
            self.advance();
        }
        Ok(ASTNode::Block(stmts))
    }

    fn parse_statement(&mut self) -> Result<ASTNode, String> {
        match self.current().kind {
            TokenKind::Let => self.parse_let(),
            TokenKind::Const => {
                self.advance();
                self.parse_const()
            }
            TokenKind::Return => {
                self.advance();
                // bare return with no expression
                if matches!(self.current().kind, TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Eof) {
                    if self.current().kind == TokenKind::Semicolon { self.advance(); }
                    return Ok(ASTNode::Return(Box::new(ASTNode::Literal(crate::parser::Literal::Null))));
                }
                let value = self.parse_expression()?;
                if self.current().kind == TokenKind::Semicolon {
                    self.advance();
                }
                Ok(ASTNode::Return(Box::new(value)))
            }
            TokenKind::Break => {
                self.advance();
                let label = if self.current().kind == TokenKind::Identifier {
                    let l = self.current().value.clone(); self.advance(); Some(l)
                } else { None };
                if self.current().kind == TokenKind::Semicolon { self.advance(); }
                Ok(ASTNode::Break(label))
            }
            TokenKind::Continue => {
                self.advance();
                let label = if self.current().kind == TokenKind::Identifier {
                    let l = self.current().value.clone(); self.advance(); Some(l)
                } else { None };
                if self.current().kind == TokenKind::Semicolon { self.advance(); }
                Ok(ASTNode::Continue(label))
            }
            TokenKind::Loop => self.parse_loop(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Unsafe => {
                self.advance();
                let block = self.parse_block()?;
                Ok(ASTNode::Unsafe(Box::new(block)))
            }
            TokenKind::Match => self.parse_match(),
            TokenKind::Spawn => {
                self.advance();
                let body = self.parse_block()?;
                Ok(ASTNode::Spawn(Box::new(body)))
            }
            TokenKind::Try => self.parse_try_catch(),
            TokenKind::Throw => {
                self.advance();
                let expr = self.parse_expression()?;
                if self.current().kind == TokenKind::Semicolon { self.advance(); }
                Ok(ASTNode::Throw(Box::new(expr)))
            }
            _ => {
                // Check for space-separated function call: "println x" or "print "hello""
                // Also handle "println "x = " + x" where the argument is a binary expression
                if self.current().kind == TokenKind::Identifier {
                    let name = self.current().value.clone();
                    let next_idx = self.pos + 1;

                    if next_idx < self.tokens.len() {
                        let next_kind = self.tokens[next_idx].kind.clone();

                        // If next token is a literal or identifier, it's potentially a function call
                        if self.is_argument_token(&next_kind) {
                            self.advance(); // consume function name
                                            // Parse the entire expression as the argument (handles binary ops)
                            let arg = self.parse_expression()?;
                            if self.current().kind == TokenKind::Semicolon {
                                self.advance();
                            }
                            return Ok(ASTNode::Call {
                                func: Box::new(ASTNode::Identifier(name)),
                                args: vec![arg],
                            });
                        }
                    }
                }

                let expr = self.parse_expression()?;
                if self.current().kind == TokenKind::Semicolon {
                    self.advance();
                }
                Ok(expr)
            }
        }
    }

    fn peek_next(&self) -> &Token {
        if self.pos + 1 < self.tokens.len() {
            &self.tokens[self.pos + 1]
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    fn is_argument_token(&self, kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Int
                | TokenKind::Float
                | TokenKind::String
                | TokenKind::FString
                | TokenKind::True
                | TokenKind::False
        )
    }

    fn is_operator_token(&self, kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::Eq
                | TokenKind::EqEq
                | TokenKind::Neq
                | TokenKind::Lt
                | TokenKind::Gt
                | TokenKind::Lte
                | TokenKind::Gte
                | TokenKind::Ampersand
                | TokenKind::Pipe
                | TokenKind::Caret
                | TokenKind::Shl
                | TokenKind::Shr
        )
    }

    /// Public wrapper so external code (e.g. C codegen) can parse a single expr
    pub fn parse_expression_pub(&mut self) -> Result<ASTNode, String> {
        self.parse_assignment()
    }

    fn parse_expression(&mut self) -> Result<ASTNode, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<ASTNode, String> {
        let left = self.parse_null_coalesce()?;
        // Compound assignment operators
        let compound_op: Option<&str> = match self.current().kind {
            TokenKind::PlusEq    => Some("+"),
            TokenKind::MinusEq   => Some("-"),
            TokenKind::StarEq    => Some("*"),
            TokenKind::SlashEq   => Some("/"),
            TokenKind::PercentEq => Some("%"),
            TokenKind::AmpEq     => Some("&"),
            TokenKind::BarEq     => Some("|"),
            TokenKind::CaretEq   => Some("^"),
            TokenKind::ShlEq     => Some("<<"),
            TokenKind::ShrEq     => Some(">>"),
            _ => None,
        };
        if let Some(op) = compound_op {
            let op = op.to_string();
            self.advance();
            let right = self.parse_assignment()?;
            if self.current().kind == TokenKind::Semicolon { self.advance(); }
            return Ok(ASTNode::AssignOp {
                target: Box::new(left),
                op,
                value: Box::new(right),
            });
        }
        if self.current().kind == TokenKind::Eq {
            self.advance();
            let right = self.parse_assignment()?;
            if self.current().kind == TokenKind::Semicolon { self.advance(); }
            return Ok(ASTNode::Assign {
                target: Box::new(left),
                value: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_null_coalesce(&mut self) -> Result<ASTNode, String> {
        let mut left = self.parse_or()?;
        while self.current().kind == TokenKind::NullCoalesce {
            self.advance();
            let right = self.parse_or()?;
            left = ASTNode::NullCoalesce {
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<ASTNode, String> {
        let mut left = self.parse_and()?;
        while self.current().kind == TokenKind::Or {
            self.advance();
            let right = self.parse_and()?;
            left = ASTNode::Binary {
                op: "||".to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<ASTNode, String> {
        let mut left = self.parse_bitwise()?;
        while self.current().kind == TokenKind::And {
            self.advance();
            let right = self.parse_bitwise()?;
            left = ASTNode::Binary {
                op: "&&".to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_bitwise(&mut self) -> Result<ASTNode, String> {
        let mut left = self.parse_equality()?;
        loop {
            // Pipeline operator: left |> right -> right(left)
            if self.current().kind == TokenKind::Pipeline {
                self.advance();
                let right = self.parse_equality()?;
                left = ASTNode::Pipeline { left: Box::new(left), right: Box::new(right) };
                continue;
            }
            let op = match self.current().kind {
                TokenKind::Ampersand => "&",
                TokenKind::Pipe      => "|",
                TokenKind::Caret     => "^",
                TokenKind::Shl       => "<<",
                TokenKind::Shr       => ">>",
                _ => break,
            }.to_string();
            self.advance();
            let right = self.parse_equality()?;
            left = ASTNode::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<ASTNode, String> {
        let mut left = self.parse_comparison()?;
        while let TokenKind::EqEq | TokenKind::Neq = self.current().kind {
            let op = self.current().value.clone();
            self.advance();
            let right = self.parse_comparison()?;
            left = ASTNode::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<ASTNode, String> {
        let mut left = self.parse_additive()?;
        while let TokenKind::Lt | TokenKind::Gt | TokenKind::Lte | TokenKind::Gte =
            self.current().kind
        {
            let op = self.current().value.clone();
            self.advance();
            let right = self.parse_additive()?;
            left = ASTNode::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<ASTNode, String> {
        let mut left = self.parse_multiplicative()?;
        while let TokenKind::Plus | TokenKind::Minus = self.current().kind {
            let op = self.current().value.clone();
            self.advance();
            let right = self.parse_multiplicative()?;
            left = ASTNode::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<ASTNode, String> {
        let mut left = self.parse_unary()?;
        while let TokenKind::Star | TokenKind::Slash | TokenKind::Percent = self.current().kind {
            let op = self.current().value.clone();
            self.advance();
            let right = self.parse_unary()?;
            left = ASTNode::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<ASTNode, String> {
        if let TokenKind::Bang | TokenKind::Minus = self.current().kind {
            let op = self.current().value.clone();
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(ASTNode::Unary {
                op,
                operand: Box::new(operand),
            });
        }
        if self.current().kind == TokenKind::Yield {
            self.advance();
            if self.current().kind == TokenKind::Semicolon
                || self.current().kind == TokenKind::RBrace
                || self.current().kind == TokenKind::Comma
                || self.current().kind == TokenKind::RParen
                || self.current().kind == TokenKind::LBracket
            {
                return Ok(ASTNode::Yield(None));
            }
            let expr = self.parse_expression()?;
            return Ok(ASTNode::Yield(Some(Box::new(expr))));
        }
        if self.current().kind == TokenKind::Await {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(ASTNode::Await(Box::new(expr)));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<ASTNode, String> {
        let mut node = self.parse_primary()?;

        loop {
            if self.current().kind == TokenKind::LParen {
                self.advance();
                let mut args = Vec::new();
                while self.current().kind != TokenKind::RParen
                    && self.current().kind != TokenKind::Eof
                {
                    args.push(self.parse_expression()?);
                    if self.current().kind == TokenKind::Comma {
                        self.advance();
                    }
                }
                if self.current().kind == TokenKind::RParen {
                    self.advance();
                }
                // Function call
                if let ASTNode::Identifier(name) = node {
                    node = ASTNode::Call {
                        func: Box::new(ASTNode::Identifier(name)),
                        args,
                    };
                } else {
                    // Method call: obj.method(args)
                    if let ASTNode::FieldAccess { obj, field } = node {
                        node = ASTNode::MethodCall {
                            obj,
                            method: field,
                            args,
                        };
                    } else {
                        node = ASTNode::Call {
                            func: Box::new(node),
                            args,
                        };
                    }
                }
            } else if self.current().kind == TokenKind::LBracket {
                self.advance();
                let index = self.parse_expression()?;
                self.expect(TokenKind::RBracket)?;
                node = ASTNode::Index {
                    obj: Box::new(node),
                    index: Box::new(index),
                };
            } else if self.current().kind == TokenKind::SafeNav {
                // ?. safe navigation
                self.advance();
                let field = self.parse_identifier()?;
                // Check if it's a method call
                if self.current().kind == TokenKind::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    while self.current().kind != TokenKind::RParen && self.current().kind != TokenKind::Eof {
                        args.push(self.parse_expression()?);
                        if self.current().kind == TokenKind::Comma { self.advance(); }
                    }
                    if self.current().kind == TokenKind::RParen { self.advance(); }
                    node = ASTNode::MethodCall {
                        obj: Box::new(ASTNode::SafeNav { obj: Box::new(node), field }),
                        method: "__safe_call__".to_string(),
                        args,
                    };
                } else {
                    node = ASTNode::SafeNav { obj: Box::new(node), field };
                }
            } else if self.current().kind == TokenKind::Dot {
                self.advance();
                let field = self.parse_identifier()?;
                node = ASTNode::FieldAccess {
                    obj: Box::new(node),
                    field,
                };
            } else if self.current().kind == TokenKind::LBrace {
                // Check for struct literal: must be a single identifier followed by { field: value, ... }
                if let ASTNode::Identifier(name) = &node {
                    // Try to parse as struct literal
                    let checkpoint = self.pos;
                    self.advance();
                    let mut fields = Vec::new();
                    let mut is_struct_literal = true;

                    while self.current().kind != TokenKind::RBrace
                        && self.current().kind != TokenKind::Eof
                    {
                        if let Ok(field_name) = self.parse_identifier() {
                            if self.current().kind == TokenKind::Colon {
                                self.advance();
                                if let Ok(field_value) = self.parse_expression() {
                                    fields.push((field_name, field_value));
                                    if self.current().kind == TokenKind::Comma {
                                        self.advance();
                                    }
                                    continue;
                                }
                            }
                        }
                        // Failed to parse as struct literal
                        is_struct_literal = false;
                        break;
                    }

                    if is_struct_literal && self.current().kind == TokenKind::RBrace {
                        self.advance();
                        node = ASTNode::StructLiteral {
                            name: name.clone(),
                            fields,
                        };
                    } else {
                        // Not a struct literal, restore position
                        self.pos = checkpoint;
                        break;
                    }
                } else {
                    break;
                }
            } else if self.current().kind == TokenKind::As {
                self.advance();
                let target_type = self.parse_type()?;
                node = ASTNode::AsCast {
                    expr: Box::new(node),
                    target_type,
                };
            } else if self.current().kind == TokenKind::Question {
                // Try operator: expr?
                self.advance();
                node = ASTNode::TryOp(Box::new(node));
            } else {
                break;
            }
        }
        Ok(node)
    }

    fn parse_primary(&mut self) -> Result<ASTNode, String> {
        let token = self.current().clone();

        match &token.kind {
            // ── Interpolated strings ─────────────────────────────────────────
            TokenKind::FString => {
                self.advance();
                let raw = token.value.clone();
                let segments = Self::parse_fstring_segments(&raw);
                Ok(ASTNode::InterpolatedString(segments))
            }
            // ── Numeric / bool / null literals ───────────────────────────────
            TokenKind::Int => {
                self.advance();
                let n: i64 = if token.value.starts_with("0x") || token.value.starts_with("0X") {
                    i64::from_str_radix(&token.value[2..], 16).unwrap_or(0)
                } else if token.value.starts_with("0o") || token.value.starts_with("0O") {
                    i64::from_str_radix(&token.value[2..], 8).unwrap_or(0)
                } else if token.value.starts_with("0b") || token.value.starts_with("0B") {
                    i64::from_str_radix(&token.value[2..], 2).unwrap_or(0)
                } else {
                    token.value.parse().unwrap_or(0)
                };
                Ok(ASTNode::Literal(Literal::Int(n)))
            }
            TokenKind::Float => {
                self.advance();
                let f: f64 = token.value.parse().unwrap_or(0.0);
                Ok(ASTNode::Literal(Literal::Float(f)))
            }
            TokenKind::String => {
                self.advance();
                Ok(ASTNode::Literal(Literal::String(token.value)))
            }
            TokenKind::True => {
                self.advance();
                Ok(ASTNode::Literal(Literal::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(ASTNode::Literal(Literal::Bool(false)))
            }
            TokenKind::Null => {
                self.advance();
                Ok(ASTNode::Literal(Literal::Null))
            }
            // ── Array literal / list comprehension ─────────────────────────
            TokenKind::LBracket => {
                self.advance();
                if self.current().kind == TokenKind::RBracket {
                    self.advance();
                    return Ok(ASTNode::Array(vec![]));
                }
                // Spread: [...arr]
                if self.current().kind == TokenKind::DotDot {
                    self.advance();
                    let spreaded = self.parse_expression()?;
                    self.expect(TokenKind::RBracket)?;
                    return Ok(ASTNode::Spread(Box::new(spreaded)));
                }
                let first = self.parse_expression()?;
                // Check for list comprehension: [expr for var in iter]
                if self.current().kind == TokenKind::For {
                    self.advance();
                    let var = self.parse_identifier()?;
                    self.expect(TokenKind::In)?;
                    let iter = self.parse_expression()?;
                    let filter = if self.current().kind == TokenKind::If {
                        self.advance();
                        Some(Box::new(self.parse_expression()?))
                    } else { None };
                    self.expect(TokenKind::RBracket)?;
                    return Ok(ASTNode::ListComp {
                        expr: Box::new(first),
                        var,
                        iter: Box::new(iter),
                        filter,
                    });
                }
                let mut items = vec![first];
                while self.current().kind == TokenKind::Comma && self.current().kind != TokenKind::Eof {
                    self.advance();
                    if self.current().kind == TokenKind::RBracket { break; }
                    if self.current().kind == TokenKind::DotDot {
                        self.advance();
                        let spreaded = self.parse_expression()?;
                        items.push(ASTNode::Spread(Box::new(spreaded)));
                    } else {
                        items.push(self.parse_expression()?);
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(ASTNode::Array(items))
            }
            // ── Map literal  { key: val, ... } ───────────────────────────────
            TokenKind::LBrace => {
                // Peek ahead: if first non-whitespace content looks like `key:`, it's a map
                let checkpoint = self.pos;
                self.advance(); // consume `{`
                // Try to parse as map: key can be string literal or identifier, then `:`
                let is_map = match self.current().kind {
                    TokenKind::String | TokenKind::Int | TokenKind::Float => {
                        // peek if next is `:`
                        self.pos + 1 < self.tokens.len()
                            && self.tokens[self.pos + 1].kind == TokenKind::Colon
                    }
                    TokenKind::Identifier => {
                        self.pos + 1 < self.tokens.len()
                            && self.tokens[self.pos + 1].kind == TokenKind::Colon
                    }
                    TokenKind::RBrace => false, // empty `{}` → empty map
                    _ => false,
                };
                let is_empty = self.current().kind == TokenKind::RBrace;
                if is_map || is_empty {
                    let mut pairs = Vec::new();
                    while self.current().kind != TokenKind::RBrace
                        && self.current().kind != TokenKind::Eof
                    {
                        let key = match self.current().kind {
                            TokenKind::String => {
                                let v = self.current().value.clone();
                                self.advance();
                                ASTNode::Literal(Literal::String(v))
                            }
                            TokenKind::Identifier => {
                                let v = self.current().value.clone();
                                self.advance();
                                ASTNode::Literal(Literal::String(v))
                            }
                            _ => self.parse_expression()?,
                        };
                        self.expect(TokenKind::Colon)?;
                        let val = self.parse_expression()?;
                        pairs.push((key, val));
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RBrace)?;
                    Ok(ASTNode::Map(pairs))
                } else {
                    // Not a map — restore and return error so caller can try block
                    self.pos = checkpoint;
                    // Can't have a block as a primary expression here; return empty block
                    let block = self.parse_block()?;
                    Ok(block)
                }
            }
            // ── Tuple or parenthesised expression ────────────────────────────
            TokenKind::LParen => {
                self.advance();
                // empty tuple ()
                if self.current().kind == TokenKind::RParen {
                    self.advance();
                    return Ok(ASTNode::Tuple(vec![]));
                }
                let first = self.parse_expression()?;
                if self.current().kind == TokenKind::Comma {
                    // It's a tuple
                    let mut elems = vec![first];
                    while self.current().kind == TokenKind::Comma {
                        self.advance();
                        if self.current().kind == TokenKind::RParen { break; }
                        elems.push(self.parse_expression()?);
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(ASTNode::Tuple(elems))
                } else {
                    self.expect(TokenKind::RParen)?;
                    Ok(first)
                }
            }
            // ── Lambda  |x, y| body  or  || body ────────────────────────────
            TokenKind::Or => {
                // `||` as zero-param lambda: || body
                self.advance();
                if self.current().kind == TokenKind::Arrow {
                    self.advance();
                    let _ = self.parse_type();
                }
                let body = if self.current().kind == TokenKind::LBrace {
                    self.parse_block()?
                } else {
                    self.parse_expression()?
                };
                Ok(ASTNode::Lambda { params: vec![], body: Box::new(body) })
            }
            TokenKind::Pipe => {
                self.advance(); // consume first `|`
                let mut params = Vec::new();
                while self.current().kind != TokenKind::Pipe
                    && self.current().kind != TokenKind::Or
                    && self.current().kind != TokenKind::Eof
                {
                    if self.current().kind == TokenKind::Identifier {
                        params.push(self.current().value.clone());
                        self.advance();
                    } else {
                        self.advance(); // skip type annotations etc.
                    }
                    if self.current().kind == TokenKind::Comma { self.advance(); }
                }
                if self.current().kind == TokenKind::Pipe { self.advance(); } // consume closing `|`
                // Optional `->` return type annotation
                if self.current().kind == TokenKind::Arrow {
                    self.advance();
                    let _ = self.parse_type(); // discard
                }
                let body = if self.current().kind == TokenKind::LBrace {
                    self.parse_block()?
                } else {
                    self.parse_expression()?
                };
                Ok(ASTNode::Lambda { params, body: Box::new(body) })
            }
            TokenKind::SelfValue => {
                self.advance();
                Ok(ASTNode::Identifier("self".to_string()))
            }
            // ── try { } catch e { } as expression ───────────────────────────
            TokenKind::Try => {
                self.parse_try_catch()
            }
            // ── Anonymous function: fn(x, y) { body } ───────────────────────
            TokenKind::Fn => {
                self.advance(); // consume `fn`
                // Skip optional name (e.g. `fn foo(x) { ... }` used inline)
                if self.current().kind == TokenKind::Identifier {
                    self.advance();
                }
                let mut params = Vec::new();
                if self.current().kind == TokenKind::LParen {
                    self.advance();
                    while self.current().kind != TokenKind::RParen
                        && self.current().kind != TokenKind::Eof
                    {
                        if self.current().kind == TokenKind::SelfValue {
                            params.push("self".to_string());
                            self.advance();
                        } else {
                            let pname = self.parse_identifier()?;
                            params.push(pname);
                            // skip optional type annotation
                            if self.current().kind == TokenKind::Colon {
                                self.advance();
                                let _ = self.parse_type();
                            }
                        }
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    if self.current().kind == TokenKind::RParen {
                        self.advance();
                    }
                }
                // skip optional return type
                if self.current().kind == TokenKind::Arrow {
                    self.advance();
                    let _ = self.parse_type();
                }
                let body = self.parse_block()?;
                Ok(ASTNode::Lambda { params, body: Box::new(body) })
            }
            // ── throw expr as expression ─────────────────────────────────────
            TokenKind::Throw => {
                self.advance();
                let expr = self.parse_expression()?;
                Ok(ASTNode::Throw(Box::new(expr)))
            }
            TokenKind::Identifier => {
                self.advance();
                Ok(ASTNode::Identifier(token.value))
            }
            _ => {
                self.advance();
                Ok(ASTNode::Identifier(token.value.clone()))
            }
        }
    }

    /// Parse f-string raw content into segments: (is_expr, text)
    fn parse_fstring_segments(raw: &str) -> Vec<(bool, String)> {
        let mut segments = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = raw.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] != '{' {
                // Start of expression
                if !current.is_empty() {
                    segments.push((false, current.clone()));
                    current.clear();
                }
                i += 1; // skip `{`
                let mut depth = 1;
                let mut expr = String::new();
                while i < chars.len() {
                    match chars[i] {
                        '{' => { depth += 1; expr.push('{'); }
                        '}' => {
                            depth -= 1;
                            if depth == 0 { i += 1; break; }
                            expr.push('}');
                        }
                        c => expr.push(c),
                    }
                    i += 1;
                }
                segments.push((true, expr));
            } else if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '{' {
                // Escaped `{{` → literal `{`
                current.push('{');
                i += 2;
            } else if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
                // Escaped `}}` → literal `}`
                current.push('}');
                i += 2;
            } else {
                current.push(chars[i]);
                i += 1;
            }
        }
        if !current.is_empty() {
            segments.push((false, current));
        }
        segments
    }

    fn parse_identifier(&mut self) -> Result<String, String> {
        if let TokenKind::Identifier = self.current().kind {
            let name = self.current().value.clone();
            self.advance();
            Ok(name)
        } else {
            Err(format!(
                "Expected identifier, got {:?}",
                self.current().kind
            ))
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), String> {
        if self.current().kind == kind {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, got {:?}",
                kind,
                self.current().kind
            ))
        }
    }

    // Parse use declarations
    fn parse_use_decl(&mut self) -> Result<ASTNode, String> {
        self.advance(); // skip 'use'/'import'
        // If the path is a string literal, just take that one token
        if self.current().kind == TokenKind::String {
            let path = self.current().value.clone();
            self.advance();
            if self.current().kind == TokenKind::Semicolon { self.advance(); }
            return Ok(ASTNode::Use(path));
        }
        // Otherwise accumulate dotted/module path tokens until ; or EOF or {
        let mut path = String::new();
        while self.current().kind != TokenKind::Semicolon
            && self.current().kind != TokenKind::Eof
            && self.current().kind != TokenKind::LBrace
            && self.current().kind != TokenKind::String  // stop at any string
        {
            path.push_str(&self.current().value);
            self.advance();
            if self.current().kind == TokenKind::DoubleColon {
                path.push_str("::");
                self.advance();
            }
        }
        if self.current().kind == TokenKind::Semicolon {
            self.advance();
        }
        Ok(ASTNode::Use(path))
    }

    fn parse_try_catch(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Try)?;
        let try_body = self.parse_block()?;
        // optional 'catch var { body }'
        let (catch_var, catch_body) = if self.current().kind == TokenKind::Catch {
            self.advance();
            // catch var or catch (var)
            let var = if self.current().kind == TokenKind::LParen {
                self.advance();
                let v = self.parse_identifier()?;
                self.expect(TokenKind::RParen)?;
                v
            } else if self.current().kind == TokenKind::Identifier {
                self.parse_identifier()?
            } else {
                "_err".to_string()
            };
            let body = self.parse_block()?;
            (var, body)
        } else {
            // no catch — just a try block, swallow errors silently (return null on error)
            ("_err".to_string(), ASTNode::Block(vec![ASTNode::Literal(Literal::Null)]))
        };
        Ok(ASTNode::TryCatch {
            try_body: Box::new(try_body),
            catch_var,
            catch_body: Box::new(catch_body),
        })
    }

    // Parse mode declaration
    fn parse_mode(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Mode)?;
        let mode_name = self.parse_identifier()?;
        Ok(ASTNode::Mode(mode_name))
    }

    // Parse match expression
    fn parse_match(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Match)?;
        let expr = self.parse_expression()?;
        self.expect(TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while self.current().kind != TokenKind::RBrace && self.current().kind != TokenKind::Eof {
            let mut pattern = self.parse_pattern()?;
            // Handle OR-patterns: 1 | 2 | 3 => ...
            if self.current().kind == TokenKind::Pipe {
                let mut pats = vec![pattern];
                while self.current().kind == TokenKind::Pipe {
                    self.advance();
                    pats.push(self.parse_pattern()?);
                }
                pattern = Pattern::Or(pats);
            }
            // Optional guard: if cond
            let guard = if self.current().kind == TokenKind::If {
                self.advance();
                Some(self.parse_expression()?)
            } else { None };
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_expression()?;
            arms.push(MatchArm { pattern, guard, body });

            if self.current().kind == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(ASTNode::Match {
            expr: Box::new(expr),
            arms,
        })
    }

    // Parse pattern for match arms
    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        match self.current().kind {
            TokenKind::Underscore => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            TokenKind::Int => {
                let v = self.current().value.clone();
                let val = if v.starts_with("0x") || v.starts_with("0X") {
                    i64::from_str_radix(&v[2..], 16).unwrap_or(0)
                } else if v.starts_with("0o") || v.starts_with("0O") {
                    i64::from_str_radix(&v[2..], 8).unwrap_or(0)
                } else if v.starts_with("0b") || v.starts_with("0B") {
                    i64::from_str_radix(&v[2..], 2).unwrap_or(0)
                } else { v.parse::<i64>().unwrap_or(0) };
                self.advance();
                Ok(Pattern::Literal(Literal::Int(val)))
            }
            TokenKind::Float => {
                let val = self.current().value.parse::<f64>().unwrap_or(0.0);
                self.advance();
                Ok(Pattern::Literal(Literal::Float(val)))
            }
            TokenKind::String => {
                let val = self.current().value.clone();
                self.advance();
                Ok(Pattern::Literal(Literal::String(val)))
            }
            TokenKind::True => {
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(false)))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Pattern::Literal(Literal::Null))
            }
            TokenKind::Minus => {
                // Negative number pattern: -42 or -3.14
                self.advance();
                if self.current().kind == TokenKind::Int {
                    let val = -(self.current().value.parse::<i64>().unwrap_or(0));
                    self.advance();
                    Ok(Pattern::Literal(Literal::Int(val)))
                } else if self.current().kind == TokenKind::Float {
                    let val = -(self.current().value.parse::<f64>().unwrap_or(0.0));
                    self.advance();
                    Ok(Pattern::Literal(Literal::Float(val)))
                } else {
                    Err("Expected number after `-` in pattern".to_string())
                }
            }
            TokenKind::Identifier => {
                let name = self.parse_identifier()?;
                Ok(Pattern::Identifier(name))
            }
            _ => {
                // Fallback: treat as wildcard to not break match
                self.advance();
                Ok(Pattern::Wildcard)
            }
        }
    }

    // Parse loop expression
    fn parse_loop(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Loop)?;
        let body = self.parse_block()?;
        Ok(ASTNode::Loop(Box::new(body)))
    }

    // Parse struct definition
    fn parse_struct_def(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Struct)?;
        let name = self.parse_identifier()?;

        let mut fields = Vec::new();
        if self.current().kind == TokenKind::LBrace {
            self.advance();
            while self.current().kind != TokenKind::RBrace && self.current().kind != TokenKind::Eof
            {
                let field_name = self.parse_identifier()?;
                // Type annotation is optional: `x: Int` or just `x`
                let ty = if self.current().kind == TokenKind::Colon {
                    self.advance();
                    self.parse_type()?
                } else {
                    Type::Custom("any".to_string())
                };
                fields.push((field_name, ty));

                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBrace)?;
        }
        Ok(ASTNode::StructDef { name, fields })
    }

    // Parse enum definition
    fn parse_enum_def(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Enum)?;
        let name = self.parse_identifier()?;

        let mut variants = Vec::new();
        if self.current().kind == TokenKind::LBrace {
            self.advance();
            while self.current().kind != TokenKind::RBrace && self.current().kind != TokenKind::Eof
            {
                let variant_name = self.parse_identifier()?;
                let data = if self.current().kind == TokenKind::LParen {
                    self.advance();
                    let ty = self.parse_type()?;
                    self.expect(TokenKind::RParen)?;
                    Some(ty)
                } else {
                    None
                };
                variants.push(EnumVariant {
                    name: variant_name,
                    data,
                });

                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBrace)?;
        }
        Ok(ASTNode::EnumDef { name, variants })
    }

    // Parse impl block
    fn parse_impl(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Impl)?;
        let ty = self.parse_identifier()?;

        let mut methods = Vec::new();
        if self.current().kind == TokenKind::LBrace {
            self.advance();
            while self.current().kind != TokenKind::RBrace && self.current().kind != TokenKind::Eof
            {
                if self.current().kind == TokenKind::Fn {
                    methods.push(self.parse_function()?);
                } else {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBrace)?;
        }
        Ok(ASTNode::Impl { ty, methods })
    }

    // Parse trait definition
    fn parse_trait(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Trait)?;
        let name = self.parse_identifier()?;
        // Skip trait body for now
        if self.current().kind == TokenKind::LBrace {
            self.advance();
            while self.current().kind != TokenKind::RBrace && self.current().kind != TokenKind::Eof
            {
                self.advance();
            }
            self.expect(TokenKind::RBrace)?;
        }
        Ok(ASTNode::Identifier(name))
    }

    // Parse module declaration
    fn parse_module_decl(&mut self) -> Result<ASTNode, String> {
        self.advance(); // skip 'mod'
        let name = self.parse_identifier()?;
        Ok(ASTNode::Mod(name))
    }

    // Parse type alias
    fn parse_type_alias(&mut self) -> Result<ASTNode, String> {
        // NEW: returns ASTNode::TypeAlias
        let name = self.parse_identifier()?;
        self.expect(TokenKind::Eq)?;
        let ty = self.parse_type()?;
        if self.current().kind == TokenKind::Semicolon { self.advance(); }
        Ok(ASTNode::TypeAlias { name, ty })
    }

    // Parse type
    fn parse_type(&mut self) -> Result<Type, String> {
        match self.current().kind {
            TokenKind::Identifier => {
                let name = self.current().value.clone();
                self.advance();
                Type::from_str(&name).ok_or_else(|| format!("Unknown type: {}", name))
            }
            TokenKind::Star => {
                self.advance();
                let ty = self.parse_type()?;
                Ok(Type::RawPtr(Box::new(ty)))
            }
            TokenKind::Ampersand => {
                self.advance();
                let ty = self.parse_type()?;
                Ok(Type::Ref(Box::new(ty)))
            }
            TokenKind::LBracket => {
                self.advance();
                let inner_ty = self.parse_type()?;
                if self.current().kind == TokenKind::Semicolon {
                    self.advance();
                    if let TokenKind::Int = self.current().kind {
                        let size: usize = self.current().value.parse().unwrap_or(1);
                        self.advance();
                        self.expect(TokenKind::RBracket)?;
                        return Ok(Type::Array(Box::new(inner_ty), size));
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Type::Slice(Box::new(inner_ty)))
            }
            _ => Ok(Type::Custom(self.current().value.clone())),
        }
    }

    // Parse inline assembly
    fn parse_asm(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Asm)?;
        self.expect(TokenKind::LParen)?;
        let code = self.current().value.clone();
        self.advance(); // string literal
        self.expect(TokenKind::RParen)?;
        Ok(ASTNode::Asm(code))
    }

    // Parse syscall
    fn parse_syscall(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Syscall)?;
        self.expect(TokenKind::LParen)?;
        let mut args = Vec::new();
        while self.current().kind != TokenKind::RParen && self.current().kind != TokenKind::Eof {
            args.push(self.parse_expression()?);
            if self.current().kind == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(ASTNode::Syscall(args))
    }

    // Parse consume expression (forces linear use)
    fn parse_consume(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Consume)?;
        let expr = self.parse_expression()?;
        Ok(ASTNode::Consume(Box::new(expr)))
    }

    // Parse linear expression
    fn parse_linear_expr(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Linear)?;
        let inner = self.parse_expression()?;
        Ok(ASTNode::LinearExpr(Box::new(inner), LinearKind::Linear))
    }

    // Parse effect annotation
    fn parse_effect_annotation(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Effect)?;
        self.expect(TokenKind::LBrace)?;

        let mut effects = Vec::new();
        while self.current().kind != TokenKind::RBrace && self.current().kind != TokenKind::Eof {
            let effect_name = self.parse_identifier()?;
            let effect = match effect_name.to_lowercase().as_str() {
                "io" => Effect::IO,
                "mutation" | "mut" => Effect::Mutation,
                "panic" => Effect::Panic,
                "async" => Effect::Async,
                "nondet" | "nondeterminism" => Effect::NonDeterminism,
                "divergence" => Effect::Divergence,
                _ => Effect::Custom(effect_name),
            };
            effects.push(effect);

            if self.current().kind == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect(TokenKind::RBrace)?;

        let expr = self.parse_expression()?;
        Ok(ASTNode::EffectAnnotation {
            expr: Box::new(expr),
            effects,
        })
    }

    // Parse #run compile-time execution block
    fn parse_run_block(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::HashRun)?;
        let block = self.parse_block()?;
        Ok(ASTNode::CompileTimeRun(Box::new(block)))
    }

    // Parse class definition: `class Foo { fields; methods }`
    // Desugars to: struct Foo { fields } + impl Foo { methods }
    fn parse_class(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Class)?;
        let name = self.parse_identifier()?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        if self.current().kind == TokenKind::LBrace {
            self.advance();
            while self.current().kind != TokenKind::RBrace && self.current().kind != TokenKind::Eof {
                self.skip_semis();
                if self.current().kind == TokenKind::RBrace { break; }
                if self.current().kind == TokenKind::Fn {
                    methods.push(self.parse_function()?);
                } else if self.current().kind == TokenKind::Identifier {
                    // Field: `name: Type`
                    if self.pos + 1 < self.tokens.len()
                        && self.tokens[self.pos + 1].kind == TokenKind::Colon
                    {
                        let field_name = self.parse_identifier()?;
                        self.expect(TokenKind::Colon)?;
                        let ty = self.parse_type()?;
                        fields.push((field_name, ty));
                        if self.current().kind == TokenKind::Comma
                            || self.current().kind == TokenKind::Semicolon
                        {
                            self.advance();
                        }
                    } else {
                        self.advance();
                    }
                } else {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBrace)?;
        }

        // Emit: StructDef + Impl as a Block
        let struct_node = ASTNode::StructDef { name: name.clone(), fields };
        let impl_node = ASTNode::Impl { ty: name, methods };
        Ok(ASTNode::Block(vec![struct_node, impl_node]))
    }

    // Parse async function
    fn parse_async_function(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Async)?;
        self.expect(TokenKind::Fn)?;
        let name = self.parse_identifier()?;

        let mut params = Vec::new();
        if self.current().kind == TokenKind::LParen {
            self.advance();
            while self.current().kind != TokenKind::RParen && self.current().kind != TokenKind::Eof
            {
                let param_name = self.parse_identifier()?;
                let ty = if self.current().kind == TokenKind::Colon {
                    self.advance();
                    Some(self.parse_type()?)
                } else {
                    None
                };
                params.push(Param {
                    name: param_name,
                    ty,
                });
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            if self.current().kind == TokenKind::RParen {
                self.advance();
            }
        }

        let ret_type = if self.current().kind == TokenKind::Arrow {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;
        Ok(ASTNode::AsyncFunction {
            name,
            params,
            ret_type,
            body: Box::new(body),
        })
    }
}

pub fn parse(source: &str) -> Result<ASTNode, String> {
    let mut parser = Parser::new(source);
    parser.parse()
}

