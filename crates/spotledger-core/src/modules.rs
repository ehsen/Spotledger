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
/// ```
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
        ModuleSeed { name: Self::CORE,       app: "frappe", label: "Core",       icon: "cpu",              order:  10, show_in_menu: false },
        ModuleSeed { name: Self::CUSTOM,     app: "frappe", label: "Custom",     icon: "sliders",          order:  20, show_in_menu: false },
        ModuleSeed { name: Self::DESK,       app: "frappe", label: "Desk",       icon: "layout-dashboard", order:  30, show_in_menu: true  },
        ModuleSeed { name: Self::EMAIL,      app: "frappe", label: "Email",      icon: "mail",             order:  40, show_in_menu: false },
        ModuleSeed { name: Self::GEO,        app: "frappe", label: "Geo",        icon: "globe",            order:  50, show_in_menu: false },
        ModuleSeed { name: Self::PRINTING,   app: "frappe", label: "Printing",   icon: "printer",          order:  60, show_in_menu: false },
        ModuleSeed { name: Self::WEBSITE,    app: "frappe", label: "Website",    icon: "globe",            order:  70, show_in_menu: false },
        ModuleSeed { name: Self::SETUP,      app: "frappe", label: "Setup",      icon: "settings",         order:  80, show_in_menu: true  },
        ModuleSeed { name: Self::CONTACTS,   app: "frappe", label: "Contacts",   icon: "users",            order:  90, show_in_menu: true  },
        ModuleSeed { name: Self::ACCOUNTS,   app: "erpnext", label: "Accounts",  icon: "calculator",       order: 100, show_in_menu: true  },
        ModuleSeed { name: Self::AUTOMATION, app: "frappe", label: "Automation", icon: "zap",              order: 110, show_in_menu: false },
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
