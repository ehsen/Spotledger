//! Validation utilities — Rust equivalents of Frappe's email/phone/URL validators.
//!
//! All functions are pure (no DB access, no context dependency) — they match
//! Frappe's `validate_email_address`, `validate_phone_number`, and related helpers.

use once_cell::sync::Lazy;
use regex::Regex;

// ── Compiled patterns ─────────────────────────────────────────────────────────

/// RFC 5321-compliant email regex (simplified, matching Frappe's validator).
static EMAIL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$",
    )
    .expect("EMAIL_PATTERN")
});

/// URL pattern accepting http/https/ftp schemes.
static URL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^(https?|ftp)://[^\s/$.?#].[^\s]*$",
    )
    .expect("URL_PATTERN")
});

/// Phone number: allows `+`, digits, spaces, hyphens, parentheses.
/// Min 7 digits, max 20 characters.
static PHONE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\+?[\d\s\-().]{7,20}$").expect("PHONE_PATTERN")
});

/// Digits-only extractor for counting digits in a phone number.
static DIGITS_ONLY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\d").expect("DIGITS_ONLY"));

// ── Email ──────────────────────────────────────────────────────────────────────

/// Validate a single email address.
///
/// Returns `true` when the address looks valid. Strips surrounding whitespace
/// before checking. Returns `false` for empty strings.
///
/// Mirrors Frappe's `validate_email_address(email)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::validation::validate_email_address;
/// assert!(validate_email_address("user@example.com"));
/// assert!(validate_email_address("first.last+tag@sub.domain.org"));
/// assert!(!validate_email_address("not-an-email"));
/// assert!(!validate_email_address("missing@tld"));
/// assert!(!validate_email_address(""));
/// ```
pub fn validate_email_address(email: &str) -> bool {
    let email = email.trim();
    if email.is_empty() {
        return false;
    }
    EMAIL_PATTERN.is_match(email)
}

/// Validate a comma/semicolon-separated list of email addresses.
///
/// Returns `true` only if **all** addresses in the list are valid.
/// Useful for multi-recipient fields.
///
/// Mirrors Frappe's `validate_email_address(emails, throw=False)` with a list.
pub fn validate_email_list(emails: &str) -> bool {
    emails
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .all(validate_email_address)
}

// ── Phone ──────────────────────────────────────────────────────────────────────

/// Validate a phone number string.
///
/// Accepts international format (`+` prefix), digits, spaces, hyphens,
/// dots, and parentheses. Requires 7–15 actual digits.
///
/// Mirrors Frappe's `validate_phone_number(phone_number)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::validation::validate_phone_number;
/// assert!(validate_phone_number("+1 (555) 123-4567"));
/// assert!(validate_phone_number("0300-1234567"));
/// assert!(validate_phone_number("1234567"));
/// assert!(!validate_phone_number("123"));   // too short
/// assert!(!validate_phone_number("abc"));
/// ```
pub fn validate_phone_number(phone: &str) -> bool {
    let phone = phone.trim();
    if phone.is_empty() {
        return false;
    }
    if !PHONE_PATTERN.is_match(phone) {
        return false;
    }
    // Count actual digits: ITU E.164 requires 7-15 digits
    let digit_count = DIGITS_ONLY.find_iter(phone).count();
    (7..=15).contains(&digit_count)
}

// ── URL ───────────────────────────────────────────────────────────────────────

/// Validate a URL (must have http, https, or ftp scheme).
///
/// Mirrors Frappe's `validate_url(url)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::validation::validate_url;
/// assert!(validate_url("https://example.com"));
/// assert!(validate_url("http://foo.bar/path?q=1#anchor"));
/// assert!(!validate_url("example.com"));
/// assert!(!validate_url(""));
/// ```
pub fn validate_url(url: &str) -> bool {
    let url = url.trim();
    if url.is_empty() {
        return false;
    }
    URL_PATTERN.is_match(url)
}

// ── Name / identifier validation ──────────────────────────────────────────────

/// Return `true` if `name` is a valid Frappe document name.
///
/// Document names cannot be empty and must not contain `|` (pipe).
pub fn validate_doc_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && !name.contains('|')
}

/// Return `true` if `fieldname` is a valid Frappe fieldname.
///
/// Fieldnames must start with a letter or underscore and contain only
/// alphanumeric characters and underscores.
pub fn validate_fieldname(fieldname: &str) -> bool {
    let mut chars = fieldname.chars();
    match chars.next() {
        None => false,
        Some(first) => {
            (first.is_alphabetic() || first == '_')
                && chars.all(|c| c.is_alphanumeric() || c == '_')
        }
    }
}

/// Return `true` if `date_str` is a well-formed `"yyyy-mm-dd"` date string.
pub fn validate_date_string(date_str: &str) -> bool {
    use chrono::NaiveDate;
    NaiveDate::parse_from_str(date_str.trim(), "%Y-%m-%d").is_ok()
}

/// Return `true` if `email` roughly looks like an email address (used for
/// quick field-level partial checks — for strict validation use
/// `validate_email_address`).
pub fn is_email_like(s: &str) -> bool {
    s.contains('@') && s.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email() {
        assert!(validate_email_address("user@example.com"));
        assert!(validate_email_address("  user@example.com  ")); // whitespace trimmed
        assert!(validate_email_address("first.last+tag@sub.domain.org"));
        assert!(validate_email_address("x@x.co"));
        assert!(!validate_email_address("not-an-email"));
        assert!(!validate_email_address("@domain.com"));
        assert!(!validate_email_address("user@"));
        assert!(!validate_email_address(""));
        assert!(!validate_email_address("user@domain")); // no TLD dot
    }

    #[test]
    fn test_validate_email_list() {
        assert!(validate_email_list("a@b.com, c@d.org"));
        assert!(!validate_email_list("a@b.com, not-email"));
        assert!(validate_email_list(""));  // empty list is vacuously valid
    }

    #[test]
    fn test_validate_phone() {
        assert!(validate_phone_number("+1 (555) 123-4567"));
        assert!(validate_phone_number("0300-1234567"));
        assert!(validate_phone_number("1234567"));
        assert!(!validate_phone_number("123"));
        assert!(!validate_phone_number("abc-def-ghij"));
        assert!(!validate_phone_number(""));
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://example.com"));
        assert!(validate_url("http://foo.bar/path?q=1#anchor"));
        assert!(validate_url("ftp://files.example.org/pub"));
        assert!(!validate_url("example.com")); // no scheme
        assert!(!validate_url("//example.com")); // no full scheme
        assert!(!validate_url(""));
    }

    #[test]
    fn test_validate_fieldname() {
        assert!(validate_fieldname("first_name"));
        assert!(validate_fieldname("_hidden"));
        assert!(!validate_fieldname("1invalid"));
        assert!(!validate_fieldname("has space"));
        assert!(!validate_fieldname(""));
    }

    #[test]
    fn test_validate_date_string() {
        assert!(validate_date_string("2026-04-06"));
        assert!(!validate_date_string("06/04/2026"));
        assert!(!validate_date_string("not-a-date"));
        assert!(!validate_date_string(""));
    }
}
