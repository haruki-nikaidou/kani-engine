use logos::{Logos, Span};
use miette::SourceSpan;

/// All terminal tokens produced by the KAG lexer.
///
/// `logos` drives the scan; each variant's regex/token annotation
/// corresponds to a KAG syntactic element.
#[derive(Logos, Debug, Clone, PartialEq)]
pub enum Token<'src> {
    // ── Line structure ────────────────────────────────────────────────────

    /// A Unix or Windows newline.
    #[token("\n")]
    #[token("\r\n")]
    Newline,

    /// A line-comment starting with `;` or `//` (consumed to end of line).
    #[regex(r";[^\n]*", allow_greedy = true)]
    #[regex(r"//[^\n]*", allow_greedy = true)]
    LineComment,

    /// The literal `/*` or `*/` on its own line (block comment delimiters).
    #[token("/*")]
    BlockCommentOpen,

    #[token("*/")]
    BlockCommentClose,

    // ── Line-type sigils ─────────────────────────────────────────────────

    /// `@` starts a line-level tag.
    #[token("@")]
    At,

    /// `#` starts a character-name shorthand.
    #[token("#")]
    Hash,

    /// `*` starts a label definition (or is the macro-splat inside a tag).
    #[token("*")]
    Star,

    // ── Tag delimiters ───────────────────────────────────────────────────

    /// `[` opens an inline tag.
    #[token("[")]
    LBracket,

    /// `]` closes an inline tag.
    #[token("]")]
    RBracket,

    // ── Parameter syntax ─────────────────────────────────────────────────

    /// `=` separates a parameter key from its value.
    #[token("=")]
    Eq,

    /// `&` signals a runtime-evaluated entity expression (`&f.counter`).
    #[token("&")]
    Amp,

    /// `%` signals a macro parameter reference (`%name` or `%name|default`).
    #[token("%")]
    Percent,

    /// `|` separates the label name from its title in `*label|title`, or
    /// separates a macro-param key from its default (`%key|default`).
    #[token("|")]
    Pipe,

    /// `:` separates the character name from a face name in `#name:face`.
    #[token(":")]
    Colon,

    // ── Literals ─────────────────────────────────────────────────────────

    /// A double-quoted string value (includes the quotes).
    #[regex(r#""(?:[^"\\]|\\.)*""#)]
    DoubleQuoted(&'src str),

    /// A single-quoted string value (includes the quotes).
    #[regex(r"'(?:[^'\\]|\\.)*'")]
    SingleQuoted(&'src str),

    /// An identifier: starts with a letter or `_`, followed by alphanumerics,
    /// `_`, `-`, or `.`.  Used for tag names, parameter keys, and bare values.
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_\-\.]*", priority = 3)]
    Ident(&'src str),

    /// A numeric literal (integer or float).
    #[regex(r"-?[0-9]+(?:\.[0-9]+)?", priority = 3)]
    Number(&'src str),

    /// A backslash escape (e.g. `\[` to print a literal `[`).
    #[token("\\")]
    Backslash,

    /// Any other non-whitespace, non-special character sequence that forms
    /// plain text content on a text line (punctuation, CJK, etc.).
    #[regex(r#"[^\s@#*\[\]=&%|:;"'\\/<>\n]+"#)]
    Text(&'src str),

    /// Horizontal whitespace (spaces and tabs) — preserved so text content
    /// retains its spacing.  Tag-parameter parsers skip these explicitly.
    #[regex(r"[ \t]+")]
    Whitespace,

    /// A forward-slash that is not part of `//` or `*/`.
    #[token("/")]
    Slash,

    /// A `<` character (used in HTML/entity contexts).
    #[token("<")]
    Lt,

    /// A `>` character.
    #[token(">")]
    Gt,
}

/// A token together with its byte-offset span in the source.
#[derive(Debug, Clone)]
pub struct Spanned<'src> {
    pub token: Token<'src>,
    pub span: SourceSpan,
    /// The raw source slice for this token.
    pub slice: &'src str,
}

/// Tokenize a KAG script source string into a flat `Vec<Spanned>`.
///
/// Lex errors are collected as `Token`-level errors by `logos` but we
/// surface them via `KagError` at a higher layer; here we simply skip any
/// unrecognised bytes and record them as `None` entries in the raw iterator
/// (logos returns `None` for skipped/error tokens), which the parser will
/// reject with a proper diagnostic.
///
/// Returns the spanned token list on success, along with the byte offsets of
/// any bytes that logos could not classify (lex errors).
pub fn tokenize(source: &str) -> (Vec<Spanned<'_>>, Vec<Span>) {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    let mut lexer = Token::lexer(source);
    while let Some(result) = lexer.next() {
        let span = lexer.span();
        match result {
            Ok(token) => {
                tokens.push(Spanned {
                    token,
                    span: SourceSpan::new(span.start.into(), span.len()),
                    slice: lexer.slice(),
                });
            }
            Err(()) => {
                errors.push(span);
            }
        }
    }

    (tokens, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok_types(src: &str) -> Vec<Token<'_>> {
        let (tokens, _) = tokenize(src);
        tokens.into_iter().map(|s| s.token).collect()
    }

    fn tok_slices(src: &str) -> Vec<&str> {
        let (tokens, _) = tokenize(src);
        tokens.into_iter().map(|s| s.slice).collect()
    }

    #[test]
    fn test_line_comment_semicolon() {
        let (tokens, errs) = tokenize("; this is a comment\n");
        assert!(errs.is_empty());
        assert_eq!(tokens[0].token, Token::LineComment);
        assert_eq!(tokens[1].token, Token::Newline);
    }

    #[test]
    fn test_line_comment_double_slash() {
        let (tokens, errs) = tokenize("// another comment\n");
        assert!(errs.is_empty());
        assert_eq!(tokens[0].token, Token::LineComment);
    }

    #[test]
    fn test_block_comment_delimiters() {
        let (tokens, errs) = tokenize("/*\n*/\n");
        assert!(errs.is_empty());
        assert_eq!(tokens[0].token, Token::BlockCommentOpen);
        assert_eq!(tokens[2].token, Token::BlockCommentClose);
    }

    #[test]
    fn test_label_sigils() {
        let types = tok_types("*scene_start|Opening\n");
        assert_eq!(types[0], Token::Star);
        assert!(matches!(types[1], Token::Ident("scene_start")));
        assert_eq!(types[2], Token::Pipe);
        assert!(matches!(types[3], Token::Ident("Opening")));
        assert_eq!(types[4], Token::Newline);
    }

    #[test]
    fn test_at_tag_line() {
        let (tokens, _) = tokenize("@jump storage=main target=*start\n");
        // Filter out whitespace to get the meaningful tokens
        let meaningful: Vec<&str> = tokens
            .iter()
            .filter(|t| !matches!(t.token, Token::Whitespace))
            .map(|t| t.slice)
            .collect();
        assert_eq!(meaningful[0], "@");
        assert_eq!(meaningful[1], "jump");
        assert_eq!(meaningful[2], "storage");
        assert_eq!(meaningful[3], "=");
        assert_eq!(meaningful[4], "main");
        assert_eq!(meaningful[5], "target");
        assert_eq!(meaningful[6], "=");
        assert_eq!(meaningful[7], "*");
        assert_eq!(meaningful[8], "start");
    }

    #[test]
    fn test_inline_tag() {
        let types = tok_types("[r]");
        assert_eq!(types[0], Token::LBracket);
        assert!(matches!(types[1], Token::Ident("r")));
        assert_eq!(types[2], Token::RBracket);
    }

    #[test]
    fn test_quoted_string_parameter() {
        let (tokens, _) = tokenize(r#"[bg storage="forest.png"]"#);
        let quoted = tokens.iter().find(|t| matches!(t.token, Token::DoubleQuoted(_)));
        assert!(quoted.is_some());
        assert_eq!(quoted.unwrap().slice, r#""forest.png""#);
    }

    #[test]
    fn test_entity_amp() {
        let types = tok_types("[eval exp=&f.counter]");
        assert!(types.contains(&Token::Amp));
    }

    #[test]
    fn test_macro_param_percent() {
        let types = tok_types("[text val=%message|hello]");
        assert!(types.contains(&Token::Percent));
        assert!(types.contains(&Token::Pipe));
    }

    #[test]
    fn test_hash_chara_shorthand() {
        let types = tok_types("#Alice:happy\n");
        assert_eq!(types[0], Token::Hash);
        assert!(matches!(types[1], Token::Ident("Alice")));
        assert_eq!(types[2], Token::Colon);
        assert!(matches!(types[3], Token::Ident("happy")));
    }

    #[test]
    fn test_number_token() {
        let slices = tok_slices("500");
        assert_eq!(slices[0], "500");
    }

    #[test]
    fn test_span_accuracy() {
        let src = "@r\n";
        let (tokens, _) = tokenize(src);
        // "@" at offset 0, "r" at offset 1
        assert_eq!(tokens[0].span.offset(), 0);
        assert_eq!(tokens[1].span.offset(), 1);
    }

    #[test]
    fn test_plain_text_content() {
        let (tokens, _) = tokenize("Hello, world!\n");
        let text_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.token, Token::Text(_)))
            .collect();
        // Should capture text slices (comma and space cause splits)
        assert!(!text_tokens.is_empty());
    }
}
