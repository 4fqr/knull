//! Knull Parser - AST Builder

use crate::lexer::{Lexer, Token, TokenKind};

#[derive(Debug, Clone)]
pub enum ASTNode {
    Program(Vec<ASTNode>),
    Function {
        name: String,
        body: Box<ASTNode>,
    },
    Let {
        name: String,
        value: Box<ASTNode>,
    },
    Return(Box<ASTNode>),
    Binary {
        op: String,
        left: Box<ASTNode>,
        right: Box<ASTNode>,
    },
    Call {
        func: String,
        args: Vec<ASTNode>,
    },
    Block(Vec<ASTNode>),
    Literal(Literal),
    Identifier(String),
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
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
            self.skip_to_next();
            match self.current().kind {
                TokenKind::Fn => items.push(self.parse_function()?),
                TokenKind::Let => items.push(self.parse_let()?),
                _ => {
                    self.advance();
                }
            }
        }
        Ok(ASTNode::Program(items))
    }

    fn skip_to_next(&mut self) {
        while self.current().kind == TokenKind::Semicolon {
            self.advance();
        }
    }

    fn parse_function(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Fn)?;
        let name = self.parse_identifier()?;

        // Skip params
        self.expect(TokenKind::LParen)?;
        while self.current().kind != TokenKind::RParen && self.current().kind != TokenKind::Eof {
            self.advance();
        }
        if self.current().kind == TokenKind::RParen {
            self.advance();
        }

        // Skip return type
        if self.current().kind == TokenKind::Minus {
            self.advance();
            if self.current().kind == TokenKind::Gt {
                self.advance();
            }
        }

        let body = self.parse_block()?;
        Ok(ASTNode::Function {
            name,
            body: Box::new(body),
        })
    }

    fn parse_let(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::Let)?;
        let name = self.parse_identifier()?;
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expression()?;
        if self.current().kind == TokenKind::Semicolon {
            self.advance();
        }
        Ok(ASTNode::Let {
            name,
            value: Box::new(value),
        })
    }

    fn parse_block(&mut self) -> Result<ASTNode, String> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while self.current().kind != TokenKind::RBrace && self.current().kind != TokenKind::Eof {
            self.skip_to_next();
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
            TokenKind::Return => {
                self.advance();
                let value = self.parse_expression()?;
                if self.current().kind == TokenKind::Semicolon {
                    self.advance();
                }
                Ok(ASTNode::Return(Box::new(value)))
            }
            TokenKind::LBrace => self.parse_block(),
            _ => self.parse_expression(),
        }
    }

    fn parse_expression(&mut self) -> Result<ASTNode, String> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> Result<ASTNode, String> {
        let mut left = self.parse_primary()?;
        while self.current().kind == TokenKind::Plus || self.current().kind == TokenKind::Minus {
            let op = self.current().value.clone();
            self.advance();
            let right = self.parse_primary()?;
            left = ASTNode::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<ASTNode, String> {
        let token = self.current().clone();
        match &token.kind {
            TokenKind::Int => {
                self.advance();
                let n: i64 = token.value.parse().unwrap_or(0);
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
            TokenKind::Identifier => {
                self.advance();
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
                    Ok(ASTNode::Call {
                        func: token.value,
                        args,
                    })
                } else {
                    Ok(ASTNode::Identifier(token.value))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                if self.current().kind == TokenKind::RParen {
                    self.advance();
                }
                Ok(expr)
            }
            _ => {
                self.advance();
                Ok(ASTNode::Identifier(token.value.clone()))
            }
        }
    }

    fn parse_identifier(&mut self) -> Result<String, String> {
        if self.current().kind == TokenKind::Identifier {
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
}

pub fn parse(source: &str) -> Result<ASTNode, String> {
    let mut parser = Parser::new(source);
    parser.parse()
}
