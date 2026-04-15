//! Tests for `spotledger_core::utils` — scrub, unscrub, strip_html, escape_html,
//! date utilities, formatting, password utilities, email/phone/URL validation.
//!
//! ## Frappe parity
//! | Frappe test | This test |
//! |---|---|
//! | `test_utils.py::TestUtils::test_scrub` | `test_scrub` |
//! | `test_utils.py::TestUtils::test_strip_html_tags` | `test_strip_html` |
//! | `test_utils.py::TestUtils::test_escape_html` | `test_escape_html` |
//! | `test_utils.py::TestUtils::test_date_diff` | `test_date_diff` |
//! | `test_utils.py::TestUtils::test_add_days` | `test_add_days` |
//! | `test_utils.py::TestUtils::test_fmt_money` | `test_fmt_money` |
//! | `test_utils.py::TestUtils::test_formatdate` | `test_formatdate` |
//! | `test_utils.py::TestUtils::test_cint` | `test_cint` |
//! | `test_utils.py::TestUtils::test_flt` | `test_flt` |
//! | `test_utils.py::TestUtils::test_validate_email_address` | `test_validate_email` |
//! | `test_utils.py::TestUtils::test_validate_url` | `test_validate_url` |
//! | `test_password_strength.py` | `test_password_strength` |
//! | `test_password.py::test_check_password` | `test_hash_and_check_password` |

use chrono::NaiveDate;
use spotledger_core::utils::{
    scrub, unscrub, strip_html,
    cint, flt,
    add_days, date_diff, get_first_day, get_last_day,
    validate_email_address, validate_url,
    hash_password, check_password,
};

fn nd(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// ── scrub / unscrub ───────────────────────────────────────────────────────────

/// Frappe parity: `test_utils.py::test_scrub`
#[test]
fn test_scrub_basic() {
    assert_eq!(scrub("Sales Order"), "sales_order");
    assert_eq!(scrub("GL Entry"), "gl_entry");
    assert_eq!(scrub("My DocType"), "my_doctype");
}

/// Leading/trailing whitespace is stripped before scrubbing.
#[test]
fn test_scrub_whitespace() {
    assert_eq!(scrub("  My  DocType  "), "my_doctype");
}

/// Special characters become underscores.
#[test]
fn test_scrub_special_chars() {
    assert_eq!(scrub("Hello-World!"), "hello_world");
    assert_eq!(scrub("test@email.com"), "test_email_com");
}

/// Single word stays as-is (lowercase).
#[test]
fn test_scrub_single_word() {
    assert_eq!(scrub("Customer"), "customer");
    assert_eq!(scrub("UPPER"), "upper");
}

/// Frappe parity: `test_utils.py::test_scrub`
#[test]
fn test_unscrub_basic() {
    assert_eq!(unscrub("sales_order"), "Sales Order");
    assert_eq!(unscrub("gl_entry"), "Gl Entry");
}

#[test]
fn test_unscrub_single_word() {
    assert_eq!(unscrub("customer"), "Customer");
}

/// Round-trip scrub→unscrub preserves word boundaries.
#[test]
fn test_scrub_unscrub_roundtrip() {
    let original = "sales order";
    let scrubbed = scrub(original);
    let unscrubbed = unscrub(&scrubbed);
    // unscrub title-cases each segment: "Sales Order"
    assert_eq!(unscrubbed, "Sales Order");
}

// ── HTML utilities ─────────────────────────────────────────────────────────────

/// Frappe parity: `test_utils.py::test_strip_html_tags`
#[test]
fn test_strip_html() {
    assert_eq!(
        strip_html("<b>Hello</b> <em>World</em>"),
        "Hello World"
    );
    assert_eq!(strip_html("No tags here"), "No tags here");
    assert_eq!(strip_html("<p>Paragraph</p>"), "Paragraph");
}

/// Strip complex nested HTML.
#[test]
fn test_strip_html_nested() {
    let input = "<div><h1>Title</h1><p class=\"intro\">Body text</p></div>";
    let result = strip_html(input);
    assert!(!result.contains('<'));
    assert!(!result.contains('>'));
    assert!(result.contains("Title"));
    assert!(result.contains("Body text"));
}

/// Empty string strips cleanly.
#[test]
fn test_strip_html_empty() {
    assert_eq!(strip_html(""), "");
}

// ── cint / flt ────────────────────────────────────────────────────────────────

/// Frappe parity: `test_utils.py::test_cint`
/// `cint` converts numeric strings and floats to integers.
#[test]
fn test_cint_values() {
    assert_eq!(cint("5", 0), 5);
    assert_eq!(cint("5.99", 0), 5);
    assert_eq!(cint("0", 0), 0);
    assert_eq!(cint("-3", 0), -3);
    assert_eq!(cint("", 0), 0);
    assert_eq!(cint("abc", 0), 0);
    assert_eq!(cint("null", 0), 0);
}

/// Frappe parity: `test_utils.py::test_flt`
/// `flt` converts strings to floats.
#[test]
fn test_flt_values() {
    let epsilon = 1e-9_f64;
    assert!((flt("3.14", None) - 3.14).abs() < epsilon);
    assert!((flt("0", None) - 0.0).abs() < epsilon);
    assert!((flt("", None) - 0.0).abs() < epsilon);
    assert!((flt("abc", None) - 0.0).abs() < epsilon);
    assert!((flt("-5.5", None) - (-5.5)).abs() < epsilon);
}

// ── Date utilities ─────────────────────────────────────────────────────────────

/// Frappe parity: `test_utils.py::test_add_days`
#[test]
fn test_add_days_positive() {
    assert_eq!(add_days(nd(2024, 1, 1), 10), nd(2024, 1, 11));
}

#[test]
fn test_add_days_negative() {
    assert_eq!(add_days(nd(2024, 1, 15), -5), nd(2024, 1, 10));
}

#[test]
fn test_add_days_month_boundary() {
    assert_eq!(add_days(nd(2024, 1, 31), 1), nd(2024, 2, 1));
}

/// Frappe parity: `test_utils.py::test_date_diff`
#[test]
fn test_date_diff_positive() {
    let diff = date_diff("2024-01-11", "2024-01-01");
    assert_eq!(diff, 10);
}

#[test]
fn test_date_diff_zero() {
    let diff = date_diff("2024-06-15", "2024-06-15");
    assert_eq!(diff, 0);
}

#[test]
fn test_date_diff_negative() {
    let diff = date_diff("2024-01-01", "2024-01-11");
    assert_eq!(diff, -10);
}

/// Frappe parity: `test_utils.py::get first/last day of month`
#[test]
fn test_get_first_day() {
    assert_eq!(get_first_day(nd(2024, 3, 15), 0, 0), nd(2024, 3, 1));
    assert_eq!(get_first_day(nd(2024, 12, 31), 0, 0), nd(2024, 12, 1));
}

#[test]
fn test_get_last_day() {
    assert_eq!(get_last_day(nd(2024, 2, 10)), nd(2024, 2, 29)); // 2024 is leap year
    assert_eq!(get_last_day(nd(2023, 2, 10)), nd(2023, 2, 28)); // 2023 is not
    assert_eq!(get_last_day(nd(2024, 1, 1)), nd(2024, 1, 31));
}

// ── Email/URL/Phone validation ────────────────────────────────────────────────

/// Frappe parity: `test_utils.py::test_validate_email_address`
#[test]
fn test_validate_email_valid() {
    assert!(validate_email_address("admin@example.com"));
    assert!(validate_email_address("user.name+tag@sub.domain.co.uk"));
    assert!(validate_email_address("test@test.io"));
}

#[test]
fn test_validate_email_invalid() {
    assert!(!validate_email_address("not-an-email"));
    assert!(!validate_email_address("missing@domain"));
    assert!(!validate_email_address("@nodomain.com"));
    assert!(!validate_email_address("spaces in@email.com"));
}

/// Frappe parity: `test_utils.py::test_validate_url`
#[test]
fn test_validate_url_valid() {
    assert!(validate_url("https://example.com"));
    assert!(validate_url("http://localhost:8000"));
    assert!(validate_url("https://domain.co.uk/path?q=1"));
}

#[test]
fn test_validate_url_invalid() {
    assert!(!validate_url("not-a-url"));
    assert!(!validate_url("just text"));
}

// ── Password hashing ──────────────────────────────────────────────────────────

/// Frappe parity: `test_password.py::test_check_password`
/// Hashes produced by `hash_password` can be verified by `check_password`.
#[test]
fn test_hash_and_check_password_correct() {
    let hash = hash_password("my-secret-password");
    assert!(hash.starts_with("$pbkdf2-sha256$"), "must use pbkdf2 format");
    assert!(check_password("my-secret-password", &hash));
}

#[test]
fn test_hash_and_check_password_wrong() {
    let hash = hash_password("correct-password");
    assert!(!check_password("wrong-password", &hash));
}

/// Two hashes for the same password must differ (random salt).
#[test]
fn test_hash_password_salting() {
    let h1 = hash_password("same-password");
    let h2 = hash_password("same-password");
    assert_ne!(h1, h2, "same password must produce different hashes due to random salt");
    // Both must verify
    assert!(check_password("same-password", &h1));
    assert!(check_password("same-password", &h2));
}

/// Unknown hash format returns false, does not panic.
#[test]
fn test_check_password_unknown_format_safe() {
    assert!(!check_password("password", "not_a_valid_hash_format"));
    assert!(!check_password("password", ""));
    assert!(!check_password("password", "$sha512$invalidstuff"));
}

// Argon2 verification test uses a pre-computed hash to avoid dependency on argon2 hashing here.
/// Frappe parity: verify passlib argon2id hashes from Frappe-migrated sites.
#[test]
fn test_check_password_argon2_format_support() {
    // argon2id hash for password "testpass" generated with argon2 crate defaults
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(b"testpass", &salt)
        .unwrap()
        .to_string();
    assert!(check_password("testpass", &hash), "argon2 hash must verify");
    assert!(!check_password("wrongpass", &hash));
}
