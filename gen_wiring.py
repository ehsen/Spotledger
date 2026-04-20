"""Regenerate all wiring.surql files with clean static stage IDs."""
import os

BASE = r"F:\Sources\spotledger\apps\spotledger-finance\spotledger-finance\surql\doctypes"

def write(doctype, content):
    path = os.path.join(BASE, doctype, "wiring.surql")
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)
    print(f"Wrote {doctype}/wiring.surql")

def rel(stage, node, ord_):
    return (
        f"IF (SELECT id FROM has_node WHERE in = pipeline_stage:{stage} AND out = pipeline_node:{node} LIMIT 1) = [] {{\n"
        f"    RELATE pipeline_stage:{stage} -> has_node -> pipeline_node:{node} CONTENT {{ ord: {ord_}, config: {{}} }};\n"
        f"}};\n\n"
    )

def upsert_node(nid, fn_name):
    return (
        f"IF (SELECT id FROM pipeline_node:{nid} LIMIT 1) = [] {{\n"
        f"    UPSERT pipeline_node:{nid} CONTENT {{\n"
        f"        name:    \"{nid}\",\n"
        f"        fn_name: \"{fn_name}\"\n"
        f"    }};\n"
        f"}};\n\n"
    )

# ── Currency ────────────────────────────────────────────────────────────────
write("currency",
    "-- Currency — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Currency\", \"tabCurrency\", false);\n\n"
    + upsert_node("currency_validate_code_format", "fn::validate::currency_code_format")
    + upsert_node("currency_validate_single_base",  "fn::validate::currency_single_base")
    + rel("tabCurrency_save_validate", "currency_validate_code_format", 5)
    + rel("tabCurrency_save_validate", "currency_validate_single_base",  10)
)

# ── Company ─────────────────────────────────────────────────────────────────
write("company",
    "-- Company — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Company\", \"tabCompany\", false);\n\n"
    + upsert_node("company_validate_abbr_format", "fn::validate::company_abbr_format")
    + upsert_node("company_validate_abbr_unique", "fn::validate::company_abbr_unique")
    + rel("tabCompany_save_validate", "company_validate_abbr_format", 5)
    + rel("tabCompany_save_validate", "company_validate_abbr_unique",  10)
)

# ── Fiscal Year ──────────────────────────────────────────────────────────────
write("fiscal_year",
    "-- Fiscal Year — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Fiscal Year\", \"tabFiscal_Year\", false);\n\n"
    + upsert_node("fiscal_year_validate_dates",      "fn::validate::fiscal_year_dates")
    + upsert_node("fiscal_year_validate_no_overlap", "fn::validate::fiscal_year_no_overlap")
    + rel("tabFiscal_Year_save_validate", "fiscal_year_validate_dates",      5)
    + rel("tabFiscal_Year_save_validate", "fiscal_year_validate_no_overlap", 10)
)

# ── Accounting Period ────────────────────────────────────────────────────────
write("accounting_period",
    "-- Accounting Period — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Accounting Period\", \"tabAccounting_Period\", false);\n\n"
    + upsert_node("accounting_period_validate_dates",  "fn::validate::accounting_period_dates")
    + upsert_node("accounting_period_validate_in_fy",  "fn::validate::accounting_period_in_fiscal_year")
    + rel("tabAccounting_Period_save_validate", "accounting_period_validate_dates",  5)
    + rel("tabAccounting_Period_save_validate", "accounting_period_validate_in_fy",  10)
)

# ── Cost Center ──────────────────────────────────────────────────────────────
write("cost_center",
    "-- Cost Center — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Cost Center\", \"tabCost_Center\", false);\n\n"
    + upsert_node("cost_center_validate_code_unique", "fn::validate::cost_center_code_unique")
    + upsert_node("cost_center_relate_parent",        "fn::on_save::cost_center_relate_parent")
    + rel("tabCost_Center_save_validate", "cost_center_validate_code_unique", 5)
    + rel("tabCost_Center_save_persist",  "cost_center_relate_parent",        90)
)

# ── Payment Terms ────────────────────────────────────────────────────────────
write("payment_terms",
    "-- Payment Terms — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Payment Terms\", \"tabPayment_Terms\", false);\n\n"
    + upsert_node("payment_terms_validate_days", "fn::validate::payment_terms_days")
    + rel("tabPayment_Terms_save_validate", "payment_terms_validate_days", 5)
)

# ── Party ────────────────────────────────────────────────────────────────────
write("party",
    "-- Party — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Party\", \"tabParty\", false);\n\n"
    + upsert_node("party_validate_roles",          "fn::validate::party_roles")
    + upsert_node("party_validate_tax_id_unique",  "fn::validate::party_tax_id_unique")
    + upsert_node("party_role_config_autocreate",  "fn::on_save::party_role_config_autocreate")
    + rel("tabParty_save_validate", "party_validate_roles",         5)
    + rel("tabParty_save_validate", "party_validate_tax_id_unique", 10)
    + rel("tabParty_save_persist",  "party_role_config_autocreate", 90)
)

# ── Party Role Config ────────────────────────────────────────────────────────
write("party_role_config",
    "-- Party Role Config — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Party Role Config\", \"tabParty_Role_Config\", false);\n\n"
    + upsert_node("party_role_config_validate_unique",        "fn::validate::party_role_config_unique")
    + upsert_node("party_role_config_validate_account_types", "fn::validate::party_role_config_account_types")
    + rel("tabParty_Role_Config_save_validate", "party_role_config_validate_unique",        5)
    + rel("tabParty_Role_Config_save_validate", "party_role_config_validate_account_types", 10)
)

# ── Chart of Accounts ────────────────────────────────────────────────────────
write("chart_of_accounts",
    "-- Chart of Accounts — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Chart of Accounts\", \"tabChart_of_Accounts\", false);\n\n"
    + upsert_node("coa_validate_parent_required", "fn::validate::coa_parent_required")
    + upsert_node("coa_validate_parent_type",     "fn::validate::coa_parent_type")
    + upsert_node("coa_validate_single_default",  "fn::validate::coa_single_default")
    + rel("tabChart_of_Accounts_save_validate", "coa_validate_parent_required", 5)
    + rel("tabChart_of_Accounts_save_validate", "coa_validate_parent_type",     10)
    + rel("tabChart_of_Accounts_save_validate", "coa_validate_single_default",  15)
)

# ── Account Group ────────────────────────────────────────────────────────────
write("account_group",
    "-- Account Group — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Account Group\", \"tabAccount_Group\", false);\n\n"
    + upsert_node("account_group_compute_classification", "fn::compute::account_group_derive_classification")
    + upsert_node("account_group_validate_number_range",  "fn::validate::account_group_number_range")
    + upsert_node("account_group_validate_code_unique",   "fn::validate::account_group_code_unique")
    + rel("tabAccount_Group_save_validate", "account_group_validate_number_range",  5)
    + rel("tabAccount_Group_save_validate", "account_group_validate_code_unique",   10)
    + rel("tabAccount_Group_save_compute",  "account_group_compute_classification", 5)
)

# ── Account ──────────────────────────────────────────────────────────────────
write("account",
    "-- Account — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Account\", \"tabAccount\", false);\n\n"
    + upsert_node("account_compute_from_group",           "fn::compute::account_derive_from_group")
    + upsert_node("account_validate_number_in_range",     "fn::validate::account_number_in_group_range")
    + upsert_node("account_validate_group_matches_coa",   "fn::validate::account_group_matches_coa")
    + upsert_node("account_validate_retained_earnings",   "fn::validate::account_retained_earnings_required")
    + upsert_node("account_validate_type_immutable",      "fn::validate::account_type_immutable")
    + upsert_node("account_relate_parent",                "fn::on_save::account_relate_parent")
    + upsert_node("account_extend_companies",             "fn::on_save::account_extend_companies")
    + "-- save/validate: structural checks that don't depend on computed fields\n"
    + rel("tabAccount_save_validate", "account_validate_group_matches_coa",  5)
    + rel("tabAccount_save_validate", "account_validate_type_immutable",     10)
    + "-- save/compute: derive from group, then validate derived values\n"
    + rel("tabAccount_save_compute",  "account_compute_from_group",          5)
    + rel("tabAccount_save_compute",  "account_validate_number_in_range",    10)
    + rel("tabAccount_save_compute",  "account_validate_retained_earnings",  15)
    + "-- save/persist: graph edges and Account Company records\n"
    + rel("tabAccount_save_persist",  "account_relate_parent",               85)
    + rel("tabAccount_save_persist",  "account_extend_companies",            90)
)

# ── Account Company ──────────────────────────────────────────────────────────
write("account_company",
    "-- Account Company — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Account Company\", \"tabAccount_Company\", false);\n\n"
    + upsert_node("account_company_validate_unique",     "fn::validate::account_company_unique")
    + upsert_node("account_company_compute_open_item",   "fn::compute::account_company_derive_open_item")
    + rel("tabAccount_Company_save_validate", "account_company_validate_unique",   5)
    + rel("tabAccount_Company_save_compute",  "account_company_compute_open_item", 5)
)

# ── Financial Statement Version ──────────────────────────────────────────────
write("financial_statement_version",
    "-- Financial Statement Version — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Financial Statement Version\", \"tabFinancial_Statement_Version\", false);\n\n"
    + upsert_node("fsv_validate_single_default", "fn::validate::fsv_single_default_per_coa")
    + rel("tabFinancial_Statement_Version_save_validate", "fsv_validate_single_default", 5)
)

# ── FSV Node ─────────────────────────────────────────────────────────────────
write("fsv_node",
    "-- FSV Node — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"FSV Node\", \"tabFSV_Node\", false);\n\n"
    + upsert_node("fsv_node_validate_sequence_unique", "fn::validate::fsv_node_sequence_unique")
    + rel("tabFSV_Node_save_validate", "fsv_node_validate_sequence_unique", 5)
)

# ── FSV Account Mapping ──────────────────────────────────────────────────────
write("fsv_account_mapping",
    "-- FSV Account Mapping — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"FSV Account Mapping\", \"tabFSV_Account_Mapping\", false);\n\n"
    + upsert_node("fsv_mapping_validate_range", "fn::validate::fsv_mapping_range")
    + rel("tabFSV_Account_Mapping_save_validate", "fsv_mapping_validate_range", 5)
)

# ── GL Entry ─────────────────────────────────────────────────────────────────
write("gl_entry",
    "-- GL Entry — pipeline wiring (submittable)\n\n"
    "RETURN fn::pipeline::wire_generic(\"GL Entry\", \"tabGL_Entry\", true);\n\n"
    + upsert_node("gl_entry_validate_account",       "fn::validate::gl_entry_account")
    + upsert_node("gl_entry_validate_period_open",   "fn::validate::gl_entry_period_open")
    + upsert_node("gl_entry_validate_party_required","fn::validate::gl_entry_party_required")
    + upsert_node("gl_entry_validate_amounts",       "fn::validate::gl_entry_amounts")
    + upsert_node("gl_entry_validate_immutable",     "fn::validate::gl_entry_immutable")
    + upsert_node("gl_entry_compute_derive_period",  "fn::compute::gl_entry_derive_period")
    + upsert_node("gl_entry_on_cancel_reverse",      "fn::on_cancel::gl_entry_reverse")
    + "-- save/validate\n"
    + rel("tabGL_Entry_save_validate", "gl_entry_validate_immutable",      2)
    + rel("tabGL_Entry_save_validate", "gl_entry_validate_account",        5)
    + rel("tabGL_Entry_save_validate", "gl_entry_validate_amounts",        10)
    + rel("tabGL_Entry_save_validate", "gl_entry_validate_period_open",    15)
    + rel("tabGL_Entry_save_validate", "gl_entry_validate_party_required", 20)
    + "-- save/compute\n"
    + rel("tabGL_Entry_save_compute",  "gl_entry_compute_derive_period",   5)
    + "-- submit/validate (same checks, immutable not needed post-create)\n"
    + rel("tabGL_Entry_submit_validate", "gl_entry_validate_account",        5)
    + rel("tabGL_Entry_submit_validate", "gl_entry_validate_amounts",        10)
    + rel("tabGL_Entry_submit_validate", "gl_entry_validate_period_open",    15)
    + rel("tabGL_Entry_submit_validate", "gl_entry_validate_party_required", 20)
    + "-- cancel/on_cancel: reverse debits/credits before generic mark_cancelled (ord 99)\n"
    + rel("tabGL_Entry_cancel_on_cancel", "gl_entry_on_cancel_reverse", 10)
)

# ── Journal Entry Account (child table — no wiring) ──────────────────────────
# Already correct — skip

# ── Journal Entry ────────────────────────────────────────────────────────────
write("journal_entry",
    "-- Journal Entry — pipeline wiring (submittable)\n\n"
    "RETURN fn::pipeline::wire_generic(\"Journal Entry\", \"tabJournal_Entry\", true);\n\n"
    + upsert_node("je_validate_has_accounts",    "fn::validate::je_has_accounts")
    + upsert_node("je_validate_account_validity","fn::validate::je_account_validity")
    + upsert_node("je_validate_period_open",     "fn::validate::je_period_open")
    + upsert_node("je_validate_balance",         "fn::validate::je_balance")
    + upsert_node("je_compute_totals",           "fn::compute::je_totals")
    + upsert_node("je_on_submit_create_gl",      "fn::on_submit::je_create_gl_entries")
    + upsert_node("je_on_cancel_gl_entries",     "fn::on_cancel::je_cancel_gl_entries")
    + "-- save/validate\n"
    + rel("tabJournal_Entry_save_validate", "je_validate_has_accounts",    5)
    + rel("tabJournal_Entry_save_validate", "je_validate_account_validity", 10)
    + rel("tabJournal_Entry_save_validate", "je_validate_period_open",     15)
    + "-- save/compute\n"
    + rel("tabJournal_Entry_save_compute",  "je_compute_totals",           5)
    + "-- submit/validate: full balance check required\n"
    + rel("tabJournal_Entry_submit_validate", "je_validate_has_accounts",    5)
    + rel("tabJournal_Entry_submit_validate", "je_validate_account_validity", 10)
    + rel("tabJournal_Entry_submit_validate", "je_validate_period_open",     15)
    + rel("tabJournal_Entry_submit_validate", "je_validate_balance",         20)
    + "-- submit/on_submit: post GL entries\n"
    + rel("tabJournal_Entry_submit_on_submit", "je_on_submit_create_gl",   5)
    + "-- cancel/on_cancel: reverse GL entries before generic mark_cancelled (ord 99)\n"
    + rel("tabJournal_Entry_cancel_on_cancel", "je_on_cancel_gl_entries",  10)
)

# ── Casual Party ─────────────────────────────────────────────────────────────
write("casual_party",
    "-- Casual Party — pipeline wiring\n\n"
    "RETURN fn::pipeline::wire_generic(\"Casual Party\", \"tabCasual_Party\", false);\n\n"
    + upsert_node("casual_party_validate_not_duplicate", "fn::validate::casual_party_not_duplicate")
    + rel("tabCasual_Party_save_validate", "casual_party_validate_not_duplicate", 5)
)

# ── Payment Line (child table — no wiring) ───────────────────────────────────
# Already correct — skip

# ── Invoice Allocation (child table — no wiring) ─────────────────────────────
# Already correct — skip

# ── Payment ──────────────────────────────────────────────────────────────────
write("payment",
    "-- Payment — pipeline wiring (submittable)\n\n"
    "RETURN fn::pipeline::wire_generic(\"Payment\", \"tabPayment\", true);\n\n"
    + upsert_node("payment_validate_type_direction",    "fn::validate::payment_type_direction")
    + upsert_node("payment_validate_line_accounts",     "fn::validate::payment_line_accounts")
    + upsert_node("payment_validate_party_role",        "fn::validate::payment_party_role")
    + upsert_node("payment_validate_transfer_accounts", "fn::validate::payment_transfer_accounts")
    + upsert_node("payment_validate_paid_accounts",     "fn::validate::payment_paid_accounts")
    + upsert_node("payment_validate_casual_party_scope","fn::validate::payment_casual_party_scope")
    + upsert_node("payment_validate_invoice_allocation","fn::validate::payment_invoice_allocation")
    + upsert_node("payment_compute_totals",             "fn::compute::payment_totals")
    + upsert_node("payment_on_submit_post_gl",          "fn::on_submit::payment_post_gl")
    + upsert_node("payment_on_cancel_reverse_gl",       "fn::on_cancel::payment_reverse_gl")
    + "-- save/validate\n"
    + rel("tabPayment_save_validate", "payment_validate_type_direction",    5)
    + rel("tabPayment_save_validate", "payment_validate_casual_party_scope", 8)
    + rel("tabPayment_save_validate", "payment_validate_line_accounts",     10)
    + rel("tabPayment_save_validate", "payment_validate_party_role",        15)
    + rel("tabPayment_save_validate", "payment_validate_transfer_accounts", 20)
    + rel("tabPayment_save_validate", "payment_validate_paid_accounts",     25)
    + rel("tabPayment_save_validate", "payment_validate_invoice_allocation", 30)
    + "-- save/compute\n"
    + rel("tabPayment_save_compute",  "payment_compute_totals",             5)
    + "-- submit/validate\n"
    + rel("tabPayment_submit_validate", "payment_validate_type_direction",    5)
    + rel("tabPayment_submit_validate", "payment_validate_casual_party_scope", 8)
    + rel("tabPayment_submit_validate", "payment_validate_line_accounts",     10)
    + rel("tabPayment_submit_validate", "payment_validate_party_role",        15)
    + rel("tabPayment_submit_validate", "payment_validate_transfer_accounts", 20)
    + rel("tabPayment_submit_validate", "payment_validate_paid_accounts",     25)
    + rel("tabPayment_submit_validate", "payment_validate_invoice_allocation", 30)
    + "-- submit/on_submit: post GL entries\n"
    + rel("tabPayment_submit_on_submit", "payment_on_submit_post_gl",       5)
    + "-- cancel/on_cancel: reverse GL entries\n"
    + rel("tabPayment_cancel_on_cancel", "payment_on_cancel_reverse_gl",    10)
)

print("All wiring files generated successfully!")
