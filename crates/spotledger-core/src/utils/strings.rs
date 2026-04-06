//! String utilities — Rust equivalents of `frappe.utils.data` string helpers.
//!
//! Functions:
//! - `scrub`       — convert "Human Label" → "human_label" (Frappe `frappe.scrub`)
//! - `unscrub`     — reverse: "human_label" → "Human Label"
//! - `cstr`        — safe any-to-String conversion
//! - `strip_html`  — remove all HTML tags
//! - `escape_html` — escape `< > & " '` for safe HTML insertion
//! - `get_abbr`    — abbreviation from words ("John Doe" → "JD")
//! - `slug`        — URL-safe slug ("My Doc" → "my-doc")
//! - `truncate`    — truncate to max length with ellipsis

use once_cell::sync::Lazy;
use regex::Regex;

// ── Compiled patterns ─────────────────────────────────────────────────────────

/// Matches anything inside or including `< ... >` HTML tags.
static HTML_TAG_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<[^>]+>").expect("HTML_TAG_PATTERN"));

/// Matches runs of non-alphanumeric characters used in `scrub`.
static NON_ALNUM_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[^a-zA-Z0-9]+").expect("NON_ALNUM_PATTERN"));

/// Matches runs of whitespace used in `unscrub`.
static UNDERSCORE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"_+").expect("UNDERSCORE_PATTERN"));

// ── scrub / unscrub ───────────────────────────────────────────────────────────

/// Convert a human-readable label to a Frappe `fieldname` / `DocType name` slug.
///
/// Rules (matching Frappe's `frappe.scrub`):
/// 1. Strip leading/trailing whitespace.
/// 2. Replace spaces with underscores, collapse multiple; lowercase everything.
/// 3. Replace any sequence of non-alphanumeric characters with `_`.
/// 4. Strip leading/trailing `_`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::strings::scrub;
/// assert_eq!(scrub("Sales Order"), "sales_order");
/// assert_eq!(scrub("  My  DocType  "), "my_doctype");
/// assert_eq!(scrub("GL Entry"), "gl_entry");
/// assert_eq!(scrub("Hello-World!"), "hello_world");
/// ```
pub fn scrub(s: &str) -> String {
    let s = s.trim().to_lowercase();
    // Replace any non-alphanumeric run with a single underscore
    let s = NON_ALNUM_PATTERN.replace_all(&s, "_");
    // Strip leading/trailing underscores
    s.trim_matches('_').to_owned()
}

/// Reverse of `scrub`: convert `"sales_order"` → `"Sales Order"`.
///
/// Each underscore-delimited word is title-cased.
///
/// # Examples
/// ```
/// use spotledger_core::utils::strings::unscrub;
/// assert_eq!(unscrub("sales_order"), "Sales Order");
/// assert_eq!(unscrub("gl_entry"), "Gl Entry");
/// ```
pub fn unscrub(s: &str) -> String {
    UNDERSCORE_PATTERN
        .replace_all(s, " ")
        .split_whitespace()
        .map(title_case_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_case_word(w: &str) -> String {
    let mut chars = w.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// ── cstr ──────────────────────────────────────────────────────────────────────

/// Convert any Display-able value to a `String`.  
/// `None` and empty values become empty strings.
///
/// Mirrors Frappe's `cstr(s)`.
pub fn cstr<T: std::fmt::Display>(v: T) -> String {
    v.to_string()
}

// ── HTML utilities ─────────────────────────────────────────────────────────────

/// Remove all HTML tags from `text` — leaves only inner text.
///
/// Mirrors Frappe's `strip_html(text)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::strings::strip_html;
/// assert_eq!(strip_html("<b>Hello</b> <em>World</em>"), "Hello World");
/// assert_eq!(strip_html("No tags here"), "No tags here");
/// ```
pub fn strip_html(text: &str) -> String {
    HTML_TAG_PATTERN.replace_all(text, "").into_owned()
}

/// Escape HTML special characters (`& < > " '`) to their HTML entity equivalents.
///
/// Mirrors Frappe's `escape_html(text)`.
pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            c => out.push(c),
        }
    }
    out
}

// ── Abbreviation & slug ────────────────────────────────────────────────────────

/// Return an abbreviation from the first characters of each word.
///
/// Mirrors Frappe's `get_abbr(string, max_len)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::strings::get_abbr;
/// assert_eq!(get_abbr("John Doe", 2), "JD");
/// assert_eq!(get_abbr("Jenny Jane Doe", 2), "JJ");
/// assert_eq!(get_abbr("", 2), "?");
/// ```
pub fn get_abbr(s: &str, max_len: usize) -> String {
    let abbr: String = s
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(max_len)
        .collect();
    if abbr.is_empty() {
        "?".to_owned()
    } else {
        abbr.to_uppercase()
    }
}

/// Convert a string to a URL-safe slug: lowercase, hyphens, no special chars.
///
/// Mirrors Frappe desk's `slug(text)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::strings::slug;
/// assert_eq!(slug("My Work Space"), "my-work-space");
/// assert_eq!(slug("Sales & Marketing"), "sales-marketing");
/// ```
pub fn slug(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ── Truncation ────────────────────────────────────────────────────────────────

/// Truncate `text` to at most `max_len` characters, appending `"…"` if truncated.
pub fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_owned()
    } else {
        let end = text
            .char_indices()
            .nth(max_len.saturating_sub(1))
            .map(|(i, _)| i)
            .unwrap_or(text.len());
        format!("{}…", &text[..end])
    }
}

// ── Boolean conversion ────────────────────────────────────────────────────────

/// Convert common string representations to `bool` where unambiguous.
///
/// Mirrors Frappe's `sbool(x)`: `"true"/"1"` → `true`, `"false"/"0"` → `false`.
/// Returns `None` for ambiguous strings.
pub fn sbool(s: &str) -> Option<bool> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrub() {
        assert_eq!(scrub("Sales Order"), "sales_order");
        assert_eq!(scrub("  My  DocType  "), "my_doctype");
        assert_eq!(scrub("GL Entry"), "gl_entry");
        assert_eq!(scrub("Hello-World!"), "hello_world");
        assert_eq!(scrub("Already_scrubbed"), "already_scrubbed");
        assert_eq!(scrub(""), "");
    }

    #[test]
    fn test_unscrub() {
        assert_eq!(unscrub("sales_order"), "Sales Order");
        assert_eq!(unscrub("gl_entry"), "Gl Entry");
        assert_eq!(unscrub("hello"), "Hello");
    }

    #[test]
    fn test_strip_html() {
        assert_eq!(strip_html("<b>Hello</b> <em>World</em>"), "Hello World");
        assert_eq!(strip_html("No tags"), "No tags");
        assert_eq!(strip_html("<p>Line1</p><p>Line2</p>"), "Line1Line2");
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<script>alert('xss')</script>"), "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
    }

    #[test]
    fn test_get_abbr() {
        assert_eq!(get_abbr("John Doe", 2), "JD");
        assert_eq!(get_abbr("Jenny Jane Doe", 2), "JJ");
        assert_eq!(get_abbr("Jenny Jane Doe", 3), "JJD");
        assert_eq!(get_abbr("", 2), "?");
    }

    #[test]
    fn test_slug() {
        assert_eq!(slug("My Work Space"), "my-work-space");
        assert_eq!(slug("Sales & Marketing"), "sales-marketing");
        assert_eq!(slug("  Trimmed  "), "trimmed");
    }

    #[test]
    fn test_sbool() {
        assert_eq!(sbool("true"), Some(true));
        assert_eq!(sbool("1"), Some(true));
        assert_eq!(sbool("false"), Some(false));
        assert_eq!(sbool("0"), Some(false));
        assert_eq!(sbool("maybe"), None);
    }
}
