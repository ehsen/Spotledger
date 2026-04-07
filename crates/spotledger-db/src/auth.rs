//! Authentication helpers: password verification, session management.
//!
//! ## Password formats
//!
//! Frappe uses `passlib` with two schemes:
//! - `$pbkdf2-sha256$<rounds>$<ab64salt>$<ab64hash>` — most installations
//! - `$argon2id$v=19$m=...` — newer installs or migrated passwords
//!
//! Passlib's "ab64" encoding is standard base64 with `.` replacing `+`
//! (the `/` character is unchanged, `=` padding is omitted).
//!
//! ## Session storage
//!
//! Sessions are stored in `tabSessions` (created by framework_surrealdb.surql):
//!   sid, user, status ("Active"/"Expired"), lastupdate, ipaddress, sessiondata

use serde_json::Value;

use crate::adapter::DbAdapter;
use crate::error::DbError;

// ── User lookup ───────────────────────────────────────────────────────────────

/// Minimal user info needed for authentication.
#[derive(Debug)]
pub struct UserInfo {
    pub name: String,
    pub full_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub user_type: Option<String>,
    pub enabled: bool,
    pub language: Option<String>,
}

/// Look up a user by their username (`name`) or `email` field.
/// Returns `DbError::NotFound` if no user matches.
pub async fn lookup_user(adapter: &DbAdapter, usr: &str) -> Result<UserInfo, DbError> {
    let rows = adapter
        .run(
            "SELECT name, full_name, first_name, last_name, user_type, enabled, language \
             FROM tabUser WHERE name = $usr OR email = $usr LIMIT 1",
            vec![("usr".into(), usr.into())],
        )
        .await?;
    let row = rows.into_iter().next().ok_or_else(|| DbError::NotFound {
        doctype: "User".into(),
        name: usr.into(),
    })?;

    let name = row
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(usr)
        .to_owned();

    let enabled = row
        .get("enabled")
        .map(|v| {
            v.as_bool().unwrap_or(false) || v.as_i64().map(|n| n != 0).unwrap_or(false)
        })
        .unwrap_or(true);

    Ok(UserInfo {
        name,
        full_name: str_field(&row, "full_name"),
        first_name: str_field(&row, "first_name"),
        last_name: str_field(&row, "last_name"),
        user_type: str_field(&row, "user_type"),
        enabled,
        language: str_field(&row, "language"),
    })
}

// ── Password verification ─────────────────────────────────────────────────────

/// Fetch the stored password hash from `__Auth` for a given user.
/// The hash is in passlib format (pbkdf2-sha256 or argon2).
pub async fn get_password_hash(adapter: &DbAdapter, user: &str) -> Result<String, DbError> {
    let rows = adapter
        .run(
            "SELECT password FROM __Auth \
             WHERE doctype = 'User' AND name = $user \
               AND fieldname = 'password' AND encrypted = false \
             LIMIT 1",
            vec![("user".into(), user.into())],
        )
        .await?;
    rows.into_iter()
        .next()
        .and_then(|v| v.get("password").and_then(Value::as_str).map(str::to_owned))
        .ok_or_else(|| DbError::NotFound {
            doctype: "__Auth".into(),
            name: user.into(),
        })
}

/// Verify a plain-text `password` against a passlib `hash_str`.
/// Returns `true` if the password matches.
///
/// Supports:
/// - `$pbkdf2-sha256$<rounds>$<ab64salt>$<ab64hash>` (passlib pbkdf2_sha256)
/// - `$argon2id$...`, `$argon2i$...`, `$argon2d$...` (argon2)
pub fn verify_password(hash_str: &str, password: &str) -> bool {
    if hash_str.starts_with("$pbkdf2-sha256$") {
        verify_pbkdf2_sha256(hash_str, password)
    } else if hash_str.starts_with("$argon2") {
        verify_argon2(hash_str, password)
    } else {
        false
    }
}

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

    use pbkdf2::pbkdf2_hmac;
    use sha2::Sha256;

    let mut computed = vec![0u8; expected.len()];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, rounds, &mut computed);
    computed == expected
}

fn verify_argon2(hash_str: &str, password: &str) -> bool {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let parsed = match PasswordHash::new(hash_str) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// Decode a passlib "ab64" string to bytes.
///
/// Passlib ab64 is standard base64 with:
/// - `.` used instead of `+`
/// - `/` unchanged
/// - `=` padding omitted
pub fn ab64_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    // Restore `+` from `.`
    let b64 = s.replace('.', "+");
    // Re-add `=` padding
    let pad = (4 - b64.len() % 4) % 4;
    let padded = format!("{b64}{}", "=".repeat(pad));
    STANDARD.decode(&padded)
}

/// Encode raw bytes as passlib ab64 (standard base64, `+`→`.`, no `=` padding).
pub fn ab64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.encode(data).replace('+', ".").replace('=', "")
}

/// Hash a plaintext password using passlib-compatible `$pbkdf2-sha256$` format.
///
/// The result can be stored directly in `__Auth` and verified by both
/// this crate and Frappe's Python `passlib` library.
pub fn hash_password(password: &str) -> String {
    use pbkdf2::pbkdf2_hmac;
    use rand::RngCore;
    use sha2::Sha256;

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

/// Upsert the password hash for a user in `__Auth`.
///
/// Safe to call on both freshly created and existing records — uses
/// `INSERT ... ON DUPLICATE KEY UPDATE` semantics via SurrealDB's UPSERT.
pub async fn set_user_password(
    adapter: &DbAdapter,
    user: &str,
    password: &str,
) -> Result<(), DbError> {
    let hash = hash_password(password);
    // Use a literal record ID `__Auth:<user>:password` so UPSERT always
    // creates-or-replaces regardless of whether the __Auth table is empty.
    let sql = format!(
        "UPSERT `__Auth`:`{user}:password` SET \
           doctype = 'User', \
           name = '{user}', \
           fieldname = 'password', \
           password = $hash, \
           encrypted = false"
    );
    adapter
        .run(&sql, vec![("hash".into(), hash.into())])
        .await
        .map(|_| ())
}

// ── Session management ────────────────────────────────────────────────────────

/// Session information retrieved from `tabSessions`.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub sid: String,
    pub user: String,
    pub status: String,
}

/// Create a new active session in `tabSessions`.
/// Returns the generated `sid` (32 hex chars).
pub async fn create_session(
    adapter: &DbAdapter,
    user: &str,
    ip: Option<&str>,
) -> Result<String, DbError> {
    let sid = generate_sid();
    let session_data = serde_json::json!({
        "user": user,
        "session_ip": ip.unwrap_or(""),
    })
    .to_string();

    adapter
        .execute(
            "INSERT INTO tabSessions {
                sid: $sid,
                user: $user,
                status: 'Active',
                lastupdate: time::now(),
                ipaddress: $ip,
                sessiondata: $sessiondata
            }",
            vec![
                ("sid".into(),         sid.clone().into()),
                ("user".into(),        user.to_owned().into()),
                ("ip".into(),          ip.unwrap_or("").to_owned().into()),
                ("sessiondata".into(), session_data.into()),
            ],
        )
        .await?;

    Ok(sid)
}

/// Look up an active session by `sid`.
/// Returns `None` if the session does not exist or is not active.
pub async fn get_session(adapter: &DbAdapter, sid: &str) -> Result<Option<SessionInfo>, DbError> {
    let rows = adapter
        .run(
            "SELECT sid, user, status FROM tabSessions \
             WHERE sid = $sid AND status = 'Active' LIMIT 1",
            vec![("sid".into(), sid.to_owned().into())],
        )
        .await?;

    Ok(rows.into_iter().next().and_then(|v: Value| {
        let sid = v.get("sid")?.as_str()?.to_owned();
        let user = v.get("user")?.as_str()?.to_owned();
        let status = v.get("status")?.as_str()?.to_owned();
        Some(SessionInfo { sid, user, status })
    }))
}

/// Expire a session (sets `status = 'Expired'`).
/// Does not delete the record so audit trails are preserved.
pub async fn expire_session(adapter: &DbAdapter, sid: &str) -> Result<(), DbError> {
    adapter
        .execute(
            "UPDATE tabSessions SET status = 'Expired', lastupdate = time::now() WHERE sid = $sid",
            vec![("sid".into(), sid.to_owned().into())],
        )
        .await
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn generate_sid() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes) // 32 lowercase hex chars
}

fn str_field(row: &Value, key: &str) -> Option<String> {
    row.get(key).and_then(Value::as_str).map(str::to_owned)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ab64_decode_basic_no_dot() {
        // "Hello" = "SGVsbG8=" in standard base64 = "SGVsbG8" without padding
        // passlib ab64 is identical to std base64 when no "+" appears in the encoded value
        let result = ab64_decode("SGVsbG8").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn ab64_decode_replaces_dot_with_plus() {
        // 0xFB encodes as "+w==" in standard base64 (2 chars + 2 pads).
        // Passlib ab64 strips padding and replaces '+' with '.', giving ".w".
        // Decoding ".w" should produce [0xFB].
        let result = ab64_decode(".w").unwrap();
        assert_eq!(result, &[0xFBu8]);

        // "AAAA" encodes 3 zero bytes in both standard and ab64 (no '+' present).
        let r2 = ab64_decode("AAAA").unwrap();
        assert_eq!(r2, [0u8, 0u8, 0u8]);
    }

    #[test]
    fn verify_password_rejects_unknown_scheme() {
        assert!(!verify_password("", "password"));
        assert!(!verify_password("$sha256$...", "password"));
        assert!(!verify_password("plaintext", "password"));
    }

    #[test]
    fn verify_password_rejects_malformed_pbkdf2() {
        // Incomplete format
        assert!(!verify_password("$pbkdf2-sha256$29000", "password"));
        // Wrong field count
        assert!(!verify_password("$pbkdf2-sha256$abc$salt$hash", "password"));
    }

    #[test]
    fn generate_sid_produces_32_hex_chars() {
        let sid = generate_sid();
        assert_eq!(sid.len(), 32);
        assert!(sid.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generate_sid_is_unique() {
        let s1 = generate_sid();
        let s2 = generate_sid();
        assert_ne!(s1, s2);
    }

    #[test]
    fn hash_password_round_trips() {
        let hash = hash_password("secret123");
        // Hash must start with the pbkdf2-sha256 scheme identifier
        assert!(hash.starts_with("$pbkdf2-sha256$"), "hash: {hash}");
        // verify_password must accept the correct password
        assert!(verify_password(&hash, "secret123"));
        // verify_password must reject a wrong password
        assert!(!verify_password(&hash, "wrong"));
    }

    #[test]
    fn ab64_encode_decode_roundtrip() {
        let data: &[u8] = &[0xFB, 0x00, 0xAB, 0xCD, 0x12, 0xFF];
        let encoded = ab64_encode(data);
        // No `=` padding, no `+`
        assert!(!encoded.contains('='));
        assert!(!encoded.contains('+'));
        let decoded = ab64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }
}
