//! Knull Language Server Protocol (LSP) Implementation
//! 
//! This module provides IDE integration through the Language Server Protocol.

use anyhow::Result;
use tower_lsp::jsonrpc::Result as JsonResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Printer};
use std::sync::Arc;

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::compiler::Compiler;

pub struct KnullLanguageServer {
    client: Arc<Client>,
    documents: std::sync::Mutex<DocumentStore>,
}

pub struct DocumentStore {
    documents: std::collections::HashMap<Url, String>,
}

impl DocumentStore {
    pub fn new() -> Self {
        DocumentStore {
            documents: std::collections::HashMap::new(),
        }
    }

    pub fn get(&self, uri: &Url) -> Option<String> {
        self.documents.get(uri).cloned()
    }

    pub fn insert(&mut self, uri: Url, text: String) {
        self.documents.insert(uri, text);
    }

    pub fn remove(&mut self, uri: &Url) {
        self.documents.remove(uri);
    }
}

impl KnullLanguageServer {
    pub fn new(client: Client) -> Self {
        KnullLanguageServer {
            client: Arc::new(client),
            documents: std::sync::Mutex::new(DocumentStore::new()),
        }
    }

    fn compile_document(&self, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Lex
        match Lexer::new(source).lex() {
            Ok(tokens) => {
                // Parse
                match Parser::new(tokens).parse() {
                    Ok(ast) => {
                        // Type check
                        let mut compiler = Compiler::new();
                        if let Err(e) = compiler.compile(source) {
                            diagnostics.push(Diagnostic {
                                range: Range::default(),
                                severity: Some(DiagnosticSeverity::ERROR),
                                message: e.to_string(),
                                source: Some("knull".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                    Err(e) => {
                        diagnostics.push(Diagnostic {
                            range: Range::default(),
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!("Parse error: {}", e),
                            source: Some("knull".to_string()),
                            ..Default::default()
                        });
                    }
                }
            }
            Err(e) => {
                diagnostics.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("Lex error: {}", e),
                    source: Some("knull".to_string()),
                    ..Default::default()
                });
            }
        }

        diagnostics
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for KnullLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> JsonResult<InitializeResult> {
        log::info!("Client initialized: {:?}", params.client_info);

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "knull".to_string(),
                version: Some("1.0.0".to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
                references_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        ":".to_string(),
                        " ".to_string(),
                    ]),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    all_commit_characters: None,
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: Some(vec![",".to_string()]),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        log::info!("Knull LSP server initialized");
    }

    async fn shutdown(&self) -> JsonResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.documents.lock().unwrap().insert(
            params.text_document.uri,
            params.text_document.text,
        );
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.documents.lock().unwrap().insert(
            params.text_document.uri,
            params.content_changes.into_iter().last().unwrap().text,
        );
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.lock().unwrap().remove(&params.text_document.uri);
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        log::info!("Workspace folders changed: {:?}", params.event.added);
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        log::info!("Configuration changed: {:?}", params.settings);
    }

    async fn hover(&self, params: HoverParams) -> JsonResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        
        if let Some(source) = self.documents.lock().unwrap().get(&uri) {
            let position = params.text_document_position_params.position;
            
            // Simple hover - show keyword info
            let word = extract_word_at_position(source, position);
            
            let content = match word.as_str() {
                "fn" => "**fn** - Function declaration\n\nDeclares a new function.",
                "let" => "**let** - Immutable binding\n\nDeclares a variable that cannot be reassigned.",
                "var" => "**var** - Mutable binding\n\nDeclares a variable that can be reassigned.",
                "struct" => "**struct** - Structure definition\n\nDefines a new struct type.",
                "enum" => "**enum** - Enumeration definition\n\nDefines a new enum type.",
                "if" => "**if** - Conditional\n\nConditional branching.",
                "else" => "**else** - Alternative branch\n\nAlternative branch for if statements.",
                "match" => "**match** - Pattern matching\n\nPattern matching with exhaustiveness checking.",
                "loop" => "**loop** - Infinite loop\n\nInfinite loop that must be broken explicitly.",
                "while" => "**while** - Conditional loop\n\nLoop that continues while condition is true.",
                "for" => "**for** - Iterator loop\n\nLoop over an iterator or range.",
                "return" => "**return** - Return value\n\nReturn from a function.",
                "unsafe" => "**unsafe** - Unsafe block\n\nDisables safety checks for low-level operations.",
                "comptime" => "**comptime** - Compile-time execution\n\nExecutes code at compile time.",
                "import" => "**import** - Import module\n\nImports items from another module.",
                "pub" => "**pub** - Public visibility\n\nMakes an item publicly accessible.",
                "i32" => "**i32** - 32-bit signed integer\n\nSigned 32-bit integer type.",
                "i64" => "**i64** - 64-bit signed integer\n\nSigned 64-bit integer type.",
                "u32" => "**u32** - 32-bit unsigned integer\n\nUnsigned 32-bit integer type.",
                "u64" => "**u64** - 64-bit unsigned integer\n\nUnsigned 64-bit integer type.",
                "f64" => "**f64** - 64-bit floating point\n\nDouble-precision floating point type.",
                "bool" => "**bool** - Boolean\n\nBoolean type (true or false).",
                "String" => "**String** - String type\n\nDynamically allocated UTF-8 string.",
                "Vec" => "**Vec<T>** - Vector type\n\nDynamically growing array.",
                _ => "**Knull** - The God Language\n\nHover for more info",
            };

            let range = Range::new(position, Position::new(position.line, position.character + word.len() as u32));
            
            return Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: content.to_string(),
                }),
                range: Some(range),
            }));
        }
        
        Ok(None)
    }

    async fn definition(&self, params: DefinitionParams) -> JsonResult<Option<Location>> {
        let uri = params.text_document_position_params.text_document.uri;
        
        if let Some(source) = self.documents.lock().unwrap().get(&uri) {
            let position = params.text_document_position_params.position;
            
            // Simple definition - return the same location
            return Ok(Some(Location {
                uri,
                range: Range::new(position, Position::new(position.line, position.character + 1)),
            }));
        }
        
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> JsonResult<Option<Vec<Location>>> {
        let uri = params.text_document_position_params.text_document.uri;
        
        if let Some(_source) = self.documents.lock().unwrap().get(&uri) {
            // Return empty references for now
            return Ok(Some(Vec::new()));
        }
        
        Ok(None)
    }

    async fn completion(&self, _params: CompletionParams) -> JsonResult<Option<CompletionList>> {
        let items = vec![
            CompletionItem::new_simple("fn".to_string(), "Function declaration".to_string()),
            CompletionItem::new_simple("let".to_string(), "Immutable binding".to_string()),
            CompletionItem::new_simple("var".to_string(), "Mutable binding".to_string()),
            CompletionItem::new_simple("own".to_string(), "Ownership transfer".to_string()),
            CompletionItem::new_simple("mut".to_string(), "Mutable reference".to_string()),
            CompletionItem::new_simple("ref".to_string(), "Borrowed reference".to_string()),
            CompletionItem::new_simple("const".to_string(), "Constant".to_string()),
            CompletionItem::new_simple("static".to_string(), "Static variable".to_string()),
            CompletionItem::new_simple("struct".to_string(), "Struct definition".to_string()),
            CompletionItem::new_simple("enum".to_string(), "Enum definition".to_string()),
            CompletionItem::new_simple("union".to_string(), "Union definition".to_string()),
            CompletionItem::new_simple("trait".to_string(), "Trait definition".to_string()),
            CompletionItem::new_simple("impl".to_string(), "Implementation block".to_string()),
            CompletionItem::new_simple("type".to_string(), "Type alias".to_string()),
            CompletionItem::new_simple("if".to_string(), "Conditional".to_string()),
            CompletionItem::new_simple("else".to_string(), "Else branch".to_string()),
            CompletionItem::new_simple("match".to_string(), "Pattern matching".to_string()),
            CompletionItem::new_simple("loop".to_string(), "Infinite loop".to_string()),
            CompletionItem::new_simple("while".to_string(), "While loop".to_string()),
            CompletionItem::new_simple("for".to_string(), "For loop".to_string()),
            CompletionItem::new_simple("in".to_string(), "Iterator binding".to_string()),
            CompletionItem::new_simple("return".to_string(), "Return value".to_string()),
            CompletionItem::new_simple("break".to_string(), "Break loop".to_string()),
            CompletionItem::new_simple("continue".to_string(), "Continue loop".to_string()),
            CompletionItem::new_simple("defer".to_string(), "Deferred cleanup".to_string()),
            CompletionItem::new_simple("unsafe".to_string(), "Unsafe block".to_string()),
            CompletionItem::new_simple("comptime".to_string(), "Compile-time block".to_string()),
            CompletionItem::new_simple("import".to_string(), "Import module".to_string()),
            CompletionItem::new_simple("pub".to_string(), "Public visibility".to_string()),
            CompletionItem::new_simple("i8".to_string(), "8-bit signed integer".to_string()),
            CompletionItem::new_simple("i16".to_string(), "16-bit signed integer".to_string()),
            CompletionItem::new_simple("i32".to_string(), "32-bit signed integer".to_string()),
            CompletionItem::new_simple("i64".to_string(), "64-bit signed integer".to_string()),
            CompletionItem::new_simple("u8".to_string(), "8-bit unsigned integer".to_string()),
            CompletionItem::new_simple("u16".to_string(), "16-bit unsigned integer".to_string()),
            CompletionItem::new_simple("u32".to_string(), "32-bit unsigned integer".to_string()),
            CompletionItem::new_simple("u64".to_string(), "64-bit unsigned integer".to_string()),
            CompletionItem::new_simple("f32".to_string(), "32-bit float".to_string()),
            CompletionItem::new_simple("f64".to_string(), "64-bit float".to_string()),
            CompletionItem::new_simple("bool".to_string(), "Boolean".to_string()),
            CompletionItem::new_simple("char".to_string(), "Character".to_string()),
            CompletionItem::new_simple("String".to_string(), "String type".to_string()),
            CompletionItem::new_simple("Vec".to_string(), "Vector type".to_string()),
            CompletionItem::new_simple("HashMap".to_string(), "Hash map type".to_string()),
            CompletionItem::new_simple("Option".to_string(), "Option type".to_string()),
            CompletionItem::new_simple("Result".to_string(), "Result type".to_string()),
        ];

        Ok(Some(CompletionList {
            is_incomplete: false,
            items,
        }))
    }

    async fn signature_help(&self, _params: SignatureHelpParams) -> JsonResult<Option<SignatureHelp>> {
        Ok(Some(SignatureHelp {
            signatures: vec![],
            active_signature: None,
            active_parameter: None,
        }))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> JsonResult<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        
        if let Some(source) = self.documents.lock().unwrap().get(&uri) {
            let formatted = simple_format(source);
            
            return Ok(Some(vec![TextEdit {
                range: Range::new(
                    Position::new(0, 0),
                    Position::new(usize::MAX, usize::MAX),
                ),
                new_text: formatted,
            }]));
        }
        
        Ok(None)
    }

    async fn range_formatting(&self, params: DocumentRangeFormattingParams) -> JsonResult<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        
        if let Some(source) = self.documents.lock().unwrap().get(&uri) {
            return Ok(Some(vec![TextEdit {
                range: params.range,
                new_text: source.clone(),
            }]));
        }
        
        Ok(None)
    }

    async fn publish_diagnostics(&self, params: PublishDiagnosticsParams) {
        log::info!("Diagnostics for {}: {:?}", params.uri, params.diagnostics);
    }
}

fn extract_word_at_position(source: &str, position: Position) -> String {
    let mut start = 0;
    let mut end = 0;
    let mut current_pos = 0;
    let mut in_word = false;
    
    for (i, c) in source.char_indices() {
        if current_pos == position.line as usize {
            if c.is_alphanumeric() || c == '_' {
                if !in_word {
                    start = i;
                    in_word = true;
                }
                end = i + c.len_utf8();
            } else if in_word {
                break;
            }
            
            if current_pos == position.line as usize && i >= position.character as usize {
                break;
            }
        }
        
        if c == '\n' {
            current_pos += 1;
        }
    }
    
    source[start..end].to_string()
}

fn simple_format(source: &str) -> String {
    let mut result = String::new();
    let mut indent = 0;
    let mut in_string = false;
    
    for c in source.chars() {
        if c == '"' && !in_string {
            in_string = true;
        } else if c == '"' && in_string {
            in_string = false;
        }
        
        if !in_string {
            if c == '{' {
                result.push(c);
                result.push('\n');
                indent += 1;
                for _ in 0..indent {
                    result.push_str("    ");
                }
                continue;
            }
            if c == '}' {
                indent = indent.saturating_sub(1);
                result.push('\n');
                for _ in 0..indent {
                    result.push_str("    ");
                }
                result.push(c);
                continue;
            }
            if c == '\n' {
                result.push(c);
                for _ in 0..indent {
                    result.push_str("    ");
                }
                continue;
            }
        }
        
        result.push(c);
    }
    
    result
}
