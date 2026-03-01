//! Knull Lexer - Tokenizer
//!
//! This module handles lexical analysis, converting source code into a stream of tokens.

use anyhow::{bail, Result};
use std::iter::Peekable;
use std::str::Chars;

use crate::lexer::TokenKind::*;

/// A single token in the Knull language
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub value: String,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token {
            kind,
            span,
            value: String::new(),
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }
}

/// Source code span (line, column)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Span { start, end }
    }

    pub fn merge(self, other: Span) -> Self {
        Span {
            start: self.start,
            end: other.end,
        }
    }
}

/// Source code position
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl Position {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Position {
            line,
            column,
            offset,
        }
    }
}

/// Token types in Knull
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Integer,
    Float,
    StrLiteral,
    Char,
    Boolean,
    Identifier,

    // Keywords
    KwFn,
    KwLet,
    KwVar,
    KwOwn,
    KwMut,
    KwRef,
    KwConst,
    KwStatic,
    KwStruct,
    KwEnum,
    KwUnion,
    KwTrait,
    KwImpl,
    KwType,
    KwActor,
    KwMacro,
    KwUnsafe,
    KwComptime,
    KwGhost,
    KwIf,
    KwElse,
    KwMatch,
    KwLoop,
    KwWhile,
    KwFor,
    KwIn,
    KwReturn,
    KwBreak,
    KwContinue,
    KwDefer,
    KwGoto,
    KwEmbed,
    KwAsm,
    KwSyscall,
    KwImport,
    KwExport,
    KwPub,
    KwPriv,
    KwModule,
    KwAs,
    KwIs,
    KwNull,
    KwTrue,
    KwFalse,
    KwSelf,
    KwSuper,

    // Operators
    OpAdd,    // +
    OpSub,    // -
    OpMul,    // *
    OpDiv,    // /
    OpRem,    // %
    OpPow,    // **
    OpShl,    // <<
    OpShr,    // >>
    OpBitAnd, // &
    OpBitOr,  // |
    OpBitXor, // ^
    OpBitNot, // ~
    OpEq,     // ==
    OpNe,     // !=
    OpLt,     // <
    OpGt,     // >
    OpLe,     // <=
    OpGe,     // >=
    OpLogAnd, // &&
    OpLogOr,  // ||
    OpLogNot, // !

    // Assignment
    Assign,       // =
    AddAssign,    // +=
    SubAssign,    // -=
    MulAssign,    // *=
    DivAssign,    // /=
    RemAssign,    // %=
    ShlAssign,    // <<=
    ShrAssign,    // >>=
    BitAndAssign, // &=
    BitOrAssign,  // |=
    BitXorAssign, // ^=

    // Special operators
    OpElvis,     // ?:
    OpSafeNav,   // ?.
    OpPipeline,  // |>
    OpRange,     // ..
    OpRangeIncl, // ..=
    OpScopeRes,  // ::
    OpArrow,     // ->
    OpAt,        // @
    OpDollar,    // $
    Question,    // ?
    Underscore,  // _

    // Delimiters
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    LBrace,    // {
    RBrace,    // }
    Comma,     // ,
    Semicolon, // ;
    Colon,     // :
    Dot,       // .

    // Comments and whitespace
    Comment,
    Whitespace,

    // End of file
    Eof,
}

impl TokenKind {
    pub fn is_keyword(&self) -> bool {
        matches!(self, k if matches!(k,
            KwFn | KwLet | KwVar | KwOwn | KwMut | KwRef | KwConst | KwStatic |
            KwStruct | KwEnum | KwUnion | KwTrait | KwImpl | KwType | KwActor |
            KwMacro | KwUnsafe | KwComptime | KwGhost | KwIf | KwElse | KwMatch |
            KwLoop | KwWhile | KwFor | KwIn | KwReturn | KwBreak | KwContinue |
            KwDefer | KwGoto | KwEmbed | KwAsm | KwSyscall | KwImport | KwExport |
            KwPub | KwPriv | KwModule | KwAs | KwIs | KwNull | KwTrue | KwFalse |
            KwSelf | KwSuper
        ))
    }

    pub fn keyword(&self) -> Option<&'static str> {
        match self {
            KwFn => Some("fn"),
            KwLet => Some("let"),
            KwVar => Some("var"),
            KwOwn => Some("own"),
            KwMut => Some("mut"),
            KwRef => Some("ref"),
            KwConst => Some("const"),
            KwStatic => Some("static"),
            KwStruct => Some("struct"),
            KwEnum => Some("enum"),
            KwUnion => Some("union"),
            KwTrait => Some("trait"),
            KwImpl => Some("impl"),
            KwType => Some("type"),
            KwActor => Some("actor"),
            KwMacro => Some("macro"),
            KwUnsafe => Some("unsafe"),
            KwComptime => Some("comptime"),
            KwGhost => Some("ghost"),
            KwIf => Some("if"),
            KwElse => Some("else"),
            KwMatch => Some("match"),
            KwLoop => Some("loop"),
            KwWhile => Some("while"),
            KwFor => Some("for"),
            KwIn => Some("in"),
            KwReturn => Some("return"),
            KwBreak => Some("break"),
            KwContinue => Some("continue"),
            KwDefer => Some("defer"),
            KwGoto => Some("goto"),
            KwEmbed => Some("embed"),
            KwAsm => Some("asm"),
            KwSyscall => Some("syscall"),
            KwImport => Some("import"),
            KwExport => Some("export"),
            KwPub => Some("pub"),
            KwPriv => Some("priv"),
            KwModule => Some("module"),
            KwAs => Some("as"),
            KwIs => Some("is"),
            KwNull => Some("null"),
            KwTrue => Some("true"),
            KwFalse => Some("false"),
            KwSelf => Some("self"),
            KwSuper => Some("super"),
            _ => None,
        }
    }
}

/// The Knull Lexer
pub struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<Chars<'a>>,
    position: Position,
    tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer from source code
    pub fn new(source: &'a str) -> Self {
        Lexer {
            source: source,
            chars: source.chars().peekable(),
            position: Position::new(1, 0, 0),
            tokens: Vec::new(),
        }
    }

    /// Perform lexical analysis
    pub fn lex(mut self) -> Result<Vec<Token>> {
        while let Some(c) = self.chars.next() {
            let start = self.position;

            match c {
                // Whitespace
                ' ' | '\t' | '\r' | '\n' => {
                    self.handle_whitespace(c);
                }

                // Comments
                '/' => {
                    if self.peek() == Some(&'/') {
                        self.handle_line_comment();
                    } else if self.peek() == Some(&'*') {
                        self.handle_block_comment()?;
                    } else {
                        self.emit_char(c, start);
                    }
                }

                // Identifiers and keywords
                'a'..='z' | 'A'..='Z' | '_' => {
                    self.handle_identifier(c, start);
                }

                // Numbers
                '0'..='9' => {
                    self.handle_number(c, start)?;
                }

                // Strings
                '"' => {
                    self.handle_string(start)?;
                }

                // Characters
                '\'' => {
                    self.handle_char(start)?;
                }

                // Operators and delimiters
                c => {
                    self.handle_operator(c, start)?;
                }
            }
        }

        // Add EOF token
        self.tokens
            .push(Token::new(Eof, Span::new(self.position, self.position)));

        Ok(self.tokens)
    }

    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(ch) = c {
            if ch == '\n' {
                self.position.line += 1;
                self.position.column = 0;
            } else {
                self.position.column += 1;
            }
            self.position.offset += 1;
        }
        c
    }

    fn emit(&mut self, token: Token) {
        self.tokens.push(token);
    }

    fn emit_char(&mut self, c: char, start: Position) {
        let kind = match c {
            '+' => OpAdd,
            '-' => OpSub,
            '*' => OpMul,
            '/' => OpDiv,
            '%' => OpRem,
            '<' => OpLt,
            '>' => OpGt,
            '=' => Assign,
            '&' => OpBitAnd,
            '|' => OpBitOr,
            '^' => OpBitXor,
            '!' => OpLogNot,
            '~' => OpBitNot,
            '(' => LParen,
            ')' => RParen,
            '[' => LBracket,
            ']' => RBracket,
            '{' => LBrace,
            '}' => RBrace,
            ',' => Comma,
            ';' => Semicolon,
            ':' => Colon,
            '.' => Dot,
            _ => return,
        };

        self.emit(Token::new(kind, Span::new(start, self.position)));
    }

    fn handle_whitespace(&mut self, c: char) {
        let start = self.position;
        while let Some(&ch) = self.peek() {
            if !ch.is_whitespace() {
                break;
            }
            self.next();
        }
        // We don't emit whitespace tokens
    }

    fn handle_line_comment(&mut self) {
        let start = self.position;
        while let Some(&c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.next();
        }
        // Emit as comment token
        self.emit(Token::new(Comment, Span::new(start, self.position)));
    }

    fn handle_block_comment(&mut self) -> Result<()> {
        let start = self.position;
        self.next(); // Skip '*'

        let mut depth = 1;
        while let Some(c) = self.next() {
            if c == '/' && self.peek() == Some(&'*') {
                depth += 1;
                self.next();
            } else if c == '*' && self.peek() == Some(&'/') {
                depth -= 1;
                self.next();
                if depth == 0 {
                    break;
                }
            }
        }

        if depth != 0 {
            bail!("Unterminated block comment");
        }

        self.emit(Token::new(Comment, Span::new(start, self.position)));
        Ok(())
    }

    fn handle_identifier(&mut self, c: char, start: Position) {
        let mut value = String::new();
        value.push(c);

        while let Some(&ch) = self.peek() {
            if !ch.is_alphanumeric() && ch != '_' {
                break;
            }
            if let Some(ch) = self.next() {
                value.push(ch);
            }
        }

        let kind = match value.as_str() {
            // Keywords
            "fn" => KwFn,
            "let" => KwLet,
            "var" => KwVar,
            "own" => KwOwn,
            "mut" => KwMut,
            "ref" => KwRef,
            "const" => KwConst,
            "static" => KwStatic,
            "struct" => KwStruct,
            "enum" => KwEnum,
            "union" => KwUnion,
            "trait" => KwTrait,
            "impl" => KwImpl,
            "type" => KwType,
            "actor" => KwActor,
            "macro" => KwMacro,
            "unsafe" => KwUnsafe,
            "comptime" => KwComptime,
            "ghost" => KwGhost,
            "if" => KwIf,
            "else" => KwElse,
            "match" => KwMatch,
            "loop" => KwLoop,
            "while" => KwWhile,
            "for" => KwFor,
            "in" => KwIn,
            "return" => KwReturn,
            "break" => KwBreak,
            "continue" => KwContinue,
            "defer" => KwDefer,
            "goto" => KwGoto,
            "embed" => KwEmbed,
            "asm" => KwAsm,
            "syscall" => KwSyscall,
            "import" => KwImport,
            "export" => KwExport,
            "pub" => KwPub,
            "priv" => KwPriv,
            "module" => KwModule,
            "as" => KwAs,
            "is" => KwIs,
            "null" => KwNull,
            "true" => KwTrue,
            "false" => KwFalse,
            "self" => KwSelf,
            "super" => KwSuper,
            "_" => Underscore,
            _ => Identifier,
        };

        self.emit(Token::new(kind, Span::new(start, self.position)).with_value(value));
    }

    fn handle_number(&mut self, c: char, start: Position) -> Result<()> {
        let mut value = String::new();
        value.push(c);

        let mut is_float = false;

        // Check for hex, octal, binary
        if c == '0' {
            if let Some(&next) = self.peek() {
                match next {
                    'x' | 'X' => {
                        if let Some(ch) = self.next() {
                            value.push(ch);
                        } // consume 'x'
                        while let Some(&ch) = self.peek() {
                            if ch.is_ascii_hexdigit() || ch == '_' {
                                if let Some(ch) = self.next() {
                                    value.push(ch);
                                }
                            } else {
                                break;
                            }
                        }
                        self.emit(
                            Token::new(Integer, Span::new(start, self.position)).with_value(value),
                        );
                        return Ok(());
                    }
                    'o' | 'O' => {
                        if let Some(ch) = self.next() {
                            value.push(ch);
                        }
                        while let Some(&ch) = self.peek() {
                            if ('0' <= ch && ch <= '7') || ch == '_' {
                                if let Some(ch) = self.next() {
                                    value.push(ch);
                                }
                            } else {
                                break;
                            }
                        }
                        self.emit(
                            Token::new(Integer, Span::new(start, self.position)).with_value(value),
                        );
                        return Ok(());
                    }
                    'b' | 'B' => {
                        if let Some(ch) = self.next() {
                            value.push(ch);
                        }
                        while let Some(&ch) = self.peek() {
                            if ch == '0' || ch == '1' || ch == '_' {
                                if let Some(ch) = self.next() {
                                    value.push(ch);
                                }
                            } else {
                                break;
                            }
                        }
                        self.emit(
                            Token::new(Integer, Span::new(start, self.position)).with_value(value),
                        );
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        // Decimal number
        while let Some(&ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '_' {
                if let Some(ch) = self.next() {
                    value.push(ch);
                }
            } else if ch == '.' && !is_float {
                // Check if it's a float (e.g., 1.2) or range operator (1..2)
                is_float = true;
                if let Some(ch) = self.next() {
                    value.push(ch);
                }

                // Handle exponent
                if let Some(&exp) = self.peek() {
                    if exp == 'e' || exp == 'E' {
                        if let Some(ch) = self.next() {
                            value.push(ch);
                        }
                        if let Some(&sign) = self.peek() {
                            if sign == '+' || sign == '-' {
                                if let Some(ch) = self.next() {
                                    value.push(ch);
                                }
                            }
                        }
                        while let Some(&ch) = self.peek() {
                            if ch.is_ascii_digit() {
                                if let Some(ch) = self.next() {
                                    value.push(ch);
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }
            } else if ch == 'e' || ch == 'E' {
                // Exponent without decimal
                is_float = true;
                if let Some(ch) = self.next() {
                    value.push(ch);
                }
                if let Some(&sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        if let Some(ch) = self.next() {
                            value.push(ch);
                        }
                    }
                }
                while let Some(&ch) = self.peek() {
                    if ch.is_ascii_digit() {
                        if let Some(ch) = self.next() {
                            value.push(ch);
                        }
                    } else {
                        break;
                    }
                }
            } else {
                break;
            }
        }

        let kind = if is_float { Float } else { Integer };
        self.emit(Token::new(kind, Span::new(start, self.position)).with_value(value));
        Ok(())
    }

    fn handle_string(&mut self, start: Position) -> Result<()> {
        let mut value = String::new();

        while let Some(c) = self.next() {
            match c {
                '"' => {
                    // End of string
                    self.emit(
                        Token::new(StrLiteral, Span::new(start, self.position)).with_value(value),
                    );
                    return Ok(());
                }
                '\\' => {
                    // Escape sequence
                    if let Some(esc) = self.next() {
                        let ch = match esc {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '"' => '"',
                            '\'' => '\'',
                            '0' => '\0',
                            'x' => {
                                // Hex escape
                                let hi = self.next().and_then(|c| c.to_digit(16));
                                let lo = self.next().and_then(|c| c.to_digit(16));
                                match (hi, lo) {
                                    (Some(h), Some(l)) => {
                                        char::from_digit(h * 16 + l, 16).unwrap_or('\0')
                                    }
                                    _ => '\0',
                                }
                            }
                            c => c,
                        };
                        value.push(ch);
                    }
                }
                '\n' => {
                    bail!("Unterminated string literal");
                }
                c => {
                    value.push(c);
                }
            }
        }

        bail!("Unterminated string literal");
    }

    fn handle_char(&mut self, start: Position) -> Result<()> {
        let c = match self.next() {
            Some('\\') => {
                // Escape sequence
                match self.next() {
                    Some('n') => '\n',
                    Some('r') => '\r',
                    Some('t') => '\t',
                    Some('\\') => '\\',
                    Some('\'') => '\'',
                    Some('0') => '\0',
                    Some(c) => c,
                    None => bail!("Unterminated character literal"),
                }
            }
            Some(c) if c != '\'' => c,
            _ => bail!("Empty character literal"),
        };

        // Expect closing quote
        if self.next() != Some('\'') {
            bail!("Unterminated character literal");
        }

        self.emit(Token::new(Char, Span::new(start, self.position)).with_value(c.to_string()));
        Ok(())
    }

    fn handle_operator(&mut self, c: char, start: Position) -> Result<()> {
        // Multi-character operators
        match c {
            '+' => {
                if self.peek() == Some(&'=') {
                    self.next();
                    self.emit(Token::new(AddAssign, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(OpAdd, Span::new(start, self.position)));
                }
            }
            '-' => {
                if self.peek() == Some(&'=') {
                    self.next();
                    self.emit(Token::new(SubAssign, Span::new(start, self.position)));
                } else if self.peek() == Some(&'>') {
                    self.next();
                    self.emit(Token::new(OpArrow, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(OpSub, Span::new(start, self.position)));
                }
            }
            '*' => {
                if self.peek() == Some(&'=') {
                    self.next();
                    self.emit(Token::new(MulAssign, Span::new(start, self.position)));
                } else if self.peek() == Some(&'*') {
                    self.next();
                    self.emit(Token::new(OpPow, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(OpMul, Span::new(start, self.position)));
                }
            }
            '/' => {
                if self.peek() == Some(&'=') {
                    self.next();
                    self.emit(Token::new(DivAssign, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(OpDiv, Span::new(start, self.position)));
                }
            }
            '%' => {
                if self.peek() == Some(&'=') {
                    self.next();
                    self.emit(Token::new(RemAssign, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(OpRem, Span::new(start, self.position)));
                }
            }
            '<' => {
                if self.peek() == Some(&'<') {
                    self.next();
                    if self.peek() == Some(&'=') {
                        self.next();
                        self.emit(Token::new(ShlAssign, Span::new(start, self.position)));
                    } else {
                        self.emit(Token::new(OpShl, Span::new(start, self.position)));
                    }
                } else if self.peek() == Some(&'=') {
                    self.next();
                    self.emit(Token::new(OpLe, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(OpLt, Span::new(start, self.position)));
                }
            }
            '>' => {
                if self.peek() == Some(&'>') {
                    self.next();
                    if self.peek() == Some(&'=') {
                        self.next();
                        self.emit(Token::new(ShrAssign, Span::new(start, self.position)));
                    } else {
                        self.emit(Token::new(OpShr, Span::new(start, self.position)));
                    }
                } else if self.peek() == Some(&'=') {
                    self.next();
                    self.emit(Token::new(OpGe, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(OpGt, Span::new(start, self.position)));
                }
            }
            '=' => {
                if self.peek() == Some(&'=') {
                    self.next();
                    self.emit(Token::new(OpEq, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(Assign, Span::new(start, self.position)));
                }
            }
            '!' => {
                if self.peek() == Some(&'=') {
                    self.next();
                    self.emit(Token::new(OpNe, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(OpLogNot, Span::new(start, self.position)));
                }
            }
            '&' => {
                if self.peek() == Some(&'&') {
                    self.next();
                    self.emit(Token::new(OpLogAnd, Span::new(start, self.position)));
                } else if self.peek() == Some(&'=') {
                    self.next();
                    self.emit(Token::new(BitAndAssign, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(OpBitAnd, Span::new(start, self.position)));
                }
            }
            '|' => {
                if self.peek() == Some(&'|') {
                    self.next();
                    self.emit(Token::new(OpLogOr, Span::new(start, self.position)));
                } else if self.peek() == Some(&'=') {
                    self.next();
                    self.emit(Token::new(BitOrAssign, Span::new(start, self.position)));
                } else if self.peek() == Some(&'>') {
                    self.next();
                    self.emit(Token::new(OpPipeline, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(OpBitOr, Span::new(start, self.position)));
                }
            }
            '^' => {
                if self.peek() == Some(&'=') {
                    self.next();
                    self.emit(Token::new(BitXorAssign, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(OpBitXor, Span::new(start, self.position)));
                }
            }
            '.' => {
                if self.peek() == Some(&'.') {
                    self.next();
                    if self.peek() == Some(&'=') {
                        self.next();
                        self.emit(Token::new(OpRangeIncl, Span::new(start, self.position)));
                    } else {
                        self.emit(Token::new(OpRange, Span::new(start, self.position)));
                    }
                } else {
                    self.emit(Token::new(Dot, Span::new(start, self.position)));
                }
            }
            ':' => {
                if self.peek() == Some(&':') {
                    self.next();
                    self.emit(Token::new(OpScopeRes, Span::new(start, self.position)));
                } else {
                    self.emit(Token::new(Colon, Span::new(start, self.position)));
                }
            }
            '?' => {
                if self.peek() == Some(&'.') {
                    self.next();
                    self.emit(Token::new(OpSafeNav, Span::new(start, self.position)));
                } else if self.peek() == Some(&':') {
                    self.next();
                    self.emit(Token::new(OpElvis, Span::new(start, self.position)));
                } else {
                    // Could be part of pattern matching
                    self.emit(Token::new(Question, Span::new(start, self.position)));
                }
            }
            '@' => {
                self.emit(Token::new(OpAt, Span::new(start, self.position)));
            }
            '$' => {
                self.emit(Token::new(OpDollar, Span::new(start, self.position)));
            }
            _ => {
                bail!("Unexpected character: {}", c);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_keywords() {
        let source = "fn let var struct enum";
        let tokens = Lexer::new(source).lex().unwrap();

        assert_eq!(tokens[0].kind, KwFn);
        assert_eq!(tokens[1].kind, KwLet);
        assert_eq!(tokens[2].kind, KwVar);
        assert_eq!(tokens[3].kind, KwStruct);
        assert_eq!(tokens[4].kind, KwEnum);
    }

    #[test]
    fn test_lexer_numbers() {
        let source = "42 3.14 0xFF 0o77 0b1010";
        let tokens = Lexer::new(source).lex().unwrap();

        assert_eq!(tokens[0].kind, Integer);
        assert_eq!(tokens[1].kind, Float);
        assert_eq!(tokens[2].kind, Integer);
        assert_eq!(tokens[3].kind, Integer);
        assert_eq!(tokens[4].kind, Integer);
    }

    #[test]
    fn test_lexer_strings() {
        let source = "\"hello world\"";
        let tokens = Lexer::new(source).lex().unwrap();

        assert_eq!(tokens[0].kind, StrLiteral);
        assert_eq!(tokens[0].value, "hello world");
    }

    #[test]
    fn test_lexer_operators() {
        let source = "== != <= >= && || |> ..";
        let tokens = Lexer::new(source).lex().unwrap();

        assert_eq!(tokens[0].kind, OpEq);
        assert_eq!(tokens[1].kind, OpNe);
        assert_eq!(tokens[2].kind, OpLe);
        assert_eq!(tokens[3].kind, OpGe);
        assert_eq!(tokens[4].kind, OpLogAnd);
        assert_eq!(tokens[5].kind, OpLogOr);
        assert_eq!(tokens[6].kind, OpPipeline);
        assert_eq!(tokens[7].kind, OpRange);
    }
}
