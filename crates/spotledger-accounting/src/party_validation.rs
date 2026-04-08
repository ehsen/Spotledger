//! Party validation — three-tier logic for JournalEntryAccount party fields.
//!
//! ## The three tiers
//!
//! 1. `party_type` not found in the `PartyType` kernel table
//!    → **hard error**: "Unknown party type: X"
//!
//! 2. `party_type` found, but the owning plugin is not loaded
//!    → **hard error**: "Party type Customer requires the 'selling' plugin — not loaded"
//!
//! 3. Plugin loaded, but the named party document doesn't exist
//!    → **validation error**: "Customer 'CUST-9999' does not exist"
//!
//! ## Trust levels
//!
//! When a plugin's `on_submit` hook calls `sl_make_gl_entries()`, the host
//! sets `TrustLevel::Plugin`.  At that level tier-3 existence checks are
//! skipped — the plugin already read the customer record to build the entry.
//! Tiers 1 and 2 still apply (a plugin cannot invent a party_type that no
//! manifest declared).
//!
//! For desk-originated saves (a user typing into a Journal Entry form) the
//! trust level is `TrustLevel::User` and all three tiers are enforced.

/// Controls how strictly party validation is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    /// Full validation — all three tiers.  Used for desk/API saves.
    User,
    /// Plugin-originated GL post — skip tier-3 existence check.
    Plugin,
}

/// A single resolved party type row from the `PartyType` kernel table.
#[derive(Debug, Clone)]
pub struct ResolvedPartyType {
    pub party_type:   String,
    pub doctype_name: String,
    pub account_type: String, // "Receivable" | "Payable"
    pub plugin_id:    String,
}

/// Errors returned by party validation.
#[derive(Debug, thiserror::Error)]
pub enum PartyValidationError {
    #[error("Unknown party type: '{0}'. No plugin has registered this party type.")]
    UnknownPartyType(String),

    #[error(
        "Party type '{party_type}' is provided by the '{plugin_id}' plugin, which is not loaded. \
         Install and enable the plugin to use this party type."
    )]
    PluginNotLoaded { party_type: String, plugin_id: String },

    #[error("'{doctype}' '{name}' does not exist.")]
    PartyNotFound { doctype: String, name: String },
}

/// Validates a `(party_type, party)` pair from a JournalEntryAccount row.
///
/// # Arguments
/// * `party_type`  – value of the `party_type` field (e.g. `"Customer"`)
/// * `party`       – value of the `party` field (e.g. `"CUST-001"`)
/// * `resolved`    – the `PartyType` row fetched from the kernel table.
///                   `None` means the party_type was not found in the table.
/// * `plugin_loaded` – whether the plugin declared in `resolved.plugin_id` is
///                     currently in the `PluginRegistry`.
/// * `party_exists` – closure/bool indicating whether the party document exists
///                    in the DB.  Only called at `TrustLevel::User`.
/// * `trust`       – `TrustLevel::User` (desk save) or `TrustLevel::Plugin`
///                   (programmatic GL post from plugin hook).
pub fn validate_party(
    party_type:    &str,
    party:         &str,
    resolved:      Option<&ResolvedPartyType>,
    plugin_loaded: bool,
    party_exists:  bool,
    trust:         TrustLevel,
) -> Result<(), PartyValidationError> {
    // Tier 1 — party_type not registered at all.
    let resolved = resolved.ok_or_else(|| {
        PartyValidationError::UnknownPartyType(party_type.to_owned())
    })?;

    // Tier 2 — party_type registered but plugin not loaded.
    if !plugin_loaded {
        return Err(PartyValidationError::PluginNotLoaded {
            party_type: party_type.to_owned(),
            plugin_id:  resolved.plugin_id.clone(),
        });
    }

    // Tier 3 — existence check (skipped for plugin-originated posts).
    if trust == TrustLevel::User && !party_exists {
        return Err(PartyValidationError::PartyNotFound {
            doctype: resolved.doctype_name.clone(),
            name:    party.to_owned(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(plugin_id: &str) -> ResolvedPartyType {
        ResolvedPartyType {
            party_type:   "Customer".into(),
            doctype_name: "Customer".into(),
            account_type: "Receivable".into(),
            plugin_id:    plugin_id.into(),
        }
    }

    #[test]
    fn tier1_unknown_party_type() {
        let err = validate_party("Contractor", "CONT-001", None, false, false, TrustLevel::User)
            .unwrap_err();
        assert!(matches!(err, PartyValidationError::UnknownPartyType(_)));
    }

    #[test]
    fn tier2_plugin_not_loaded() {
        let r = resolved("selling");
        let err = validate_party("Customer", "CUST-001", Some(&r), false, false, TrustLevel::User)
            .unwrap_err();
        assert!(matches!(err, PartyValidationError::PluginNotLoaded { .. }));
    }

    #[test]
    fn tier3_party_not_found() {
        let r = resolved("selling");
        let err = validate_party("Customer", "CUST-9999", Some(&r), true, false, TrustLevel::User)
            .unwrap_err();
        assert!(matches!(err, PartyValidationError::PartyNotFound { .. }));
    }

    #[test]
    fn tier3_skipped_for_plugin_trust() {
        let r = resolved("selling");
        // party_exists = false but TrustLevel::Plugin → should pass
        validate_party("Customer", "CUST-9999", Some(&r), true, false, TrustLevel::Plugin)
            .expect("Plugin trust should skip existence check");
    }

    #[test]
    fn valid_user_save() {
        let r = resolved("selling");
        validate_party("Customer", "CUST-001", Some(&r), true, true, TrustLevel::User)
            .expect("All conditions met — should pass");
    }
}
