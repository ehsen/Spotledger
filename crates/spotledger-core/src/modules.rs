/// Canonical framework module names.
///
/// Every built-in DocType that ships with Spotledger/Frappe belongs to one of
/// these modules.  Using typed constants (rather than raw `&str` literals)
/// makes refactoring safe and gives a single source of truth for the set of
/// framework modules that must be seeded into `tabModule_Def` at startup.
///
/// App-specific modules (e.g. "Accounts", "Buying") are deliberately excluded
/// here — they are installed via app hooks and live in `tabModule_Def` as
/// regular records created at install time.
///
/// # Usage
/// ```ignore
/// use spotledger_core::modules::FM;
///
/// let meta = DocTypeMeta { module: FM::CORE.into(), .. };
/// ```
pub struct FM;

impl FM {
    // ── Framework / tier-0 modules ────────────────────────────────────────────
    pub const CORE:       &'static str = "Core";
    pub const CUSTOM:     &'static str = "Custom";
    pub const DESK:       &'static str = "Desk";
    pub const EMAIL:      &'static str = "Email";
    pub const GEO:        &'static str = "Geo";
    pub const PRINTING:   &'static str = "Printing";
    pub const WEBSITE:    &'static str = "Website";
    pub const SETUP:      &'static str = "Setup";
    pub const CONTACTS:   &'static str = "Contacts";

    // ── App-level modules (used by spotledger-* crates) ───────────────────────
    pub const ACCOUNTS:   &'static str = "Accounts";
    pub const AUTOMATION: &'static str = "Automation";

    /// Every framework module that must be pre-seeded into `tabModule_Def`
    /// at server startup so that the sidebar and module-picker work without
    /// an explicit `install-app` step.
    ///
    /// Each tuple is `(module_name, app_name, label, icon, order, show_in_menu)`.
    pub const FRAMEWORK_MODULES: &'static [ModuleSeed] = &[
        // Tier-0 modules: always present, hard-coded in the binary.
        ModuleSeed { name: Self::CORE,   app: "spotledger", label: "Core",   icon: "cpu",     order: 10, show_in_menu: false },
        ModuleSeed { name: Self::CUSTOM, app: "spotledger", label: "Custom", icon: "sliders", order: 20, show_in_menu: false },
        // NOTE: Contacts, Geo, Email, Desk, Website, Printing, Automation, Setup,
        // and Accounts are now provided by DB-native apps (spotledger-core, erpnext)
        // and are seeded via `install-app` / `new-site` auto-install rather than
        // being hard-coded here. Removing them from this list does not delete
        // the module name constants (FM::CONTACTS etc.) — those remain for
        // use in DocType metadata.
    ];
}

/// Seed data for a single Module Def record.
pub struct ModuleSeed {
    pub name:         &'static str,
    pub app:          &'static str,
    pub label:        &'static str,
    pub icon:         &'static str,
    pub order:        i32,
    pub show_in_menu: bool,
}
