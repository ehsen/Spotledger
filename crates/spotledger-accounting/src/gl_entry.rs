//! `GL Entry` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn gl_entry_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "GL Entry".into(),
        module: "Accounts".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("dates_section", "Dates", FieldType::SectionBreak),
            DocField::new("posting_date", "Posting Date", FieldType::Date)
                .in_list(),
            DocField::new("transaction_date", "Transaction Date", FieldType::Date)
                .in_list(),
            DocField::new("column_break_avko", "column_break_avko", FieldType::ColumnBreak),
            DocField::new("fiscal_year", "Fiscal Year", FieldType::Link)
                .options("Fiscal Year"),
            DocField::new("due_date", "Due Date", FieldType::Date),
            DocField::new("account_details_section", "Account Details", FieldType::SectionBreak),
            DocField::new("account", "Account", FieldType::Link)
                .options("Account")
                .in_list()
                .in_standard_filter(),
            DocField::new("account_currency", "Account Currency", FieldType::Link)
                .options("Currency"),
            DocField::new("column_break_ifvf", "column_break_ifvf", FieldType::ColumnBreak),
            DocField::new("against", "Against", FieldType::Text),
            DocField::new("party_type", "Party Type", FieldType::Link)
                .options("PartyType"), // kernel-level party registry — not DocType
            DocField::new("party", "Party", FieldType::DynamicLink)
                .options("party_type")
                .in_standard_filter(),
            DocField::new("transaction_details_section", "Transaction Details", FieldType::SectionBreak),
            DocField::new("voucher_type", "Voucher Type", FieldType::Link)
                .options("DocType"),
            DocField::new("voucher_no", "Voucher No", FieldType::DynamicLink)
                .options("voucher_type")
                .in_standard_filter(),
            DocField::new("voucher_subtype", "Voucher Subtype", FieldType::SmallText),
            DocField::new("transaction_currency", "Transaction Currency", FieldType::Link)
                .options("Currency"),
            DocField::new("column_break_dpsx", "column_break_dpsx", FieldType::ColumnBreak),
            DocField::new("against_voucher_type", "Against Voucher Type", FieldType::Link)
                .options("DocType"),
            DocField::new("against_voucher", "Against Voucher", FieldType::DynamicLink)
                .options("against_voucher_type"),
            DocField::new("voucher_detail_no", "Voucher Detail No", FieldType::Data)
                .read_only(),
            DocField::new("transaction_exchange_rate", "Transaction Exchange Rate", FieldType::Float),
            DocField::new("reporting_currency_exchange_rate", "Reporting Currency Exchange Rate", FieldType::Float),
            DocField::new("amounts_section", "Amounts", FieldType::SectionBreak),
            DocField::new("debit_in_account_currency", "Debit Amount in Account Currency", FieldType::Currency)
                .options("account_currency"),
            DocField::new("debit", "Debit Amount", FieldType::Currency)
                .options("Company:company:default_currency"),
            DocField::new("debit_in_transaction_currency", "Debit Amount in Transaction Currency", FieldType::Currency)
                .options("transaction_currency"),
            DocField::new("debit_in_reporting_currency", "Debit Amount in Reporting Currency", FieldType::Currency)
                .options("Company:company:reporting_currency"),
            DocField::new("column_break_bm1w", "column_break_bm1w", FieldType::ColumnBreak),
            DocField::new("credit_in_account_currency", "Credit Amount in Account Currency", FieldType::Currency)
                .options("account_currency"),
            DocField::new("credit", "Credit Amount", FieldType::Currency)
                .options("Company:company:default_currency"),
            DocField::new("credit_in_transaction_currency", "Credit Amount in Transaction Currency", FieldType::Currency)
                .options("transaction_currency"),
            DocField::new("credit_in_reporting_currency", "Credit Amount in Reporting Currency", FieldType::Currency)
                .options("Company:company:reporting_currency"),
            DocField::new("dimensions_section", "Dimensions", FieldType::SectionBreak),
            DocField::new("cost_center", "Cost Center", FieldType::Link)
                .options("Cost Center")
                .in_list(),
            DocField::new("column_break_lmnm", "column_break_lmnm", FieldType::ColumnBreak),
            DocField::new("project", "Project", FieldType::Link)
                .options("Project")
                .provided_by("projects"),  // Project is a plugin-provided DocType
            DocField::new("more_info_section", "More Info", FieldType::SectionBreak),
            DocField::new("finance_book", "Finance Book", FieldType::Link)
                .options("Finance Book"),
            DocField::new("company", "Company", FieldType::Link)
                .options("Company"),
            DocField::new("is_opening", "Is Opening", FieldType::Select)
                .options("No\nYes"),
            DocField::new("is_advance", "Is Advance", FieldType::Select)
                .options("No\nYes"),
            DocField::new("column_break_8abq", "column_break_8abq", FieldType::ColumnBreak),
            DocField::new("to_rename", "To Rename", FieldType::Check)
                .hidden()
                .default_value("1"),
            DocField::new("is_cancelled", "Is Cancelled", FieldType::Check)
                .default_value("0"),
            DocField::new("remarks", "Remarks", FieldType::Text),
        ],
        permissions: vec![
            Permission::read_only("Accounts User"),
            Permission::read_only("Accounts Manager"),
            Permission::read_only("Auditor"),
        ],
        title_field:   None,
        search_fields: vec![],
        sort_field:    Some("creation".into()),
        sort_order:    Some("DESC".into()),
        autoname:      Some("ACC-GLE-.YYYY.-.#####".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "GL Entry",
    meta: gl_entry_meta,
});
