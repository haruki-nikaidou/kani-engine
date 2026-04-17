//! Inline XML markup parser for KAG message text.
//!
//! KAG message text may contain XML-style tags to apply character-level
//! formatting:
//!
//! ```text
//! This is <b>bold</b> and <i>italic</i>.
//! <color value="#ff0000">red text</color>
//! <ruby rt="かんじ">漢字</ruby>
//! <size value="24">large</size>
//! <shadow>shadowed</shadow>
//! <outline>outlined</outline>
//! <nowrap>no wrap</nowrap>
//! ```
//!
//! The parser produces a flat list of [`TextSpan`] values, each carrying a
//! plain text fragment and accumulated style attributes.  The concatenation of
//! all span texts equals the input with all XML tags removed.

use crate::events::{TextSpan, TextStyle};

// ─── Parser ───────────────────────────────────────────────────────────────────

/// Parse an inline-markup string into a list of styled text spans.
///
/// Unknown tags are silently ignored (their content is still emitted).
/// Mismatched or unclosed tags do not produce errors — the parser is lenient.
pub fn parse_inline_markup(input: &str) -> Vec<TextSpan> {
    let mut spans: Vec<TextSpan> = Vec::new();
    let mut style_stack: Vec<TextStyle> = vec![TextStyle::default()];
    let mut pending_ruby: Option<String> = None;

    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'<' {
            // Find closing `>`
            if let Some(rel) = memchr(b'>', &bytes[i + 1..]) {
                let tag_content = &input[i + 1..i + 1 + rel];
                let end = i + 1 + rel + 1;

                if let Some(stripped) = tag_content.strip_prefix('/') {
                    // Closing tag
                    let tag_name = stripped.trim();
                    handle_closing_tag(tag_name, &mut style_stack, &mut pending_ruby);
                } else {
                    // Opening (or self-closing) tag
                    let (tag_name, attrs) = split_tag_name(tag_content);
                    let new_style = apply_opening_tag(
                        tag_name,
                        attrs,
                        style_stack.last().unwrap_or(&TextStyle::default()),
                    );
                    if tag_name == "ruby" {
                        pending_ruby = parse_attr(tag_content, "rt");
                    }
                    style_stack.push(new_style);
                }

                i = end;
            } else {
                // No closing `>` found — treat `<` as literal text
                push_char(
                    &mut spans,
                    style_stack.last().unwrap_or(&TextStyle::default()),
                    '<',
                );
                i += 1;
            }
        } else {
            // Collect a run of non-`<` characters
            let start = i;
            while i < len && bytes[i] != b'<' {
                i += 1;
            }
            let text = &input[start..i];
            if !text.is_empty() {
                let default_style = TextStyle::default();
                let current_style = style_stack.last().unwrap_or(&default_style);
                let mut style = current_style.clone();
                if let Some(ruby) = &pending_ruby {
                    style.ruby = Some(ruby.clone());
                }
                push_str(&mut spans, &style, text);
            }
        }
    }

    if spans.is_empty() {
        spans.push(TextSpan {
            text: input.to_owned(),
            style: TextStyle::default(),
        });
    }

    merge_adjacent(&mut spans);
    spans
}

/// Extract the plain text from a list of spans (strip all markup).
pub fn spans_to_plain(spans: &[TextSpan]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    haystack.iter().position(|&b| b == needle)
}

fn push_char(spans: &mut Vec<TextSpan>, style: &TextStyle, ch: char) {
    let mut s = String::with_capacity(ch.len_utf8());
    s.push(ch);
    push_str(spans, style, &s);
}

fn push_str(spans: &mut Vec<TextSpan>, style: &TextStyle, text: &str) {
    if let Some(last) = spans.last_mut()
        && &last.style == style {
            last.text.push_str(text);
            return;
        }
    spans.push(TextSpan {
        text: text.to_owned(),
        style: style.clone(),
    });
}

fn merge_adjacent(spans: &mut Vec<TextSpan>) {
    let mut i = 1;
    while i < spans.len() {
        if spans[i].style == spans[i - 1].style {
            let text = spans.remove(i).text;
            spans[i - 1].text.push_str(&text);
        } else {
            i += 1;
        }
    }
}

fn split_tag_name(tag_content: &str) -> (&str, &str) {
    let tag_content = tag_content.trim_end_matches('/').trim();
    match tag_content.find(|c: char| c.is_whitespace()) {
        Some(pos) => (&tag_content[..pos], &tag_content[pos + 1..]),
        None => (tag_content, ""),
    }
}

fn parse_attr(tag_content: &str, attr: &str) -> Option<String> {
    let search = format!("{}=\"", attr);
    let start = tag_content.find(search.as_str())? + search.len();
    let rest = &tag_content[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn apply_opening_tag(name: &str, attrs: &str, parent: &TextStyle) -> TextStyle {
    let mut style = parent.clone();
    match name {
        "b" => style.bold = true,
        "i" => style.italic = true,
        "shadow" => style.shadow = true,
        "outline" => style.outline = true,
        "nowrap" => style.nowrap = true,
        "color" => {
            if let Some(v) = parse_attr_from(attrs, "value") {
                style.color = Some(v);
            }
        }
        "size" => {
            if let Some(v) = parse_attr_from(attrs, "value")
                && let Ok(f) = v.parse::<f32>() {
                    style.size = Some(f);
                }
        }
        "ruby" => {
            // ruby reading is stored separately via pending_ruby; style otherwise unchanged
        }
        _ => {}
    }
    style
}

fn parse_attr_from(attrs: &str, attr: &str) -> Option<String> {
    let search = format!("{}=\"", attr);
    let start = attrs.find(search.as_str())? + search.len();
    let rest = &attrs[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn handle_closing_tag(
    name: &str,
    style_stack: &mut Vec<TextStyle>,
    pending_ruby: &mut Option<String>,
) {
    match name {
        "ruby" => {
            *pending_ruby = None;
            if style_stack.len() > 1 {
                style_stack.pop();
            }
        }
        _ => {
            if style_stack.len() > 1 {
                style_stack.pop();
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_produces_one_span() {
        let spans = parse_inline_markup("hello world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "hello world");
        assert!(!spans[0].style.bold);
    }

    #[test]
    fn bold_tag_wraps_correctly() {
        let spans = parse_inline_markup("This is <b>bold</b> text.");
        let plain: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(plain, "This is bold text.");
        let bold_span = spans.iter().find(|s| s.style.bold).unwrap();
        assert_eq!(bold_span.text, "bold");
    }

    #[test]
    fn color_tag_sets_color() {
        let spans = parse_inline_markup("<color value=\"#ff0000\">red</color>");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.color, Some("#ff0000".to_owned()));
    }

    #[test]
    fn ruby_tag_sets_ruby() {
        let spans = parse_inline_markup("<ruby rt=\"かんじ\">漢字</ruby>");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.ruby, Some("かんじ".to_owned()));
        assert_eq!(spans[0].text, "漢字");
    }

    #[test]
    fn size_tag_sets_size() {
        let spans = parse_inline_markup("<size value=\"24\">large</size>");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.size, Some(24.0));
    }

    #[test]
    fn nested_tags_accumulate_style() {
        let spans = parse_inline_markup("<b><i>both</i></b>");
        assert_eq!(spans.len(), 1);
        assert!(spans[0].style.bold);
        assert!(spans[0].style.italic);
    }

    #[test]
    fn spans_to_plain_strips_tags() {
        let spans = parse_inline_markup("Before <b>bold</b> after.");
        let plain = spans_to_plain(&spans);
        assert_eq!(plain, "Before bold after.");
    }
}
