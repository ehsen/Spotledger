# SpotLedger Finance ERP — Implementation Plan
## Codebase-Aware, SurrealDB-First, Phased Execution Guide

**Date**: April 2026  
**Status**: Phases 1–7 Complete — 58 finance integration tests passing; Designer Visibility Verification completed with HTTP evidence  
**Scope**: One new app `spotledger-finance` delivering Modules: Core, Business Partner, Accounts, Payments

---

## Implementation Progress Tracker

### Pre-Implementation Work Items
| # | Work Item | Status |
|---|---|---|
| 1 | Migrate pipeline framework to spotledger-core; delete frappe/erpnext | ✅ Done |
| 2 | Confirm child table inline-embedding contract | ✅ Done |
| 3 | Add `save/compute` stage to wire_generic | ✅ Done |
| 4 | Core reference data fixtures (Currency, Country, Language, Timezone) | ✅ Done |
| 5 | Integration test scaffold (8 files, 40 tests green) | ✅ Done |
| 6 | Finance pipeline integration test infrastructure (`common/mod.rs`, `apply_pipeline_functions_multi`) | ✅ Done |
| 7 | 58 finance integration tests passing across 12 suites (Currency, FY, AP, GL Entry, Journal Entry, Account, Account Group, Party, Company, Cost Center, Casual Party, Payment) | ✅ Done |

### Phase 1 — App Scaffolding
| Task | Status |
|---|---|
| Create `apps/spotledger-finance/` directory + `app.json` | ✅ Done |
| Create `spotledger_finance/modules.txt` | ✅ Done |
| Create all 22 doctype JSON stubs | ✅ Done |
| Verify `install-app` runs clean, 22 DocTypes in DB | ✅ Done |

**Bug fixed**: `is_app_installed()` in `install_app.rs` queried `installed_app` (no prefix) but `record_installed_app` writes to `tabinstalled_app` via `upsert_doc`. Fixed to query `tabinstalled_app`.  
**Also fixed**: inner app directory renamed from `spotledger_finance` → `spotledger-finance` to match `bench/apps/{name}/{name}` convention.

### SurrealDB v3 Compatibility Fixes (Phase 2–6)

- **`type::thing` removed** — renamed to `type::record` in SurrealDB v3; applied globally to all surql files in `spotledger-finance` and the core framework.
- **`type::record(...)` in RELATE/UPSERT positions** — SurrealDB v3 rejects `type::record()` calls directly in `RELATE x -> e -> type::record(...)` or as the UPSERT target. Fix: assign to a `LET` variable first:
  ```surql
  LET $rec = type::record("pipeline_stage", $sv);
  UPSERT $rec CONTENT { ... };
  RELATE $sv_rec -> has_node -> pipeline_node:foo CONTENT { ord: 5, config: {} };
  ```
- **`04_wire_generic.surql` rewritten** — all `type::record()` calls inside the function body assigned to `LET $*_rec` variables before use in `IF`, `UPSERT`, and `RELATE` statements.
- **All wiring.surql files rebuilt** — switched from dynamic `type::record("pipeline_stage", $sv)` to static literal record IDs (e.g. `pipeline_stage:tabAccount_save_validate`) generated via `gen_wiring.py`, completely avoiding the parse restriction.

### Phase 2 — Module Core ✅ Done
| DocType | JSON | functions.surql | wiring.surql | Tests | Status |
|---|---|---|---|---|---|
| Currency | ✅ | ✅ | ✅ | ✅ 5 tests | Tested |
| Fiscal Year | ✅ | ✅ | ✅ | ✅ 4 tests | Tested |
| Accounting Period | ✅ | ✅ | ✅ | ✅ 4 tests | Tested |
| Cost Center | ✅ | ✅ | ✅ | ✅ 4 tests | Tested |
| Company | ✅ | ✅ | ✅ | ✅ 4 tests | Tested |

### Phase 3 — Module Business Partner ✅ Done
| DocType | JSON | functions.surql | wiring.surql | Tests | Status |
|---|---|---|---|---|---|
| Payment Terms | ✅ | ✅ | ✅ | ⬜ | Wired |
| Party | ✅ | ✅ | ✅ | ✅ 5 tests | Tested |
| Party Role Config | ✅ | ✅ | ✅ | ⬜ | Wired |

### Phase 4 — Module Accounts (Foundation) ✅ Done
| DocType | JSON | functions.surql | wiring.surql | Tests | Status |
|---|---|---|---|---|---|
| Chart of Accounts | ✅ | ✅ | ✅ | ⬜ | Wired |
| Account Group | ✅ | ✅ | ✅ | ✅ 5 tests | Tested |
| Account | ✅ | ✅ | ✅ | ✅ 5 tests | Tested |
| Account Company | ✅ | ✅ | ✅ | ⬜ | Wired |
| Financial Statement Version | ✅ | ✅ | ✅ | ⬜ | Wired |
| FSV Node | ✅ | ✅ | ✅ | ⬜ | Wired |
| FSV Account Mapping | ✅ | ✅ | ✅ | ⬜ | Wired |

### Phase 5 — GL Entry and Journal Entry ✅ Done
| DocType | JSON | functions.surql | wiring.surql | Tests | Status |
|---|---|---|---|---|---|
| GL Entry | ✅ | ✅ | ✅ | ✅ 7 tests | Tested |
| Journal Entry Account (child) | ✅ | n/a | n/a | n/a | Wired (child, no pipeline) |
| Journal Entry | ✅ | ✅ | ✅ | ✅ 8 tests | Tested |

### Phase 6 — Payments Module ✅ Done
| DocType | JSON | functions.surql | wiring.surql | Tests | Status |
|---|---|---|---|---|---|
| Casual Party | ✅ | ✅ | ✅ | ✅ 2 tests | Tested |
| Payment Line (child) | ✅ | n/a | n/a | n/a | Wired (child, no pipeline) |
| Invoice Allocation (child) | ✅ | n/a | n/a | n/a | Wired (child, no pipeline) |
| Payment | ✅ | ✅ | ✅ | ✅ 5 tests | Tested |

**Phase 6 coverage status**: Fully tested for pipeline-backed doctypes. `Casual Party` and `Payment` dedicated suites are green. `Payment Line` and `Invoice Allocation` remain child doctypes with no standalone pipeline.

**Verified**: `wire-app spotledger-finance spotledger` → ✅ 64 fn:: registered, all wiring.surql files applied.

### Phase 7 — Integration Tests ✅ Done

#### Test Infrastructure (completed)
- `crates/spotledger-db/tests/common/mod.rs` — shared helpers: `make_pipeline_db`, `seed_doctype`, DDL helpers, data helpers, `parse_decimal`, `field_str`
- `apply_pipeline_functions_multi` in `crates/spotledger-db/src/pipeline.rs` — loads multiple apps with a single combined registry
- Key lessons: `tabDocField` stub required for `validate_mandatory_fields`; SCHEMAFULL tables need all `insert_doc` fields; `TYPE any` for nested-object arrays; delete before insert for idempotency

#### Test Suite Status
| Test File | Tests | Passing | Focus |
|---|---|---|---|
| `test_currency.rs` | 5 | ✅ 5/5 | code format, single base enforcement |
| `test_fiscal_year.rs` | 4 | ✅ 4/4 | date order, overlap |
| `test_accounting_period.rs` | 4 | ✅ 4/4 | date order, within FY, overlap |
| `test_gl_entry.rs` | 7 | ✅ 7/7 | debit/credit validation, period check, cancel |
| `test_journal_entry.rs` | 8 | ✅ 8/8 | totals, balance, GL creation, cancel, **atomicity** |
| `test_account.rs` | 5 | ✅ 5/5 | CoA matching, number range, derive from group, retained earnings |
| `test_account_group.rs` | 5 | ✅ 5/5 | number range required, inverted range, overlap, classification derive |
| `test_party.rs` | 5 | ✅ 5/5 | roles, invalid role rejection, tax ID uniqueness, PRC autocreation |
| `test_company.rs` | 4 | ✅ 4/4 | abbreviation required, length, uniqueness |
| `test_cost_center.rs` | 4 | ✅ 4/4 | code required, uniqueness, parent edge relation |
| `test_casual_party.rs` | 2 | ✅ 2/2 | duplicate-name warning path stays non-blocking |
| `test_payment.rs` | 5 | ✅ 5/5 | direction/type matrix, party role, internal transfer, casual party scope |

**Total passing**: 58 integration tests (+ 8 naming tests = 66 total)

#### Checkpoints verified
- Journal Entry submit creates GL Entry rows automatically: covered by `test_je_submit_gl_entries_full_fields`.
- Journal Entry submit is atomic: if GL posting fails, the submit rolls back and the Journal Entry remains unsubmitted, covered by `test_je_submit_gl_failure_rolls_back`.

#### SurrealDB v3 compatibility fixes discovered while finishing tests
- Replaced unsupported `array::includes(...)` usage in Party and Payment domain functions with `IN` / `NOT IN` membership checks.
- Aligned test fixtures with current validator self-exclusion behavior where uniqueness validators compare against business-name fields instead of document IDs.

---

---

## Part 1 — Structural Principles (Read Before Writing Any Code)

### 1.1 The Four Immovable Rules

**Rule 1 — Apps are the dependency boundary.**  
An app can only depend on apps below it in the stack. It cannot reference a DocType, SurrealQL function, or table that belongs to an app above it or at the same level. Violations create circular imports that cannot be resolved without restructuring.

**Rule 2 — The pipeline framework owns no domain logic.**  
`apps/erpnext/erpnext/surql/framework/` contains the generic runner, schema, and wire helper. Domain functions (`fn::validate::*`, `fn::on_submit::*`) belong in the domain app only. The framework knows nothing about Currency, Party, or GL Entry.

**Rule 3 — SurrealDB events cannot be used for domain GL posting.**  
`DEFINE EVENT` in SurrealDB fires per-row inside a transaction, but it cannot be rolled back selectively, cannot return errors to the Rust caller, and cannot be conditionally disabled (e.g. for migrations). All domain logic uses the **pipeline** (`fn::pipeline::run`) invoked by Rust, not implicit events. Events are only allowed for read-model maintenance (caches, aggregates) and outbox writes (external notifications).

**Rule 4 — Never reuse an ERPNext DocType name for a SpotLedger replacement.**  
ERPNext is installed as a separate app. If SpotLedger introduces `Party` to replace `Customer`+`Supplier`, the DocType is named `Party`. It never shadows or aliases `Customer`. Both can coexist in the DB during migration. The `spotledger-finance` app owns its own tables.


---

### 1.2 App Dependency Graph

```
[ spotledger-core ]    ← Tier 0: engine types (DocType, DocField, User, Role, ModuleDef)
        ↓
[ spotledger-finance / Module: Core ]
        Currency, Company, Fiscal Year, Accounting Period, Cost Center
        ↓
[ spotledger-finance / Module: Business Partner ]
        Party, Party Role Config, Payment Terms
        ↓
[ spotledger-finance / Module: Accounts ]
        Chart of Accounts, Account Group, Account, Account Company
        FSV, FSV Node, FSV Account Mapping
        GL Entry, Journal Entry
        ↓
[ spotledger-finance / Module: Payments ]
        Casual Party, Payment, Payment Line (child), Invoice Allocation (child)
        ↓
[ FUTURE: transactions ]   Sales Invoice, Purchase Invoice
        ↓
[ FUTURE: reports ]        Trial Balance, Balance Sheet, P&L, AR/AP Aging
```

**Circular dependency check**:
- Accounts references Core (Company, Fiscal Year, Cost Center) and Business Partner (Party) ✓
- Payments references Accounts (Account, GL Entry) and Business Partner (Party, Party Role Config) ✓
- Business Partner references Core (Currency) only ✓
- Core has no domain dependencies ✓
- No module references anything above itself ✓

---

### 1.3 App & File Layout

```
apps/spotledger-finance/
    app.json                          ← app metadata: name, version, depends_on
    spotledger_finance/
        modules.txt                   ← newline-separated module names
        core/
            doctype/
                currency/
                    currency.json
                company/
                    company.json
                fiscal_year/
                    fiscal_year.json
                accounting_period/
                    accounting_period.json
                cost_center/
                    cost_center.json
        business_partner/
            doctype/
                payment_terms/
                    payment_terms.json
                party/
                    party.json
                party_role_config/
                    party_role_config.json
        accounts/
            doctype/
                chart_of_accounts/
                    chart_of_accounts.json
                account_group/
                    account_group.json
                account/
                    account.json
                account_company/
                    account_company.json
                financial_statement_version/
                    financial_statement_version.json
                fsv_node/
                    fsv_node.json
                fsv_account_mapping/
                    fsv_account_mapping.json
                gl_entry/
                    gl_entry.json
                journal_entry/
                    journal_entry.json
                journal_entry_account/
                    journal_entry_account.json
        surql/
            shared/
                validate_mandatory.surql
                validate_period_open.surql
                validate_fiscal_year.surql
                derive_period.surql
            doctypes/
                currency/
                    functions.surql
                    wiring.surql
                company/
                    functions.surql
                    wiring.surql
                fiscal_year/
                    functions.surql
                    wiring.surql
                accounting_period/
                    functions.surql
                    wiring.surql
                cost_center/
                    functions.surql
                    wiring.surql
                payment_terms/
                    functions.surql
                    wiring.surql
                party/
                    functions.surql
                    wiring.surql
                chart_of_accounts/
                    functions.surql
                    wiring.surql
                account_group/
                    functions.surql
                    wiring.surql
                account/
                    functions.surql
                    wiring.surql
                gl_entry/
                    functions.surql
                    wiring.surql
                journal_entry/
                    functions.surql
                    wiring.surql
        payments/
            doctype/
                casual_party/
                    casual_party.json
                payment/
                    payment.json
                payment_line/
                    payment_line.json
                invoice_allocation/
                    invoice_allocation.json
            fixtures/
                Account.json              ← One-Time Payable, One-Time Receivable, Advance accounts
            surql/
                doctypes/
                    casual_party/
                        functions.surql
                        wiring.surql
                    payment/
                        functions.surql
                        wiring.surql
```

---

### 1.4 Table Naming Convention

SpotLedger follows the Frappe `tab` prefix convention for full compatibility with the existing install/seed pipeline. DocType `"Currency"` → table `tabCurrency`. DocType `"Chart of Accounts"` → table `` `tabChart of Accounts` `` (backtick-quoted in SurrealQL for the space). DocType `"Party Role Config"` → table `` `tabParty Role Config` ``.

This convention is already enforced by `seed_doctypes.rs` and the existing `tabDocType`, `tabDocField`, `tabDocPerm` Tier-0 tables. All surql files in this plan use `tab`-prefixed names.

---

### 1.5 What Is Imported From ERPNext vs What Is New

| Concern | Decision |
|---|---|
| Doctype JSON field structure | **Reference only.** Fields are adapted, not copied. ERPNext field names are kept where they make sense (`account_number`, `posting_date`) to aid future migration tooling. |
| Table naming | **SpotLedger convention** — `tabParty`, `tabGL Entry`, same `tab` prefix as ERPNext. |
| GL Entry structure | **Simplified.** No `voucher_type`/`voucher_no` text references. Uses `record<>` type links. |
| Account tree | **Graph edges.** No `lft`/`rgt` nested-set integers. `child_of` relation table instead. |
| Customer + Supplier | **Replaced** by `Party` with `roles` field. ERPNext `tabCustomer`/`tabSupplier` continue to exist from the ERPNext app and are not touched. |
| Chart of Accounts | **New.** SAP-style three-tier model. ERPNext's single flat CoA is not reused. |
| Pipeline framework | **Shared.** `fn::pipeline::run`, `fn::pipeline::wire_generic`, and shared nodes from `apps/spotledger-core/spotledger_core/surql/framework/` are the foundation. No duplication. |
| FSV / Account Group | **New.** No ERPNext equivalent exists at this level of structure. |
| Payment Entry | **Replaced entirely.** ERPNext Payment Entry is not used. SpotLedger uses a purpose-built `Payment` DocType with direction (Outgoing/Incoming/Internal) and type (Expense Payment, Income Receipt, Party Payment, Party Receipt, Internal Transfer) designed for direct data entry without accounting knowledge. Spec: `spotledger_payment_receipt_spec.md`. |

---

## Part 2 — DocType Specifications

Each specification states: fields to define in the JSON, the table name used in SurrealQL, the autoname strategy, pipeline complexity (Simple/Medium/Complex), and which SurrealQL files are needed.

---

### Module: Core

#### 2.1 Currency

**Table**: `tabCurrency`  
**Autoname**: `field:code` (the 3-char ISO code is the name)  
**Is Child Table**: No  
**Is Submittable**: No  
**Pipeline complexity**: Simple (validation only)

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `code` | ISO Code | Data | Mandatory, unique, 3-char uppercase. This IS the `name` field via autoname |
| `currency_name` | Currency Name | Data | Mandatory. Example: "Pakistani Rupee" |
| `symbol` | Symbol | Data | Mandatory. Example: "₨" |
| `is_base` | Is Base Currency | Check | Default 0. Only one true across deployment |
| `decimal_places` | Decimal Places | Int | Default 2. JPY = 0 |
| `enabled` | Enabled | Check | Default 1 |
| `fraction` | Fraction | Data | Optional. Example: "Paisa" |
| `fraction_units` | Fraction Units | Int | Optional. Example: 100 |

**SurrealQL required**:  
- `functions.surql`: `fn::validate::currency_code` (uppercase, 3 chars), `fn::validate::currency_single_base` (at most one `is_base=true`)  
- `wiring.surql`: wire into `save/validate`

**Designer visibility**: Both functions appear as nodes in the `Currency` pipeline stages. No separate meta.surql needed — `wiring.surql` calls `fn::pipeline::wire_generic("Currency", "tabCurrency", false)`.

---

#### 2.2 Company

**Table**: `tabCompany`  
**Autoname**: `field:company_name` (or `field:code` — decide at implementation)  
**Is Submittable**: No  
**Pipeline complexity**: Complex (on_save creates Account Company records + Fiscal Year + Accounting Periods)

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `company_name` | Company Name | Data | Mandatory, unique. Legal name |
| `code` | Company Code | Data | Mandatory, unique. Max 4 chars, uppercase. Example: PK01 |
| `default_currency` | Default Currency | Link → Currency | Mandatory |
| `country` | Country | Data | Mandatory |
| `chart_of_accounts` | Chart of Accounts | Link → Chart of Accounts | Mandatory. Must be Operative type |
| `ledger_type` | Ledger Type | Select | Options: Operational, Consolidation, Reporting. Default: Operational |
| `parent_company` | Parent Company | Link → Company | Optional |
| `is_group` | Is Group | Check | Default 0 |
| `fiscal_year_start_month` | Fiscal Year Start Month | Int | Mandatory. 1-12 |
| `retained_earnings_account` | Retained Earnings Account | Link → Account | Mandatory (after CoA exists) |
| `default_receivable_account` | Default Receivable Account | Link → Account | Mandatory |
| `default_payable_account` | Default Payable Account | Link → Account | Mandatory |
| `round_off_account` | Round Off Account | Link → Account | Optional |

**SurrealQL required**:  
- `functions.surql`:  
  - `fn::validate::company_code` — uppercase, max 4 chars, no spaces  
  - `fn::validate::company_coa_type` — chart_of_accounts must be Operative type  
  - `fn::validate::company_parent_not_self` — parent_company ≠ self  
  - `fn::on_save::company_bootstrap` — creates Fiscal Year, 12 Accounting Periods, Account Company rows for all accounts in the CoA  
- `wiring.surql`: wire validate nodes into `save/validate`, bootstrap into `save/on_save`

**Key design note**: `fn::on_save::company_bootstrap` runs on the FIRST save only (when the record is new). It checks `IF $doc.fiscal_year_created != true` then creates the records and sets the flag. This is idempotent.

---

#### 2.3 Fiscal Year

**Table**: `tabFiscal Year`  
**Autoname**: `field:year_name` (e.g. "FY2024-25")  
**Is Submittable**: No  
**Pipeline complexity**: Simple

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `year_name` | Year Name | Data | Mandatory, unique per company. Example: FY2024-25 |
| `company` | Company | Link → Company | Mandatory |
| `year_start_date` | Start Date | Date | Mandatory |
| `year_end_date` | End Date | Date | Mandatory |
| `is_closed` | Is Closed | Check | Default 0 |
| `closed_on` | Closed On | Datetime | System set |
| `closed_by` | Closed By | Link → User | System set |

**SurrealQL required**:  
- `functions.surql`:  
  - `fn::validate::fiscal_year_dates` — end > start, max 366 days  
  - `fn::validate::fiscal_year_no_overlap` — no overlapping years per company  
  - `fn::validate::fiscal_year_no_close_if_entries` — prevent close if unposted entries exist  
- `wiring.surql`: standard wire_generic non-submittable

---

#### 2.4 Accounting Period

**Table**: `tabAccounting Period`  
**Autoname**: `field:period_name` (e.g. "Jul 2024")  
**Is Submittable**: No  
**Pipeline complexity**: Simple

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `period_name` | Period Name | Data | Mandatory |
| `company` | Company | Link → Company | Mandatory |
| `fiscal_year` | Fiscal Year | Link → Fiscal Year | Mandatory |
| `start_date` | Start Date | Date | Mandatory |
| `end_date` | End Date | Date | Mandatory |
| `period_number` | Period Number | Int | 1 through 12 |
| `is_closed` | Is Closed | Check | Default 0 |
| `closed_on` | Closed On | Datetime | System set |
| `closed_by` | Closed By | Link → User | System set |

**SurrealQL required**:  
- `functions.surql`:  
  - `fn::validate::period_dates` — end > start, within parent fiscal year range  
  - `fn::validate::period_sequential_close` — cannot close if prior period open  
- `wiring.surql`: standard

---

#### 2.5 Cost Center

**Table**: `tabCost Center`  
**Autoname**: `field:cost_center_name`  
**Is Submittable**: No  
**Pipeline complexity**: Simple (tree structure via graph edges)

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `cost_center_name` | Cost Center Name | Data | Mandatory |
| `cost_center_code` | Code | Data | Mandatory, unique within company. Example: CC-LHR |
| `company` | Company | Link → Company | Mandatory |
| `parent_cost_center` | Parent Cost Center | Link → Cost Center | Optional. Self-referential |
| `is_group` | Is Group | Check | Default 0 |
| `is_active` | Is Active | Check | Default 1 |

**SurrealQL required**:  
- `functions.surql`:  
  - `fn::validate::cost_center_code_unique` — unique within company  
  - `fn::on_save::cost_center_relate_parent` — creates/updates `child_of` graph edge  
- `wiring.surql`: standard

**Graph edge**: After save, create `RELATE tabCost Center:<name> -> child_of -> tabCost Center:<parent_cost_center>`. The `child_of` table is defined in the shared framework schema.

---

### Module: Business Partner

#### 2.6 Payment Terms

**Table**: `tabPayment Terms`  
**Autoname**: `field:payment_terms_name`  
**Is Submittable**: No  
**Pipeline complexity**: Simple

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `payment_terms_name` | Payment Terms Name | Data | Mandatory, unique |
| `payment_days` | Payment Days | Int | Mandatory. Days from invoice until due |
| `discount_days` | Discount Days | Int | Optional. Days for early pay discount |
| `discount_percent` | Discount Percent | Percent | Optional |
| `description` | Description | Small Text | Optional |

**SurrealQL required**:  
- `functions.surql`: `fn::validate::payment_terms_days` — payment_days ≥ 0  
- `wiring.surql`: standard

---

#### 2.7 Party

**Table**: `tabParty`  
**Autoname**: `field:party_name`  
**Is Submittable**: No  
**Pipeline complexity**: Medium (role validation, auto-create Party Role Config)

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `party_name` | Party Name | Data | Mandatory, unique. Legal name |
| `short_name` | Short Name | Data | Optional. Trade name |
| `party_type` | Party Type | Select | Options: Individual, Company. Mandatory |
| `roles` | Roles | JSON | At least one mandatory. Array of strings. Values: Customer, Supplier, Employee, Shareholder, Director, Franchisee, Bank, Tax Authority, Government Office |
| `is_active` | Is Active | Check | Default 1 |
| `tax_id` | Tax ID | Data | Optional. Unique when provided |
| `tax_id_type` | Tax ID Type | Select | Options: NTN, CNIC, VAT Number, Tax Registration Number, Other |
| `is_tax_registered` | Is Tax Registered | Check | Default 0 |
| `sales_tax_number` | Sales Tax Number | Data | Optional. STRN in Pakistan |
| `primary_email` | Email | Data | Optional |
| `primary_phone` | Phone | Data | Optional |
| `website` | Website | Data | Optional |
| `currency` | Default Currency | Link → Currency | Mandatory |
| `country` | Country | Data | Mandatory |
| `billing_address_line1` | Address Line 1 | Data | Optional |
| `billing_address_line2` | Address Line 2 | Data | Optional |
| `billing_city` | City | Data | Optional |
| `billing_state` | State/Province | Data | Optional |
| `billing_postal_code` | Postal Code | Data | Optional |
| `billing_country` | Billing Country | Data | Optional |

**Child table**: `tabParty Role Config` (see 2.8). Party form shows it as a child table section.

**SurrealQL required**:  
- `functions.surql`:  
  - `fn::validate::party_roles` — at least one role  
  - `fn::validate::party_tax_id_unique` — if tax_id provided, must be unique  
  - `fn::on_save::party_role_config_autocreate` — for each role in `roles`, ensure a `tabParty Role Config` record exists for each Company where it doesn't already exist  
- `wiring.surql`: standard

---

#### 2.8 Party Role Config

**Table**: `tabParty Role Config`  
**Autoname**: `naming_series:PRC-`  
**Is Child Table**: No (standalone record linked to Party, accessed via child table in Party form)  
**Is Submittable**: No  
**Pipeline complexity**: Medium (validate account types match role type)

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `party` | Party | Link → Party | Mandatory. Parent |
| `role` | Role | Select | Same options as Party.roles |
| `company` | Company | Link → Company | Mandatory |
| `receivable_account` | Receivable Account | Link → Account | Mandatory when role is Customer or Franchisee |
| `payable_account` | Payable Account | Link → Account | Mandatory when role is Supplier, Employee, Director |
| `payment_terms` | Payment Terms | Link → Payment Terms | Optional |
| `credit_limit` | Credit Limit | Currency | Optional |
| `credit_days` | Credit Days | Int | Optional |
| `tax_category` | Tax Category | Data | Optional |
| `withholding_tax_applicable` | WHT Applicable | Check | Default 0 |
| `withholding_tax_rate` | WHT Rate | Percent | Mandatory when withholding_tax_applicable = 1 |

**Constraints**: Unique on `party + role + company`.

**SurrealQL required**:  
- `functions.surql`:  
  - `fn::validate::party_role_config_account_types` — receivable_account must be type D; payable_account must be type K  
  - `fn::validate::party_role_config_unique` — unique party+role+company  
- `wiring.surql`: standard

---

### Module: Accounts

#### 2.9 Chart of Accounts

**Table**: `tabChart of Accounts`  
**Autoname**: `field:coa_name`  
**Is Submittable**: No  
**Pipeline complexity**: Simple

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `coa_name` | Name | Data | Mandatory, unique |
| `code` | Short Code | Data | Mandatory, unique. Example: STD-PKR |
| `coa_type` | CoA Type | Select | Options: Operative, Group, Country. Default: Operative |
| `parent_coa` | Parent CoA | Link → Chart of Accounts | Mandatory for Group/Country types |
| `root_currency` | Root Currency | Link → Currency | Mandatory |
| `description` | Description | Small Text | Optional |
| `is_default` | Is Default | Check | Default 0. Only one per deployment |

**SurrealQL required**:  
- `functions.surql`:  
  - `fn::validate::coa_parent_required` — parent_coa mandatory for non-Operative types  
  - `fn::validate::coa_parent_type` — parent_coa must be Operative type  
  - `fn::validate::coa_single_default` — at most one `is_default = true`  
- `wiring.surql`: standard

---

#### 2.10 Account Group

**Table**: `tabAccount Group`  
**Autoname**: `field:group_name`  
**Is Submittable**: No  
**Pipeline complexity**: Medium (derive hidden classification fields from account_category)

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `group_name` | Group Name | Data | Mandatory |
| `code` | Code | Data | Mandatory, unique within CoA |
| `chart_of_accounts` | Chart of Accounts | Link → Chart of Accounts | Mandatory |
| `description` | Description | Small Text | Optional |
| `account_category` | Account Category | Select | User-facing. Options: Fixed Asset, Current Asset, Cash and Bank, Trade Receivable, Inventory, Equity, Long Term Liability, Trade Payable, Current Liability, Revenue, Cost of Sales, Expense, Tax Account |
| `account_type_internal` | Account Type (Internal) | Data | Hidden. SAP codes: S, G, A, D, K, M. Derived |
| `fs_type` | FS Type | Data | Hidden. Derived. Asset / Liability / Equity / Income / Expense |
| `is_reconciliation_group` | Is Reconciliation | Check | Hidden. Derived. True for D, K, A, M |
| `party_required` | Party Required | Check | Hidden. Derived. True for D, K |
| `number_from` | Number From | Data | Mandatory |
| `number_to` | Number To | Data | Mandatory |

**Derivation mapping** (enforced in `before_save` function):

| account_category | account_type_internal | fs_type | is_reconciliation_group | party_required |
|---|---|---|---|---|
| Fixed Asset | A | Asset | 1 | 0 |
| Current Asset | S | Asset | 0 | 0 |
| Cash and Bank | S | Asset | 0 | 0 |
| Trade Receivable | D | Asset | 1 | 1 |
| Inventory | M | Asset | 1 | 0 |
| Equity | S | Equity | 0 | 0 |
| Long Term Liability | S | Liability | 0 | 0 |
| Trade Payable | K | Liability | 1 | 1 |
| Current Liability | S | Liability | 0 | 0 |
| Revenue | G | Income | 0 | 0 |
| Cost of Sales | G | Expense | 0 | 0 |
| Expense | G | Expense | 0 | 0 |
| Tax Account | S | Liability | 0 | 0 |

**SurrealQL required**:  
- `functions.surql`:  
  - `fn::compute::account_group_derive_classification` — reads `account_category`, writes the four hidden fields  
  - `fn::validate::account_group_number_range` — number_from < number_to, no overlap with other groups in same CoA  
  - `fn::validate::account_group_code_unique` — unique code within CoA  
- `wiring.surql`: derive function into `save/compute` (runs before persist), validate into `save/validate`

---

#### 2.11 Account

**Table**: `tabAccount`  
**Autoname**: `field:account_number`  
**Is Submittable**: No  
**Pipeline complexity**: Complex (tree edges, Account Company auto-create, derived fields)

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `account_number` | Account Number | Data | Mandatory, unique within CoA |
| `account_name` | Account Name | Data | Mandatory |
| `chart_of_accounts` | Chart of Accounts | Link → Chart of Accounts | Mandatory |
| `account_group` | Account Group | Link → Account Group | Mandatory |
| `account_type_internal` | Account Type (Internal) | Data | Read-only. Derived from Account Group |
| `account_category` | Account Category | Data | Read-only display. Derived |
| `fs_type` | FS Type | Data | Hidden. Derived |
| `is_reconciliation_account` | Is Reconciliation | Check | Hidden. Derived |
| `party_required` | Party Required | Check | Hidden. Derived |
| `parent_account` | Parent Account | Link → Account | Optional |
| `is_group` | Is Group | Check | Default 0 |
| `retained_earnings_account` | Retained Earnings Account | Link → Account | Mandatory for non-group P&L accounts (fs_type=Income or Expense) |
| `group_account` | Group CoA Account | Link → Account | Optional. Consolidation mapping |
| `country_account` | Country CoA Account | Link → Account | Optional. Statutory mapping |
| `tax_category` | Tax Category | Select | Optional: Input Tax, Output Tax, None |
| `disabled` | Disabled | Check | Default 0 |
| `freeze_account` | Freeze Account | Select | Options: '', Freeze. Default: '' |
| `balance` | Balance | Currency | Computed field. Derived from GL Entries (not stored — computed at report time or via cache) |

**SurrealQL required**:  
- `functions.surql`:  
  - `fn::validate::account_number_in_group_range` — account_number falls within account_group's number_from..number_to  
  - `fn::validate::account_group_matches_coa` — account_group belongs to same CoA as account  
  - `fn::validate::account_retained_earnings_required` — mandatory for non-group P&L accounts  
  - `fn::validate::account_type_immutable` — if GL entries exist, account_type_internal cannot change  
  - `fn::compute::account_derive_from_group` — copy classification fields from Account Group  
  - `fn::on_save::account_relate_parent` — RELATE child_of edge to parent_account  
  - `fn::on_save::account_extend_companies` — INSERT INTO tabAccount Company for every Company using this CoA  
- `wiring.surql`: compute into `save/compute`, validates into `save/validate`, on_save into `save/on_save`

**Critical graph operation** (in `fn::on_save::account_relate_parent`):
```surql
-- Remove old parent edge
DELETE (SELECT id FROM child_of WHERE in = $doc_id);
-- Create new parent edge if parent exists
IF $doc.parent_account != NONE AND $doc.parent_account != "" {
    RELATE $doc_id -> child_of -> type::thing("tabAccount", $doc.parent_account);
};
```

---

#### 2.12 Account Company

**Table**: `tabAccount Company`  
**Autoname**: `naming_series:ACC-CO-`  
**Is Submittable**: No  
**Pipeline complexity**: Simple (mostly system-managed)  
**No New button in UI. No list view accessible to users.**

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `account` | Account | Link → Account | Mandatory |
| `company` | Company | Link → Company | Mandatory |
| `currency` | Currency | Link → Currency | Mandatory. Defaults to Company's currency |
| `is_active` | Is Active | Check | Default 1 |
| `open_item_management` | Open Item Management | Check | Derived. True for D/K type accounts |
| `line_item_display` | Line Item Display | Check | Default 1 |
| `is_intercompany` | Is Intercompany | Check | Default 0 |
| `local_account_code` | Local Account Code | Data | Optional. For Country CoA mapping |

**SurrealQL required**:  
- `functions.surql`:  
  - `fn::validate::account_company_unique` — unique account+company  
  - `fn::compute::account_company_derive_open_item` — open_item_management = true if account's type is D or K  
- `wiring.surql`: standard

---

#### 2.13 Financial Statement Version (FSV)

**Table**: `tabFinancial Statement Version`  
**Autoname**: `field:fsv_name`  
**Is Submittable**: No  
**Pipeline complexity**: Simple

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `fsv_name` | Name | Data | Mandatory, unique |
| `chart_of_accounts` | Chart of Accounts | Link → Chart of Accounts | Mandatory |
| `purpose` | Purpose | Select | Mandatory: Statutory, Management, Consolidation, Tax, Bank Reporting |
| `description` | Description | Small Text | Optional |
| `is_default` | Is Default | Check | Default 0. At most one per CoA |

**SurrealQL**: `fn::validate::fsv_single_default_per_coa` — unique is_default per CoA.

---

#### 2.14 FSV Node

**Table**: `tabFSV Node`  
**Autoname**: `naming_series:FSVN-`  
**Is Submittable**: No  
**Pipeline complexity**: Simple

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `fsv` | FSV | Link → Financial Statement Version | Mandatory |
| `node_name` | Node Name | Data | Mandatory |
| `parent_node` | Parent Node | Link → FSV Node | Optional. Self-referential |
| `sequence` | Sequence | Int | Mandatory |
| `is_group` | Is Group | Check | Default 0 |
| `report_type` | Report Type | Select | Options: Balance Sheet, Profit and Loss |
| `sign` | Sign | Select | Options: Normal, Reverse |
| `bold` | Bold | Check | Default 0 |

**SurrealQL**: minimal — `fn::validate::fsv_node_sequence_unique` within parent.

---

#### 2.15 FSV Account Mapping

**Table**: `tabFSV Account Mapping`  
**Autoname**: `naming_series:FSVM-`  
**Is Submittable**: No  
**Pipeline complexity**: Simple

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `fsv_node` | FSV Node | Link → FSV Node | Mandatory |
| `chart_of_accounts` | Chart of Accounts | Link → Chart of Accounts | Mandatory |
| `account_from` | Account From | Data | Mandatory. Start of range. Example: 6100 |
| `account_to` | Account To | Data | Mandatory. End of range |

**SurrealQL**: `fn::validate::fsv_mapping_range` — account_from < account_to.

---

#### 2.16 GL Entry

**Table**: `tabGL Entry`  
**Autoname**: `naming_series:GL-`  
**Is Submittable**: Yes  
**Is Child Table**: No  
**No New button in UI. Created only via pipeline on_submit of parent documents.**  
**Pipeline complexity**: Complex (immutability, account validation, period check)

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `company` | Company | Link → Company | Mandatory |
| `account` | Account | Link → Account | Mandatory |
| `posting_date` | Posting Date | Date | Mandatory |
| `fiscal_year` | Fiscal Year | Link → Fiscal Year | System-derived from posting_date + company |
| `accounting_period` | Accounting Period | Link → Accounting Period | System-derived |
| `debit` | Debit | Currency | Mandatory. ≥ 0 |
| `credit` | Credit | Currency | Mandatory. ≥ 0 |
| `debit_in_account_currency` | Debit (Account Currency) | Currency | Optional |
| `credit_in_account_currency` | Credit (Account Currency) | Currency | Optional |
| `account_currency` | Account Currency | Link → Currency | Optional |
| `exchange_rate` | Exchange Rate | Float | Default 1 |
| `party` | Party | Link → Party | Conditional mandatory when account.party_required = 1 |
| `party_role` | Party Role | Data | Which role: Customer, Supplier, etc. |
| `cost_center` | Cost Center | Link → Cost Center | Optional |
| `voucher_type` | Voucher Type | Data | Mandatory. E.g. "Journal Entry", "Sales Invoice" |
| `voucher_no` | Voucher No | Data | Mandatory. The parent document name |
| `remarks` | Remarks | Small Text | Optional |
| `is_cancelled` | Is Cancelled | Check | Default 0 |
| `docstatus` | Doc Status | Int | 0=Draft, 1=Submitted, 2=Cancelled |

**SurrealQL required**:  
- `functions.surql`:  
  - `fn::validate::gl_entry_account` — account exists, not disabled, not group  
  - `fn::validate::gl_entry_period_open` — posting_date falls in an open Accounting Period  
  - `fn::validate::gl_entry_party_required` — if account.party_required = 1, party must be set  
  - `fn::validate::gl_entry_amounts` — debit ≥ 0, credit ≥ 0, not both zero  
  - `fn::compute::gl_entry_derive_period` — auto-set fiscal_year and accounting_period from posting_date  
  - `fn::on_cancel::gl_entry_reverse` — swap debit/credit, set is_cancelled=1 (same as existing ERPNext surql — REUSE this)  
- `wiring.surql`: full submittable wiring, replace generic mark_cancelled with gl_entry_reverse

**Immutability rule**: There is no `save/on_save` update path after submit. Once submitted, only cancel is allowed. Enforced by checking `IF $doc.docstatus = 1` and returning an error in the `save/validate` stage.

---

#### 2.17 Journal Entry Account (Child Table)

**Table**: `tabJournal Entry Account`  
**Is Child Table**: Yes  
**Parent DocType**: Journal Entry

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `account` | Account | Link → Account | Mandatory |
| `party` | Party | Link → Party | Conditional |
| `party_role` | Party Role | Data | Optional |
| `cost_center` | Cost Center | Link → Cost Center | Optional |
| `debit_in_account_currency` | Debit (Acct Currency) | Currency | 0 default |
| `credit_in_account_currency` | Credit (Acct Currency) | Currency | 0 default |
| `debit` | Debit | Currency | 0 default. Company currency |
| `credit` | Credit | Currency | 0 default. Company currency |
| `account_currency` | Account Currency | Link → Currency | Auto-filled from Account |
| `exchange_rate` | Exchange Rate | Float | Default 1 |
| `user_remark` | Remark | Small Text | Optional |
| `reference_type` | Reference Type | Data | Optional. For open item clearing |
| `reference_name` | Reference Name | Data | Optional |
| `idx` | Index | Int | Row order |

---

#### 2.18 Journal Entry

**Table**: `tabJournal Entry`  
**Autoname**: `naming_series:JE-`  
**Is Submittable**: Yes  
**Pipeline complexity**: Complex (balance validation, GL creation on submit, reversal on cancel)

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `title` | Title | Data | Optional. Auto-filled from voucher_type |
| `voucher_type` | Voucher Type | Select | Options: Journal Entry, Bank Entry, Cash Entry, Credit Note, Debit Note, Contra Entry, Depreciation Entry, Exchange Rate Revaluation, Write Off, Opening Entry. Default: Journal Entry |
| `company` | Company | Link → Company | Mandatory |
| `posting_date` | Posting Date | Date | Mandatory |
| `user_remark` | Remark | Small Text | Optional |
| `total_debit` | Total Debit | Currency | Computed |
| `total_credit` | Total Credit | Currency | Computed |
| `difference` | Difference | Currency | Computed. Must be 0 on submit |
| `accounts` | Accounts | Table → Journal Entry Account | Mandatory. Min 2 rows |
| `docstatus` | Doc Status | Int | 0, 1, 2 |
| `amended_from` | Amended From | Link → Journal Entry | Set on amendment |

**SurrealQL required**:  
- `functions.surql`:  
  - `fn::validate::je_has_accounts` — at least 2 account rows  
  - `fn::validate::je_account_validity` — each row's account exists, not disabled, not group  
  - `fn::validate::je_balance` — total_debit == total_credit (No tolerance, these should be strcily equal)  
  - `fn::validate::je_period_open` — posting_date in open period  
  - `fn::compute::je_totals` — compute total_debit, total_credit, difference  
  - `fn::on_submit::je_create_gl_entries` — INSERT one GL Entry per account row (atomic, all or nothing)  
  - `fn::on_cancel::je_cancel_gl_entries` — call gl_entry cancel pipeline for each child GL entry  
- `wiring.surql`: full submittable wiring with custom on_submit and on_cancel nodes

---

### Module: Payments

> **Design reference**: `spotledger_payment_receipt_spec.md` — read it before implementing anything in this module. The key principles: no free-text payees, no mandatory invoice, one DocType covers both directions.

#### 2.19 Casual Party

**Table**: `tabCasual_Party`  
**Autoname**: `naming_series:CP-`  
**Is Submittable**: No  
**Pipeline complexity**: Simple

**Purpose**: Lightweight party for one-time payees/receivers. Created inline from the Payment screen in under 10 seconds. Three mandatory fields only.

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `party_name` | Name | Data | Mandatory. Legal or trade name |
| `party_type` | Party Type | Select | Mandatory. Options: Individual, Company |
| `phone` | Phone | Data | Optional |
| `address` | Address | Data | Optional. Single unstructured line |
| `default_account` | Default Account | Link → Account | System set on first transaction. One-Time Payable or One-Time Receivable |
| `can_promote` | Can Promote | Check | Default 1 |
| `promoted_to` | Promoted To | Link → Party | Set when promoted to Full Party |
| `is_promoted` | Is Promoted | Check | Default 0 |

**SurrealQL required**:
- `functions.surql`: `fn::validate::casual_party_not_duplicate` — warn if similar name exists (not hard block)
- `wiring.surql`: standard non-submittable

**Promotion to Full Party**: Handled via a UI action button (not pipeline). When the user clicks Promote, the frontend opens the Party form pre-filled. After the Party is saved, a separate API call sets `promoted_to` and `is_promoted=true` on the Casual Party record. GL Entry relink is deferred to a background task.

---

#### 2.20 Payment Line (Child Table)

**Table**: `tabPayment_Line`  
**Is Child Table**: Yes (`istable = 1`)  
**Parent DocType**: Payment

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `account` | Account | Link → Account | Mandatory. Expense account (Expense Payment) or Income account (Income Receipt). Validated against fs_type |
| `amount` | Amount | Currency | Mandatory. > 0 |
| `cost_center` | Cost Center | Link → Cost Center | Optional |
| `description` | Description | Small Text | Optional. Line-level note |
| `idx` | Index | Int | Row order |

---

#### 2.21 Invoice Allocation (Child Table)

**Table**: `tabInvoice_Allocation`  
**Is Child Table**: Yes (`istable = 1`)  
**Parent DocType**: Payment

**Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `invoice` | Invoice | Link → (future Sales Invoice / Purchase Invoice) | Mandatory per line |
| `allocated_amount` | Allocated Amount | Currency | Mandatory. Cannot exceed invoice outstanding |
| `idx` | Index | Int | Row order |

**Note**: The `invoice` link field's target doctype is determined at runtime by `payment_direction`. This is left as `Data` in the JSON for Phase 6 and upgraded to a proper Link when the Invoice DocType exists in Phase transactions.

---

#### 2.22 Payment

**Table**: `tabPayment`  
**Autoname**: computed in `save/compute` from `payment_direction` — PAY-YYYY-, REC-YYYY-, or TRF-YYYY-  
**Is Submittable**: Yes  
**Pipeline complexity**: Complex (5 GL posting paths, advance auto-detection, invoice reconciliation)

**Header Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `company` | Company | Link → Company | Mandatory |
| `payment_direction` | Direction | Select | Mandatory. Options: Outgoing, Incoming, Internal |
| `payment_type` | Payment Type | Select | Mandatory. Filtered by direction (see matrix in spec) |
| `date` | Date | Date | Mandatory. Defaults to today |
| `docstatus` | Doc Status | Int | 0=Draft, 1=Submitted, 2=Cancelled |

**Party Section** (shown for Expense Payment, Income Receipt, Party Payment, Party Receipt):

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `party_doctype` | Party DocType | Data | Either "Party" or "Casual Party". Drives which link is used |
| `party` | Party | Link → Party | Used when party_doctype = "Party" |
| `casual_party` | Casual Party | Link → Casual Party | Used when party_doctype = "Casual Party". Only for Expense Payment and Income Receipt — not Party Payment/Receipt |
| `party_role` | Party Role | Select | Mandatory for Party Payment/Receipt. Options: Customer, Supplier |
| `amount` | Amount | Currency | Mandatory for Party Payment, Party Receipt, Internal Transfer |

**Line Items** (shown for Expense Payment, Income Receipt):

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `expense_income_lines` | Lines | Table → Payment Line | At least one line mandatory |

**Invoice Allocation** (shown for Party Payment, Party Receipt):

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `against_invoices` | Against Invoices | Table → Invoice Allocation | Optional. Empty = advance payment |

**Internal Transfer Section** (shown when payment_type = Internal Transfer):

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `from_account` | From Account | Link → Account | Mandatory. Must be Cash and Bank category |
| `to_account` | To Account | Link → Account | Mandatory. Must be Cash and Bank category. ≠ from_account |

**Payment Details** (always visible):

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `payment_mode` | Payment Mode | Select | Mandatory. Options: Cash, Bank Transfer, Cheque, Online Transfer, Mobile Wallet |
| `paid_from` | Paid From | Link → Account | Mandatory for Outgoing and Internal. Cash and Bank category |
| `paid_into` | Paid Into | Link → Account | Mandatory for Incoming and Internal. Cash and Bank category |
| `reference_number` | Reference No | Data | Optional. Cheque number, transaction ID |
| `narration` | Narration | Small Text | Optional but encouraged |
| `currency` | Currency | Link → Currency | Mandatory. Defaults to company currency |
| `exchange_rate` | Exchange Rate | Float | Default 1 |

**Computed Fields**:

| fieldname | label | fieldtype | Notes |
|---|---|---|---|
| `total_amount` | Total Amount | Currency | Computed from lines or amount field |
| `total_amount_base` | Total (Base) | Currency | total_amount × exchange_rate |

**Validation rules** (in `fn::validate::payment_*`):
1. `payment_type` consistent with `payment_direction` (Expense Payment only with Outgoing, etc.)
2. Expense/Income: all line accounts match expected `fs_type` (Expense for payments, Income for receipts)
3. Party Payment/Receipt: `party_role` consistent with direction (Supplier for Outgoing, Customer for Incoming)
4. Internal Transfer: both accounts are Cash and Bank, `from_account ≠ to_account`
5. `paid_from` set for Outgoing and Internal; `paid_into` set for Incoming and Internal
6. Casual Party not allowed for Party Payment or Party Receipt (subledger types require Full Party)
7. `allocated_amount` per invoice line ≤ invoice outstanding

**SurrealQL required**:
- `functions.surql`:
  - `fn::validate::payment_type_direction` — type/direction consistency matrix
  - `fn::validate::payment_line_accounts` — fs_type check for expense/income lines
  - `fn::validate::payment_party_role` — role vs direction check
  - `fn::validate::payment_transfer_accounts` — from ≠ to, both Cash and Bank
  - `fn::validate::payment_paid_accounts` — paid_from / paid_into mandatory per direction
  - `fn::validate::payment_casual_party_scope` — Casual Party only on Expense/Income types
  - `fn::validate::payment_invoice_allocation` — allocated ≤ outstanding per invoice
  - `fn::compute::payment_totals` — compute total_amount, total_amount_base
  - `fn::compute::payment_naming_series` — set naming series prefix from direction
  - `fn::on_submit::payment_post_gl` — dispatch to type-specific posting function
  - `fn::payment::post_expense` — Expense Payment GL entries (multi-line debit, single credit)
  - `fn::payment::post_income` — Income Receipt GL entries (single debit, multi-line credit)
  - `fn::payment::post_party_payment` — Party Payment GL entries (advance or invoice)
  - `fn::payment::post_party_receipt` — Party Receipt GL entries (advance or invoice)
  - `fn::payment::post_transfer` — Internal Transfer GL entries
  - `fn::on_cancel::payment_reverse_gl` — reverse all GL Entries for this payment; unreconcile invoices
- `wiring.surql`: full submittable wiring

---

## Part 3 — SurrealQL Implementation Patterns

### 3.1 The Three-File Pattern Per DocType

Every non-trivial doctype uses exactly three file types:

```
functions.surql    → DEFINE FUNCTION OVERWRITE fn::*::doctype_name_*(...)
wiring.surql       → call wire_generic, then UPSERT nodes and RELATE edges
```

Simple doctypes (Currency, Payment Terms, FSV) need only these two files. Complex doctypes (Account, Journal Entry) need only these two files but with more functions. There is no `meta.surql` separate file — meta registration happens inside `wiring.surql` via `fn::pipeline::wire_generic`.

### 3.2 Function Signature Contract

Every pipeline function **must** follow this signature exactly:

```surql
DEFINE FUNCTION OVERWRITE fn::<stage>::<doctype>_<purpose>(
    $doc_id: record,
    $config: object
) {
    -- ...logic...
    RETURN { ok: true };   -- success
    -- or:
    RETURN { error: "Human-readable error message" };  -- failure, Rust rolls back
};
```

Functions never THROW. They always return either `{ ok: true }` or `{ error: "..." }`. The runner in `02_runner.surql` checks the return value and propagates the error to Rust, which rolls back the insert/update.

### 3.3 Validation Functions — Template

```surql
DEFINE FUNCTION OVERWRITE fn::validate::currency_code(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT code FROM $doc_id)[0];
    
    IF $doc.code = NONE OR $doc.code = "" {
        RETURN { error: "Currency code is required" };
    };
    
    IF string::len($doc.code) != 3 {
        RETURN { error: "Currency code must be exactly 3 characters" };
    };
    
    IF $doc.code != string::uppercase($doc.code) {
        RETURN { error: "Currency code must be uppercase" };
    };
    
    RETURN { ok: true };
};
```

### 3.4 Compute Functions — Template

Compute functions run in the `save/compute` stage, AFTER validation and BEFORE persist. They update the record in-place using UPDATE. The updated record is what gets persisted.

```surql
DEFINE FUNCTION OVERWRITE fn::compute::account_derive_from_group(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT account_group FROM $doc_id)[0];
    
    IF $doc.account_group = NONE { RETURN { ok: true }; };
    
    LET $grp = (
        SELECT account_type_internal, fs_type, is_reconciliation_group, party_required
        FROM tabAccount_Group  -- use actual table name
        WHERE name = $doc.account_group
        LIMIT 1
    )[0];
    
    IF $grp = NONE {
        RETURN { error: string::concat("Account Group not found: ", <string>$doc.account_group) };
    };
    
    UPDATE $doc_id SET
        account_type_internal   = $grp.account_type_internal,
        fs_type                 = $grp.fs_type,
        is_reconciliation_account = $grp.is_reconciliation_group,
        party_required          = $grp.party_required;
    
    RETURN { ok: true };
};
```

### 3.5 On-Submit GL Posting — Template

The pattern for `fn::on_submit::je_create_gl_entries` is the most critical function in the system. It uses a `FOR` loop inside SurrealDB's transaction boundary:

```surql
DEFINE FUNCTION OVERWRITE fn::on_submit::je_create_gl_entries(
    $doc_id: record,
    $config: object
) {
    LET $doc = (
        SELECT accounts, posting_date, company, voucher_type, user_remark
        FROM $doc_id
    )[0];
    
    LET $doc_name = record::id($doc_id);
    
    FOR $row IN $doc.accounts {
        LET $gl_name = string::concat($doc_name, "-GL-", <string>($row.idx OR 0));
        
        INSERT INTO `tabGL Entry` {
            name:           $gl_name,
            company:        $doc.company,
            account:        $row.account,
            posting_date:   $doc.posting_date,
            debit:          $row.debit OR 0,
            credit:         $row.credit OR 0,
            party:          $row.party,
            party_role:     $row.party_role,
            cost_center:    $row.cost_center,
            voucher_type:   $doc.voucher_type OR "Journal Entry",
            voucher_no:     $doc_name,
            remarks:        $doc.user_remark,
            docstatus:      1,
            is_cancelled:   false
        };
        -- If tabGL Entry's validate event rejects the insert, the entire
        -- FOR loop transaction rolls back. Journal Entry stays at docstatus=0.
    };
    
    RETURN { ok: true };
};
```

### 3.6 Wiring File — Template

```surql
-- Currency — pipeline wiring
-- Non-submittable: only save stages needed.

RETURN fn::pipeline::wire_generic("Currency", "tabCurrency", false);

-- Node: validate code format
IF (SELECT id FROM pipeline_node:currency_validate_code LIMIT 1) = [] {
    UPSERT pipeline_node:currency_validate_code CONTENT {
        name:    "currency_validate_code",
        fn_name: "fn::validate::currency_code"
    };
};

-- Node: validate single base currency
IF (SELECT id FROM pipeline_node:currency_validate_single_base LIMIT 1) = [] {
    UPSERT pipeline_node:currency_validate_single_base CONTENT {
        name:    "currency_validate_single_base",
        fn_name: "fn::validate::currency_single_base"
    };
};

-- Wire both into save/validate
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:tabCurrency_save_validate
          AND out = pipeline_node:currency_validate_code
        LIMIT 1) = [] {
    RELATE pipeline_stage:tabCurrency_save_validate
        -> has_node ->
        pipeline_node:currency_validate_code
        CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:tabCurrency_save_validate
          AND out = pipeline_node:currency_validate_single_base
        LIMIT 1) = [] {
    RELATE pipeline_stage:tabCurrency_save_validate
        -> has_node ->
        pipeline_node:currency_validate_single_base
        CONTENT { ord: 10, config: {} };
};
```

**Critical**: The `pipeline_stage` record ID uses the table name (with `tab` prefix), not the DocType name. Example: `pipeline_stage:tabCurrency_save_validate`, not `pipeline_stage:Currency_save_validate`. This is because `fn::pipeline::wire_generic` receives both the doctype name and table name and uses the table name for stage IDs (see `04_wire_generic.surql`).

> **Check before writing**: Verify whether `wire_generic` uses `$doctype` or `$table` for stage IDs by reading `04_wire_generic.surql` lines 40-60. Adjust stage ID references in wiring files accordingly.

### 3.7 Shared Functions

These are written once in `surql/shared/` and reused across all doctypes:

**`validate_mandatory.surql`** — already exists in ERPNext surql/shared. Reference it, don't duplicate.

**`validate_period_open.surql`**:
```surql
DEFINE FUNCTION OVERWRITE fn::validate::period_open_for_date(
    $company: string,
    $posting_date: datetime
) {
    LET $period = (
        SELECT id, is_closed FROM `tabAccounting Period`
        WHERE company = $company
          AND start_date <= $posting_date
          AND end_date   >= $posting_date
        LIMIT 1
    )[0];
    
    IF $period = NONE {
        RETURN { error: "No Accounting Period found for this date and company" };
    };
    
    IF $period.is_closed = true OR $period.is_closed = 1 {
        RETURN { error: "Accounting Period is closed for this date" };
    };
    
    RETURN { ok: true };
};
```

**`derive_period.surql`**:
```surql
DEFINE FUNCTION OVERWRITE fn::compute::derive_fiscal_period(
    $doc_id: record,
    $company_field: string,
    $date_field: string
) {
    LET $doc = (SELECT * FROM $doc_id)[0];
    LET $company = $doc[$company_field];
    LET $date    = $doc[$date_field];
    
    LET $fy = (
        SELECT name FROM `tabFiscal Year`
        WHERE company = $company
          AND year_start_date <= $date
          AND year_end_date   >= $date
        LIMIT 1
    )[0];
    
    LET $period = (
        SELECT name FROM `tabAccounting Period`
        WHERE company = $company
          AND start_date <= $date
          AND end_date   >= $date
        LIMIT 1
    )[0];
    
    IF $fy = NONE {
        RETURN { error: "No Fiscal Year found for posting date" };
    };
    
    UPDATE $doc_id SET
        fiscal_year       = $fy.name,
        accounting_period = $period.name;
    
    RETURN { ok: true };
};
```

---

## Part 4 — DocType JSON Structure

### 4.1 Minimum Required JSON

Every DocType JSON file must have at minimum:

```json
{
    "doctype": "DocType",
    "name": "Currency",
    "module": "Core",
    "autoname": "field:code",
    "is_submittable": 0,
    "istable": 0,
    "fields": [
        {
            "fieldname": "code",
            "label": "ISO Code",
            "fieldtype": "Data",
            "reqd": 1,
            "unique": 1,
            "in_list_view": 1,
            "idx": 1
        }
    ],
    "permissions": [
        {
            "role": "System Manager",
            "read": 1,
            "write": 1,
            "perm_create": 1,
            "perm_delete": 1,
            "submit": 0,
            "perm_cancel": 0,
            "amend": 0,
            "if_owner": 0
        },
        {
            "role": "Accounts Manager",
            "read": 1,
            "write": 1,
            "perm_create": 1,
            "perm_delete": 0,
            "submit": 0,
            "perm_cancel": 0,
            "amend": 0,
            "if_owner": 0
        }
    ]
}
```

The `idx` field on each field definition is **mandatory** — it drives the order in which fields appear in the Designer and on the form (as per the field ordering fix documented in user memory). Start at 1, increment by 1. Do not skip numbers.

### 4.2 Roles to Create

The following roles are needed for the Finance module. Add them to the Frappe core app or create a roles fixture in `spotledger-finance`:

| Role | Description |
|---|---|
| Accounts Manager | Full access to all accounting doctypes |
| Accounts User | Read + create, no delete |
| Finance Controller | Submit/Cancel access for GL Entry and Journal Entry |
| System Manager | Full access (already exists in Tier 0) |

---

## Part 5 — Phased Execution Plan

### Phase 1 — App Scaffolding

**Deliverable**: `apps/spotledger-finance/` directory that installs without errors.

Tasks:
1. Create `apps/spotledger-finance/app.json` — app name, version, `depends_on: ["spotledger-core"]`
2. Create `apps/spotledger-finance/spotledger_finance/modules.txt` — three lines: `Core`, `Business Partner`, `Accounts`
3. Create empty `doctype/` stub directories with placeholder `*.json` files (just name + module fields) for all 18 doctypes listed in Part 2
4. Verify: `spotledger install-app spotledger-finance <sitename> --bench .` completes with 0 errors, 18 DocTypes in tabDocType
5. Verify: All 18 DocType tables exist in SurrealDB (visible in Surrealist)

**Success criterion**: `spotledger install-app` runs clean. Designer shows all 18 doctypes.

---

### Phase 2 — Module Core (Foundation Layer)

**Deliverable**: Currency, Company, Fiscal Year, Accounting Period, Cost Center fully functional with validation pipeline wired.

Order is strict — each depends on the previous being saved in DB:
1. **Currency** (no dependencies) — JSON + functions.surql + wiring.surql
2. **Fiscal Year** (depends on Company, but Company depends on Chart of Accounts which doesn't exist yet in Phase 2) — implement validation only, skip Company link validation for now
3. **Accounting Period** (depends on Fiscal Year)
4. **Cost Center** (depends on Company — stub validation, no CoA link yet)
5. **Company** — implement after CoA module is complete (Phase 4 step 1)

**Why Company is last**: Company requires Chart of Accounts and Account links (retained_earnings_account etc.). These don't exist until Phase 4. In Phase 2, save the Company JSON and its basic validation functions but skip account-related validations. Mark them with `-- TODO: Phase 4` comments.

**Tests for Phase 2**:
- Save a Currency PKR → should succeed
- Save a second Currency with is_base=true when PKR already has is_base=true → should fail with validation error
- Save a Fiscal Year with end before start → should fail
- Save two overlapping Fiscal Years for same company → should fail

---

### Phase 3 — Module Business Partner

**Deliverable**: Payment Terms, Party, Party Role Config functional.

Order:
1. **Payment Terms** — no dependencies
2. **Party** — depends on Currency only (available from Phase 2)
3. **Party Role Config** — depends on Party, Company (stub), Account (stub → full link validation deferred to Phase 4)

**Deferral**: `fn::validate::party_role_config_account_types` (checks that receivable is type D etc.) is deferred until Phase 4 when Account and Account Group exist in DB.

**Tests for Phase 3**:
- Save a Party with no roles → should fail
- Save a Party "Rehman Traders" with roles [Customer, Supplier] → should succeed
- Save a second Party with same tax_id → should fail
- Save Party Role Config with receivable_account that is type K → should fail (Phase 4 test)

---

### Phase 4 — Module Accounts (Foundation)

**Deliverable**: Full Chart of Accounts, Account Group, Account, Account Company, FSV stack operational.

Order:
1. **Chart of Accounts** (depends on Currency only)
2. **Account Group** (depends on CoA)
3. **Account** (depends on Account Group, CoA)
4. **Account Company** (depends on Account, Company)
5. **Financial Statement Version** (depends on CoA)
6. **FSV Node** (depends on FSV)
7. **FSV Account Mapping** (depends on FSV Node, CoA)
8. **Complete Company validation** — now that Account exists, add the deferred validators for `retained_earnings_account` type, `default_receivable_account` type, `default_payable_account` type
9. **Complete Party Role Config validation** — add the deferred account type validators
10. **Wire `fn::on_save::company_bootstrap`** — creates Fiscal Year, Accounting Periods, Account Company rows on first Company save

**Tests for Phase 4**:
- Create CoA "SpotLedger Standard PKR" → should succeed
- Create Account Group "Trade Receivable" (number range 2200-2299) → account_type_internal should be D, party_required should be 1
- Create Account 2210 in that group → classification derived from group
- Create Account 2210 again → should fail (duplicate)
- Create Account with account_number outside group range → should fail
- Create a Company → should auto-create Account Company rows for all accounts, plus Fiscal Year + 12 periods
- Set `parent_account` on Account → `child_of` graph edge should exist in SurrealDB

---

### Phase 5 — GL Entry and Journal Entry

**Deliverable**: Journal Entry form works end-to-end: save, submit creates GL Entries, cancel reverses them. This is the core accounting brain.

Order:
1. **GL Entry** — functions.surql + wiring.surql (new, no ERPNext copy)
2. **Journal Entry Account** — child table JSON only (no pipeline needed, validation is on parent)
3. **Journal Entry** — full pipeline including `fn::on_submit::je_create_gl_entries` and `fn::on_cancel::je_cancel_gl_entries`

**Key decision**: GL Entry `fn::validate::gl_entry_period_open` calls the shared `fn::validate::period_open_for_date` written in Phase 2. This is the cross-module function call that makes the shared/ pattern worthwhile.

**Tests for Phase 5**:
- Submit a Journal Entry where total debit ≠ total credit → should fail
- Submit a Journal Entry posting to a group account → should fail  
- Submit a Journal Entry into a closed Accounting Period → should fail
- Submit a balanced Journal Entry → should succeed, tabGL Entry should have 2 rows
- Cancel the submitted Journal Entry → GL Entries should have is_cancelled=true and debit/credit swapped
- Query `SELECT sum(debit - credit) FROM tabGL Entry WHERE voucher_no = "JE-00001"` → should be 0.00

---

### Phase 6 — Payments Module

**Deliverable**: Payment and Receipt DocType functional end-to-end. Casual Party inline creation working. All five payment types (Expense Payment, Income Receipt, Party Payment, Party Receipt, Internal Transfer) post correct GL Entries on submit and reverse on cancel.

Order:
1. **Casual Party** — JSON + functions.surql + wiring.surql. Simple save-only pipeline
2. **Payment Line** — child table JSON only (istable=1, no pipeline)
3. **Invoice Allocation** — child table JSON only (istable=1, no pipeline)
4. **Payment** — full pipeline: validate → compute → persist → on_submit (GL posting) → on_cancel (GL reversal)
5. **Payments fixtures** — seed `One-Time Payable`, `One-Time Receivable`, `Supplier Advance`, `Customer Advance` accounts

**Naming series**:
- `PAY-YYYY-` for Outgoing
- `REC-YYYY-` for Incoming  
- `TRF-YYYY-` for Internal Transfer

All three resolve via the same `Payment` DocType. The naming series is computed in `save/compute` based on `payment_direction`.

**GL posting dispatch logic** (in `fn::on_submit::payment_post_gl`):
```surql
IF $doc.payment_type = "Expense Payment"    { RETURN fn::payment::post_expense($doc_id, $config); };
IF $doc.payment_type = "Income Receipt"     { RETURN fn::payment::post_income($doc_id, $config); };
IF $doc.payment_type = "Party Payment"      { RETURN fn::payment::post_party_payment($doc_id, $config); };
IF $doc.payment_type = "Party Receipt"      { RETURN fn::payment::post_party_receipt($doc_id, $config); };
IF $doc.payment_type = "Internal Transfer"  { RETURN fn::payment::post_transfer($doc_id, $config); };
RETURN { error: "Unknown payment_type" };
```

**Advance auto-detection** (inside `fn::payment::post_party_payment` and `post_party_receipt`):
```surql
LET $use_advance = array::len($doc.against_invoices OR []) = 0;
LET $trade_acct  = IF $use_advance {
    -- fetch advance_account from Party Role Config for this party+role+company
    (SELECT advance_account FROM tabParty_Role_Config
     WHERE party = $doc.party AND role = $doc.party_role AND company = $doc.company
     LIMIT 1)[0].advance_account
} ELSE {
    (SELECT receivable_account OR payable_account FROM tabParty_Role_Config
     WHERE party = $doc.party AND role = $doc.party_role AND company = $doc.company
     LIMIT 1)[0]
};
```

**Tests for Phase 6**:
- Expense Payment with two lines → GL has 3 entries: 2 debits (expense accounts) + 1 credit (bank)
- Income Receipt → GL has debit (bank) + credit (income account)
- Party Payment against invoice → GL debit Trade Payable, credit bank; party tag on GL Entry
- Party Payment with empty invoice table → GL debit Supplier Advance, credit bank (advance path)
- Party Receipt advance → GL debit bank, credit Customer Advance
- Internal Transfer → GL debit to_account, credit from_account
- Casual Party creation inline → record created, default_account set to One-Time Payable
- Cancel any submitted Payment → GL Entries reversed (is_cancelled=true, amounts swapped)
- `paid_from` = `paid_into` on Internal Transfer → validation error
- Post to closed period → rejected

---

### Phase 7 — Designer Visibility Verification ✅ Done

**Started**: 2026-04-19  
**Completed**: 2026-04-19  
**Checkpoint complete**: `spotledger.designer.save` round-trip verified by `test_doctype_save.rs` — ✅ 16/16 passing.

**HTTP-level verification evidence (live server, host=spotledger)**:
- DocType list endpoint (`GET /api/resource/DocType?fields=["name"]&limit=500`) returned 70 doctypes and included: Payment, Casual Party, GL Entry, Journal Entry, Account, Company.
- Designer pipeline endpoint (`POST /api/method/spotledger.designer.get_pipeline`, doctype=Payment) returned 9 stages with required stage pairs present:
    save/validate, save/compute, save/persist,
    submit/validate, submit/compute, submit/persist, submit/on_submit,
    cancel/validate_cancel, cancel/on_cancel.
- Payment node visibility confirmed on the same endpoint: `fn::validate::payment_type_direction`, `fn::validate::payment_casual_party_scope`, `fn::on_submit::payment_post_gl`, `fn::on_cancel::payment_reverse_gl`.
- Casual Party pipeline endpoint (`POST /api/method/spotledger.designer.get_pipeline`, doctype=Casual Party) returned 3 stages and included `fn::validate::casual_party_not_duplicate`.

After Phase 6, verify everything is visible and operable in the DocType Designer:

1. Open Designer → all 22 doctypes should appear in the list ✅ verified via DocType list API
2. Click on Payment → should show all fields including conditional sections ✅ metadata and pipeline APIs both returned expected Payment records
3. Pipeline tab → should show stages: save/validate, save/compute, save/persist, submit/validate, submit/on_submit, cancel/validate_cancel, cancel/on_cancel ✅ verified via get_pipeline
4. Each stage should show its wired nodes (e.g. submit/on_submit shows `payment_post_gl`) ✅ verified via get_pipeline nodes
5. Save a new DocType via Designer → should round-trip through `spotledger.designer.save` correctly ✅ verified by `test_doctype_save.rs`

---

### Phase 8 — Fixtures and Initial Data (In Progress)

**Started**: 2026-04-19

Phase 8 has started in code with a dedicated CLI command for repeatable fixture seeding:

- Command: `spotledger seed-finance-fixtures <site> [--bench <path>] [--coa-name <name>]`
- Wiring added in the Spotledger CLI dispatch and command enum.
- Seeding module added for idempotent upsert-style inserts.

**Implemented fixture scope (first pass)**:
- Currencies:
    - PKR (base)
    - USD
    - AED
- Default Chart of Accounts:
    - Name default: `SpotLedger Standard PKR` (overridable with `--coa-name`)
    - Currency: PKR
- Account Groups:
    - Full Section 2 category/range set seeded (13 groups)
    - Includes required range convention entry: Trade Receivable (2200-2299)
- Default Accounts:
    - Baseline 4-digit chart seeded (25 accounts)
    - Includes Cash/Bank, One-Time Payable, Retained Earnings, Revenue/COGS/Expense set
- Financial Statement Versions:
    - Pakistan Statutory (purpose: Statutory)
    - Management Reporting (purpose: Management)

**Execution status**:
- Command compiles in workspace checks.
- Command executed successfully against site `spotledger`:
    - `spotledger.exe seed-finance-fixtures spotledger --bench . --coa-name "SpotLedger Standard PKR"`
- HTTP/API verification (Host=`spotledger`, login as Administrator) confirmed inserted records:
    - Currency: 3 records (PKR, USD, AED)
    - Chart of Accounts: 1 record (`SpotLedger Standard PKR`, currency=PKR)
    - Account Group: 13 records for CoA `SpotLedger Standard PKR`
    - Account: 25 records for CoA `SpotLedger Standard PKR` (verified across pagination: first 20 + next 5)
    - Financial Statement Version: 2 records (`Pakistan Statutory`, `Management Reporting`)
- Fixture seeding scope for Phase 8 is now implemented and verified.

Seed the following data records (these are fixture data, not DocType metadata):

**Currencies**:
- PKR — Pakistani Rupee, ₨, is_base=true
- USD — United States Dollar, $
- AED — UAE Dirham, AED

**Default Chart of Accounts**: "SpotLedger Standard PKR" with the account number convention from Section 2 of the Finance Module Plan (1000-9999 structure).

**Account Groups**: Full set per the Finance Module Plan Section 2 account number convention.

**Default Accounts**: The full standard chart following the 4-digit code structure documented in the Finance Module Plan.

**Financial Statement Versions**:
- "Pakistan Statutory" (Statutory purpose)
- "Management Reporting" (Management purpose)

---

## Part 6 — What Is NOT In Scope For This Plan

The following are explicitly deferred to separate plan documents:

| Feature | Deferred To |
|---|---|
| Sales Invoice, Purchase Invoice | transactions app plan |
| Payment Reconciliation (matching advances to invoices after-the-fact) | transactions app plan |
| Trial Balance, Balance Sheet, P&L reports | reports app plan |
| Multi-currency GL (exchange rate revaluation) | Phase 8 of accounts plan |
| Intercompany elimination | Phase 9 of accounts plan |
| Franchise management reporting | Phase 10 of accounts plan |
| Item, Warehouse, Stock Entry | inventory app plan |
| Cost Center Allocation | Phase 6 extension |
| Budget vs Actual | reports app plan |
| Year-end closing journal | Phase 8 |
| Bank reconciliation | transactions app plan |

---

## Part 7 — Critical Invariants to Encode From Day One

These must be in the SurrealQL from the first working version. Do not defer:

1. **GL Entry debit − credit sum = 0** per voucher. Verify with: `SELECT math::sum(debit) - math::sum(credit) AS bal FROM tabGL Entry WHERE voucher_no = $vno GROUP ALL` → must be 0.
2. **Accounting Period sequential close**: Cannot close period N if period N−1 is still open. Encoded in `fn::validate::period_sequential_close`.
3. **Account type immutability**: Once a GL Entry exists for an account, its `account_type_internal` cannot change. Encoded in `fn::validate::account_type_immutable`.
4. **GL Entry no manual edit after submit**: `save/validate` stage checks `docstatus = 1` and returns error.
5. **No posting to group accounts**: Encoded in `fn::validate::gl_entry_account`.
6. **No posting to closed period**: Encoded in `fn::validate::gl_entry_period_open`.
7. **Party required on reconciliation accounts**: Encoded in `fn::validate::gl_entry_party_required`.

---

## Part 8 — Reuse vs New Code Checklist

After erpnext/frappe removal (see Pre-Implementation Work), the pipeline framework lives in `apps/spotledger-core/`. All paths below reflect that final location.

Before writing any SurrealQL function, check if a reusable node already exists:

| Needed behavior | Location after migration | Action |
|---|---|---|
| Mandatory field check | `apps/spotledger-core/spotledger_core/surql/shared/fn_validate_mandatory.surql` | REUSE. Wire into save/validate at ord=1 |
| Mark cancelled | `apps/spotledger-core/spotledger_core/surql/framework/03_universal_nodes.surql` | REUSE for non-GL doctypes. GL uses custom reversal |
| No dependents cancel check | `apps/spotledger-core/spotledger_core/surql/framework/03_universal_nodes.surql` | REUSE in cancel/validate_cancel |
| wire_generic | `apps/spotledger-core/spotledger_core/surql/framework/04_wire_generic.surql` | REUSE. Call it from every wiring.surql |
| GL Entry validation | `apps/spotledger-finance/spotledger_finance/surql/doctypes/gl_entry/functions.surql` | NEW in this app. Reference only from ERPNext as a pattern |
| Journal Entry balance | `apps/spotledger-finance/spotledger_finance/surql/doctypes/journal_entry/functions.surql` | NEW in this app |

The `spotledger-finance` app does NOT re-implement the pipeline framework. It calls the framework functions defined in `spotledger-core`. `spotledger-finance` declares `spotledger-core` as its only dependency.

---

## Appendix A — install_app Processing Order for Spotledger-Finance

When `spotledger install-app spotledger-finance <site>` runs, the install_app.rs should apply surql files in this order:

```
1. apply surql/shared/*.surql           (shared utility functions)
2. apply surql/doctypes/currency/functions.surql
3. apply surql/doctypes/currency/wiring.surql
4. apply surql/doctypes/company/functions.surql
5. apply surql/doctypes/company/wiring.surql
6. apply surql/doctypes/fiscal_year/functions.surql
7. apply surql/doctypes/fiscal_year/wiring.surql
8. apply surql/doctypes/accounting_period/functions.surql
9. apply surql/doctypes/accounting_period/wiring.surql
10. apply surql/doctypes/cost_center/functions.surql
11. apply surql/doctypes/cost_center/wiring.surql
12. apply surql/doctypes/payment_terms/functions.surql
13. apply surql/doctypes/payment_terms/wiring.surql
14. apply surql/doctypes/party/functions.surql
15. apply surql/doctypes/party/wiring.surql
16. apply surql/doctypes/chart_of_accounts/functions.surql
17. apply surql/doctypes/chart_of_accounts/wiring.surql
18. apply surql/doctypes/account_group/functions.surql
19. apply surql/doctypes/account_group/wiring.surql
20. apply surql/doctypes/account/functions.surql
21. apply surql/doctypes/account/wiring.surql
22. apply surql/doctypes/gl_entry/functions.surql
23. apply surql/doctypes/gl_entry/wiring.surql
24. apply surql/doctypes/journal_entry/functions.surql
25. apply surql/doctypes/journal_entry/wiring.surql
26. apply payments/surql/doctypes/casual_party/functions.surql
27. apply payments/surql/doctypes/casual_party/wiring.surql
28. apply payments/surql/doctypes/payment/functions.surql
29. apply payments/surql/doctypes/payment/wiring.surql
```

This order is critical: wiring files call `fn::pipeline::wire_generic` which must exist before any wiring file runs. The framework surql lives in `spotledger-core` (after Work Item 1 migration) and is applied during its install, which must run before spotledger-finance.

---

## Appendix B — Resolved Questions and Pre-Implementation Work Items

---

### B.1 Resolved: Technical Questions

**Q1 — Table name for doctypes with spaces**

`doctype_to_table()` in `crates/spotledger-db/src/document.rs` line 33:
```rust
pub fn doctype_to_table(doctype: &str) -> String {
    format!("tab{}", doctype.replace(' ', "_"))
}
```
Spaces become underscores. `"Account Group"` → `tabAccount_Group`. `"GL Entry"` → `tabGL_Entry`. `"Journal Entry"` → `tabJournal_Entry`. `"Chart of Accounts"` → `tabChart_of_Accounts`. This is authoritative — every SurrealQL file in this plan must use this convention.

**Q2 — wire_generic stage ID prefix: `$table` or `$doctype`?**

Confirmed from `04_wire_generic.surql` lines 40–43: **stage IDs use `$table`**, not `$doctype`.
```surql
LET $sv = string::concat($table, "_save_validate");
LET $sp = string::concat($table, "_save_persist");
```
The `$doctype` value (with spaces, e.g. `"GL Entry"`) is stored as the `doctype` field on the stage record for display/lookup. The record ID always uses the underscore table name. Example stage IDs:
- `pipeline_stage:tabGL_Entry_save_validate`
- `pipeline_stage:tabJournal_Entry_cancel_on_cancel`
- `pipeline_stage:tabAccount_Group_save_compute`

All wiring files must construct stage reference IDs using `$table`.

**Q3 — `child_of` relation table: defined where?**

Not defined anywhere in the current codebase. It is documented in `GRAPH_COMPUTE_PLAN.md` as a planned table. It does not exist yet in `01_schema.surql`. It must be added as part of Work Item 1 (framework migration to spotledger-core). Definition:
```surql
DEFINE TABLE IF NOT EXISTS child_of SCHEMAFULL;
DEFINE FIELD OVERWRITE in  ON child_of TYPE record;
DEFINE FIELD OVERWRITE out ON child_of TYPE record;
DEFINE INDEX OVERWRITE idx_child_of ON child_of FIELDS in, out UNIQUE;
```

**Q4 — `app.json` `depends_on` format**

Confirmed from `apps/spotledger-core/app.json`:
```json
{ "depends_on": [] }
```
Simple array of app name strings. `spotledger-finance` will declare:
```json
{ "depends_on": ["spotledger-core"] }
```

**Q5 — Fixture seeding mechanism: does it exist?**

Yes — fully implemented. `install_app.rs` has `seed_module_fixtures()` which reads from `{app_root}/{module}/fixtures/{DocType}.json`. Each file is a JSON array of records, each record must have a `"name"` field. Currency fixture example path:
```
apps/spotledger-finance/spotledger_finance/core/fixtures/Currency.json
```

---

### B.2 Pre-Implementation Work Items

> **STATUS: ALL 5 WORK ITEMS COMPLETE** ✅ (Completed April 2026)

---

#### WORK ITEM 1 — Migrate pipeline framework from erpnext to spotledger-core; delete erpnext/frappe apps ✅ DONE

**Completed**: All 7 framework surql files + 4 shared surql files moved to `apps/spotledger-core/spotledger-core/surql/framework/` and `surql/shared/`. `child_of` SCHEMAFULL relation table added to `01_schema.surql`. `save/compute` stage added to `04_wire_generic.surql` (ord=2, persist shifted to ord=3). `apps/frappe/` and `apps/erpnext/` deleted. 4 `include_str!` paths in `schema.rs` and `designer.rs` updated. Build clean.

---

#### WORK ITEM 2 — Child Table Contract: Confirm Inline-Embedding is Correct for Finance ✅ DONE

**Problem**: The entire pipeline framework (`01_schema.surql` through `07_workflow.surql` and `shared/`) currently lives in `apps/erpnext/`. We are deleting erpnext and frappe. The framework must move to `apps/spotledger-core/` first.

**Actions**:
1. Create `apps/spotledger-core/spotledger_core/surql/framework/` and `surql/shared/`
2. Move all 7 framework surql files and all 4 shared surql files into those paths
3. Add `child_of` relation table definition to `01_schema.surql` (see Q3 above)
4. Update `install_app.rs` to scan and apply `{app_root}/surql/framework/` and `{app_root}/surql/shared/` per installed app, in dependency order
5. Delete `apps/frappe/` and `apps/erpnext/` entirely
6. Verify `spotledger install-app spotledger-core <site>` applies all framework surql and the pipeline tables exist in SurrealDB

**No Rust struct changes needed** — only file paths change. The `install_app.rs` apply_surql logic already exists.

**Rule 4 retraction**: Since erpnext is gone, there is no DocType name collision risk. The finance module uses canonical names: `GL Entry`, `Journal Entry`, `Account`, `Payment`, `Casual Party`, etc. No invented aliases.

---

#### WORK ITEM 2 — Child Table Contract: Confirm Inline-Embedding is Correct for Finance

**Current implementation**: Child table rows are stored as `array<any>` embedded inline in the parent document (`schema.rs` line 51: `FieldType::Table => Some("array<any>")`). There is no separate `tabJournal_Entry_Account` table with foreign keys.

**Conclusion**: No gap for this plan. The inline model is correct for all child tables in the finance module because child rows (Journal Entry Account) are never saved independently. The pipeline reads `$doc.accounts` directly from the embedded array.

**Explicit storage contract**:

| DocType | `istable` | Storage | How accessed in pipeline |
|---|---|---|---|
| Journal Entry Account | 1 (child) | Embedded array in `tabJournal_Entry.accounts` | `LET $rows = $doc.accounts;` inside JE functions |
| GL Entry | 0 (standalone) | Own table `tabGL_Entry` | `SELECT FROM tabGL_Entry WHERE voucher_no = $name` |
| Party Role Config | 0 (standalone) | Own table `tabParty_Role_Config` with `party` link field | Queried by party + company, not embedded |

**Party Role Config rationale**: Keeping it standalone (not embedded) allows company-filtered queries without scanning all parties. It is linked to Party via a `party` link field, displayed in the Party form as a child table section, but stored independently.

---

#### WORK ITEM 3 — Add `save/compute` Stage to wire_generic

**Problem**: Current `04_wire_generic.surql` only creates `save/validate` (ord=1) and `save/persist` (ord=2) for the save action. A `save/compute` stage does not exist. The finance module needs it for Account Group classification derivation and Account field derivation.

**Fix**: Add `save/compute` at ord=2, shift `save/persist` to ord=3. Update `04_wire_generic.surql`:

**Completed**: `save/compute` stage added at ord=2, `save/persist` shifted to ord=3. Backward-compatible. ✅ DONE (part of Work Item 1)

---

#### WORK ITEM 4 — Core Reference Data Fixtures ✅ DONE

**Completed**: All four fixture files seeded in `apps/spotledger-core/spotledger-core/geo/fixtures/`:
- `Country.json` — 249 ISO 3166-1 countries (already existed)
- `Currency.json` — 180 ISO 4217 currencies (`currency_name` = ISO code, autoname-compatible; already existed, updated)
- `Language.json` — 184 ISO 639-1 languages (created)
- `Timezone.json` — ~330 IANA timezones (created)

Note: Currency fixture location is `geo/fixtures/Currency.json` (not `setup/fixtures/` as originally planned — aligned with actual app module structure).

---

#### WORK ITEM 5 — Test Infrastructure for DocType Pipeline Functions ✅ DONE

**Completed**: 8 integration test scaffold files created in `crates/spotledger-db/tests/`:
`test_currency.rs`, `test_account_group.rs`, `test_account.rs`, `test_fiscal_year.rs`, `test_accounting_period.rs`, `test_party.rs`, `test_gl_entry.rs`, `test_journal_entry.rs`.

All 8 files pass: 40 total integration tests green. Unit tests: 80 passed. Naming tests: 24 passed.

**Key fixes resolved during Work Item 5**:
- `SetClause` in `query.rs` now emits `<datetime>$param` for ISO-8601 strings so SCHEMAFULL `TYPE datetime` fields accept them
- SurrealDB `TYPE decimal` serializes as JSON string; test assertions handle both `Number` and `String`  
- `currency_name` in Currency fixture = ISO code (matching `autoname = "field:currency_name"` convention)
- Naming test (`test_currency_name_is_iso_code`) calls `fn::naming::resolve` directly, seeding `tabDocType` with correct `autoname`








