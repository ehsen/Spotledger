//! `Company` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn company_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Company".into(),
        module: "Setup".into(),
        is_single:      false,
        is_tree:        true,
        is_child:       false,
        is_submittable: false,
        track_changes:  true,
        fields: vec![
            DocField::new("details", "details", FieldType::SectionBreak),
            DocField::new("company_name", "Company", FieldType::Data)
                .required()
                .unique(),
            DocField::new("abbr", "Abbr", FieldType::Data)
                .required()
                .set_only_once(),
            DocField::new("default_currency", "Default Currency", FieldType::Link)
                .options("Currency")
                .required()
                .in_list(),
            DocField::new("country", "Country", FieldType::Link)
                .options("Country")
                .required()
                .in_list(),
            DocField::new("is_group", "Is Group", FieldType::Check)
                .in_list()
                .bold()
                .default_value("0"),
            DocField::new("default_holiday_list", "Default Holiday List", FieldType::Link)
                .options("Holiday List")
                .provided_by("hrms"), // Holiday List is provided by the HRMS plugin
            DocField::new("cb0", "cb0", FieldType::ColumnBreak),
            DocField::new("default_letter_head", "Default Letter Head", FieldType::Link)
                .options("Letter Head")
                .provided_by("setup"), // Letter Head is provided by the setup plugin
            DocField::new("tax_id", "Tax ID", FieldType::Data),
            DocField::new("domain", "Domain", FieldType::Data),
            DocField::new("date_of_establishment", "Date of Establishment", FieldType::Date),
            DocField::new("parent_company", "Parent Company", FieldType::Link)
                .options("Company"),
            DocField::new("reporting_currency", "Reporting Currency", FieldType::Link)
                .options("Currency")
                .read_only(),
            DocField::new("company_info", "Address & Contact", FieldType::SectionBreak),
            DocField::new("company_logo", "Company Logo", FieldType::AttachImage)
                .hidden(),
            DocField::new("date_of_incorporation", "Date of Incorporation", FieldType::Date),
            DocField::new("phone_no", "Phone No", FieldType::Data)
                .options("Phone"),
            DocField::new("email", "Email", FieldType::Data)
                .options("Email"),
            DocField::new("company_description", "Company Description", FieldType::LongText),
            DocField::new("column_break1", "column_break1", FieldType::ColumnBreak),
            DocField::new("date_of_commencement", "Date of Commencement", FieldType::Date),
            DocField::new("fax", "Fax", FieldType::Data)
                .options("Phone"),
            DocField::new("website", "Website", FieldType::Data),
            DocField::new("address_html", "address_html", FieldType::Html),
            DocField::new("registration_info", "registration_info", FieldType::SectionBreak),
            DocField::new("registration_details", "Registration Details", FieldType::Code)
                .description("Company registration numbers for your reference. Tax numbers etc."),
            DocField::new("lft", "Lft", FieldType::Int)
                .read_only()
                .hidden(),
            DocField::new("rgt", "Rgt", FieldType::Int)
                .read_only()
                .hidden(),
            DocField::new("old_parent", "old_parent", FieldType::Data)
                .read_only()
                .hidden(),
            DocField::new("accounts_tab", "Accounts", FieldType::TabBreak),
            DocField::new("section_break_28", "Chart of Accounts", FieldType::SectionBreak),
            DocField::new("create_chart_of_accounts_based_on", "Create Chart Of Accounts Based On", FieldType::Select)
                .options("\nStandard Template\nExisting Company"),
            DocField::new("existing_company", "Existing Company ", FieldType::Link)
                .options("Company"),
            DocField::new("column_break_26", "column_break_26", FieldType::ColumnBreak),
            DocField::new("chart_of_accounts", "Chart Of Accounts Template", FieldType::Select),
            DocField::new("default_settings", "Default Accounts", FieldType::SectionBreak),
            DocField::new("default_bank_account", "Default Bank Account", FieldType::Link)
                .options("Account"),
            DocField::new("default_cash_account", "Default Cash Account", FieldType::Link)
                .options("Account"),
            DocField::new("default_receivable_account", "Default Receivable Account", FieldType::Link)
                .options("Account"),
            DocField::new("default_payable_account", "Default Payable Account", FieldType::Link)
                .options("Account"),
            DocField::new("write_off_account", "Write Off Account", FieldType::Link)
                .options("Account"),
            DocField::new("unrealized_profit_loss_account", "Unrealized Profit / Loss Account", FieldType::Link)
                .options("Account"),
            DocField::new("column_break0", "column_break0", FieldType::ColumnBreak),
            DocField::new("allow_account_creation_against_child_company", "Allow Account Creation Against Child Company", FieldType::Check)
                .default_value("0"),
            DocField::new("default_expense_account", "Default Cost of Goods Sold Account", FieldType::Link)
                .options("Account"),
            DocField::new("default_income_account", "Default Income Account", FieldType::Link)
                .options("Account"),
            DocField::new("default_discount_account", "Default Payment Discount Account", FieldType::Link)
                .options("Account"),
            DocField::new("payment_terms", "Default Payment Terms Template", FieldType::Link)
                .options("Payment Terms Template"),
            DocField::new("cost_center", "Default Cost Center", FieldType::Link)
                .options("Cost Center"),
            DocField::new("default_finance_book", "Default Finance Book", FieldType::Link)
                .options("Finance Book"),
            DocField::new("exchange_gain__loss_section", "Exchange Gain / Loss", FieldType::SectionBreak),
            DocField::new("exchange_gain_loss_account", "Exchange Gain / Loss Account", FieldType::Link)
                .options("Account"),
            DocField::new("column_break_sttp", "column_break_sttp", FieldType::ColumnBreak),
            DocField::new("unrealized_exchange_gain_loss_account", "Unrealized Exchange Gain/Loss Account", FieldType::Link)
                .options("Account"),
            DocField::new("round_off_section", "Round Off", FieldType::SectionBreak),
            DocField::new("round_off_account", "Round Off Account", FieldType::Link)
                .options("Account"),
            DocField::new("round_off_cost_center", "Round Off Cost Center", FieldType::Link)
                .options("Cost Center"),
            DocField::new("column_break_jqfo", "column_break_jqfo", FieldType::ColumnBreak),
            DocField::new("round_off_for_opening", "Round Off for Opening", FieldType::Link)
                .options("Account"),
            DocField::new("deferred_accounting_section", "Deferred Accounting", FieldType::SectionBreak),
            DocField::new("default_deferred_revenue_account", "Default Deferred Revenue Account", FieldType::Link)
                .options("Account"),
            DocField::new("column_break_dcdl", "column_break_dcdl", FieldType::ColumnBreak),
            DocField::new("default_deferred_expense_account", "Default Deferred Expense Account", FieldType::Link)
                .options("Account"),
            DocField::new("advance_payments_section", "Advance Payments", FieldType::SectionBreak),
            DocField::new("book_advance_payments_in_separate_party_account", "Book Advance Payments in Separate Party Account", FieldType::Check)
                .default_value("0")
                .description("Enabling this option will allow you to record - <br><br> 1. Advances Received in a <b>Liability Account</b> instead of the <b>Asset Account</b><br><br>2. Advances Paid in an <b>Asset Account</b> instead of the <b> Liability Account</b>"),
            DocField::new("reconcile_on_advance_payment_date", "Reconcile on Advance Payment Date", FieldType::Check)
                .hidden()
                .default_value("0")
                .description("If <b>Enabled</b> - Reconciliation happens on the <b>Advance Payment posting date</b><br> If <b>Disabled</b> - Reconciliation happens on oldest of 2 Dates: <b>Invoice Date</b> or the <b>Advance Payment posting date</b><br> "),
            DocField::new("reconciliation_takes_effect_on", "Reconciliation Takes Effect On", FieldType::Select)
                .options("Advance Payment Date\nOldest Of Invoice Or Advance\nReconciliation Date")
                .default_value("Oldest Of Invoice Or Advance"),
            DocField::new("column_break_fwcf", "column_break_fwcf", FieldType::ColumnBreak),
            DocField::new("default_advance_received_account", "Default Advance Received Account", FieldType::Link)
                .options("Account")
                .description("Only 'Payment Entries' made against this advance account are supported."),
            DocField::new("default_advance_paid_account", "Default Advance Paid Account", FieldType::Link)
                .options("Account")
                .description("Only 'Payment Entries' made against this advance account are supported."),
            DocField::new("exchange_rate_revaluation_settings_section", "Exchange Rate Revaluation Settings", FieldType::SectionBreak),
            DocField::new("auto_exchange_rate_revaluation", "Auto Create Exchange Rate Revaluation", FieldType::Check)
                .default_value("0"),
            DocField::new("auto_err_frequency", "Frequency", FieldType::Select)
                .options("Daily\nWeekly\nMonthly"),
            DocField::new("submit_err_jv", "Submit ERR Journals?", FieldType::Check)
                .default_value("0")
                .description("Upon enabling this, the JV will be submitted for a different exchange rate."),
            DocField::new("budget_detail", "Budget Detail", FieldType::SectionBreak),
            DocField::new("exception_budget_approver_role", "Exception Budget Approver Role", FieldType::Link)
                .options("Role"),
            DocField::new("fixed_asset_defaults", "Fixed Asset Defaults", FieldType::SectionBreak),
            DocField::new("accumulated_depreciation_account", "Accumulated Depreciation Account", FieldType::Link)
                .options("Account"),
            DocField::new("depreciation_expense_account", "Depreciation Expense Account", FieldType::Link)
                .options("Account"),
            DocField::new("series_for_depreciation_entry", "Series for Asset Depreciation Entry (Journal Entry)", FieldType::Data),
            DocField::new("column_break_40", "column_break_40", FieldType::ColumnBreak),
            DocField::new("disposal_account", "Gain/Loss Account on Asset Disposal", FieldType::Link)
                .options("Account"),
            DocField::new("depreciation_cost_center", "Asset Depreciation Cost Center", FieldType::Link)
                .options("Cost Center"),
            DocField::new("capital_work_in_progress_account", "Capital Work In Progress Account", FieldType::Link)
                .options("Account"),
            DocField::new("asset_received_but_not_billed", "Asset Received But Not Billed", FieldType::Link)
                .options("Account"),
            DocField::new("accounts_closing_tab", "Accounts Closing", FieldType::TabBreak),
            DocField::new("accounts_closing_section", "accounts_closing_section", FieldType::SectionBreak),
            DocField::new("accounts_frozen_till_date", "Accounts Frozen Till Date", FieldType::Date)
                .description("Accounting entries are frozen up to this date. Only users with the specified role can create or modify entries before this date."),
            DocField::new("column_break_tawz", "column_break_tawz", FieldType::ColumnBreak),
            DocField::new("role_allowed_for_frozen_entries", "Roles Allowed to Set and Edit Frozen Account Entries", FieldType::Link)
                .options("Role"),
            DocField::new("buying_and_selling_tab", "Buying and Selling", FieldType::TabBreak),
            DocField::new("sales_settings", "Buying & Selling Settings", FieldType::SectionBreak),
            DocField::new("default_buying_terms", "Default Buying Terms", FieldType::Link)
                .options("Terms and Conditions")
                .provided_by("buying"), // Terms and Conditions is provided by buying plugin
            DocField::new("sales_monthly_history", "Sales Monthly History", FieldType::SmallText)
                .read_only()
                .hidden(),
            DocField::new("monthly_sales_target", "Monthly Sales Target", FieldType::Currency)
                .options("default_currency"),
            DocField::new("total_monthly_sales", "Total Monthly Sales", FieldType::Currency)
                .options("default_currency")
                .read_only(),
            DocField::new("column_break_goals", "column_break_goals", FieldType::ColumnBreak),
            DocField::new("default_selling_terms", "Default Selling Terms", FieldType::Link)
                .options("Terms and Conditions")
                .provided_by("selling"), // Terms and Conditions is provided by selling plugin
            DocField::new("default_sales_contact", "Default Sales Contact", FieldType::Link)
                .options("Contact")
                .provided_by("crm"), // Contact is provided by the CRM plugin
            DocField::new("default_warehouse_for_sales_return", "Default Warehouse for Sales Return", FieldType::Link)
                .options("Warehouse")
                .provided_by("stock"), // Warehouse is provided by the stock plugin
            DocField::new("credit_limit", "Credit Limit", FieldType::Currency)
                .options("default_currency"),
            DocField::new("transactions_annual_history", "Transactions Annual History", FieldType::Code)
                .read_only()
                .hidden(),
            DocField::new("purchase_expense_section", "Purchase Expense", FieldType::SectionBreak),
            DocField::new("purchase_expense_account", "Purchase Expense Account", FieldType::Link)
                .options("Account"),
            DocField::new("service_expense_account", "Service Expense Account", FieldType::Link)
                .options("Account")
                .description("For service item"),
            DocField::new("column_break_ereg", "column_break_ereg", FieldType::ColumnBreak),
            DocField::new("purchase_expense_contra_account", "Purchase Expense Contra Account", FieldType::Link)
                .options("Account"),
            DocField::new("stock_tab", "Stock and Manufacturing", FieldType::TabBreak),
            DocField::new("auto_accounting_for_stock_settings", "Stock Settings", FieldType::SectionBreak),
            DocField::new("enable_perpetual_inventory", "Enable Perpetual Inventory", FieldType::Check)
                .in_list()
                .default_value("1"),
            DocField::new("enable_item_wise_inventory_account", "Enable Item-wise Inventory Account", FieldType::Check)
                .default_value("0")
                .description("If enabled, the system will use the inventory account set in the Item Master or Item Group or Brand. Otherwise, it will use the inventory account set in the Warehouse."),
            DocField::new("enable_provisional_accounting_for_non_stock_items", "Enable Provisional Accounting For Non Stock Items", FieldType::Check)
                .default_value("0"),
            DocField::new("default_inventory_account", "Default Inventory Account", FieldType::Link)
                .options("Account"),
            DocField::new("valuation_method", "Default Stock Valuation Method", FieldType::Select)
                .options("FIFO\nMoving Average\nLIFO")
                .required()
                .default_value("FIFO"),
            DocField::new("column_break_32", "column_break_32", FieldType::ColumnBreak),
            DocField::new("stock_adjustment_account", "Stock Adjustment Account", FieldType::Link)
                .options("Account"),
            DocField::new("stock_received_but_not_billed", "Stock Received But Not Billed", FieldType::Link)
                .options("Account"),
            DocField::new("default_provisional_account", "Default Provisional Account", FieldType::Link)
                .options("Account"),
            DocField::new("default_in_transit_warehouse", "Default In-Transit Warehouse", FieldType::Link)
                .options("Warehouse")
                .provided_by("stock"),
            DocField::new("manufacturing_section", "Manufacturing", FieldType::SectionBreak),
            DocField::new("default_operating_cost_account", "Default Operating Cost Account", FieldType::Link)
                .options("Account"),
            DocField::new("column_break_9prc", "column_break_9prc", FieldType::ColumnBreak),
            DocField::new("default_wip_warehouse", " Default Work In Progress Warehouse ", FieldType::Link)
                .options("Warehouse")
                .provided_by("stock"),
            DocField::new("default_fg_warehouse", "Default Finished Goods Warehouse", FieldType::Link)
                .options("Warehouse")
                .provided_by("stock"),
            DocField::new("default_scrap_warehouse", "Default Scrap Warehouse", FieldType::Link)
                .options("Warehouse")
                .provided_by("stock"),
            DocField::new("dashboard_tab", "Dashboard", FieldType::TabBreak),
        ],
        permissions: vec![
            Permission {
                role: "System Manager".into(),
                read: true,
                write: true,
                create: true,
                delete: true,
                submit: false,
                cancel: false,
                amend: false,
                report: true,
                import: false,
                export: false,
                print: true,
                email: true,
                share: true,
            },
            Permission::read_only("Accounts User"),
            Permission::read_only("Employee"),
            Permission::read_only("Sales User"),
            Permission::read_only("Purchase User"),
            Permission::read_only("Stock User"),
            Permission::read_only("Projects User"),
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
                import: false,
                export: true,
                print: true,
                email: true,
                share: true,
            },
            Permission {
                role: "Auditor".into(),
                read: false,
                write: false,
                create: false,
                delete: false,
                submit: false,
                cancel: false,
                amend: false,
                report: false,
                import: false,
                export: false,
                print: false,
                email: false,
                share: false,
            },
        ],
        title_field:   None,
        search_fields: vec![],
        sort_field:    Some("creation".into()),
        sort_order:    Some("ASC".into()),
        autoname:      Some("field:company_name".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Company",
    meta: company_meta,
});
