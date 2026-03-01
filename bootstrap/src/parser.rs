//! Knull Parser - Abstract Syntax Tree
//!
//! This module handles parsing tokens into an Abstract Syntax Tree (AST).

use anyhow::{bail, Result};
use std::iter::Peekable;

use crate::ast::*;
use crate::ast::{BinaryOp, UnaryOp};
use crate::lexer::{Position, Span, Token, TokenKind};

/// Parser for Knull
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// Create a new parser from tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    /// Parse the tokens into an AST
    pub fn parse(&mut self) -> Result<Program> {
        let mut items = Vec::new();

        while !self.is_eof() {
            // Skip whitespace and comments
            self.skip_trivia();

            if self.is_eof() {
                break;
            }

            // Parse item
            match self.peek().map(|t| &t.kind) {
                Some(t) if *t == TokenKind::KwFn => {
                    items.push(self.parse_function()?);
                }
                Some(t) if *t == TokenKind::KwStruct => {
                    items.push(self.parse_struct()?);
                }
                Some(t) if *t == TokenKind::KwEnum => {
                    items.push(self.parse_enum()?);
                }
                Some(t) if *t == TokenKind::KwTrait => {
                    items.push(self.parse_trait()?);
                }
                Some(t) if *t == TokenKind::KwImpl => {
                    items.push(self.parse_impl()?);
                }
                Some(t) if *t == TokenKind::KwType => {
                    items.push(self.parse_type_alias()?);
                }
                Some(t) if *t == TokenKind::KwConst => {
                    items.push(self.parse_const()?);
                }
                Some(t) if *t == TokenKind::KwStatic => {
                    items.push(self.parse_static()?);
                }
                Some(t) if *t == TokenKind::KwModule => {
                    items.push(self.parse_module()?);
                }
                Some(t) if *t == TokenKind::KwImport => {
                    items.push(self.parse_import()?);
                }
                Some(t) if *t == TokenKind::KwActor => {
                    items.push(self.parse_actor()?);
                }
                _ => {
                    // Maybe it's a statement at top level
                    items.push(Item::Stmt(self.parse_statement()?));
                }
            }

            self.skip_trivia();
        }

        Ok(Program { items })
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn peek_n(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.position + n)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();
        self.position += 1;
        token
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token> {
        match self.peek() {
            Some(token) if token.kind == kind => self
                .advance()
                .ok_or_else(|| anyhow::anyhow!("Unexpected end of input")),
            Some(token) => bail!(
                "Expected {:?}, got {:?} at {:?}",
                kind,
                token.kind,
                token.span
            ),
            None => bail!("Unexpected end of input"),
        }
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek().map(|t| t.kind == kind).unwrap_or(false)
    }

    fn skip_trivia(&mut self) {
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Comment | TokenKind::Whitespace => {
                    self.position += 1;
                }
                _ => break,
            }
        }
    }

    fn is_eof(&self) -> bool {
        self.peek()
            .map(|t| t.kind == TokenKind::Eof)
            .unwrap_or(true)
    }

    // =========================================================================
    // Function Parsing
    // =========================================================================

    fn parse_function(&mut self) -> Result<Item> {
        let fn_token = self.expect(TokenKind::KwFn)?;

        // Function name
        let name = match self.peek() {
            Some(t) if t.kind == TokenKind::Identifier => self.advance().map(|t| t.value).unwrap(),
            _ => bail!("Expected function name"),
        };

        // Type parameters (generics)
        let type_params = if self.check(TokenKind::OpLt) {
            self.parse_type_params()?
        } else {
            Vec::new()
        };

        // Parameters
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if !self.check(TokenKind::RParen) {
            loop {
                let param_name = match self.peek() {
                    Some(t) if t.kind == TokenKind::Identifier => {
                        self.advance().map(|t| t.value).unwrap()
                    }
                    _ => bail!("Expected parameter name"),
                };

                self.expect(TokenKind::Colon)?;
                let param_type = self.parse_type()?;

                params.push(Param {
                    name: param_name,
                    ty: param_type,
                    default: None,
                });

                if self.check(TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen)?;

        // Return type
        let return_type = if self.check(TokenKind::OpArrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Body
        let body = self.parse_block()?;
        let body_span = body.span;

        Ok(Item::Function(Function {
            name,
            type_params,
            params,
            return_type,
            body,
            span: fn_token.span.merge(body_span),
        }))
    }

    fn parse_type_params(&mut self) -> Result<Vec<TypeParam>> {
        self.expect(TokenKind::OpLt)?;
        let mut params = Vec::new();

        loop {
            let name = match self.peek() {
                Some(t) if t.kind == TokenKind::Identifier => {
                    self.advance().map(|t| t.value).unwrap()
                }
                _ => break,
            };

            params.push(TypeParam {
                name,
                bounds: Vec::new(),
            });

            if self.check(TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(TokenKind::OpGt)?;
        Ok(params)
    }

    // =========================================================================
    // Struct Parsing
    // =========================================================================

    fn parse_struct(&mut self) -> Result<Item> {
        let struct_token = self.expect(TokenKind::KwStruct)?;

        let name = match self.peek() {
            Some(t) if t.kind == TokenKind::Identifier => self.advance().map(|t| t.value).unwrap(),
            _ => bail!("Expected struct name"),
        };

        let type_params = if self.check(TokenKind::OpLt) {
            self.parse_type_params()?
        } else {
            Vec::new()
        };

        self.expect(TokenKind::LBrace)?;

        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) {
            self.skip_trivia();

            let field_name = match self.peek() {
                Some(t) if t.kind == TokenKind::Identifier => {
                    self.advance().map(|t| t.value).unwrap()
                }
                _ => break,
            };

            self.expect(TokenKind::Colon)?;
            let field_type = self.parse_type()?;

            fields.push(Field {
                name: field_name,
                ty: field_type,
                visibility: Visibility::Private,
            });

            if self.check(TokenKind::Comma) {
                self.advance();
            }

            self.skip_trivia();
        }

        self.expect(TokenKind::RBrace)?;

        Ok(Item::Struct(StructDef {
            name,
            type_params,
            fields,
            span: struct_token.span,
        }))
    }

    // =========================================================================
    // Enum Parsing
    // =========================================================================

    fn parse_enum(&mut self) -> Result<Item> {
        let enum_token = self.expect(TokenKind::KwEnum)?;

        let name = match self.peek() {
            Some(t) if t.kind == TokenKind::Identifier => self.advance().map(|t| t.value).unwrap(),
            _ => bail!("Expected enum name"),
        };

        let type_params = if self.check(TokenKind::OpLt) {
            self.parse_type_params()?
        } else {
            Vec::new()
        };

        self.expect(TokenKind::LBrace)?;

        let mut variants = Vec::new();
        while !self.check(TokenKind::RBrace) {
            self.skip_trivia();

            let variant_name = match self.peek() {
                Some(t) if t.kind == TokenKind::Identifier => {
                    self.advance().map(|t| t.value).unwrap()
                }
                _ => break,
            };

            let data = if self.check(TokenKind::LParen) {
                self.advance();
                let mut types = Vec::new();
                while !self.check(TokenKind::RParen) {
                    types.push(self.parse_type()?);
                    if self.check(TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RParen)?;
                Some(EnumData::Tuple(types))
            } else if self.check(TokenKind::LBrace) {
                self.advance();
                let mut fields = Vec::new();
                while !self.check(TokenKind::RBrace) {
                    let field_name = match self.peek() {
                        Some(t) if t.kind == TokenKind::Identifier => {
                            self.advance().map(|t| t.value).unwrap()
                        }
                        _ => bail!("Expected field name"),
                    };
                    self.expect(TokenKind::Colon)?;
                    fields.push((field_name, self.parse_type()?));
                    if self.check(TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RBrace)?;
                Some(EnumData::Struct(fields))
            } else {
                None
            };

            variants.push(EnumVariant {
                name: variant_name,
                data,
            });

            if self.check(TokenKind::Comma) {
                self.advance();
            }

            self.skip_trivia();
        }

        self.expect(TokenKind::RBrace)?;

        Ok(Item::Enum(EnumDef {
            name,
            type_params,
            variants,
            span: enum_token.span,
        }))
    }

    // =========================================================================
    // Type Parsing
    // =========================================================================

    fn parse_type(&mut self) -> Result<Type> {
        self.parse_type_with_bounds(0)
    }

    fn parse_type_with_bounds(&mut self, min_prec: u8) -> Result<Type> {
        // Parse primary type
        let mut lhs = match self.peek().map(|t| &t.kind) {
            Some(t) if *t == TokenKind::Identifier => {
                let name = self.advance().map(|t| t.value).unwrap();

                // Check for generic arguments
                if self.check(TokenKind::OpLt) {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(TokenKind::OpGt) {
                        args.push(self.parse_type()?);
                        if self.check(TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::OpGt)?;
                    Type::Generic(name, args)
                } else {
                    Type::Name(name)
                }
            }
            Some(t) if *t == TokenKind::LParen => {
                self.advance();
                let mut types = Vec::new();
                while !self.check(TokenKind::RParen) {
                    types.push(self.parse_type()?);
                    if self.check(TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RParen)?;
                Type::Tuple(types)
            }
            Some(t) if *t == TokenKind::LBracket => {
                self.advance();
                let inner = self.parse_type()?;

                if self.check(TokenKind::Semicolon) {
                    // Array type: [T; N]
                    self.advance();
                    let size = match self.peek().map(|t| &t.kind) {
                        Some(t) if *t == TokenKind::Integer => self
                            .advance()
                            .map(|t| t.value.parse::<usize>().unwrap_or(0))
                            .unwrap(),
                        _ => bail!("Expected array size"),
                    };
                    self.expect(TokenKind::RBracket)?;
                    Type::Array(Box::new(inner), size)
                } else {
                    // Slice type: [T]
                    self.expect(TokenKind::RBracket)?;
                    Type::Slice(Box::new(inner))
                }
            }
            Some(t) if *t == TokenKind::OpMul => {
                // Pointer type
                self.advance();
                let mutable = self.check(TokenKind::KwMut);
                if mutable {
                    self.advance();
                }
                let inner = self.parse_type()?;
                Type::Pointer(Box::new(inner), mutable)
            }
            Some(t) if *t == TokenKind::OpBitAnd => {
                // Reference type
                self.advance();
                let mutable = self.check(TokenKind::KwMut);
                if mutable {
                    self.advance();
                }
                let inner = self.parse_type()?;
                Type::Reference(Box::new(inner), mutable)
            }
            Some(TokenKind::KwFn) => {
                // Function type
                self.advance();
                self.expect(TokenKind::LParen)?;
                let mut param_types = Vec::new();
                while !self.check(TokenKind::RParen) {
                    param_types.push(self.parse_type()?);
                    if self.check(TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RParen)?;
                let return_type = if self.check(TokenKind::OpArrow) {
                    self.advance();
                    Some(Box::new(self.parse_type()?))
                } else {
                    None
                };
                Type::Function(param_types, return_type)
            }
            Some(t) if *t == TokenKind::Question => {
                self.advance();
                let inner = self.parse_type()?;
                Type::Option(Box::new(inner))
            }
            _ => bail!("Expected type"),
        };

        // Handle postfix operators
        while !self.is_eof() {
            match self.peek().map(|t| &t.kind) {
                Some(t) if *t == TokenKind::OpBitOr => {
                    // Function type shorthand: T | U
                    self.advance();
                    let rhs = self.parse_type_with_bounds(0)?;
                    lhs = Type::Sum(Box::new(lhs), Box::new(rhs));
                }
                _ => break,
            }
        }

        Ok(lhs)
    }

    // =========================================================================
    // Expression Parsing
    // =========================================================================

    fn parse_expression(&mut self) -> Result<Expr> {
        self.parse_expression_with_prec(0)
    }

    fn parse_expression_with_prec(&mut self, min_prec: u8) -> Result<Expr> {
        // Parse unary operators
        let mut lhs = match self.peek().map(|t| &t.kind) {
            Some(t)
                if matches!(
                    t,
                    TokenKind::OpSub
                        | TokenKind::OpLogNot
                        | TokenKind::OpBitNot
                        | TokenKind::OpMul
                        | TokenKind::OpBitAnd
                ) =>
            {
                let op = self.advance().unwrap();
                let rhs = self.parse_expression_with_prec(11)?;
                Expr::Unary {
                    op: match op.kind {
                        TokenKind::OpSub => UnaryOp::Neg,
                        TokenKind::OpLogNot => UnaryOp::Not,
                        TokenKind::OpBitNot => UnaryOp::BitNot,
                        TokenKind::OpMul => UnaryOp::Deref,
                        TokenKind::OpBitAnd => UnaryOp::Ref,
                        _ => bail!("Unexpected unary operator"),
                    },
                    expr: Box::new(rhs),
                    span: op.span,
                }
            }
            _ => self.parse_primary()?,
        };

        // Parse binary operators
        loop {
            let op = match self.peek() {
                Some(t) if Self::is_binary_op(&t.kind) => self.advance().unwrap(),
                _ => break,
            };

            let prec = Self::precedence(&op.kind);
            if prec < min_prec {
                break;
            }

            // Handle right-associative operators
            let next_prec = if Self::is_right_assoc(&op.kind) {
                prec
            } else {
                prec + 1
            };

            let rhs = self.parse_expression_with_prec(next_prec)?;

            lhs = Expr::Binary {
                op: match op.kind {
                    TokenKind::OpAdd => BinaryOp::Add,
                    TokenKind::OpSub => BinaryOp::Sub,
                    TokenKind::OpMul => BinaryOp::Mul,
                    TokenKind::OpDiv => BinaryOp::Div,
                    TokenKind::OpRem => BinaryOp::Rem,
                    TokenKind::OpEq => BinaryOp::Eq,
                    TokenKind::OpNe => BinaryOp::Ne,
                    TokenKind::OpLt => BinaryOp::Lt,
                    TokenKind::OpGt => BinaryOp::Gt,
                    TokenKind::OpLe => BinaryOp::Le,
                    TokenKind::OpGe => BinaryOp::Ge,
                    TokenKind::OpLogAnd => BinaryOp::And,
                    TokenKind::OpLogOr => BinaryOp::Or,
                    TokenKind::OpBitAnd => BinaryOp::BitAnd,
                    TokenKind::OpBitOr => BinaryOp::BitOr,
                    TokenKind::OpBitXor => BinaryOp::BitXor,
                    TokenKind::OpShl => BinaryOp::Shl,
                    TokenKind::OpShr => BinaryOp::Shr,
                    TokenKind::OpPipeline => BinaryOp::Or,
                    _ => bail!("Unexpected binary operator"),
                },
                left: Box::new(lhs),
                right: Box::new(rhs),
                span: op.span,
            };
        }

        Ok(lhs)
    }

    fn is_binary_op(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::OpAdd
                | TokenKind::OpSub
                | TokenKind::OpMul
                | TokenKind::OpDiv
                | TokenKind::OpRem
                | TokenKind::OpEq
                | TokenKind::OpNe
                | TokenKind::OpLt
                | TokenKind::OpGt
                | TokenKind::OpLe
                | TokenKind::OpGe
                | TokenKind::OpLogAnd
                | TokenKind::OpLogOr
                | TokenKind::OpBitAnd
                | TokenKind::OpBitOr
                | TokenKind::OpBitXor
                | TokenKind::OpShl
                | TokenKind::OpShr
                | TokenKind::OpPipeline
        )
    }

    fn precedence(kind: &TokenKind) -> u8 {
        match kind {
            TokenKind::OpPipeline => 2,
            TokenKind::OpLogOr => 3,
            TokenKind::OpLogAnd => 4,
            TokenKind::OpBitOr => 5,
            TokenKind::OpBitXor => 6,
            TokenKind::OpBitAnd => 7,
            TokenKind::OpEq | TokenKind::OpNe => 8,
            TokenKind::OpLt | TokenKind::OpGt | TokenKind::OpLe | TokenKind::OpGe => 9,
            TokenKind::OpShl | TokenKind::OpShr => 10,
            TokenKind::OpAdd | TokenKind::OpSub => 11,
            TokenKind::OpMul | TokenKind::OpDiv | TokenKind::OpRem => 12,
            _ => 0,
        }
    }

    fn is_right_assoc(kind: &TokenKind) -> bool {
        matches!(kind, TokenKind::OpPipeline | TokenKind::Assign)
    }

    fn is_arg_start(&self) -> bool {
        matches!(
            self.peek().map(|t| &t.kind),
            Some(
                TokenKind::StrLiteral
                    | TokenKind::Integer
                    | TokenKind::Float
                    | TokenKind::Char
                    | TokenKind::KwTrue
                    | TokenKind::KwFalse
                    | TokenKind::Identifier
                    | TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::LBrace
                    | TokenKind::OpSub
                    | TokenKind::OpLogNot
                    | TokenKind::OpBitNot
            )
        )
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        let token = self
            .advance()
            .ok_or_else(|| anyhow::anyhow!("Unexpected end of input"))?;

        match token.kind {
            TokenKind::Identifier => {
                // Could be a variable, function call, or method call
                if self.check(TokenKind::LParen) {
                    // Function call with parentheses
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(TokenKind::RParen) {
                        args.push(self.parse_expression()?);
                        if self.check(TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Expr::Call {
                        func: Box::new(Expr::Ident(token.value, token.span)),
                        args,
                        span: token.span,
                    })
                } else if self.check(TokenKind::Dot) {
                    // Field access or method call
                    self.advance();
                    let field = match self.peek() {
                        Some(t) if t.kind == TokenKind::Identifier => {
                            self.advance().map(|t| t.value).unwrap()
                        }
                        _ => bail!("Expected field name"),
                    };
                    Ok(Expr::Field {
                        expr: Box::new(Expr::Ident(token.value, token.span)),
                        field,
                        span: token.span,
                    })
                } else if self.is_arg_start() {
                    // Function call without parentheses: fn arg
                    let arg = self.parse_expression()?;
                    Ok(Expr::Call {
                        func: Box::new(Expr::Ident(token.value, token.span)),
                        args: vec![arg],
                        span: token.span,
                    })
                } else {
                    Ok(Expr::Ident(token.value, token.span))
                }
            }
            TokenKind::Integer => Ok(Expr::Literal(
                Literal::Int(token.value.parse().unwrap_or(0)),
                token.span,
            )),
            TokenKind::Float => Ok(Expr::Literal(
                Literal::Float(token.value.parse().unwrap_or(0.0)),
                token.span,
            )),
            TokenKind::StrLiteral => Ok(Expr::Literal(Literal::String(token.value), token.span)),
            TokenKind::Char => {
                let c = token.value.chars().next().unwrap_or('\0');
                Ok(Expr::Literal(Literal::Char(c), token.span))
            }
            TokenKind::KwTrue | TokenKind::KwFalse => Ok(Expr::Literal(
                Literal::Bool(token.kind == TokenKind::KwTrue),
                token.span,
            )),
            TokenKind::LParen => {
                // Tuple or parenthesized expression
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                Ok(Expr::Paren(Box::new(expr)))
            }
            TokenKind::LBracket => {
                // Array or slice
                self.advance();
                let mut elements = Vec::new();
                while !self.check(TokenKind::RBracket) {
                    elements.push(self.parse_expression()?);
                    if self.check(TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::Array(elements, token.span))
            }
            TokenKind::LBrace => {
                // Block or struct literal
                self.parse_block_like()
            }
            TokenKind::KwIf => self.parse_if_expr(token.span),
            TokenKind::KwMatch => self.parse_match_expr(token.span),
            TokenKind::KwLoop => self.parse_loop_expr(token.span),
            TokenKind::KwWhile => self.parse_while_expr(token.span),
            TokenKind::KwFor => self.parse_for_expr(token.span),
            TokenKind::KwFn => self.parse_lambda(token.span),
            _ => bail!("Unexpected token: {:?}", token.kind),
        }
    }

    fn parse_if_expr(&mut self, span: Span) -> Result<Expr> {
        let condition = self.parse_expression()?;
        let then_branch = self.parse_block()?;

        let else_branch = if self.check(TokenKind::KwElse) {
            self.advance();
            Some(Box::new(if self.check(TokenKind::KwIf) {
                self.parse_if_expr(span)?
            } else {
                Expr::Block(self.parse_block()?)
            }))
        } else {
            None
        };

        Ok(Expr::If {
            condition: Box::new(condition),
            then: Box::new(Expr::Block(then_branch)),
            else_branch: else_branch,
            span,
        })
    }

    fn parse_match_expr(&mut self, span: Span) -> Result<Expr> {
        let scrutinee = self.parse_expression()?;

        self.expect(TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while !self.check(TokenKind::RBrace) {
            let pattern = self.parse_pattern()?;

            self.expect(TokenKind::OpArrow)?;

            let guard = if self.check(TokenKind::KwIf) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            let body = if self.check(TokenKind::LBrace) {
                self.parse_block()?
            } else {
                let expr = self.parse_expression()?;
                let mut stmts = Vec::new();
                stmts.push(Stmt::Expr(expr));
                Block {
                    stmts,
                    span: Span::default(),
                }
            };

            arms.push(MatchArm {
                pattern,
                guard,
                body,
            });

            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RBrace)?;

        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            span,
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern> {
        let token = self
            .peek()
            .ok_or_else(|| anyhow::anyhow!("Unexpected end of input"))?
            .clone();

        match token.kind {
            TokenKind::Identifier => {
                let name = self.advance().map(|t| t.value).unwrap();

                if self.check(TokenKind::LParen) {
                    // Enum variant
                    self.advance();
                    let mut subpatterns = Vec::new();
                    while !self.check(TokenKind::RParen) {
                        subpatterns.push(self.parse_pattern()?);
                        if self.check(TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Pattern::TupleStruct(name, subpatterns, token.span))
                } else if self.check(TokenKind::LBrace) {
                    // Struct pattern
                    self.advance();
                    let mut fields = Vec::new();
                    while !self.check(TokenKind::RBrace) {
                        let field_name = match self.peek() {
                            Some(t) if t.kind == TokenKind::Identifier => {
                                self.advance().map(|t| t.value).unwrap()
                            }
                            _ => bail!("Expected field name"),
                        };

                        self.expect(TokenKind::Colon)?;
                        let field_pattern = self.parse_pattern()?;

                        fields.push((field_name, field_pattern));

                        if self.check(TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RBrace)?;
                    Ok(Pattern::Struct(name, fields, token.span))
                } else {
                    // Just an identifier
                    Ok(Pattern::Ident(name, token.span))
                }
            }
            TokenKind::Underscore => {
                self.advance();
                Ok(Pattern::Wildcard(token.span))
            }
            TokenKind::Integer => {
                let val = self
                    .advance()
                    .map(|t| t.value.parse::<i64>().unwrap_or(0))
                    .unwrap();
                Ok(Pattern::Int(val, token.span))
            }
            TokenKind::StrLiteral => {
                let val = self.advance().map(|t| t.value).unwrap();
                Ok(Pattern::String(val, token.span))
            }
            _ => bail!("Expected pattern"),
        }
    }

    fn parse_loop_expr(&mut self, span: Span) -> Result<Expr> {
        let label = if self.peek_n(1).map(|t| &t.kind) == Some(&TokenKind::Identifier) {
            let label = self.advance().map(|t| t.value).unwrap();
            self.expect(TokenKind::Colon)?;
            Some(label)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(Expr::Loop {
            label,
            body: Box::new(Expr::Block(body)),
            span,
        })
    }

    fn parse_while_expr(&mut self, span: Span) -> Result<Expr> {
        let label = if self.peek_n(1).map(|t| &t.kind) == Some(&TokenKind::Identifier) {
            let label = self.advance().map(|t| t.value).unwrap();
            self.expect(TokenKind::Colon)?;
            Some(label)
        } else {
            None
        };

        let condition = self.parse_expression()?;
        let body = self.parse_block()?;

        Ok(Expr::While {
            label,
            condition: Box::new(condition),
            body: Box::new(Expr::Block(body)),
            span,
        })
    }

    fn parse_for_expr(&mut self, span: Span) -> Result<Expr> {
        let pattern = match self.peek() {
            Some(t) if t.kind == TokenKind::Identifier => {
                Pattern::Ident(self.advance().map(|t| t.value).unwrap(), Span::default())
            }
            _ => bail!("Expected pattern"),
        };

        self.expect(TokenKind::KwIn)?;
        let iter = self.parse_expression()?;
        let body = self.parse_block()?;

        Ok(Expr::For {
            pattern,
            iter: Box::new(iter),
            body: Box::new(Expr::Block(body)),
            span,
        })
    }

    fn parse_lambda(&mut self, span: Span) -> Result<Expr> {
        // Parameters
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !self.check(TokenKind::RParen) {
            let name = match self.peek() {
                Some(t) if t.kind == TokenKind::Identifier => {
                    self.advance().map(|t| t.value).unwrap()
                }
                _ => bail!("Expected parameter name"),
            };
            params.push(name);
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RParen)?;

        // Return type
        let return_type = if self.check(TokenKind::OpArrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Body
        let body = if self.check(TokenKind::OpArrow) {
            // Expression body
            let expr = self.parse_expression()?;
            Expr::Block(Block {
                stmts: vec![Stmt::Expr(expr)],
                span: Span::default(),
            })
        } else {
            Expr::Block(self.parse_block()?)
        };

        Ok(Expr::Lambda {
            params,
            return_type,
            body: Box::new(body),
            span,
        })
    }

    fn parse_block(&mut self) -> Result<Block> {
        self.expect(TokenKind::LBrace)?;
        match self.parse_block_like() {
            Ok(Expr::Block(block)) => Ok(block),
            Ok(_) => bail!("Expected block expression"),
            Err(e) => Err(e),
        }
    }

    fn parse_block_like(&mut self) -> Result<Expr> {
        let mut stmts = Vec::new();
        let start = self.peek().map(|t| t.span.start).unwrap_or_default();

        while !self.check(TokenKind::RBrace) && !self.is_eof() {
            self.skip_trivia();
            if self.check(TokenKind::RBrace) {
                break;
            }
            stmts.push(self.parse_statement()?);
        }

        let end = if let Some(t) = self.peek() {
            let end_pos = t.span.end;
            self.expect(TokenKind::RBrace)?;
            end_pos
        } else {
            Position::default()
        };

        Ok(Expr::Block(Block {
            stmts,
            span: Span::new(start, end),
        }))
    }

    // =========================================================================
    // Statement Parsing
    // =========================================================================

    fn parse_statement(&mut self) -> Result<Stmt> {
        let token = self
            .peek()
            .ok_or_else(|| anyhow::anyhow!("Unexpected end of input"))?;

        match token.kind {
            TokenKind::KwLet => self.parse_let_stmt(),
            TokenKind::KwVar => self.parse_var_stmt(),
            TokenKind::KwReturn => self.parse_return_stmt(),
            TokenKind::KwBreak => self.parse_break_stmt(),
            TokenKind::KwContinue => self.parse_continue_stmt(),
            TokenKind::KwDefer => self.parse_defer_stmt(),
            TokenKind::KwUnsafe => self.parse_unsafe_stmt(),
            TokenKind::KwAsm => self.parse_asm_stmt(),
            TokenKind::KwSyscall => self.parse_syscall_stmt(),
            TokenKind::KwComptime => self.parse_comptime_stmt(),
            _ => {
                // Expression statement
                let expr = self.parse_expression()?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt> {
        self.expect(TokenKind::KwLet)?;

        let pattern = match self.peek() {
            Some(t) if t.kind == TokenKind::Identifier => {
                let name = self.advance().map(|t| t.value).unwrap();
                Pattern::Ident(name, Span::default())
            }
            _ => bail!("Expected pattern"),
        };

        let ty = if self.check(TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let init = if self.check(TokenKind::Assign) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Let { pattern, ty, init })
    }

    fn parse_var_stmt(&mut self) -> Result<Stmt> {
        self.expect(TokenKind::KwVar)?;

        let pattern = match self.peek() {
            Some(t) if t.kind == TokenKind::Identifier => {
                let name = self.advance().map(|t| t.value).unwrap();
                Pattern::Ident(name, Span::default())
            }
            _ => bail!("Expected pattern"),
        };

        let ty = if self.check(TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let init = if self.check(TokenKind::Assign) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Var { pattern, ty, init })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt> {
        self.expect(TokenKind::KwReturn)?;

        let value = if self.check(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Return { value })
    }

    fn parse_break_stmt(&mut self) -> Result<Stmt> {
        self.expect(TokenKind::KwBreak)?;

        let label = if self.check(TokenKind::Identifier) {
            Some(self.advance().map(|t| t.value).unwrap())
        } else {
            None
        };

        let value = if !self.check(TokenKind::Semicolon) && !self.check(TokenKind::Comma) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Break { label, value })
    }

    fn parse_continue_stmt(&mut self) -> Result<Stmt> {
        self.expect(TokenKind::KwContinue)?;

        let label = if self.check(TokenKind::Identifier) {
            Some(self.advance().map(|t| t.value).unwrap())
        } else {
            None
        };

        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Continue { label })
    }

    fn parse_defer_stmt(&mut self) -> Result<Stmt> {
        self.expect(TokenKind::KwDefer)?;

        let expr = self.parse_expression()?;

        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Defer {
            expr: Box::new(expr),
        })
    }

    fn parse_unsafe_stmt(&mut self) -> Result<Stmt> {
        self.expect(TokenKind::KwUnsafe)?;

        let block = self.parse_block()?;

        Ok(Stmt::Unsafe {
            block: Box::new(Stmt::Expr(Expr::Block(block))),
        })
    }

    fn parse_asm_stmt(&mut self) -> Result<Stmt> {
        self.expect(TokenKind::KwAsm)?;

        // Optional syntax (intel/att)
        let syntax = if self.check(TokenKind::Identifier) {
            Some(self.advance().map(|t| t.value).unwrap())
        } else {
            None
        };

        let code = match self.peek() {
            Some(t) if t.kind == TokenKind::StrLiteral => self.advance().map(|t| t.value).unwrap(),
            _ => bail!("Expected assembly code string"),
        };

        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Asm { syntax, code })
    }

    fn parse_syscall_stmt(&mut self) -> Result<Stmt> {
        self.expect(TokenKind::KwSyscall)?;

        let mut args = Vec::new();

        while !self.check(TokenKind::Semicolon) {
            args.push(self.parse_expression()?);
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Syscall { args })
    }

    fn parse_comptime_stmt(&mut self) -> Result<Stmt> {
        self.expect(TokenKind::KwComptime)?;

        let block = self.parse_block()?;

        Ok(Stmt::Comptime {
            block: Box::new(Stmt::Expr(Expr::Block(block))),
        })
    }

    // =========================================================================
    // Other Item Parsing
    // =========================================================================

    fn parse_trait(&mut self) -> Result<Item> {
        todo!("Parse trait")
    }

    fn parse_impl(&mut self) -> Result<Item> {
        todo!("Parse impl")
    }

    fn parse_type_alias(&mut self) -> Result<Item> {
        todo!("Parse type alias")
    }

    fn parse_const(&mut self) -> Result<Item> {
        self.expect(TokenKind::KwConst)?;

        let name = match self.peek() {
            Some(t) if t.kind == TokenKind::Identifier => self.advance().map(|t| t.value).unwrap(),
            _ => bail!("Expected constant name"),
        };

        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;

        self.expect(TokenKind::Assign)?;
        let value = self.parse_expression()?;

        if self.check(TokenKind::Semicolon) {
            self.advance();
        }

        Ok(Item::Const(ConstItem {
            name,
            ty,
            value,
            span: Span::default(),
        }))
    }

    fn parse_static(&mut self) -> Result<Item> {
        todo!("Parse static")
    }

    fn parse_module(&mut self) -> Result<Item> {
        todo!("Parse module")
    }

    fn parse_import(&mut self) -> Result<Item> {
        todo!("Parse import")
    }

    fn parse_actor(&mut self) -> Result<Item> {
        todo!("Parse actor")
    }
}
