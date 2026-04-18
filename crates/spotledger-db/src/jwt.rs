//! JWT helpers for Spotledger's stateless auth path.
//!
//! The server issues a compact JWT (HS256) after a successful password
//! verification.  The token is stored in the `token` HttpOnly cookie and
//! verified locally on every subsequent request — **no database round-trip**.
//!
//! ## Secret derivation
//!
//! The signing secret is derived deterministically from the site's DB
//! credentials so no extra configuration field is needed:
//!
//! ```text
//! secret = SHA-256( "spotledger:jwt:" || db_name || ":" || db_pass )
//! ```
//!
//! This means the secret changes if the DB password rotates, which
//! automatically invalidates all existing tokens — the correct behaviour.
//!
//! ## Token shape
//!
//! ```json
//! { "sub": "<user_name>", "iat": <unix_ts>, "exp": <unix_ts + 12h> }
//! ```
//!
//! `sub` is the canonical user name (same value stored in `tabSessions.user`).

use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::DbError;

// ── JWT Claims ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the canonical user name.
    pub sub: String,
    /// Issued-at (Unix seconds).
    pub iat: i64,
    /// Expiry (Unix seconds).
    pub exp: i64,
}

// ── Secret derivation ─────────────────────────────────────────────────────────

/// Derive a 32-byte HMAC signing secret from the site's DB credentials.
///
/// Using a deterministic derivation means:
/// - No extra config field to store / rotate.
/// - Rotating the DB password automatically invalidates all issued tokens.
pub fn derive_signing_secret(db_name: &str, db_pass: &str) -> Vec<u8> {
    let input = format!("spotledger:jwt:{db_name}:{db_pass}");
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher.finalize().to_vec()
}

// ── Token operations ──────────────────────────────────────────────────────────

/// Issue a new JWT for `user`.  Token is valid for 12 hours.
///
/// `secret` should come from [`derive_signing_secret`].
pub fn issue_token(user: &str, secret: &[u8]) -> Result<String, DbError> {
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        sub: user.to_owned(),
        iat: now,
        exp: now + 12 * 3600,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| DbError::Other(format!("JWT encode: {e}")))
}

/// Verify a JWT and return the `sub` (user name) if valid.
///
/// Returns `None` when the token is missing, expired, or has a bad signature.
pub fn verify_token(token: &str, secret: &[u8]) -> Option<String> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)
        .ok()
        .map(|data| data.claims.sub)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let secret = derive_signing_secret("mydb", "mypass");
        let token = issue_token("Administrator", &secret).unwrap();
        let user = verify_token(&token, &secret).unwrap();
        assert_eq!(user, "Administrator");
    }

    #[test]
    fn wrong_secret_rejected() {
        let secret1 = derive_signing_secret("mydb", "pass1");
        let secret2 = derive_signing_secret("mydb", "pass2");
        let token = issue_token("Administrator", &secret1).unwrap();
        assert!(verify_token(&token, &secret2).is_none());
    }

    #[test]
    fn tampered_token_rejected() {
        let secret = derive_signing_secret("mydb", "mypass");
        let token = issue_token("Administrator", &secret).unwrap();
        // Flip one byte in the signature segment
        let mut parts: Vec<&str> = token.split('.').collect();
        let mut sig = parts[2].to_string();
        let bad = if sig.starts_with('A') { "B" } else { "A" };
        sig.replace_range(0..1, bad);
        let tampered = parts[..2].join(".") + "." + &sig;
        assert!(verify_token(&tampered, &secret).is_none());
    }
}
