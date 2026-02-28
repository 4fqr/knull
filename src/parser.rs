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
    If {
        cond: Box<ASTNode>,
        then_body: Box<ASTNode>,
        else_body: Option<Box<ASTNode>>,
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
        func: String,
        args: Vec<ASTNode>,
    },
    Index {
        obj: Box<ASTNode>,
        index: Box<ASTNode>,
    },
    Block(Vec<ASTNode>),
    Literal(Literal),
    Identifier(String),
    Array(Vec<ASTNode>),
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
            self.skip_semis();
            if self.current().kind == TokenKind::Eof {
                break;
            }
            match self.current().kind {
                TokenKind::Fn => items.push(self.parse_function()?),
                TokenKind::Let => items.push(self.parse_let()?),
                TokenKind::If => items.push(self.parse_if()?),
                TokenKind::While => items.push(self.parse_while()?),
                TokenKind::For => items.push(self.parse_for()?),
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

        // Skip params
        if self.current().kind == TokenKind::LParen {
            self.advance();
            while self.current().kind != TokenKind::RParen && self.current().kind != TokenKind::Eof
            {
                self.advance();
            }
            if self.current().kind == TokenKind::RParen {
                self.advance();
            }
        }

        // Skip return type
        if self.current().kind == TokenKind::Minus {
            self.advance();
            if self.current().kind == TokenKind::Gt {
                self.advance();
            }
        }

        // Parse body
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
                                func: name,
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
                // Check if this is a function call on an object
                if let ASTNode::Identifier(name) = node {
                    node = ASTNode::Call { func: name, args };
                }
            } else if self.current().kind == TokenKind::LBracket {
                self.advance();
                let index = self.parse_expression()?;
                self.expect(TokenKind::RBracket)?;
                node = ASTNode::Index {
                    obj: Box::new(node),
                    index: Box::new(index),
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
}

pub fn parse(source: &str) -> Result<ASTNode, String> {
    let mut parser = Parser::new(source);
    parser.parse()
}
