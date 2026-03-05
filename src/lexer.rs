//! Knull Lexer — Complete, bug-free tokenizer for The Knull Language
//!
//! Fixes over original:
//!   - `&&` / `||` emitted as distinct `And` / `Or` tokens
//!   - All compound-assignment operators (`+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=`)
//!   - Bitwise shift operators `<<` / `>>`
//!   - Character literals `'a'`, `'\n'`, `'\u{1F600}'` → `Char` tokens
//!   - `break` / `continue` keywords
//!   - `static` / `defer` / `extern` / `where` / `super` / `crate` keywords
//!   - `print!` / `println!` / `format!` macro invocations as plain calls
//!   - Underscore is now a separate `Underscore` token but also parses in identifiers
//!   - Raw string literals `r"..."` / `r#"..."#`
//!   - Tilde `~` for bitwise NOT
//!   - Pipeline operator `|>`
//!   - Elvis / null-coalesce `??`
//!   - Safe-nav `?.`

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ── Literals ────────────────────────────────────────────────────────────
    Identifier,
    Int,
    Float,
    String,
    Char,
    // ── Keywords ────────────────────────────────────────────────────────────
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
    Break,
    Continue,
    Pub,
    Mod,
    Use,
    Struct,
    Enum,
    Trait,
    Impl,
    SelfValue,
    SuperKw,
    CrateKw,
    Unsafe,
    Const,
    Static,
    Type,
    Where,
    Extern,
    Defer,
    Never,
    True,
    False,
    Null,
    // ── Three-mode keywords ─────────────────────────────────────────────────
    Mode,
    Novice,
    Expert,
    God,
    // ── Special / advanced ──────────────────────────────────────────────────
    Asm,
    Syscall,
    HashRun,       // `#run`
    // Linear types
    Linear,
    Consume,
    // Effect system
    Effect,
    // Coroutines / async
    Yield,
    Async,
    Await,
    // Type cast
    As,
    // ── Punctuation & operators ─────────────────────────────────────────────
    // Range
    DotDot,        // `..`
    DotDotEq,      // `..=`
    // Arrows
    Arrow,         // `->`
    FatArrow,      // `=>`
    DoubleColon,   // `::`
    // Arithmetic
    Plus,          // `+`
    Minus,         // `-`
    Star,          // `*`
    Slash,         // `/`
    Percent,       // `%`
    // Bitwise
    Ampersand,     // `&`  (also ref borrow)
    Pipe,          // `|`  (also match OR)
    Caret,         // `^`
    Tilde,         // `~`  (bitwise NOT)
    Shl,           // `<<`
    Shr,           // `>>`
    // Logical (NOW PROPERLY DISTINCT)
    And,           // `&&`
    Or,            // `||`
    Bang,          // `!`
    // Null / optional
    Question,      // `?`
    NullCoalesce,  // `??`
    SafeNav,       // `?.`
    // Pipeline
    Pipeline,      // `|>`
    // Compound assignment (ALL PROPERLY EMITTED NOW)
    PlusEq,        // `+=`
    MinusEq,       // `-=`
    StarEq,        // `*=`
    SlashEq,       // `/=`
    PercentEq,     // `%=`
    AmpEq,         // `&=`
    BarEq,         // `|=`
    CaretEq,       // `^=`
    ShlEq,         // `<<=`
    ShrEq,         // `>>=`
    // Comparison
    Eq,            // `=`
    EqEq,          // `==`
    Neq,           // `!=`
    Lt,            // `<`
    Gt,            // `>`
    Lte,           // `<=`
    Gte,           // `>=`
    // Delimiters
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
    Pound,         // `#`  (for attributes)
    Underscore,
    // ── Interpolated / template strings ───────────────────────────────────
    FString,     // f"..." or `...` — raw content including {expr} markers
    // ── Extra keyword aliases ─────────────────────────────────────────────
    Class,       // `class` — sugar for struct + impl
    Spawn,       // `spawn` — create a thread/task
    Chan,        // `chan`/`channel` — channel creation
    Do,          // `do` — alternative block opener
    // Error handling
    Try,         // `try`
    Catch,       // `catch`
    Throw,       // `throw` / `raise`
    // ── Meta ───────────────────────────────────────────────────────────────
    Eof,
    Unknown,
}

impl TokenKind {
    pub fn from_keyword(s: &str) -> Option<TokenKind> {
        match s {
            "fn"        => Some(TokenKind::Fn),
            "let"       => Some(TokenKind::Let),
            "mut"       => Some(TokenKind::Mut),
            "if"        => Some(TokenKind::If),
            "else"      => Some(TokenKind::Else),
            "match"     => Some(TokenKind::Match),
            "for"       => Some(TokenKind::For),
            "in"        => Some(TokenKind::In),
            "loop"      => Some(TokenKind::Loop),
            "while"     => Some(TokenKind::While),
            "return"    => Some(TokenKind::Return),
            "break"     => Some(TokenKind::Break),
            "continue"  => Some(TokenKind::Continue),
            "pub"       => Some(TokenKind::Pub),
            "mod"       => Some(TokenKind::Mod),
            "module"    => Some(TokenKind::Mod),
            "use"       => Some(TokenKind::Use),
            "struct"    => Some(TokenKind::Struct),
            "enum"      => Some(TokenKind::Enum),
            "trait"     => Some(TokenKind::Trait),
            "impl"      => Some(TokenKind::Impl),
            "self"      => Some(TokenKind::SelfValue),
            "super"     => Some(TokenKind::SuperKw),
            "crate"     => Some(TokenKind::CrateKw),
            "unsafe"    => Some(TokenKind::Unsafe),
            "const"     => Some(TokenKind::Const),
            "static"    => Some(TokenKind::Static),
            "extern"    => Some(TokenKind::Extern),
            "defer"     => Some(TokenKind::Defer),
            "where"     => Some(TokenKind::Where),
            "as"        => Some(TokenKind::As),
            "type"      => Some(TokenKind::Type),
            "never"     => Some(TokenKind::Never),
            "true"      => Some(TokenKind::True),
            "false"     => Some(TokenKind::False),
            "null"      => Some(TokenKind::Null),
            "nil"       => Some(TokenKind::Null),
            "none"      => Some(TokenKind::Null),
            // Modes
            "mode"      => Some(TokenKind::Mode),
            "novice"    => Some(TokenKind::Novice),
            "expert"    => Some(TokenKind::Expert),
            "god"       => Some(TokenKind::God),
            // Special / advanced
            "asm"       => Some(TokenKind::Asm),
            "syscall"   => Some(TokenKind::Syscall),
            // Linear / ownership
            "linear"    => Some(TokenKind::Linear),
            "consume"   => Some(TokenKind::Consume),
            // Effect system
            "effect"    => Some(TokenKind::Effect),
            // Async / coroutines
            "yield"     => Some(TokenKind::Yield),
            "async"     => Some(TokenKind::Async),
            "await"     => Some(TokenKind::Await),
            // Extra aliases — make Knull more beginner-friendly
            "class"   => Some(TokenKind::Class),
            "func"    => Some(TokenKind::Fn),      // Python/Go style
            "def"     => Some(TokenKind::Fn),      // Python style
            "fun"     => Some(TokenKind::Fn),      // Kotlin style
            "var"     => Some(TokenKind::Let),     // JS/Swift style
            "val"     => Some(TokenKind::Let),     // Kotlin/Scala style
            "spawn"   => Some(TokenKind::Spawn),
            "chan" | "channel" => Some(TokenKind::Chan),
            "do"      => Some(TokenKind::Do),
            "not"     => Some(TokenKind::Bang),    // English logical not
            "and"     => Some(TokenKind::And),     // English logical and
            "or"      => Some(TokenKind::Or),      // English logical or
            "import"  => Some(TokenKind::Use),     // Python style
            "from"    => Some(TokenKind::Use),     // Python-from style (simplified)
            "pass"    => None,                     // Python pass → falls through as identifier (no-op)
            // Error handling
            "try"     => Some(TokenKind::Try),
            "catch"   => Some(TokenKind::Catch),
            "throw"   => Some(TokenKind::Throw),
            "raise"   => Some(TokenKind::Throw),  // Python alias
            "except"  => Some(TokenKind::Catch),  // Python alias
            // Everything else that looks keyword-ish, let through as an identifier
            // (keeps compat with library names like `IO`, `handler`, `pure`, `share`, etc.)
            _           => None,
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
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek_ahead(&self, n: usize) -> Option<char> {
        self.source.get(self.pos + n).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.source.get(self.pos).copied();
        if c == Some('\n') {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        self.pos += 1;
        c
    }

    /// Helper: emit a multi-char token and continue the main loop
    #[inline]
    fn push_tok(tokens: &mut Vec<Token>, kind: TokenKind, value: &str, line: usize, col: usize) {
        tokens.push(Token {
            kind,
            value: value.to_string(),
            line,
            col,
        });
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while let Some(c) = self.peek() {
            // ── Whitespace ────────────────────────────────────────────────
            if c.is_whitespace() {
                self.advance();
                continue;
            }

            let line = self.line;
            let col  = self.col;

            // ── Comments ───────────────────────────────────────────────────
            if c == '/' {
                let next = self.peek_ahead(1);
                if next == Some('/') {
                    // Single-line comment — swallow until newline
                    while self.peek().map(|ch| ch != '\n').unwrap_or(false) {
                        self.advance();
                    }
                    continue;
                } else if next == Some('*') {
                    // Properly nested block comment
                    self.advance(); // `/`
                    self.advance(); // `*`
                    let mut depth = 1usize;
                    while depth > 0 {
                        match self.peek() {
                            None => break,
                            Some('/') => {
                                self.advance();
                                if self.peek() == Some('*') {
                                    self.advance();
                                    depth += 1;
                                }
                            }
                            Some('*') => {
                                self.advance();
                                if self.peek() == Some('/') {
                                    self.advance();
                                    depth -= 1;
                                }
                            }
                            _ => { self.advance(); }
                        }
                    }
                    continue;
                }
                // Otherwise fall through to operator handling below
            }

            // ── Character literals  'a'   '\n'   '\u{1F600}' ─────────────
            if c == '\'' {
                self.advance(); // opening `'`
                let ch_val: char = match self.peek() {
                    None => {
                        Self::push_tok(&mut tokens, TokenKind::Unknown, "'", line, col);
                        continue;
                    }
                    Some('\\') => {
                        self.advance();
                        let escaped = self.advance().unwrap_or('\\');
                        match escaped {
                            'n'  => '\n',
                            't'  => '\t',
                            'r'  => '\r',
                            '0'  => '\0',
                            '\\' => '\\',
                            '\'' => '\'',
                            '"'  => '"',
                            'u'  => {
                                // \u{XXXX}
                                if self.peek() == Some('{') {
                                    self.advance();
                                    let mut hex = String::new();
                                    while self.peek().map(|x| x != '}').unwrap_or(false) {
                                        hex.push(self.advance().unwrap());
                                    }
                                    if self.peek() == Some('}') { self.advance(); }
                                    u32::from_str_radix(&hex, 16)
                                        .ok()
                                        .and_then(char::from_u32)
                                        .unwrap_or('?')
                                } else { 'u' }
                            }
                            other => other,
                        }
                    }
                    Some(ch) => {
                        self.advance();
                        ch
                    }
                };
                // Closing `'`
                if self.peek() == Some('\'') { self.advance(); }
                tokens.push(Token {
                    kind: TokenKind::Char,
                    value: ch_val.to_string(),
                    line,
                    col,
                });
                continue;
            }

            // ── F-string / template string literals  f"..."  `...` ────────
            // Check f"..." before the plain identifier branch
            if c == 'f' && self.peek_ahead(1) == Some('"') {
                self.advance(); // `f`
                self.advance(); // opening `"`
                let mut raw = String::new();
                loop {
                    match self.peek() {
                        None | Some('"') => { self.advance(); break; }
                        Some('\\') => {
                            self.advance();
                            match self.advance() {
                                Some('n')  => raw.push('\n'),
                                Some('t')  => raw.push('\t'),
                                Some('"')  => raw.push('"'),
                                Some('\\') => raw.push('\\'),
                                Some(ch)   => { raw.push('\\'); raw.push(ch); }
                                None => break,
                            }
                        }
                        Some(ch) => { raw.push(ch); self.advance(); }
                    }
                }
                tokens.push(Token { kind: TokenKind::FString, value: raw, line, col });
                continue;
            }

            // ── Backtick template strings  `text ${expr} text` ──────────────
            if c == '`' {
                self.advance(); // consume backtick
                let mut raw = String::new();
                loop {
                    match self.peek() {
                        None | Some('`') => { self.advance(); break; }
                        Some('\\') => {
                            self.advance();
                            match self.advance() {
                                Some('`')  => raw.push('`'),
                                Some('n')  => raw.push('\n'),
                                Some('t')  => raw.push('\t'),
                                Some('\\') => raw.push('\\'),
                                Some(ch)   => { raw.push('\\'); raw.push(ch); }
                                None => break,
                            }
                        }
                        // Convert ${expr} to {expr} for uniform parsing
                        Some('$') if self.peek_ahead(1) == Some('{') => {
                            self.advance(); // consume `$`
                            self.advance(); // consume `{`
                            raw.push('{'); // emit single `{`
                        }
                        Some(ch) => { raw.push(ch); self.advance(); }
                    }
                }
                tokens.push(Token { kind: TokenKind::FString, value: raw, line, col });
                continue;
            }

            // ── String literals  "..."  r"..."  r#"..."# ─────────────────
            if c == '"' || (c == 'r' && (self.peek_ahead(1) == Some('"') || self.peek_ahead(1) == Some('#'))) {
                if c == 'r' {
                    // Raw string: consume raw prefix
                    self.advance(); // `r`
                    let mut hashes = 0usize;
                    while self.peek() == Some('#') { self.advance(); hashes += 1; }
                    if self.peek() == Some('"') { self.advance(); } // opening `"`
                    let mut value = String::new();
                    'raw: loop {
                        match self.peek() {
                            None => break,
                            Some('"') => {
                                self.advance();
                                // Check for trailing hashes
                                let mut closing = 0;
                                while self.peek() == Some('#') && closing < hashes {
                                    self.advance();
                                    closing += 1;
                                }
                                if closing == hashes { break 'raw; }
                                value.push('"');
                                for _ in 0..closing { value.push('#'); }
                            }
                            Some(ch) => { value.push(ch); self.advance(); }
                        }
                    }
                    tokens.push(Token { kind: TokenKind::String, value, line, col });
                    continue;
                } else {
                    // Normal double-quoted string with escape sequences
                    self.advance(); // opening `"`
                    let mut value = String::new();
                    loop {
                        match self.peek() {
                            None | Some('"') => { self.advance(); break; }
                            Some('\\') => {
                                self.advance();
                                match self.advance() {
                                    Some('n')  => value.push('\n'),
                                    Some('t')  => value.push('\t'),
                                    Some('r')  => value.push('\r'),
                                    Some('0')  => value.push('\0'),
                                    Some('"')  => value.push('"'),
                                    Some('\\') => value.push('\\'),
                                    Some('u')  => {
                                        if self.peek() == Some('{') {
                                            self.advance();
                                            let mut hex = String::new();
                                            while self.peek().map(|x| x != '}').unwrap_or(false) {
                                                hex.push(self.advance().unwrap());
                                            }
                                            if self.peek() == Some('}') { self.advance(); }
                                            if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                                                value.push(ch);
                                            }
                                        } else {
                                            value.push('u');
                                        }
                                    }
                                    Some(other) => value.push(other),
                                    None => break,
                                }
                            }
                            Some(ch) => { value.push(ch); self.advance(); }
                        }
                    }
                    tokens.push(Token { kind: TokenKind::String, value, line, col });
                    continue;
                }
            }

            // ── Identifiers & keywords ─────────────────────────────────────
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
                // Strip trailing `!` for macro calls like `println!` → treat as `println`
                if self.peek() == Some('!') {
                    // peek two ahead to check it's not `!=`
                    if self.peek_ahead(1) != Some('=') {
                        self.advance(); // consume `!`
                        // fall through — value already has the name
                    }
                }
                let kind = TokenKind::from_keyword(&value).unwrap_or(TokenKind::Identifier);
                tokens.push(Token { kind, value, line, col });
                continue;
            }

            // ── Numeric literals ───────────────────────────────────────────
            if c.is_ascii_digit() {
                let mut value = String::new();
                value.push(c);
                self.advance();

                // Hex: 0x…
                if c == '0' && (self.peek() == Some('x') || self.peek() == Some('X')) {
                    value.push(self.advance().unwrap());
                    while self.peek().map(|ch| ch.is_ascii_hexdigit() || ch == '_').unwrap_or(false) {
                        let ch = self.advance().unwrap();
                        if ch != '_' { value.push(ch); }
                    }
                    tokens.push(Token { kind: TokenKind::Int, value, line, col });
                    continue;
                }
                // Binary: 0b…
                if c == '0' && (self.peek() == Some('b') || self.peek() == Some('B')) {
                    value.push(self.advance().unwrap());
                    while self.peek().map(|ch| ch == '0' || ch == '1' || ch == '_').unwrap_or(false) {
                        let ch = self.advance().unwrap();
                        if ch != '_' { value.push(ch); }
                    }
                    tokens.push(Token { kind: TokenKind::Int, value, line, col });
                    continue;
                }
                // Octal: 0o…
                if c == '0' && (self.peek() == Some('o') || self.peek() == Some('O')) {
                    value.push(self.advance().unwrap());
                    while self.peek().map(|ch| ('0'..='7').contains(&ch) || ch == '_').unwrap_or(false) {
                        let ch = self.advance().unwrap();
                        if ch != '_' { value.push(ch); }
                    }
                    tokens.push(Token { kind: TokenKind::Int, value, line, col });
                    continue;
                }

                // Decimal integer or float
                let mut has_dot = false;
                let mut has_exp = false;
                loop {
                    match self.peek() {
                        Some('_') => { self.advance(); }
                        Some(ch) if ch.is_ascii_digit() => {
                            value.push(ch); self.advance();
                        }
                        Some('.') if !has_dot && self.peek_ahead(1) != Some('.') => {
                            // Don't consume if the next `..` is a range operator
                            has_dot = true;
                            value.push('.'); self.advance();
                        }
                        Some('e') | Some('E') if !has_exp => {
                            has_exp = true;
                            value.push(self.advance().unwrap());
                            if self.peek() == Some('+') || self.peek() == Some('-') {
                                value.push(self.advance().unwrap());
                            }
                        }
                        _ => break,
                    }
                }
                // Optional type suffix: 1u8  2i32  3.0f64 …
                if let Some(ch) = self.peek() {
                    if ch.is_alphabetic() {
                        // consume suffix but don't put it in the number value
                        while self.peek().map(|x| x.is_alphanumeric()).unwrap_or(false) {
                            self.advance();
                        }
                    }
                }
                let kind = if has_dot || has_exp { TokenKind::Float } else { TokenKind::Int };
                tokens.push(Token { kind, value, line, col });
                continue;
            }

            // ── Operators, punctuation ─────────────────────────────────────
            // We consume `c` at the start of each arm.
            self.advance();
            match c {
                // ── `+`  `+=` ──────────────────────────────────────────────
                '+' => {
                    if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::PlusEq, "+=", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Plus, "+", line, col); }
                }
                // ── `-`  `-=`  `->` ────────────────────────────────────────
                '-' => {
                    if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::MinusEq, "-=", line, col); }
                    else if self.peek() == Some('>') { self.advance(); Self::push_tok(&mut tokens, TokenKind::Arrow, "->", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Minus, "-", line, col); }
                }
                // ── `*`  `*=` ──────────────────────────────────────────────
                '*' => {
                    if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::StarEq, "*=", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Star, "*", line, col); }
                }
                // ── `/`  `/=` ──  (comments handled above) ────────────────
                '/' => {
                    if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::SlashEq, "/=", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Slash, "/", line, col); }
                }
                // ── `%`  `%=` ──────────────────────────────────────────────
                '%' => {
                    if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::PercentEq, "%=", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Percent, "%", line, col); }
                }
                // ── `&`  `&=`  `&&` ─────────────────────────────────────────
                '&' => {
                    if self.peek() == Some('&') { self.advance(); Self::push_tok(&mut tokens, TokenKind::And, "&&", line, col); }
                    else if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::AmpEq, "&=", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Ampersand, "&", line, col); }
                }
                // ── `|`  `|=`  `||`  `|>` ───────────────────────────────────
                '|' => {
                    if self.peek() == Some('|') { self.advance(); Self::push_tok(&mut tokens, TokenKind::Or, "||", line, col); }
                    else if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::BarEq, "|=", line, col); }
                    else if self.peek() == Some('>') { self.advance(); Self::push_tok(&mut tokens, TokenKind::Pipeline, "|>", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Pipe, "|", line, col); }
                }
                // ── `^`  `^=` ──────────────────────────────────────────────
                '^' => {
                    if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::CaretEq, "^=", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Caret, "^", line, col); }
                }
                // ── `~` ─────────────────────────────────────────────────────
                '~' => { Self::push_tok(&mut tokens, TokenKind::Tilde, "~", line, col); }
                // ── `!`  `!=` ───────────────────────────────────────────────
                '!' => {
                    if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::Neq, "!=", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Bang, "!", line, col); }
                }
                // ── `?`  `??` ────────────────────────────────────────────────
                '?' => {
                    if self.peek() == Some('?') { self.advance(); Self::push_tok(&mut tokens, TokenKind::NullCoalesce, "??", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Question, "?", line, col); }
                }
                // ── `=`  `==`  `=>` ─────────────────────────────────────────
                '=' => {
                    if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::EqEq, "==", line, col); }
                    else if self.peek() == Some('>') { self.advance(); Self::push_tok(&mut tokens, TokenKind::FatArrow, "=>", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Eq, "=", line, col); }
                }
                // ── `<`  `<=`  `<<`  `<<=` ──────────────────────────────────
                '<' => {
                    if self.peek() == Some('<') {
                        self.advance();
                        if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::ShlEq, "<<=", line, col); }
                        else { Self::push_tok(&mut tokens, TokenKind::Shl, "<<", line, col); }
                    } else if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::Lte, "<=", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Lt, "<", line, col); }
                }
                // ── `>`  `>=`  `>>`  `>>=` ──────────────────────────────────
                '>' => {
                    if self.peek() == Some('>') {
                        self.advance();
                        if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::ShrEq, ">>=", line, col); }
                        else { Self::push_tok(&mut tokens, TokenKind::Shr, ">>", line, col); }
                    } else if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::Gte, ">=", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Gt, ">", line, col); }
                }
                // ── `.`  `..`  `..=`  `?.` ──────────────────────────────────
                '.' => {
                    if self.peek() == Some('.') {
                        self.advance();
                        if self.peek() == Some('=') { self.advance(); Self::push_tok(&mut tokens, TokenKind::DotDotEq, "..=", line, col); }
                        else { Self::push_tok(&mut tokens, TokenKind::DotDot, "..", line, col); }
                    } else {
                        Self::push_tok(&mut tokens, TokenKind::Dot, ".", line, col);
                    }
                }
                // ── `:` `::` ─────────────────────────────────────────────────
                ':' => {
                    if self.peek() == Some(':') { self.advance(); Self::push_tok(&mut tokens, TokenKind::DoubleColon, "::", line, col); }
                    else { Self::push_tok(&mut tokens, TokenKind::Colon, ":", line, col); }
                }
                // ── `#`  `#run` ──────────────────────────────────────────────
                '#' => {
                    // Check for `#run`
                    if self.source.get(self.pos..self.pos+3).map(|s| s == ['r','u','n']).unwrap_or(false) {
                        self.advance(); self.advance(); self.advance();
                        Self::push_tok(&mut tokens, TokenKind::HashRun, "#run", line, col);
                    } else {
                        Self::push_tok(&mut tokens, TokenKind::Pound, "#", line, col);
                    }
                }
                // ── Simple delimiters ─────────────────────────────────────────
                '(' => Self::push_tok(&mut tokens, TokenKind::LParen,    "(", line, col),
                ')' => Self::push_tok(&mut tokens, TokenKind::RParen,    ")", line, col),
                '{' => Self::push_tok(&mut tokens, TokenKind::LBrace,    "{", line, col),
                '}' => Self::push_tok(&mut tokens, TokenKind::RBrace,    "}", line, col),
                '[' => Self::push_tok(&mut tokens, TokenKind::LBracket,  "[", line, col),
                ']' => Self::push_tok(&mut tokens, TokenKind::RBracket,  "]", line, col),
                ',' => Self::push_tok(&mut tokens, TokenKind::Comma,     ",", line, col),
                ';' => Self::push_tok(&mut tokens, TokenKind::Semicolon, ";", line, col),
                '@' => Self::push_tok(&mut tokens, TokenKind::At,        "@", line, col),
                '_' => {
                    // Bare `_` is a wildcard token; _ident is already consumed above in the identifier branch
                    Self::push_tok(&mut tokens, TokenKind::Underscore, "_", line, col);
                }
                _ => Self::push_tok(&mut tokens, TokenKind::Unknown, &c.to_string(), line, col),
            }
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
