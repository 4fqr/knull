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
    Null,
    // Modes
    Mode,
    Novice,
    Expert,
    God,
    // Special
    Asm,
    Syscall,
    // Compile-time execution
    HashRun,
    // Linear types
    Linear,
    // Effect system
    Effect,
    // Coroutines
    Yield,
    // Async
    Async,
    Await,
    // Linear type consumption
    Consume,
    // Type cast
    As,
    // Range
    DotDot,
    DotDotEq,
    // Arrows
    Arrow,
    FatArrow,
    DoubleColon,
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
            "as" => Some(TokenKind::As),
            "type" => Some(TokenKind::Type),
            "never" => Some(TokenKind::Never),
            "true" => Some(TokenKind::True),
            "false" => Some(TokenKind::False),
            "null" => Some(TokenKind::Null),
            // Modes
            "mode" => Some(TokenKind::Mode),
            "novice" => Some(TokenKind::Novice),
            "expert" => Some(TokenKind::Expert),
            "god" => Some(TokenKind::God),
            // Special
            "asm" => Some(TokenKind::Asm),
            "syscall" => Some(TokenKind::Syscall),
            // Linear types
            "linear" => Some(TokenKind::Linear),
            "consume" => Some(TokenKind::Consume),
            "share" => Some(TokenKind::Identifier),
            "copy" => Some(TokenKind::Identifier),
            // Effect system
            "effect" => Some(TokenKind::Effect),
            "handler" => Some(TokenKind::Identifier),
            "pure" => Some(TokenKind::Identifier),
            // IO effect
            "IO" => Some(TokenKind::Effect),
            // Other keywords
            "yield" => Some(TokenKind::Yield),
            "async" => Some(TokenKind::Async),
            "await" => Some(TokenKind::Await),
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
                        // Single-line comment
                        while let Some(ch) = self.peek() {
                            if ch == '\n' {
                                break;
                            }
                            self.advance();
                        }
                        continue;
                    } else if next == '*' {
                        // Block comment /* ... */
                        self.advance(); // skip /
                        self.advance(); // skip *
                        let mut depth = 1;
                        while let Some(ch) = self.peek() {
                            if ch == '/' {
                                self.advance();
                                if let Some(&next_ch) = self.source.get(self.pos) {
                                    if next_ch == '*' {
                                        depth = depth + 1;
                                        self.advance();
                                    } else if next_ch == '/' {
                                        // Check for nested comment end
                                    }
                                }
                            } else if ch == '*' {
                                self.advance();
                                if let Some(&next_ch) = self.source.get(self.pos) {
                                    if next_ch == '/' {
                                        depth = depth - 1;
                                        self.advance();
                                        if depth == 0 {
                                            break;
                                        }
                                    }
                                }
                            } else {
                                self.advance();
                            }
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

            // Number (including hex 0x...)
            if c.is_numeric() {
                let mut value = String::new();
                value.push(c);
                self.advance();

                // Check for hex literal (0x...)
                if c == '0' {
                    if let Some(ch) = self.peek() {
                        if ch == 'x' || ch == 'X' {
                            value.push(ch);
                            self.advance();
                            while let Some(ch) = self.peek() {
                                if ch.is_ascii_hexdigit() {
                                    value.push(ch);
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                            tokens.push(Token {
                                kind: TokenKind::Int,
                                value,
                                line,
                                col,
                            });
                            continue;
                        }
                    }
                }

                // Regular number
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
                    // Check for -> (arrow)
                    if self.peek() == Some('>') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::Arrow,
                            value: "->".to_string(),
                            line,
                            col,
                        });
                        continue;
                    }
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
                    // Check for :: (double colon)
                    if self.peek() == Some(':') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::DoubleColon,
                            value: "::".to_string(),
                            line,
                            col,
                        });
                        continue;
                    }
                    TokenKind::Colon
                }
                '.' => {
                    self.advance();
                    // Check for .. or ..=
                    if self.peek() == Some('.') {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            tokens.push(Token {
                                kind: TokenKind::DotDotEq,
                                value: "..=".to_string(),
                                line,
                                col,
                            });
                        } else {
                            tokens.push(Token {
                                kind: TokenKind::DotDot,
                                value: "..".to_string(),
                                line,
                                col,
                            });
                        }
                        continue;
                    }
                    TokenKind::Dot
                }
                '@' => {
                    self.advance();
                    TokenKind::At
                }
                '#' => {
                    self.advance();
                    if self.peek() == Some('r') {
                        let rest = &self.source[self.pos..self.pos + 3];
                        if rest.iter().collect::<String>() == "run" {
                            self.advance();
                            self.advance();
                            self.advance();
                            tokens.push(Token {
                                kind: TokenKind::HashRun,
                                value: "#run".to_string(),
                                line,
                                col,
                            });
                            continue;
                        }
                    }
                    TokenKind::Unknown
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
                    } else if self.peek() == Some('>') {
                        self.advance();
                        tokens.push(Token {
                            kind: TokenKind::FatArrow,
                            value: "=>".to_string(),
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
