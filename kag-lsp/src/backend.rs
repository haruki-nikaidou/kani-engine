//! Tower-LSP `LanguageServer` implementation.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, Diagnostic,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse,
    Hover, HoverParams, HoverProviderCapability, InitializeParams, InitializeResult,
    InitializedParams, Location, MessageType, OneOf, ReferenceParams, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use tower_lsp::{Client, LanguageServer};

use crate::analysis::{completion, goto_def, hover, references, symbols};
use crate::convert::{parse_diagnostic_to_lsp, position_to_offset};
use crate::store::DocumentStore;

// ─── Backend ─────────────────────────────────────────────────────────────────

pub struct Backend {
    client: Client,
    store: DocumentStore,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            store: DocumentStore::new(),
        }
    }

    /// Re-parse a document and publish its diagnostics.
    async fn on_change(&self, uri: Url, text: String) {
        self.store.update(uri.clone(), text);

        let diagnostics: Vec<Diagnostic> = self
            .store
            .with(&uri, |doc| {
                doc.parse
                    .errors()
                    .iter()
                    .map(|e| parse_diagnostic_to_lsp(&doc.source, e))
                    .collect()
            })
            .unwrap_or_default();

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

// ─── LanguageServer impl ─────────────────────────────────────────────────────

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "kag-lsp".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["@".into(), "[".into(), "=".into()]),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "kag-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    // ── Document lifecycle ───────────────────────────────────────────────────

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // We use FULL sync, so there is exactly one content change.
        if let Some(change) = params.content_changes.into_iter().next() {
            self.on_change(params.text_document.uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.store.remove(&params.text_document.uri);
        // Clear diagnostics for the closed file.
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    // ── Hover ────────────────────────────────────────────────────────────────

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        Ok(self
            .store
            .with(uri, |doc| {
                let offset = position_to_offset(&doc.source, pos);
                hover::hover(doc, offset)
            })
            .flatten())
    }

    // ── Completion ───────────────────────────────────────────────────────────

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let items = self
            .store
            .with(uri, |doc| {
                let offset = position_to_offset(&doc.source, pos);
                let (in_value, key) = param_context(&doc.source, offset);
                completion::completions(doc, in_value, key.as_deref())
            })
            .unwrap_or_default();

        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }

    // ── Go-to-definition ─────────────────────────────────────────────────────

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        Ok(self
            .store
            .with(uri, |doc| {
                let offset = position_to_offset(&doc.source, pos);
                goto_def::goto_definition(doc, uri, offset).map(GotoDefinitionResponse::Scalar)
            })
            .flatten())
    }

    // ── Find references ──────────────────────────────────────────────────────

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let include_decl = params.context.include_declaration;

        Ok(self
            .store
            .with(uri, |doc| {
                let offset = position_to_offset(&doc.source, pos);
                let locs = references::find_references(doc, uri, offset, include_decl);
                if locs.is_empty() { None } else { Some(locs) }
            })
            .flatten())
    }

    // ── Document symbols ─────────────────────────────────────────────────────

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;

        Ok(self
            .store
            .with(uri, |doc| {
                let syms = symbols::document_symbols(doc);
                if syms.is_empty() {
                    None
                } else {
                    Some(DocumentSymbolResponse::Nested(syms))
                }
            })
            .flatten())
    }
}

// ─── Completion context heuristic ────────────────────────────────────────────

/// Scan backwards from `offset` in `source` to determine whether we are inside
/// a parameter value position, and if so, what the key name is.
///
/// Returns `(in_value, key)`.
fn param_context(source: &str, offset: usize) -> (bool, Option<String>) {
    let prefix = &source[..offset.min(source.len())];

    // Find the start of the current line, handling \n, \r\n, and bare \r.
    let line_start = prefix.rfind(['\n', '\r']).map(|i| i + 1).unwrap_or(0);
    // For \r\n the rfind lands on the \n; back up one more if a \r precedes it.
    let line = &prefix[line_start..];

    // If there's an `=` after the last word boundary, we're in a value.
    if let Some(eq_pos) = line.rfind('=') {
        let after_eq = &line[..eq_pos];
        // The key is the identifier immediately before the `=`.
        let key = after_eq
            .trim_end()
            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .next_back()
            .filter(|s| !s.is_empty())
            .map(String::from);
        return (true, key);
    }

    (false, None)
}
