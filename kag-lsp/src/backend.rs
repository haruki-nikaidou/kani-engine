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
                let (in_value, key, tag) = param_context(&doc.source, offset);
                completion::completions(doc, in_value, key.as_deref(), tag.as_deref())
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
///
/// The search is bounded to the innermost open tag so that `=` signs in
/// earlier closed tags or in surrounding plain text are never mistaken for the
/// current parameter assignment.
fn param_context(source: &str, offset: usize) -> (bool, Option<String>, Option<String>) {
    let prefix = &source[..offset.min(source.len())];

    // Find the start of the current line, handling \n, \r\n, and bare \r.
    let line_start = prefix.rfind(['\n', '\r']).map(|i| i + 1).unwrap_or(0);
    let line = &prefix[line_start..];

    // Determine the slice that covers the current tag only.
    //
    // Inline tag `[tag param=value`:
    //   Find the last `[` on the line prefix.  If there is a `]` between that
    //   `[` and the cursor the tag is already closed, so we are in plain text.
    //
    // Line-level tag `@tag param=value`:
    //   The `@` must be the first meaningful token on the line.  There are no
    //   closing brackets, so the whole line prefix is the tag range.
    //
    // Anything else: the cursor is in plain text — return no value context.
    let tag_slice: &str = if let Some(bracket_pos) = line.rfind('[') {
        // A `]` after the last `[` means the tag is closed; we are outside it.
        if line[bracket_pos..].contains(']') {
            return (false, None, None);
        }
        &line[bracket_pos..]
    } else if line.trim_start().starts_with('@') {
        // At-tag: the entire line prefix is the tag content.
        line
    } else {
        return (false, None, None);
    };

    // Extract the tag name — the first identifier after `[` or `@`.
    let tag_name: Option<String> = {
        let s = tag_slice.trim_start_matches(['[', '@']).trim_start();
        let name = s
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .filter(|s| !s.is_empty());
        name.map(String::from)
    };

    // Within the bounded tag slice, look for the last `=`.
    if let Some(eq_pos) = tag_slice.rfind('=') {
        let before_eq = &tag_slice[..eq_pos];
        // The key is the identifier immediately before the `=`.
        let key = before_eq
            .trim_end()
            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .next_back()
            .filter(|s| !s.is_empty())
            .map(String::from);
        return (true, key, tag_name);
    }

    (false, None, tag_name)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::param_context;

    /// Strip the tag-name field for backward-compat test assertions.
    fn ctx(src: &str) -> (bool, Option<String>) {
        let (in_val, key, _tag) = param_context(src, src.len());
        (in_val, key)
    }

    fn ctx_tag(src: &str) -> Option<String> {
        param_context(src, src.len()).2
    }

    // ── Inside an open inline tag ────────────────────────────────────────────

    #[test]
    fn inline_tag_value_simple() {
        // Cursor right after `=`; should report the key.
        assert_eq!(ctx("[jump target="), (true, Some("target".into())));
    }

    #[test]
    fn inline_tag_value_partial() {
        // Cursor mid-value; still in value context.
        assert_eq!(ctx("[jump target=*lab"), (true, Some("target".into())));
    }

    #[test]
    fn inline_tag_no_eq() {
        // Cursor after the tag name, no `=` yet — not in a value.
        assert_eq!(ctx("[jump "), (false, None));
    }

    #[test]
    fn inline_tag_name_extracted() {
        assert_eq!(ctx_tag("[jump target="), Some("jump".into()));
        assert_eq!(ctx_tag("[bgm "), Some("bgm".into()));
        assert_eq!(ctx_tag("@bg storage="), Some("bg".into()));
        assert_eq!(ctx_tag("[jump target=*a]"), None); // closed tag
    }

    #[test]
    fn inline_tag_second_param() {
        // Second parameter on the same tag; only that `=` should matter.
        assert_eq!(
            ctx("[jump target=*a storage="),
            (true, Some("storage".into()))
        );
    }

    // ── Closed tag followed by plain text ────────────────────────────────────

    #[test]
    fn after_closed_tag_plain_text() {
        // The tag is closed; cursor is in plain text — must NOT report in_value.
        assert_eq!(ctx("[jump target=*a] "), (false, None));
    }

    #[test]
    fn after_closed_tag_no_trailing_space() {
        assert_eq!(ctx("[jump target=*a]"), (false, None));
    }

    // ── Multiple tags on the same line ───────────────────────────────────────

    #[test]
    fn second_open_tag_after_closed_first() {
        // First tag closed, second tag open; key must come from second tag.
        assert_eq!(
            ctx("[jump target=*a] [call storage="),
            (true, Some("storage".into()))
        );
    }

    #[test]
    fn eq_in_plain_text_before_open_tag() {
        // An `=` in the plain-text region must not bleed into the open tag.
        assert_eq!(ctx("x=1 [jump "), (false, None));
    }

    // ── Line-level @-tag ─────────────────────────────────────────────────────

    #[test]
    fn at_tag_value() {
        assert_eq!(ctx("@jump target="), (true, Some("target".into())));
    }

    #[test]
    fn at_tag_no_eq() {
        assert_eq!(ctx("@jump "), (false, None));
    }

    #[test]
    fn at_tag_with_leading_whitespace() {
        // KAG allows leading whitespace before `@`.
        assert_eq!(ctx("  @jump target="), (true, Some("target".into())));
    }

    // ── Multi-line source: only the current line is considered ───────────────

    #[test]
    fn previous_line_eq_does_not_bleed() {
        // An `=` on a previous line must not affect the current line.
        let src = "[jump target=*a]\n[call ";
        assert_eq!(ctx(src), (false, None));
    }

    #[test]
    fn previous_line_eq_current_line_has_value() {
        let src = "[jump target=*a]\n[call storage=";
        assert_eq!(ctx(src), (true, Some("storage".into())));
    }
}
