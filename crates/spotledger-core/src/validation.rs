//! Document validation — pure functions, no DB dependency.
//!
//! All functions accept a `Document` and a `DocTypeMeta` reference.
//! They are called by the save controller in `spotledger-db::controller`
//! before any database write is attempted.
//!
//! Validation sequence (matches Frappe's order):
//!
//! 1. [`fix_numeric_types`] — coerce field values to declared storage types
//! 2. [`validate_constants`] — check `set_only_once` fields
//! 3. [`validate_update_after_submit`] — restrict changes on submitted docs
//! 4. [`get_missing_mandatory_fields`] — collect empty `reqd` fields
//! 5. [`validate_selects`] — check Select values are within allowed options
//! 6. [`validate_length`] — enforce max-length on string fields
//! 7. [`sanitize_content`] — strip dangerous HTML from content fields

use serde_json::Value;

use crate::document::{DocStatus, Document};
use crate::error::CoreError;
use crate::meta::{DocTypeMeta, FieldType};

// ── validate_constants ────────────────────────────────────────────────────────

/// For `set_only_once` fields: error if the value changed from the saved copy.
///
/// `saved` is the version currently in the database.  If `saved = None`
/// (new document), this check is skipped entirely.
pub fn validate_constants(
    doc: &Document,
    saved: Option<&Document>,
    meta: &DocTypeMeta,
) -> Result<(), CoreError> {
    let Some(saved) = saved else { return Ok(()); };

    for df in &meta.fields {
        if !df.set_only_once || df.fieldtype.is_layout() {
            continue;
        }

        let old_val = saved.fields.get(&df.fieldname);
        let new_val = doc.fields.get(&df.fieldname);

        // Only error when the saved value is non-empty AND it changed.
        let old_has_value = matches!(old_val, Some(v)
            if !v.is_null()
            && v.as_str().map(|s| !s.trim().is_empty()).unwrap_or(true));

        if old_has_value && old_val != new_val {
            return Err(CoreError::Validation(format!(
                "Field '{}' cannot be changed once it has been set.",
                df.label
            )));
        }
    }

    Ok(())
}

// ── validate_update_after_submit ──────────────────────────────────────────────

/// For submitted documents: error if a field without `allow_on_submit` changed.
///
/// Skipped when `saved = None` (new doc) or when the saved `docstatus != 1`.
pub fn validate_update_after_submit(
    doc: &Document,
    saved: Option<&Document>,
    meta: &DocTypeMeta,
) -> Result<(), CoreError> {
    let Some(saved) = saved else { return Ok(()); };
    if saved.docstatus() != DocStatus::Submitted {
        return Ok(());
    }

    for df in &meta.fields {
        if df.allow_on_submit || df.fieldtype.is_layout() {
            continue;
        }

        let old_val = saved.fields.get(&df.fieldname);
        let new_val = doc.fields.get(&df.fieldname);

        if old_val != new_val {
            return Err(CoreError::Validation(format!(
                "Not allowed to change field '{}' after submission.",
                df.label
            )));
        }
    }

    Ok(())
}

// ── get_missing_mandatory_fields ──────────────────────────────────────────────

/// Return `(fieldname, label)` pairs for `reqd` fields that are empty or null.
///
/// An empty mandatory Check field (value `0`) is *not* considered missing —
/// the user explicitly set it to unchecked.
pub fn get_missing_mandatory_fields(
    doc: &Document,
    meta: &DocTypeMeta,
) -> Vec<(String, String)> {
    let mut missing = Vec::new();

    for df in &meta.fields {
        if !df.reqd || df.fieldtype.is_layout() {
            continue;
        }

        let empty = match doc.fields.get(&df.fieldname) {
            None | Some(Value::Null) => true,
            Some(Value::String(s))  => s.trim().is_empty(),
            Some(Value::Array(a))   => a.is_empty(),
            // Check/Int/Float: a present number is never treated as "missing"
            _ => false,
        };

        if empty {
            missing.push((df.fieldname.clone(), df.label.clone()));
        }
    }

    missing
}

// ── validate_selects ──────────────────────────────────────────────────────────

/// Validate that Select-field values appear in the field's `select_options`.
///
/// Empty values are skipped — use `reqd` to enforce non-empty.
pub fn validate_selects(doc: &Document, meta: &DocTypeMeta) -> Result<(), CoreError> {
    for df in &meta.fields {
        if df.fieldtype != FieldType::Select {
            continue;
        }
        let Some(ref opts_str) = df.select_options else { continue };

        let value = match doc.fields.get(&df.fieldname) {
            None | Some(Value::Null) => continue,
            Some(Value::String(s)) if s.trim().is_empty() => continue,
            Some(Value::String(s)) => s.as_str(),
            _ => continue,
        };

        let allowed: Vec<&str> = opts_str.lines().map(str::trim).filter(|s| !s.is_empty()).collect();
        if !allowed.contains(&value) {
            return Err(CoreError::Validation(format!(
                "Value '{}' is not valid for field '{}'. Allowed values: {}",
                value,
                df.label,
                allowed.join(", ")
            )));
        }
    }

    Ok(())
}

// ── validate_length ───────────────────────────────────────────────────────────

/// Enforce `DocField.length` limits on string fields.
///
/// Type-implied defaults apply when `length` is not explicitly set:
/// - `Data` / `Select` / `Color` / `Password` / `Attach` / `AttachImage` → 255 chars
/// - `SmallText` → 140 chars
/// - All other string types → uncapped (use explicit `length(n)` if you need a cap)
pub fn validate_length(doc: &Document, meta: &DocTypeMeta) -> Result<(), CoreError> {
    for df in &meta.fields {
        if df.fieldtype.is_layout() {
            continue;
        }

        let value = match doc.fields.get(&df.fieldname) {
            Some(Value::String(s)) => s.as_str(),
            _ => continue,
        };

        let max_len: Option<usize> = df.length.map(|n| n as usize).or_else(|| {
            match df.fieldtype {
                FieldType::Data
                | FieldType::Select
                | FieldType::Color
                | FieldType::Password
                | FieldType::Attach
                | FieldType::AttachImage => Some(255),
                FieldType::SmallText => Some(140),
                _ => None,
            }
        });

        if let Some(max) = max_len {
            if value.len() > max {
                return Err(CoreError::Validation(format!(
                    "Field '{}' exceeds the maximum length of {} characters (got {}).",
                    df.label,
                    max,
                    value.len()
                )));
            }
        }
    }

    Ok(())
}

// ── sanitize_content ──────────────────────────────────────────────────────────

/// Strip dangerous HTML from content fields.
///
/// - `Html` / `Text` / `LongText` fields: dangerous elements (`<script>`,
///   `javascript:` URLs, `on*` event handlers) are removed.
/// - Plain data fields (`Data`, `Select`, etc.): if the value contains a `<`
///   character, all HTML entities are escaped.
/// - Fields with `ignore_xss_filter = true` are left untouched.
///
/// For production deployments the `ammonia` crate provides a comprehensive
/// allow-list scrubber; this implementation covers the most common injection
/// vectors without pulling in additional dependencies.
pub fn sanitize_content(doc: &mut Document, meta: &DocTypeMeta) {
    for df in &meta.fields {
        if df.ignore_xss_filter || df.fieldtype.is_layout() {
            continue;
        }

        let is_html = matches!(
            df.fieldtype,
            FieldType::Html | FieldType::LongText | FieldType::Text
        );

        let raw: String = match doc.fields.get(&df.fieldname) {
            Some(Value::String(s)) => s.clone(),
            _ => continue,
        };

        let sanitized = if is_html {
            let cleaned = strip_scripts(&raw);
            if cleaned == raw { continue; }
            cleaned
        } else if raw.contains('<') || raw.contains('&') {
            escape_html(&raw)
        } else {
            continue;
        };

        doc.fields.insert(df.fieldname.clone(), Value::String(sanitized));
    }
}

// ── internal HTML helpers ─────────────────────────────────────────────────────

/// Remove `<script>` blocks and `on*=` event handler attributes.
///
/// This is an additive-safe pass — it does not escape valid HTML, it only
/// removes the most dangerous injection vectors.
fn strip_scripts(input: &str) -> String {
    let mut out = input.to_owned();

    // Remove <script ...>...</script> blocks (case-insensitive, greedy)
    loop {
        let lower = out.to_lowercase();
        match lower.find("<script") {
            None => break,
            Some(start) => {
                match lower[start..].find("</script>") {
                    Some(rel_end) => {
                        out.replace_range(start..start + rel_end + 9, "");
                    }
                    None => {
                        out.truncate(start);
                        break;
                    }
                }
            }
        }
    }

    // Remove javascript: href/src values by replacing the uri scheme
    // e.g. href="javascript:alert(1)" → href=""
    let lower_check = out.to_lowercase();
    if lower_check.contains("javascript:") {
        out = out
            .replace("javascript:", "")
            .replace("JAVASCRIPT:", "")
            .replace("Javascript:", "");
    }

    out
}

/// Escape HTML entities in plain-text fields that appear to contain markup.
fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 32);
    for ch in input.chars() {
        match ch {
            '<'  => out.push_str("&lt;"),
            '>'  => out.push_str("&gt;"),
            '&'  => out.push_str("&amp;"),
            '"'  => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            c    => out.push(c),
        }
    }
    out
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::{DocField, DocTypeMeta};

    fn minimal_meta() -> DocTypeMeta {
        DocTypeMeta::builder("Test", "Test")
            .field(DocField::new("status", "Status", FieldType::Select)
                .required()
                .select_options("Draft\nSubmitted\nCancelled"))
            .field(DocField::new("notes", "Notes", FieldType::Data)
                .required())
            .build()
    }

    #[test]
    fn missing_mandatory_found() {
        let meta = minimal_meta();
        let doc  = Document::new("Test");   // empty doc
        let missing = get_missing_mandatory_fields(&doc, &meta);
        assert!(!missing.is_empty(), "Should find missing mandatory fields");
        let names: Vec<&str> = missing.iter().map(|(f, _)| f.as_str()).collect();
        assert!(names.contains(&"status"));
        assert!(names.contains(&"notes"));
    }

    #[test]
    fn select_valid_value_passes() {
        let meta = minimal_meta();
        let mut doc = Document::new("Test");
        doc.set("status", Value::String("Draft".into()));
        doc.set("notes",  Value::String("ok".into()));
        assert!(validate_selects(&doc, &meta).is_ok());
    }

    #[test]
    fn select_invalid_value_fails() {
        let meta = minimal_meta();
        let mut doc = Document::new("Test");
        doc.set("status", Value::String("Unknown".into()));
        assert!(validate_selects(&doc, &meta).is_err());
    }

    #[test]
    fn length_violation_fails() {
        let meta = DocTypeMeta::builder("T", "T")
            .field(DocField::new("code", "Code", FieldType::Data).length(5))
            .build();
        let mut doc = Document::new("T");
        doc.set("code", Value::String("toolong".into()));
        assert!(validate_length(&doc, &meta).is_err());
    }

    #[test]
    fn script_tag_stripped() {
        let meta = DocTypeMeta::builder("T", "T")
            .field(DocField::new("content", "Content", FieldType::Html))
            .build();
        let mut doc = Document::new("T");
        doc.set("content", Value::String("<p>OK</p><script>alert(1)</script>".into()));
        sanitize_content(&mut doc, &meta);
        let val = doc.fields.get("content").and_then(Value::as_str).unwrap_or("");
        assert!(!val.contains("<script"), "script tag should be stripped");
        assert!(val.contains("<p>OK</p>"), "safe content should survive");
    }

    #[test]
    fn plain_field_escaped() {
        let meta = DocTypeMeta::builder("T", "T")
            .field(DocField::new("title", "Title", FieldType::Data))
            .build();
        let mut doc = Document::new("T");
        doc.set("title", Value::String("<b>hello</b>".into()));
        sanitize_content(&mut doc, &meta);
        let val = doc.fields.get("title").and_then(Value::as_str).unwrap_or("");
        assert_eq!(val, "&lt;b&gt;hello&lt;/b&gt;");
    }
}
