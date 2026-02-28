//! Knull Lexer - Simple Tokenizer

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier,
    Int,
    Float,
    String,
    Char,
    Fn,
    Let,
    Mut,
    If,
    Else,
    Match,
    For,
    In,
    Loop,
    While,
    Return,
    Pub,
    Mod,
    Use,
    Struct,
    Enum,
    Trait,
    Impl,
    SelfValue,
    Unsafe,
    Const,
    Type,
    Never,
    True,
    False,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Ampersand,
    Pipe,
    Caret,
    Bang,
    Question,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    AmpEq,
    BarEq,
    Eq,
    EqEq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    And,
    Or,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Colon,
    Dot,
    At,
    Underscore,
    Eof,
    Unknown,
}

impl TokenKind {
    pub fn from_keyword(s: &str) -> Option<TokenKind> {
        match s {
            "fn" => Some(TokenKind::Fn),
            "let" => Some(TokenKind::Let),
            "mut" => Some(TokenKind::Mut),
            "if" => Some(TokenKind::If),
            "else" => Some(TokenKind::Else),
            "match" => Some(TokenKind::Match),
            "for" => Some(TokenKind::For),
            "in" => Some(TokenKind::In),
            "loop" => Some(TokenKind::Loop),
            "while" => Some(TokenKind::While),
            "return" => Some(TokenKind::Return),
            "pub" => Some(TokenKind::Pub),
            "mod" => Some(TokenKind::Mod),
            "module" => Some(TokenKind::Mod),
            "use" => Some(TokenKind::Use),
            "struct" => Some(TokenKind::Struct),
            "enum" => Some(TokenKind::Enum),
            "trait" => Some(TokenKind::Trait),
            "impl" => Some(TokenKind::Impl),
            "self" => Some(TokenKind::SelfValue),
            "unsafe" => Some(TokenKind::Unsafe),
            "const" => Some(TokenKind::Const),
            "type" => Some(TokenKind::Type),
            "never" => Some(TokenKind::Never),
            "true" => Some(TokenKind::True),
            "false" => Some(TokenKind::False),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.source.get(self.pos).copied();
        if c == Some('\n') {
            self.line += 1;
            self.col = 0;
        } else {
            self.col += 1;
        }
        self.pos += 1;
        c
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while let Some(c) = self.peek() {
            // Skip whitespace
            if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
                self.advance();
                continue;
            }

            // Skip comments
            if c == '/' {
                if let Some(&next) = self.source.get(self.pos + 1) {
                    if next == '/' {
                        while let Some(ch) = self.peek() {
                            if ch == '\n' {
                                break;
                            }
                            self.advance();
                        }
                        continue;
                    }
                }
            }

            let line = self.line;
            let col = self.col;

            // Identifier or keyword
            if c.is_alphabetic() || c == '_' {
                let mut value = String::new();
                while let Some(ch) = self.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        value.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
                let kind = TokenKind::from_keyword(&value).unwrap_or(TokenKind::Identifier);
                tokens.push(Token {
                    kind,
                    value,
                    line,
                    col,
                });
                continue;
            }

            // Number
            if c.is_numeric() {
                let mut value = String::new();
                let mut has_dot = false;
                while let Some(ch) = self.peek() {
                    if ch.is_numeric() {
                        value.push(ch);
                        self.advance();
                    } else if ch == '.' && !has_dot {
                        has_dot = true;
                        value.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
                let kind = if has_dot {
                    TokenKind::Float
                } else {
                    TokenKind::Int
                };
                tokens.push(Token {
                    kind,
                    value,
                    line,
                    col,
                });
                continue;
            }

            // String
            if c == '"' {
                self.advance();
                let mut value = String::new();
                while let Some(ch) = self.peek() {
                    if ch == '"' {
                        self.advance();
                        break;
                    }
                    if ch == '\\' {
                        self.advance();
                        if let Some(escaped) = self.peek() {
                            match escaped {
                                'n' => value.push('\n'),
                                't' => value.push('\t'),
                                'r' => value.push('\r'),
                                _ => value.push(escaped),
                            }
                            self.advance();
                        }
                    } else {
                        value.push(ch);
                        self.advance();
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::String,
                    value,
                    line,
                    col,
                });
                continue;
            }

            // Single char token
            let kind = match c {
                '+' => {
                    self.advance();
                    TokenKind::Plus
                }
                '-' => {
                    self.advance();
                    TokenKind::Minus
                }
                '*' => {
                    self.advance();
                    TokenKind::Star
                }
                '/' => {
                    self.advance();
                    TokenKind::Slash
                }
                '%' => {
                    self.advance();
                    TokenKind::Percent
                }
                '&' => {
                    self.advance();
                    TokenKind::Ampersand
                }
                '|' => {
                    self.advance();
                    TokenKind::Pipe
                }
                '^' => {
                    self.advance();
                    TokenKind::Caret
                }
                '!' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::Neq,
                            value: "!=".to_string(),
                            line,
                            col,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::Bang,
                            value: "!".to_string(),
                            line,
                            col,
                        });
                    }
                    continue;
                }
                '?' => {
                    self.advance();
                    TokenKind::Question
                }
                '(' => {
                    self.advance();
                    TokenKind::LParen
                }
                ')' => {
                    self.advance();
                    TokenKind::RParen
                }
                '{' => {
                    self.advance();
                    TokenKind::LBrace
                }
                '}' => {
                    self.advance();
                    TokenKind::RBrace
                }
                '[' => {
                    self.advance();
                    TokenKind::LBracket
                }
                ']' => {
                    self.advance();
                    TokenKind::RBracket
                }
                ',' => {
                    self.advance();
                    TokenKind::Comma
                }
                ';' => {
                    self.advance();
                    TokenKind::Semicolon
                }
                ':' => {
                    self.advance();
                    TokenKind::Colon
                }
                '.' => {
                    self.advance();
                    TokenKind::Dot
                }
                '@' => {
                    self.advance();
                    TokenKind::At
                }
                '_' => {
                    self.advance();
                    TokenKind::Underscore
                }
                '=' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::EqEq,
                            value: "==".to_string(),
                            line,
                            col,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::Eq,
                            value: "=".to_string(),
                            line,
                            col,
                        });
                    }
                    continue;
                }
                '<' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::Lte,
                            value: "<=".to_string(),
                            line,
                            col,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::Lt,
                            value: "<".to_string(),
                            line,
                            col,
                        });
                    }
                    continue;
                }
                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::Gte,
                            value: ">=".to_string(),
                            line,
                            col,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::Gt,
                            value: ">".to_string(),
                            line,
                            col,
                        });
                    }
                    continue;
                }
                _ => {
                    self.advance();
                    TokenKind::Unknown
                }
            };
            tokens.push(Token {
                kind,
                value: c.to_string(),
                line,
                col,
            });
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            value: String::new(),
            line: self.line,
            col: self.col,
        });
        tokens
    }
}
