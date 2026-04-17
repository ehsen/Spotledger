//! `Account` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;
use spotledger_core::modules::FM;

pub fn account_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Account".into(),
        module: FM::ACCOUNTS.into(),
        is_single:      false,
        is_tree:        true,
        is_child:       false,
        is_submittable: false,
        track_changes:  true,
        fields: vec![
            DocField::new("properties", "properties", FieldType::SectionBreak),
            DocField::new("column_break0", "column_break0", FieldType::ColumnBreak),
            DocField::new("disabled", "Disable", FieldType::Check)
                .default_value("0"),
            DocField::new("account_name", "Account Name", FieldType::Data)
                .required()
                .in_list(),
            DocField::new("account_number", "Account Number", FieldType::Data)
                .in_list()
                .in_standard_filter(),
            DocField::new("is_group", "Is Group", FieldType::Check)
                .default_value("0"),
            DocField::new("company", "Company", FieldType::Link)
                .options("Company")
                .required()
                .in_standard_filter()
                .fetch_if_empty()
                .fetch_from("parent_account.company"),
            DocField::new("root_type", "Root Type", FieldType::Select)
                .options("\nAsset\nLiability\nIncome\nExpense\nEquity")
                .in_standard_filter()
                .read_only(),
            DocField::new("report_type", "Report Type", FieldType::Select)
                .options("\nBalance Sheet\nProfit and Loss")
                .in_standard_filter()
                .read_only(),
            DocField::new("account_currency", "Currency", FieldType::Link)
                .options("Currency"),
            DocField::new("column_break1", "column_break1", FieldType::ColumnBreak),
            DocField::new("parent_account", "Parent Account", FieldType::Link)
                .options("Account")
                .required(),
            DocField::new("account_category", "Account Category", FieldType::Link)
                .options("Account Category")
                .description("Used with Financial Report Template"),
            DocField::new("account_type", "Account Type", FieldType::Select)
                .options("\nAccumulated Depreciation\nAsset Received But Not Billed\nBank\nCash\nChargeable\nCapital Work in Progress\nCost of Goods Sold\nCurrent Asset\nCurrent Liability\nDepreciation\nDirect Expense\nDirect Income\nEquity\nExpense Account\nExpenses Included In Asset Valuation\nExpenses Included In Valuation\nFixed Asset\nIncome Account\nIndirect Expense\nIndirect Income\nLiability\nPayable\nReceivable\nRound Off\nRound Off for Opening\nStock\nStock Adjustment\nStock Received But Not Billed\nService Received But Not Billed\nTax\nTemporary")
                .in_standard_filter()
                .description("Setting Account Type helps in selecting this Account in transactions."),
            DocField::new("tax_rate", "Tax Rate", FieldType::Float)
                .description("Rate at which this tax is applied"),
            DocField::new("freeze_account", "Frozen", FieldType::Select)
                .options("No\nYes")
                .description("If the account is frozen, entries are allowed to restricted users."),
            DocField::new("balance_must_be", "Balance must be", FieldType::Select)
                .options("\nDebit\nCredit"),
            DocField::new("lft", "Lft", FieldType::Int)
                .read_only()
                .hidden(),
            DocField::new("rgt", "Rgt", FieldType::Int)
                .read_only()
                .hidden(),
            DocField::new("old_parent", "Old Parent", FieldType::Data)
                .read_only()
                .hidden(),
            DocField::new("include_in_gross", "Include in gross", FieldType::Check)
                .default_value("0"),
        ],
        permissions: vec![
            Permission {
                role: "Accounts User".into(),
                read: true,
                write: true,
                create: true,
                delete: true,
                submit: false,
                cancel: false,
                amend: false,
                report: true,
                import: true,
                export: true,
                print: true,
                email: true,
                share: true,
            },
            Permission::read_only("Auditor"),
            Permission::read_only("Sales User"),
            Permission::read_only("Purchase User"),
            Permission {
                role: "Accounts Manager".into(),
                read: true,
                write: true,
                create: true,
                delete: true,
                submit: false,
                cancel: false,
                amend: false,
                report: true,
                import: true,
                export: true,
                print: true,
                email: true,
                share: true,
            },
        ],
        title_field:   None,
        search_fields: vec![],
        sort_field:    Some("creation".into()),
        sort_order:    Some("ASC".into()),
        autoname:      None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Account",
    meta: account_meta,
});
