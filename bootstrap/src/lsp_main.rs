//! Knull Language Server Protocol (LSP) Implementation
//! 
//! This module provides IDE integration through the Language Server Protocol.

use anyhow::Result;
use tower_lsp::jsonrpc::Result as JsonResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Printer};
use std::sync::Arc;

/// LSP Main Entry Point
fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    log::info!("Starting Knull Language Server...");

    // Create stdin/stdout reader
    let (service, socket) = LspService::new(|client| {
        KnullLspServer {
            client: Arc::new(client),
            documents: std::sync::Mutex::new(DocumentStore::new()),
        }
    });

    Printer::new(std::io::stdin(), std::io::stdout(), socket).join();

    Ok(())
}

/// Knull Language Server
struct KnullLspServer {
    client: Arc<Client>,
    documents: std::sync::Mutex<DocumentStore>,
}

impl KnullLspServer {
    fn uri_to_path(&self, uri: &Url) -> std::path::PathBuf {
        uri.to_file_path().unwrap_or_default()
    }

    fn get_document(&self, uri: &Url) -> Option<String> {
        self.documents.lock().unwrap().get(uri).cloned()
    }

    fn update_document(&self, uri: Url, text: String) {
        self.documents.lock().unwrap().insert(uri, text);
    }
}

/// Document Store
struct DocumentStore {
    documents: std::collections::HashMap<Url, String>,
}

impl DocumentStore {
    fn new() -> Self {
        DocumentStore {
            documents: std::collections::HashMap::new(),
        }
    }

    fn get(&self, uri: &Url) -> Option<String> {
        self.documents.get(uri).cloned()
    }

    fn insert(&mut self, uri: Url, text: String) {
        self.documents.insert(uri, text);
    }

    fn remove(&mut self, uri: &Url) {
        self.documents.remove(uri);
    }
}

/// Language Server Implementation
#[tower_lsp::async_trait]
impl LanguageServer for KnullLspServer {
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
                type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(
                    true,
                )),
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

    async fn initialized(&self, params: InitializedParams) {
        log::info!("Server initialized");
    }

    async fn shutdown(&self) -> JsonResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.update_document(params.text_document.uri, params.text_document.text);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.update_document(
            params.text_document.uri,
            params.content_changes.into_iter().last().unwrap().text,
        );
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.lock().unwrap().remove(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> JsonResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        
        if let Some(source) = self.get_document(&uri) {
            let position = params.text_document_position_params.position;
            
            // Simple hover - just show the token at this position
            let range = Range::new(position, position);
            
            return Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "**Knull** - The God Language\n\nHover for more info".to_string(),
                }),
                range: Some(range),
            }));
        }
        
        Ok(None)
    }

    async fn definition(&self, params: DefinitionParams) -> JsonResult<Option<Location>> {
        let uri = params.text_document_position_params.text_document.uri;
        
        if let Some(source) = self.get_document(&uri) {
            let position = params.text_document_position_params.position;
            
            // Simple definition - return the same location for now
            return Ok(Some(Location {
                uri,
                range: Range::new(position, position),
            }));
        }
        
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> JsonResult<Option<Vec<Location>>> {
        let uri = params.text_document_position_params.text_document.uri;
        
        if let Some(source) = self.get_document(&uri) {
            let position = params.text_document_position_params.position;
            
            // Return empty references for now
            return Ok(Some(Vec::new()));
        }
        
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> JsonResult<Option<CompletionList>> {
        let uri = params.text_document_position.text_document.uri;
        
        // Provide keyword completions
        let items = vec![
            CompletionItem::new_simple("fn".to_string(), "Function declaration".to_string()),
            CompletionItem::new_simple("let".to_string(), "Immutable binding".to_string()),
            CompletionItem::new_simple("var".to_string(), "Mutable binding".to_string()),
            CompletionItem::new_simple("struct".to_string(), "Struct definition".to_string()),
            CompletionItem::new_simple("enum".to_string(), "Enum definition".to_string()),
            CompletionItem::new_simple("if".to_string(), "Conditional".to_string()),
            CompletionItem::new_simple("else".to_string(), "Else branch".to_string()),
            CompletionItem::new_simple("match".to_string(), "Pattern matching".to_string()),
            CompletionItem::new_simple("loop".to_string(), "Infinite loop".to_string()),
            CompletionItem::new_simple("while".to_string(), "While loop".to_string()),
            CompletionItem::new_string_item("for".to_string(), CompletionItemLabel::Simple("for".to_string())),
            CompletionItem::new_simple("return".to_string(), "Return value".to_string()),
            CompletionItem::new_simple("unsafe".to_string(), "Unsafe block".to_string()),
            CompletionItem::new_simple("import".to_string(), "Import module".to_string()),
            CompletionItem::new_simple("pub".to_string(), "Public visibility".to_string()),
            CompletionItem::new_simple("async".to_string(), "Async function".to_string()),
            CompletionItem::new_simple("await".to_string(), "Await expression".to_string()),
        ];

        Ok(Some(CompletionList {
            is_incomplete: false,
            items,
        }))
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> JsonResult<Option<SignatureHelp>> {
        // Return empty signature help for now
        Ok(Some(SignatureHelp {
            signatures: vec![],
            active_signature: None,
            active_parameter: None,
        }))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> JsonResult<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        
        if let Some(source) = self.get_document(&uri) {
            // Simple formatting - return the same text
            let text = source.clone();
            
            return Ok(Some(vec![TextEdit {
                range: Range::new(
                    Position::new(0, 0),
                    Position::new(usize::MAX, usize::MAX),
                ),
                new_text: text,
            }]));
        }
        
        Ok(None)
    }

    async fn range_formatting(&self, params: DocumentRangeFormattingParams) -> JsonResult<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        
        if let Some(source) = self.get_document(&uri) {
            // Return the same text
            return Ok(Some(vec![TextEdit {
                range: params.range,
                new_text: source.clone(),
            }]));
        }
        
        Ok(None)
    }

    async fn publish_diagnostics(&self, params: PublishDiagnosticsParams) {
        // Handle diagnostics - log them for now
        log::info!("Diagnostics for {}: {:?}", params.uri, params.diagnostics);
    }
}
