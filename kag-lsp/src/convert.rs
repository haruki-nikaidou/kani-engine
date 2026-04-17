//! Conversions between Rowan byte offsets / `TextRange` and LSP
//! `Position` / `Range` / `Diagnostic` types.

use kag_syntax::error::{Severity, SyntaxDiagnostic};
use rowan::TextRange;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

// ─── Offset ↔ Position ────────────────────────────────────────────────────────

/// Convert a byte offset in `source` to an LSP `Position` (0-based line/col).
///
/// The column is reported as a UTF-16 code-unit count (LSP default).
pub fn offset_to_position(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let mut line = 0u32;
    let mut col_utf16 = 0u32;

    let mut prev_was_cr = false;
    for (idx, ch) in source.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\r' {
            line += 1;
            col_utf16 = 0;
            prev_was_cr = true;
        } else if ch == '\n' {
            // \r\n counts as one line break; bare \n counts on its own.
            if !prev_was_cr {
                line += 1;
                col_utf16 = 0;
            }
            prev_was_cr = false;
        } else {
            col_utf16 += ch.len_utf16() as u32;
            prev_was_cr = false;
        }
    }

    Position::new(line, col_utf16)
}

/// Convert an LSP `Position` to a byte offset in `source`.
///
/// Returns `source.len()` if the position is past the end.
pub fn position_to_offset(source: &str, pos: Position) -> usize {
    let mut line = 0u32;
    let mut col_utf16 = 0u32;
    let mut chars = source.char_indices().peekable();

    while let Some((idx, ch)) = chars.next() {
        if line == pos.line && col_utf16 == pos.character {
            return idx;
        }
        match ch {
            '\r' => {
                // Treat \r\n as a single line terminator so that a position
                // at (next_line, 0) resolves to the byte *after* the full pair.
                if chars.peek().map(|(_, c)| *c) == Some('\n') {
                    chars.next();
                }
                line += 1;
                col_utf16 = 0;
            }
            '\n' => {
                line += 1;
                col_utf16 = 0;
            }
            _ => {
                col_utf16 += ch.len_utf16() as u32;
            }
        }

        if line > pos.line {
            return idx;
        }
    }

    source.len()
}

// ─── TextRange → Range ────────────────────────────────────────────────────────

/// Convert a Rowan `TextRange` to an LSP `Range`.
pub fn text_range_to_lsp_range(source: &str, range: TextRange) -> Range {
    let start = offset_to_position(source, usize::from(range.start()));
    let end = offset_to_position(source, usize::from(range.end()));
    Range::new(start, end)
}

// ─── SyntaxDiagnostic → Diagnostic ──────────────────────────────────────────────
/// Convert a [`SyntaxDiagnostic`] to an LSP [`Diagnostic`].
pub fn parse_diagnostic_to_lsp(source: &str, diag: &SyntaxDiagnostic) -> Diagnostic {
    let start = offset_to_position(source, diag.span.offset());
    let end = offset_to_position(source, diag.span.offset() + diag.span.len());
    let range = Range::new(start, end);

    let severity = match diag.severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
    };

    Diagnostic {
        range,
        severity: Some(severity),
        message: diag.message.clone(),
        source: Some("kag-lsp".to_owned()),
        ..Default::default()
    }
}
