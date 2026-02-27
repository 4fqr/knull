//! Knull Language Server
//! 
//! This module provides LSP integration for Knull IDE support.

pub mod server {
    use tower_lsp::jsonrpc::Result as JsonResult;
    use tower_lsp::lsp_types::*;
    use tower_lsp::{Client, LanguageServer, LspService, Printer};
    use std::sync::Arc;

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
    }

    #[tower_lsp::async_trait]
    impl LanguageServer for KnullLanguageServer {
        async fn initialize(&self, params: InitializeParams) -> JsonResult<InitializeResult> {
            log::info!("Initializing Knull LSP server");

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
                        trigger_characters: Some(vec![".".to_string(), ":".to_string(), " ".to_string()]),
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

        async fn hover(&self, params: HoverParams) -> JsonResult<Option<Hover>> {
            let uri = params.text_document_position_params.text_document.uri;
            
            if let Some(source) = self.documents.lock().unwrap().get(&uri) {
                let position = params.text_document_position_params.position;
                let range = Range::new(position, position);
                
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: "**Knull** - The God Language\n\nHover info".to_string(),
                    }),
                    range: Some(range),
                }));
            }
            
            Ok(None)
        }

        async fn completion(&self, _params: CompletionParams) -> JsonResult<Option<CompletionList>> {
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
                CompletionItem::new_simple("for".to_string(), "For loop".to_string()),
                CompletionItem::new_simple("return".to_string(), "Return value".to_string()),
                CompletionItem::new_simple("unsafe".to_string(), "Unsafe block".to_string()),
                CompletionItem::new_simple("import".to_string(), "Import module".to_string()),
                CompletionItem::new_simple("pub".to_string(), "Public visibility".to_string()),
                CompletionItem::new_simple("comptime".to_string(), "Compile-time block".to_string()),
                CompletionItem::new_simple("defer".to_string(), "Deferred cleanup".to_string()),
                CompletionItem::new_simple("async".to_string(), "Async function".to_string()),
                CompletionItem::new_simple("await".to_string(), "Await expression".to_string()),
            ];

            Ok(Some(CompletionList {
                is_incomplete: false,
                items,
            }))
        }

        async fn formatting(&self, params: DocumentFormattingParams) -> JsonResult<Option<Vec<TextEdit>>> {
            let uri = params.text_document.uri;
            
            if let Some(source) = self.documents.lock().unwrap().get(&uri) {
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
    }
}
