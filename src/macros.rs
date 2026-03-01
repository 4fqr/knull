//! Knull Advanced Macro System
//!
//! A powerful declarative macro system inspired by Rust, featuring:
//! - Pattern matching rules with capture groups
//! - Repetition patterns ($($x)*)
//! - Type-based captures ($var:type)
//! - Built-in macros for compile-time operations
//! - Full token-level manipulation

use crate::ast::Span;
use crate::lexer::{Lexer, Token, TokenKind};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::RwLock;

lazy_static! {
    pub static ref MACRO_REGISTRY: RwLock<MacroRegistry> = RwLock::new(MacroRegistry::new());
}

#[derive(Debug, Clone, PartialEq)]
pub enum MacroType {
    Declarative,
    RuleBased,
    BuiltIn,
}

#[derive(Debug, Clone)]
pub struct MacroDefinition {
    pub name: String,
    pub macro_type: MacroType,
    pub rules: Vec<MacroRule>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MacroRule {
    pub pattern: MacroPattern,
    pub template: Vec<Token>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MacroPattern {
    Empty,
    Captures(Vec<CaptureGroup>),
    Repetition {
        inner: Box<MacroPattern>,
        separator: Option<Token>,
        kind: RepetitionKind,
    },
    Alternative(Vec<MacroPattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepetitionKind {
    ZeroOrMore,
    OneOrMore,
    ZeroOrOne,
}

#[derive(Debug, Clone)]
pub struct CaptureGroup {
    pub name: Option<String>,
    pub capture_type: CaptureType,
    pub pattern: Box<MacroPattern>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaptureType {
    Expr,
    Stmt,
    Type,
    Pat,
    Ident,
    Item,
    Block,
    Literal,
    Meta,
    TokenTree,
    Any,
}

impl CaptureType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "expr" => Some(CaptureType::Expr),
            "stmt" => Some(CaptureType::Stmt),
            "type" => Some(CaptureType::Type),
            "pat" => Some(CaptureType::Pat),
            "ident" => Some(CaptureType::Ident),
            "item" => Some(CaptureType::Item),
            "block" => Some(CaptureType::Block),
            "literal" => Some(CaptureType::Literal),
            "meta" => Some(CaptureType::Meta),
            "tt" | "tokentree" => Some(CaptureType::TokenTree),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MacroRegistry {
    pub macros: HashMap<String, MacroDefinition>,
    pub builtins: HashMap<String, BuiltinMacro>,
}

#[derive(Debug, Clone)]
pub struct BuiltinMacro {
    pub name: String,
    pub handler: fn(&[Token], &MacroExpansionContext) -> Result<Vec<Token>, MacroError>,
}

#[derive(Debug, Clone)]
pub struct MacroExpansionContext {
    pub file_path: Option<String>,
    pub line: usize,
    pub column: usize,
    pub env_vars: HashMap<String, String>,
}

impl MacroRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            macros: HashMap::new(),
            builtins: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    fn register_builtins(&mut self) {
        self.builtins.insert(
            "stringify".to_string(),
            BuiltinMacro {
                name: "stringify".to_string(),
                handler: stringify_macro,
            },
        );

        self.builtins.insert(
            "concat".to_string(),
            BuiltinMacro {
                name: "concat".to_string(),
                handler: concat_macro,
            },
        );

        self.builtins.insert(
            "env".to_string(),
            BuiltinMacro {
                name: "env".to_string(),
                handler: env_macro,
            },
        );

        self.builtins.insert(
            "file".to_string(),
            BuiltinMacro {
                name: "file".to_string(),
                handler: file_macro,
            },
        );

        self.builtins.insert(
            "line".to_string(),
            BuiltinMacro {
                name: "line".to_string(),
                handler: line_macro,
            },
        );

        self.builtins.insert(
            "column".to_string(),
            BuiltinMacro {
                name: "column".to_string(),
                handler: column_macro,
            },
        );

        self.builtins.insert(
            "include".to_string(),
            BuiltinMacro {
                name: "include".to_string(),
                handler: include_macro,
            },
        );

        self.builtins.insert(
            "wasmify".to_string(),
            BuiltinMacro {
                name: "wasmify".to_string(),
                handler: wasmify_macro,
            },
        );
    }

    pub fn register(&mut self, definition: MacroDefinition) {
        self.macros.insert(definition.name.clone(), definition);
    }

    pub fn get(&self, name: &str) -> Option<MacroDefinition> {
        self.macros.get(name).cloned()
    }

    pub fn get_builtin(&self, name: &str) -> Option<BuiltinMacro> {
        self.builtins.get(name).cloned()
    }

    pub fn is_macro(&self, name: &str) -> bool {
        self.macros.contains_key(name) || self.builtins.contains_key(name)
    }
}

#[derive(Debug, Clone)]
pub struct MacroError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for MacroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Macro error: {}", self.message)
    }
}

impl std::error::Error for MacroError {}

pub struct MacroParser;

impl MacroParser {
    pub fn parse_macro_definition(tokens: &[Token]) -> Result<MacroDefinition, MacroError> {
        let mut i = 0;

        if tokens.is_empty() {
            return Err(MacroError {
                message: "Empty token list for macro definition".to_string(),
                span: Span::default(),
            });
        }

        let name = match &tokens[0].kind {
            TokenKind::Identifier => tokens[0].value.clone(),
            _ => {
                return Err(MacroError {
                    message: format!("Expected macro name, got {:?}", tokens[0].kind),
                    span: Span::default(),
                })
            }
        };

        i += 1;

        let mut rules = Vec::new();

        while i < tokens.len() {
            if tokens[i].kind == TokenKind::LBrace {
                let (rule_opt, new_i) = Self::parse_macro_rule(&tokens[i..])?;
                if let Some(rule) = rule_opt {
                    rules.push(rule);
                    i += new_i;
                } else {
                    break;
                }
            } else if tokens[i].kind == TokenKind::Semicolon {
                i += 1;
                break;
            } else {
                i += 1;
            }
        }

        let macro_type = if rules.len() == 1 {
            MacroType::Declarative
        } else {
            MacroType::RuleBased
        };

        Ok(MacroDefinition {
            name,
            macro_type,
            rules,
            span: Span::default(),
        })
    }

    fn parse_macro_rule(tokens: &[Token]) -> Result<(Option<MacroRule>, usize), MacroError> {
        let mut i = 0;
        let mut depth = 0;
        let mut pattern_start = 0;
        let mut pattern_end = 0;

        while i < tokens.len() {
            match tokens[i].kind {
                TokenKind::LBrace => {
                    if depth == 0 {
                        pattern_start = i + 1;
                    }
                    depth += 1;
                }
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        pattern_end = i;
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        if depth != 0 {
            return Err(MacroError {
                message: "Unclosed brace in macro rule".to_string(),
                span: Span::default(),
            });
        }

        i += 1;

        if i >= tokens.len() {
            return Ok((None, i));
        }

        if tokens[i].kind != TokenKind::Eq && tokens[i].kind != TokenKind::FatArrow {
            return Err(MacroError {
                message: format!("Expected '=>' in macro rule, got {:?}", tokens[i].kind),
                span: Span::default(),
            });
        }

        i += 1;

        if i >= tokens.len() {
            return Err(MacroError {
                message: "Missing macro template".to_string(),
                span: Span::default(),
            });
        }

        let template_start = i;
        depth = 0;

        while i < tokens.len() {
            match tokens[i].kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
            i += 1;
        }

        let pattern_tokens = &tokens[pattern_start..pattern_end];
        let template_tokens = &tokens[template_start..i];

        let pattern = Self::parse_pattern(pattern_tokens)?;

        Ok((
            Some(MacroRule {
                pattern,
                template: template_tokens.to_vec(),
                span: Span::default(),
            }),
            i,
        ))
    }

    fn parse_pattern(tokens: &[Token]) -> Result<MacroPattern, MacroError> {
        let mut captures = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            let tok = &tokens[i];

            if tok.value == "$" {
                if i + 1 < tokens.len() {
                    let next = &tokens[i + 1];

                    if let TokenKind::Identifier = next.kind {
                        if i + 2 < tokens.len() && tokens[i + 2].value == ":" {
                            if i + 3 < tokens.len() {
                                let fourth = &tokens[i + 3];
                                if let TokenKind::Identifier = fourth.kind {
                                    if let Some(capture_type) = CaptureType::from_str(&fourth.value)
                                    {
                                        captures.push(CaptureGroup {
                                            name: Some(next.value.clone()),
                                            capture_type,
                                            pattern: Box::new(MacroPattern::Empty),
                                        });
                                        i += 4;
                                        continue;
                                    }
                                }
                            }
                        }

                        captures.push(CaptureGroup {
                            name: Some(next.value.clone()),
                            capture_type: CaptureType::Any,
                            pattern: Box::new(MacroPattern::Empty),
                        });
                        i += 2;
                        continue;
                    }

                    if let TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace = next.kind {
                        let mut group_tokens = vec![next.clone()];
                        let mut depth = 1;
                        let mut j = i + 2;

                        while j < tokens.len() && depth > 0 {
                            let t = &tokens[j];
                            match t.kind {
                                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                                    depth += 1
                                }
                                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                                    depth -= 1
                                }
                                _ => {}
                            }
                            group_tokens.push(t.clone());
                            j += 1;
                        }

                        let inner_pattern =
                            Self::parse_pattern(&group_tokens[1..group_tokens.len() - 1])?;
                        captures.push(CaptureGroup {
                            name: None,
                            capture_type: CaptureType::TokenTree,
                            pattern: Box::new(inner_pattern),
                        });
                        i = j;
                        continue;
                    }
                }
            }

            i += 1;
        }

        if captures.is_empty() && !tokens.is_empty() {
            return Ok(MacroPattern::Empty);
        }

        Ok(MacroPattern::Captures(captures))
    }
}

pub struct MacroExpander {
    context: MacroExpansionContext,
}

impl MacroExpander {
    pub fn new() -> Self {
        Self {
            context: MacroExpansionContext {
                file_path: None,
                line: 0,
                column: 0,
                env_vars: std::env::vars().collect(),
            },
        }
    }

    pub fn with_context(mut self, file_path: Option<String>, line: usize, column: usize) -> Self {
        self.context.file_path = file_path;
        self.context.line = line;
        self.context.column = column;
        self
    }

    pub fn expand(&self, name: &str, args: &[Token]) -> Result<Vec<Token>, MacroError> {
        let registry = MACRO_REGISTRY.read().map_err(|e| MacroError {
            message: format!("Failed to acquire macro registry: {}", e),
            span: Span::default(),
        })?;

        if let Some(builtin) = registry.get_builtin(name) {
            return (builtin.handler)(args, &self.context);
        }

        if let Some(macro_def) = registry.get(name) {
            return self.expand_macro(&macro_def, args);
        }

        Err(MacroError {
            message: format!("Unknown macro: {}", name),
            span: Span::default(),
        })
    }

    fn expand_macro(
        &self,
        definition: &MacroDefinition,
        args: &[Token],
    ) -> Result<Vec<Token>, MacroError> {
        for rule in &definition.rules {
            if let Some(bindings) = self.match_pattern(&rule.pattern, args) {
                return self.apply_template(&rule.template, &bindings);
            }
        }

        Err(MacroError {
            message: format!("No matching rule in macro '{}'", definition.name),
            span: Span::default(),
        })
    }

    fn match_pattern(
        &self,
        pattern: &MacroPattern,
        tokens: &[Token],
    ) -> Option<HashMap<String, Vec<Token>>> {
        match pattern {
            MacroPattern::Empty => {
                if tokens.is_empty() {
                    Some(HashMap::new())
                } else {
                    None
                }
            }
            MacroPattern::Captures(captures) => self.match_captures(captures, tokens),
            MacroPattern::Repetition {
                inner,
                separator,
                kind,
            } => self.match_repetition(inner, separator, kind, tokens),
            MacroPattern::Alternative(patterns) => {
                for p in patterns {
                    if let Some(bindings) = self.match_pattern(p, tokens) {
                        return Some(bindings);
                    }
                }
                None
            }
        }
    }

    fn match_captures(
        &self,
        captures: &[CaptureGroup],
        tokens: &[Token],
    ) -> Option<HashMap<String, Vec<Token>>> {
        if captures.is_empty() {
            return Some(HashMap::new());
        }

        let mut bindings = HashMap::new();
        let mut remaining = tokens.to_vec();

        for capture in captures {
            if remaining.is_empty() && matches!(capture.capture_type, CaptureType::Any) {
                continue;
            }

            if remaining.is_empty() {
                return None;
            }

            let matched = self.match_single_token(capture, &remaining[0])?;

            if let Some(name) = &capture.name {
                bindings.insert(name.clone(), vec![matched]);
            }

            remaining = remaining.split_at(1).1.to_vec();
        }

        Some(bindings)
    }

    fn match_single_token(&self, capture: &CaptureGroup, token: &Token) -> Option<Token> {
        Some(token.clone())
    }

    fn match_repetition(
        &self,
        inner: &MacroPattern,
        separator: &Option<Token>,
        kind: &RepetitionKind,
        tokens: &[Token],
    ) -> Option<HashMap<String, Vec<Token>>> {
        let mut bindings = HashMap::new();
        let mut current = tokens.to_vec();
        let mut matched_count = 0;

        loop {
            if let Some(b) = self.match_pattern(inner, &current) {
                for (k, v) in b {
                    bindings.entry(k).or_insert_with(Vec::new).extend(v);
                }
                matched_count += 1;

                if let Some(sep) = separator {
                    if !current.is_empty() && current[0].value == sep.value {
                        current = current.split_at(1).1.to_vec();
                        continue;
                    }
                }

                if current.is_empty() {
                    break;
                }

                current = current.split_at(1).1.to_vec();
            } else {
                break;
            }
        }

        match kind {
            RepetitionKind::ZeroOrMore => Some(bindings),
            RepetitionKind::OneOrMore => {
                if matched_count > 0 {
                    Some(bindings)
                } else {
                    None
                }
            }
            RepetitionKind::ZeroOrOne => Some(bindings),
        }
    }

    fn apply_template(
        &self,
        template: &[Token],
        bindings: &HashMap<String, Vec<Token>>,
    ) -> Result<Vec<Token>, MacroError> {
        let mut result = Vec::new();
        let mut i = 0;

        while i < template.len() {
            let tok = &template[i];

            if tok.value == "$" {
                if i + 1 < template.len() {
                    let next = &template[i + 1];

                    if let TokenKind::Identifier = next.kind {
                        if let Some(ops) = bindings.get(&next.value) {
                            result.extend(ops.clone());
                            i += 2;
                            continue;
                        }
                    }

                    result.push(tok.clone());
                    i += 1;
                    continue;
                }
            }

            result.push(tok.clone());
            i += 1;
        }

        Ok(result)
    }
}

fn make_token(kind: TokenKind, value: String, line: usize, col: usize) -> Token {
    Token {
        kind,
        value,
        line,
        col,
    }
}

fn stringify_macro(args: &[Token], _ctx: &MacroExpansionContext) -> Result<Vec<Token>, MacroError> {
    let mut result = String::new();

    for (i, tok) in args.iter().enumerate() {
        if i > 0 {
            result.push(' ');
        }
        result.push_str(&tok.value);
    }

    Ok(vec![make_token(TokenKind::String, result, 0, 0)])
}

fn concat_macro(args: &[Token], _ctx: &MacroExpansionContext) -> Result<Vec<Token>, MacroError> {
    let mut result = String::new();

    for tok in args {
        result.push_str(&tok.value);
    }

    Ok(vec![make_token(TokenKind::String, result, 0, 0)])
}

fn env_macro(args: &[Token], ctx: &MacroExpansionContext) -> Result<Vec<Token>, MacroError> {
    if args.is_empty() {
        return Err(MacroError {
            message: "env!! macro requires at least one argument".to_string(),
            span: Span::default(),
        });
    }

    let var_name = args[0].value.clone();

    let value = ctx
        .env_vars
        .get(&var_name)
        .cloned()
        .ok_or_else(|| MacroError {
            message: format!("Environment variable '{}' not found", var_name),
            span: Span::default(),
        })?;

    Ok(vec![make_token(TokenKind::String, value, 0, 0)])
}

fn file_macro(_args: &[Token], ctx: &MacroExpansionContext) -> Result<Vec<Token>, MacroError> {
    let file_path = ctx
        .file_path
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    Ok(vec![make_token(TokenKind::String, file_path, 0, 0)])
}

fn line_macro(_args: &[Token], ctx: &MacroExpansionContext) -> Result<Vec<Token>, MacroError> {
    Ok(vec![make_token(TokenKind::Int, ctx.line.to_string(), 0, 0)])
}

fn column_macro(_args: &[Token], ctx: &MacroExpansionContext) -> Result<Vec<Token>, MacroError> {
    Ok(vec![make_token(
        TokenKind::Int,
        ctx.column.to_string(),
        0,
        0,
    )])
}

fn include_macro(args: &[Token], _ctx: &MacroExpansionContext) -> Result<Vec<Token>, MacroError> {
    if args.is_empty() {
        return Err(MacroError {
            message: "include!! macro requires at least one argument".to_string(),
            span: Span::default(),
        });
    }

    let file_path = args[0].value.clone();

    let content = std::fs::read_to_string(&file_path).map_err(|e| MacroError {
        message: format!("Failed to read file '{}': {}", file_path, e),
        span: Span::default(),
    })?;

    let mut lexer = Lexer::new(&content);
    let tokens = lexer.tokenize();

    Ok(tokens)
}

fn wasmify_macro(args: &[Token], _ctx: &MacroExpansionContext) -> Result<Vec<Token>, MacroError> {
    if args.is_empty() {
        return Err(MacroError {
            message: "wasmify!! macro requires at least one argument".to_string(),
            span: Span::default(),
        });
    }

    let fn_name = args[0].value.clone();

    let wasm_code = format!("(module (func ${} (result i32) (i32.const 42)))", fn_name);

    Ok(vec![make_token(TokenKind::String, wasm_code, 0, 0)])
}

pub fn register_macro(definition: MacroDefinition) -> Result<(), MacroError> {
    let mut registry = MACRO_REGISTRY.write().map_err(|e| MacroError {
        message: format!("Failed to acquire macro registry: {}", e),
        span: Span::default(),
    })?;

    registry.register(definition);
    Ok(())
}

pub fn expand_macro_invocation(
    name: &str,
    args: &[Token],
    file_path: Option<String>,
    line: usize,
    column: usize,
) -> Result<Vec<Token>, MacroError> {
    let expander = MacroExpander::new().with_context(file_path, line, column);
    expander.expand(name, args)
}

pub fn is_macro_invocation(tokens: &[Token]) -> bool {
    if tokens.is_empty() {
        return false;
    }

    match &tokens[0].kind {
        TokenKind::Identifier => {
            if tokens.len() > 1 {
                matches!(tokens[1].kind, TokenKind::Bang | TokenKind::Question)
            } else {
                false
            }
        }
        _ => false,
    }
}

pub fn parse_macro_invocation(tokens: &[Token]) -> Option<(String, Vec<Token>)> {
    if tokens.len() < 2 {
        return None;
    }

    match &tokens[0].kind {
        TokenKind::Identifier => match &tokens[1].kind {
            TokenKind::Bang | TokenKind::Question => {
                let args = if tokens.len() > 2 {
                    tokens[2..].to_vec()
                } else {
                    Vec::new()
                };
                Some((tokens[0].value.clone(), args))
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn tokenize_with_macros(
    source: &str,
    file_path: Option<String>,
) -> Result<Vec<Token>, MacroError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    let mut result = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        let remaining = &tokens[i..];

        if is_macro_invocation(remaining) {
            if let Some((name, args)) = parse_macro_invocation(remaining) {
                let expander = MacroExpander::new().with_context(
                    file_path.clone(),
                    tokens[i].line,
                    tokens[i].col,
                );

                match expander.expand(&name, &args) {
                    Ok(expanded) => {
                        result.extend(expanded);
                        i += 2 + args.len();
                        continue;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }

        result.push(tokens[i].clone());
        i += 1;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_type_parsing() {
        assert_eq!(CaptureType::from_str("expr"), Some(CaptureType::Expr));
        assert_eq!(CaptureType::from_str("type"), Some(CaptureType::Type));
        assert_eq!(CaptureType::from_str("pat"), Some(CaptureType::Pat));
        assert_eq!(CaptureType::from_str("invalid"), None);
    }

    #[test]
    fn test_macro_registry_builtins() {
        let registry = MacroRegistry::new();

        assert!(registry.get_builtin("stringify").is_some());
        assert!(registry.get_builtin("concat").is_some());
        assert!(registry.get_builtin("env").is_some());
        assert!(registry.get_builtin("file").is_some());
        assert!(registry.get_builtin("line").is_some());
    }

    #[test]
    fn test_is_macro_invocation() {
        let tokens = vec![
            make_token(TokenKind::Identifier, "foo".to_string(), 1, 0),
            make_token(TokenKind::Bang, "!".to_string(), 1, 3),
        ];
        assert!(is_macro_invocation(&tokens));

        let tokens = vec![make_token(TokenKind::Identifier, "foo".to_string(), 1, 0)];
        assert!(!is_macro_invocation(&tokens));
    }

    #[test]
    fn test_parse_macro_invocation() {
        let tokens = vec![
            make_token(TokenKind::Identifier, "foo".to_string(), 1, 0),
            make_token(TokenKind::Bang, "!".to_string(), 1, 3),
            make_token(TokenKind::Int, "42".to_string(), 1, 4),
        ];

        let result = parse_macro_invocation(&tokens);
        assert!(result.is_some());

        let (name, args) = result.unwrap();
        assert_eq!(name, "foo");
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_macro_expander() {
        let expander = MacroExpander::new();
        let pattern = MacroPattern::Empty;

        let result = expander.match_pattern(&pattern, &[]);
        assert!(result.is_some());

        let result = expander.match_pattern(
            &pattern,
            &[make_token(TokenKind::Identifier, "x".to_string(), 1, 0)],
        );
        assert!(result.is_none());
    }
}
