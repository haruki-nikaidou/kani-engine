//! Document store: maps open file URLs to their parsed representation.

use dashmap::DashMap;
use kag_syntax::cst::Root;
use kag_syntax::parser::Parse;
use kag_syntax::parse_cst;
use tower_lsp::lsp_types::Url;

use crate::analysis::Index;

// ─── ParsedDoc ────────────────────────────────────────────────────────────────

/// Everything we keep per open document.
pub struct ParsedDoc {
    /// The full source text (owned).
    pub source: String,
    /// The lossless Rowan CST result (tree + parse errors).
    pub parse: Parse<Root>,
    /// Pre-built semantic index (labels, macros, tag references).
    pub index: Index,
}

impl ParsedDoc {
    /// Parse `source` and build the index.
    pub fn new(source: String, uri: &Url) -> Self {
        let source_name = uri.path();
        let parse = parse_cst(&source, source_name);
        let index = Index::build(&parse, &source);
        Self { source, parse, index }
    }
}

// ─── DocumentStore ────────────────────────────────────────────────────────────

/// Concurrent map from open document URI to its [`ParsedDoc`].
#[derive(Default)]
pub struct DocumentStore {
    docs: DashMap<Url, ParsedDoc>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a document, returning the new [`ParsedDoc`].
    pub fn update(&self, uri: Url, source: String) {
        let doc = ParsedDoc::new(source, &uri);
        self.docs.insert(uri, doc);
    }

    /// Remove a document (called on `didClose`).
    pub fn remove(&self, uri: &Url) {
        self.docs.remove(uri);
    }

    /// Run `f` with a shared reference to the document, returning `None` if
    /// the document is not open.
    pub fn with<F, R>(&self, uri: &Url, f: F) -> Option<R>
    where
        F: FnOnce(&ParsedDoc) -> R,
    {
        self.docs.get(uri).map(|entry| f(&entry))
    }
}
