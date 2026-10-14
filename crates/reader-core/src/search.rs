//! Full-text search across publication content.
//!
//! Provides case-insensitive and accent-insensitive search across all spine items,
//! returning results with spine index, progression within the chapter, and a snippet
//! around each match.

use crate::model::Publication;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

/// A single search result with location and context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    /// Index of the spine item containing this match.
    pub spine_index: usize,
    /// Approximate progression (0.0..=1.0) within the spine item.
    pub progression: f32,
    /// Short snippet of text around the match, with the match marked.
    pub snippet: String,
    /// The href of the spine item (for display/debugging).
    pub href: String,
}

/// Context size (characters) to show around a match in the snippet.
const SNIPPET_CONTEXT: usize = 40;

/// Strips HTML tags from content, returning plain text.
fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script_or_style = false;
    let mut tag_name = String::new();

    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '<' => {
                in_tag = true;
                tag_name.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                let lower_tag = tag_name.to_lowercase();
                if lower_tag.starts_with("script") || lower_tag.starts_with("style") {
                    in_script_or_style = true;
                } else if lower_tag.starts_with("/script") || lower_tag.starts_with("/style") {
                    in_script_or_style = false;
                }
            }
            _ if in_tag => {
                tag_name.push(c);
            }
            _ if in_script_or_style => {}
            '&' => {
                let mut entity = String::new();
                while let Some(&next) = chars.peek() {
                    if next == ';' || entity.len() > 10 {
                        chars.next();
                        break;
                    }
                    if next.is_whitespace() {
                        break;
                    }
                    entity.push(chars.next().unwrap());
                }
                let decoded = match entity.as_str() {
                    "nbsp" | "#160" => ' ',
                    "lt" | "#60" => '<',
                    "gt" | "#62" => '>',
                    "amp" | "#38" => '&',
                    "quot" | "#34" => '"',
                    "apos" | "#39" => '\'',
                    _ => ' ',
                };
                result.push(decoded);
            }
            '\n' | '\r' | '\t' => {
                if !result.ends_with(' ') {
                    result.push(' ');
                }
            }
            _ => result.push(c),
        }
    }

    collapse_whitespace(&result)
}

/// Collapses multiple whitespace characters into single spaces.
fn collapse_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut last_was_space = true;

    for c in text.chars() {
        if c.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(c);
            last_was_space = false;
        }
    }

    result.trim().to_string()
}

/// Normalizes text for accent-insensitive search.
/// Converts to NFD form and strips combining characters (diacritics).
fn normalize_for_search(text: &str) -> String {
    text.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

/// Finds all occurrences of `needle` in `haystack` (normalized),
/// returning byte positions in the original `haystack`.
fn find_matches(haystack: &str, needle: &str) -> Vec<usize> {
    let norm_haystack = normalize_for_search(haystack);
    let norm_needle = normalize_for_search(needle);

    if norm_needle.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let mut norm_idx = 0;
    let mut orig_idx = 0;
    let mut norm_to_orig: Vec<usize> = Vec::with_capacity(norm_haystack.len());

    for c in haystack.chars() {
        let norm_c: String = c
            .to_string()
            .nfd()
            .filter(|nc| !unicode_normalization::char::is_combining_mark(*nc))
            .collect::<String>()
            .to_lowercase();

        for _ in norm_c.chars() {
            norm_to_orig.push(orig_idx);
        }
        orig_idx += c.len_utf8();
    }
    norm_to_orig.push(orig_idx);

    while let Some(rel_pos) = norm_haystack[norm_idx..].find(&norm_needle) {
        let abs_norm_pos = norm_idx + rel_pos;
        if abs_norm_pos < norm_to_orig.len() {
            matches.push(norm_to_orig[abs_norm_pos]);
        }
        norm_idx = abs_norm_pos + norm_needle.len().max(1);
        if norm_idx >= norm_haystack.len() {
            break;
        }
    }

    matches
}

/// Creates a snippet around a match position.
fn create_snippet(text: &str, match_pos: usize, query_len: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();

    let mut char_pos = 0;
    let mut match_char_idx = 0;
    for (i, c) in chars.iter().enumerate() {
        if char_pos >= match_pos {
            match_char_idx = i;
            break;
        }
        char_pos += c.len_utf8();
    }

    let query_chars: usize = query_len.min(total_chars.saturating_sub(match_char_idx));

    let start = match_char_idx.saturating_sub(SNIPPET_CONTEXT);
    let end = (match_char_idx + query_chars + SNIPPET_CONTEXT).min(total_chars);

    let mut snippet = String::new();
    if start > 0 {
        snippet.push('…');
    }

    snippet.push_str(&chars[start..end].iter().collect::<String>());

    if end < total_chars {
        snippet.push('…');
    }

    snippet
}

impl Publication {
    /// Searches for `query` across all spine items.
    ///
    /// Returns results with spine index, approximate progression within each chapter,
    /// and a snippet of text around each match. Search is case-insensitive and
    /// accent-insensitive (e.g., "lição" matches "licao" and vice versa).
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        if query.trim().is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        for (spine_index, href) in self.spine.iter().enumerate() {
            let Ok(bytes) = self.provider.read(href) else {
                continue;
            };
            let Ok(html) = String::from_utf8(bytes) else {
                continue;
            };

            let plain_text = strip_html(&html);
            if plain_text.is_empty() {
                continue;
            }

            let matches = find_matches(&plain_text, query);
            let text_len = plain_text.len() as f32;

            for match_pos in matches {
                let progression = if text_len > 0.0 {
                    (match_pos as f32 / text_len).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                let snippet = create_snippet(&plain_text, match_pos, query.chars().count());

                results.push(SearchResult {
                    spine_index,
                    progression,
                    snippet,
                    href: href.clone(),
                });
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_removes_tags() {
        let html = "<p>Hello <strong>world</strong>!</p>";
        assert_eq!(strip_html(html), "Hello world!");
    }

    #[test]
    fn strip_html_handles_script_and_style() {
        let html = "<style>body { color: red; }</style><p>Text</p><script>alert('x')</script>";
        assert_eq!(strip_html(html), "Text");
    }

    #[test]
    fn strip_html_decodes_entities() {
        let html = "&lt;tag&gt; &amp; &quot;text&quot;";
        assert_eq!(strip_html(html), "<tag> & \"text\"");
    }

    #[test]
    fn normalize_removes_accents() {
        assert_eq!(normalize_for_search("lição"), normalize_for_search("licao"));
        assert_eq!(normalize_for_search("Café"), normalize_for_search("cafe"));
        assert_eq!(
            normalize_for_search("São Paulo"),
            normalize_for_search("sao paulo")
        );
    }

    #[test]
    fn find_matches_case_insensitive() {
        let text = "Hello WORLD hello World";
        let matches = find_matches(text, "hello");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn find_matches_accent_insensitive() {
        let text = "A lição foi aprendida";
        let matches = find_matches(text, "licao");
        assert_eq!(matches.len(), 1);

        let text2 = "Café da manhã";
        let matches2 = find_matches(text2, "cafe");
        assert_eq!(matches2.len(), 1);
    }

    #[test]
    fn find_matches_portuguese_accents() {
        let text = "coração avô ação maçã";

        let matches = find_matches(text, "coracao");
        assert_eq!(matches.len(), 1);

        let matches = find_matches(text, "avo");
        assert_eq!(matches.len(), 1);

        let matches = find_matches(text, "maca");
        assert_eq!(matches.len(), 1);

        let text2 = "esta é a lição do dia";
        let matches = find_matches(text2, "licao");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn create_snippet_includes_context() {
        let text =
            "This is a longer piece of text where we want to find a specific word in the middle.";
        let snippet = create_snippet(text, 43, 4);
        assert!(snippet.contains("find"));
        assert!(snippet.starts_with('…') || snippet.len() < text.len());
    }

    #[test]
    fn create_snippet_handles_start() {
        let text = "Hello world this is a test";
        let snippet = create_snippet(text, 0, 5);
        assert!(snippet.starts_with("Hello"));
    }

    #[test]
    fn create_snippet_handles_end() {
        let text = "This is a test at the end";
        let snippet = create_snippet(text, 22, 3);
        assert!(snippet.contains("end"));
    }

    #[test]
    fn empty_query_returns_no_results() {
        let matches = find_matches("Some text", "");
        assert!(matches.is_empty());

        let matches = find_matches("Some text", "   ");
        assert!(matches.is_empty());
    }
}
