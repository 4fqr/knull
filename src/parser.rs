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
        ty: Option<Type>,
        value: Box<ASTNode>,
    },
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
    pub body: ASTNode,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Literal(Literal),
    Identifier(String),
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
                TokenKind::Fn => items.push(self.parse_function()?),
                TokenKind::Let => items.push(self.parse_let()?),
                TokenKind::If => items.push(self.parse_if()?),
                TokenKind::Match => items.push(self.parse_match()?),
                TokenKind::While => items.push(self.parse_while()?),
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
                    self.parse_type_alias()?;
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
                TokenKind::Async => items.push(self.parse_async_function()?),
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
                // Parse parameter name
                let param_name = self.parse_identifier()?;

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
        let _is_mut = if self.current().kind == TokenKind::Mut {
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
        Ok(ASTNode::Let {
            name,
            ty,
            value: Box::new(value),
        })
    }

    fn parse_if(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::If)?;
        let cond = self.parse_expression()?;
        let then_body = self.parse_block()?;
        let else_body = if self.current().kind == TokenKind::Else {
            self.advance();
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };
        Ok(ASTNode::If {
            cond: Box::new(cond),
            then_body: Box::new(then_body),
            else_body,
        })
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
                let value = self.parse_expression()?;
                if self.current().kind == TokenKind::Semicolon {
                    self.advance();
                }
                Ok(ASTNode::Return(Box::new(value)))
            }
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
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
                | TokenKind::Identifier
                | TokenKind::LBracket
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
        )
    }

    fn parse_expression(&mut self) -> Result<ASTNode, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<ASTNode, String> {
        let left = self.parse_or()?;
        if self.current().kind == TokenKind::Eq {
            self.advance();
            let right = self.parse_assignment()?;
            // This is assignment to a variable - create a Let
            if let ASTNode::Identifier(name) = left {
                return Ok(ASTNode::Let {
                    name,
                    ty: None,
                    value: Box::new(right),
                });
            }
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<ASTNode, String> {
        let mut left = self.parse_and()?;
        while self.current().kind == TokenKind::Pipe {
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
        let mut left = self.parse_equality()?;
        while self.current().kind == TokenKind::Ampersand {
            self.advance();
            let right = self.parse_equality()?;
            left = ASTNode::Binary {
                op: "&&".to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
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
            } else if self.current().kind == TokenKind::Dot {
                self.advance();
                let field = self.parse_identifier()?;
                node = ASTNode::FieldAccess {
                    obj: Box::new(node),
                    field,
                };
            } else if self.current().kind == TokenKind::As {
                self.advance();
                let target_type = self.parse_type()?;
                node = ASTNode::AsCast {
                    expr: Box::new(node),
                    target_type,
                };
            } else {
                break;
            }
        }
        Ok(node)
    }

    fn parse_primary(&mut self) -> Result<ASTNode, String> {
        let token = self.current().clone();

        match &token.kind {
            TokenKind::Int => {
                self.advance();
                let n: i64 = if token.value.starts_with("0x") || token.value.starts_with("0X") {
                    i64::from_str_radix(&token.value[2..], 16).unwrap_or(0)
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
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while self.current().kind != TokenKind::RBracket
                    && self.current().kind != TokenKind::Eof
                {
                    items.push(self.parse_expression()?);
                    if self.current().kind == TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(ASTNode::Array(items))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
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
        self.advance(); // skip 'use'
        let mut path = String::new();
        while self.current().kind != TokenKind::Semicolon
            && self.current().kind != TokenKind::Eof
            && self.current().kind != TokenKind::LBrace
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
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_expression()?;
            arms.push(MatchArm { pattern, body });

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
                let val = self.current().value.parse::<i64>().unwrap_or(0);
                self.advance();
                Ok(Pattern::Literal(Literal::Int(val)))
            }
            TokenKind::True => {
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(false)))
            }
            TokenKind::Identifier => {
                let name = self.parse_identifier()?;
                Ok(Pattern::Identifier(name))
            }
            _ => Err(format!(
                "Unexpected token in pattern: {:?}",
                self.current().kind
            )),
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
                self.expect(TokenKind::Colon)?;
                let ty = self.parse_type()?;
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
    fn parse_type_alias(&mut self) -> Result<(), String> {
        self.parse_identifier()?; // type name
        self.expect(TokenKind::Eq)?;
        self.parse_type()?;
        Ok(())
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
