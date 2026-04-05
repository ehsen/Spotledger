//! Integration tests for the plugin system.
//!
//! Tests for:
//! - Plugin registry loading and metadata
//! - Version resolver compatibility checks
//! - Database adapter stubs
//! - GL engine adapter stubs

#[cfg(test)]
mod tests {
    use spotledger_plugins::{
        DbAdapter, GlAdapter, PluginManifest, VersioningResolver, VersioningResolver as VR,
    };

    #[test]
    fn test_plugin_manifest_compatibility() {
        let manifest = PluginManifest {
            id: spotledger_plugins::PluginId("selling".into()),
            abi_version: 1,
            name: "Selling".into(),
            version: "1.0".into(),
            dependencies: vec![],
        };
        assert!(manifest.is_compatible());
    }

    #[test]
    fn test_versioning_resolver() {
        let mut resolver = VR::new();
        assert!(resolver.validate().is_empty());

        resolver.register(PluginManifest {
            id: spotledger_plugins::PluginId("selling".into()),
            abi_version: 1,
            name: "Selling".into(),
            version: "1.0".into(),
            dependencies: vec![],
        });

        assert!(resolver.get_manifest("selling").is_some());
    }

    #[test]
    fn test_db_adapter_operations() {
        let db = DbAdapter::new();

        // Test get_doc
        let doc = db.get_doc("User", "Administrator");
        assert!(doc.is_ok());
        let doc_val = doc.unwrap();
        assert_eq!(doc_val["doctype"], "User");

        // Test save_doc
        let saved = db.save_doc("User", "NewUser", &serde_json::json!({}));
        assert!(saved.is_ok());

        // Test delete_doc
        let deleted = db.delete_doc("User", "TempUser");
        assert!(deleted.is_ok());

        // Test has_permission
        let perm = db.has_permission("Administrator", "read", "User", "Administrator");
        assert!(perm.is_ok());
        assert!(perm.unwrap());
    }

    #[test]
    fn test_gl_adapter_operations() {
        let gl = GlAdapter::new();

        // Test balanced GL entries
        let payload = serde_json::json!({
            "company": "Test Co",
            "posting_date": "2026-01-01",
            "entries": [
                { "account": "Debtors", "debit": 1000.0, "credit": 0.0 },
                { "account": "Sales", "debit": 0.0, "credit": 1000.0 }
            ]
        });

        let result = gl.make_gl_entries(&payload);
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(!entries.is_empty());

        // Test reverse
        let reversed = gl.reverse_gl_entries("SI", "SI-001");
        assert!(reversed.is_ok());

        // Test account balance
        let balance = gl.get_account_balance("Debtors", "Test Co", "2026-01-01");
        assert!(balance.is_ok());

        // Test fiscal year
        let fy = gl.get_fiscal_year("Test Co", "2026-01-01");
        assert!(fy.is_ok());

        // Test exchange rate
        let rate = gl.get_exchange_rate("USD", "INR", "2026-01-01");
        assert!(rate.is_ok());
        assert_eq!(rate.unwrap(), 1.0);
    }

    #[test]
    fn test_gl_adapter_unbalanced_entries() {
        let gl = GlAdapter::new();

        let payload = serde_json::json!({
            "company": "Test Co",
            "entries": [
                { "account": "Debtors", "debit": 1000.0, "credit": 0.0 },
                { "account": "Sales", "debit": 0.0, "credit": 500.0 }
            ]
        });

        let result = gl.make_gl_entries(&payload);
        assert!(result.is_err());
    }
}
