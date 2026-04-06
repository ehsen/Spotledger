//! Password utilities — moved from `spotledger-db/src/auth.rs`.
//!
//! Supports the same hash formats as Frappe's Python `passlib` library so that
//! database entries created by Frappe are verified correctly, and hashes
//! created here are accepted by Frappe.
//!
//! ## Supported formats
//! - `$pbkdf2-sha256$<rounds>$<ab64salt>$<ab64hash>` — passlib pbkdf2_sha256
//! - `$argon2id$v=19$...` / `$argon2i$...` / `$argon2d$...` — Argon2 variants
//!
//! ## Passlib "ab64" encoding
//! Standard base64 with `.` replacing `+`; `/` unchanged; no `=` padding.

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::{engine::general_purpose::STANDARD, Engine};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;

// ── Public API ─────────────────────────────────────────────────────────────────

/// Hash a plaintext password using passlib-compatible `$pbkdf2-sha256$` format.
///
/// The result can be stored directly in `__Auth.password` and verified by both
/// this function and Frappe's Python `passlib` library.
///
/// Uses 260 000 rounds (matching Frappe's default passlib config as of v15).
///
/// # Examples
/// ```
/// use spotledger_core::utils::passwords::{hash_password, check_password};
/// let hash = hash_password("secret");
/// assert!(hash.starts_with("$pbkdf2-sha256$"));
/// assert!(check_password("secret", &hash));
/// assert!(!check_password("wrong", &hash));
/// ```
pub fn hash_password(password: &str) -> String {
    const ROUNDS: u32 = 260_000;
    const SALT_LEN: usize = 16;
    const HASH_LEN: usize = 32;

    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);

    let mut hash = [0u8; HASH_LEN];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, ROUNDS, &mut hash);

    format!(
        "$pbkdf2-sha256${ROUNDS}${}${}",
        ab64_encode(&salt),
        ab64_encode(&hash),
    )
}

/// Verify a plaintext `password` against a stored passlib hash.
///
/// Returns `true` if the password matches, `false` otherwise.
///
/// Supports both `$pbkdf2-sha256$` and `$argon2*$` hashes.
pub fn check_password(password: &str, hash_str: &str) -> bool {
    if hash_str.starts_with("$pbkdf2-sha256$") {
        verify_pbkdf2_sha256(hash_str, password)
    } else if hash_str.starts_with("$argon2") {
        verify_argon2(hash_str, password)
    } else {
        false
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn verify_pbkdf2_sha256(hash_str: &str, password: &str) -> bool {
    // passlib format: $pbkdf2-sha256$<rounds>$<ab64salt>$<ab64hash>
    // splitn(5, '$') → ["", "pbkdf2-sha256", "<rounds>", "<salt>", "<hash>"]
    let parts: Vec<&str> = hash_str.splitn(5, '$').collect();
    if parts.len() != 5 || parts[1] != "pbkdf2-sha256" {
        return false;
    }

    let rounds: u32 = match parts[2].parse() {
        Ok(r) => r,
        Err(_) => return false,
    };
    let salt = match ab64_decode(parts[3]) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let expected = match ab64_decode(parts[4]) {
        Ok(h) => h,
        Err(_) => return false,
    };

    let mut computed = vec![0u8; expected.len()];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, rounds, &mut computed);
    computed == expected
}

fn verify_argon2(hash_str: &str, password: &str) -> bool {
    let parsed = match PasswordHash::new(hash_str) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// Decode a passlib "ab64" string to raw bytes.
///
/// ab64 is base64 with `.` instead of `+`; no padding.
pub fn ab64_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    let b64 = s.replace('.', "+");
    let pad = (4 - b64.len() % 4) % 4;
    let padded = format!("{b64}{}", "=".repeat(pad));
    STANDARD.decode(&padded)
}

/// Encode raw bytes as passlib ab64 (base64 with `+`→`.`, no `=` padding).
pub fn ab64_encode(data: &[u8]) -> String {
    STANDARD.encode(data).replace('+', ".").replace('=', "")
}

// ── Password strength heuristic ───────────────────────────────────────────────

/// Simple password strength score (0–4), based on Frappe's passphrase checker heuristics.
///
/// Scores:
/// - 0 — very weak (< 8 chars)
/// - 1 — weak (only one character class)
/// - 2 — fair (two classes, ≥ 8 chars)
/// - 3 — good (three classes, ≥ 10 chars)
/// - 4 — strong (all four classes, ≥ 12 chars)
pub fn password_strength_score(password: &str) -> u8 {
    if password.len() < 8 {
        return 0;
    }
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());
    let classes = [has_lower, has_upper, has_digit, has_special]
        .iter()
        .filter(|&&b| b)
        .count();

    match (password.len(), classes) {
        (_, 1) => 1,
        (l, 2) if l >= 8 => 2,
        (l, 3) if l >= 10 => 3,
        (l, 4) if l >= 12 => 4,
        (l, c) if l >= 8 && c >= 2 => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_check_password() {
        let hash = hash_password("correct-horse-battery-staple");
        assert!(hash.starts_with("$pbkdf2-sha256$260000$"));
        assert!(check_password("correct-horse-battery-staple", &hash));
        assert!(!check_password("wrong-password", &hash));
    }

    #[test]
    fn test_ab64_roundtrip() {
        let data = b"hello world!!";
        let encoded = ab64_encode(data);
        let decoded = ab64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_check_password_unknown_format() {
        assert!(!check_password("any", "not_a_valid_hash"));
    }

    #[test]
    fn test_password_strength() {
        assert_eq!(password_strength_score("abc"), 0);       // < 8
        assert_eq!(password_strength_score("aaaaaaaa"), 1);  // 1 class
        assert_eq!(password_strength_score("Aaaaaaaa"), 2);  // 2 classes, 8 chars
        assert_eq!(password_strength_score("Aaaaaaaaaa1"), 3); // 3 classes, ≥10
        assert_eq!(password_strength_score("Aaaaaaaaaa1!"), 4); // 4 classes, ≥12
    }
}
