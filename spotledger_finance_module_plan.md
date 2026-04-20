# SpotLedger Finance Module — Implementation Plan
## Master Reference Document for LLM-Assisted Development

---

## Document Purpose

This document is a complete implementation plan for the Finance Module of SpotLedger ERP. It is written to be fed directly to an LLM for code generation. Every design decision is explained with its reasoning. No ambiguity is intentional. Where a choice exists between two approaches, the chosen approach is stated explicitly with its reasoning.

---

## Technology Context

| Concern | Choice |
|---|---|
| Database | SurrealDB |
| Query Language | SurrealQL |
| Architecture | ERPNext-style DocTypes as the fundamental unit |
| Graph | SurrealDB graph edges used for traversable relationships |
| Backend | Rust — thin layer for routing, auth, websocket only |
| Business Logic | Lives in SurrealQL functions, not the application layer |
| Key Principle | Accounting brain lives in the database, not the application |

---

## Fundamental Design Principles

**Principle 1 — DocType is the unit**
Every entity in the system is a DocType. A DocType is a SurrealDB table with defined schema, events, and associated SurrealQL functions. It has a list view, a form view, and a controller. This is ERPNext's mental model and it is retained completely.

**Principle 2 — SAP account model, ERPNext DocType structure**
The Chart of Accounts and account classification follows SAP's battle-tested model exactly. The implementation wrapper around it follows ERPNext's DocType pattern.

**Principle 3 — Master CoA shared across companies**
One Operative Chart of Accounts exists in the system. All companies use it. Company-specific behavior is handled through a thin override table called Account Company. Users never interact with Account Company directly. It is entirely system-managed. This mirrors SAP's operative CoA concept.

**Principle 4 — Strong model from day one, complexity added gradually**
Every field needed for multi-company, intercompany, consolidation, local statutory reporting, and franchise management exists in the schema from day one. Application logic for advanced features is added in later stages. No breaking migrations ever.

**Principle 5 — GL Entry is sacred**
GL Entry is never deleted. It is never manually edited after submission. Corrections happen through reversal entries only. The sum of all debit_base minus credit_base across all GL Entries is always zero. This invariant is enforced at the database level, not the application level.

**Principle 6 — Business Partner over split party model**
A single Party master replaces ERPNext's separate Customer and Supplier doctypes. Role-based behavior handles the distinction. A party that is both customer and supplier is one record with two roles. This follows SAP S/4HANA's Business Partner concept.

**Principle 7 — User-facing simplicity, system-facing correctness**
Users see friendly labels and simple forms. The system stores technically correct SAP classifications derived automatically from user choices. Users never see SAP account type codes S, G, A, D, K, M.

**Principle 8 — Branches are dimensions, not companies**
Branches of the same legal entity are Cost Centers, not separate companies. Multi-company architecture is reserved for separate legal entities. This is the correct model in SAP, Oracle, and Dynamics 365.

**Principle 9 — FSV separates structure from presentation**
The Chart of Accounts structure is fixed. Financial statement presentation is a separate configuration layer called Financial Statement Version. Multiple FSVs can exist against the same CoA for management reporting, statutory filing, and bank reporting without touching underlying data.

---

## Section 1 — Multi-Entity Architecture

Before designing any DocType, the correct entity model must be established. Three distinct scenarios exist and each is handled differently.

### Scenario A — Branches

Branches are departments or locations of the same legal entity. Same tax registration, same audited accounts, same legal books. Management wants branch-wise P&L and balance sheet.

**Solution: Cost Centers, not Companies.**

Branches never get their own Chart of Accounts. They never need consolidation entries. Every GL Entry carries a Cost Center dimension. All reports filter by Cost Center to produce branch-wise statements. This is how SAP, Oracle, and Dynamics 365 handle branches universally.

```
Legal Entity: Rehman Traders (one Company record)
    Branch Lahore    → Cost Center: CC-LHR
    Branch Karachi   → Cost Center: CC-KHI
    Branch Islamabad → Cost Center: CC-ISB
```

### Scenario B — Subsidiaries (Legal Consolidation)

Separate legal entities with separate tax registrations and separately audited accounts. Parent owns shares in subsidiaries. Statutory requirement exists to produce consolidated group financial statements.

**Solution: Separate Company records with consolidation ledger.**

```
Holding Co (ledger_type: Consolidation)
    └── Subsidiary A (ledger_type: Operational, own books)
    └── Subsidiary B (ledger_type: Operational, own books)
    └── Subsidiary C (ledger_type: Operational, own books)
```

Consolidation requires elimination of intercompany transactions, elimination of investment against equity, minority interest calculation, and currency translation where subsidiaries operate in foreign currencies. This is handled in Stage 5 and Stage 6 of the implementation plan. The schema supports it from day one.

### Scenario C — Franchise Network (Management Consolidation)

Franchisees are legally independent entities. No statutory consolidation required. Franchisor needs combined management reports covering the full network — revenue, costs, royalties, performance benchmarks.

**Solution: Franchisees are Parties with role Franchisee, not Companies.**

Franchisees exist in SpotLedger as Party records, not Company records. Their financial data enters the system as royalty invoices, submitted performance reports, and imported data. The consolidated management report is a reporting construct built from this data. No elimination entries are needed because there is no intercompany ownership relationship.

```
Franchisor Company (operational Company record)
    Franchisee A → Party record, role: Franchisee
    Franchisee B → Party record, role: Franchisee
    Franchisee C → Party record, role: Franchisee
```

---

## Section 2 — Chart of Accounts Architecture

### The Three-Tier CoA Model

SpotLedger implements SAP's three-tier Chart of Accounts model. All posting happens against the Operative CoA. Group and Country CoAs are mapping layers only. No transaction ever references a Group or Country account directly.

```
Tier 1 — Operative CoA
    The working CoA. Every GL posting uses this.
    Shared across all company codes.
    One instance per deployment for SME use.

Tier 2 — Group CoA
    Consolidation CoA. Summary level.
    Each operative account maps to a group account via group_account field.
    Used only for consolidated group reporting.
    Example: 20 detailed revenue accounts in operative
             map to 3 revenue lines in group CoA.

Tier 3 — Country CoA
    Local statutory requirements.
    Each operative account maps to a local account via country_account field.
    Used only for statutory filing.
    Example: FBR Pakistan requires specific account classifications
             different from the operative structure.
```

### The Financial Statement Version Layer

In addition to the three CoA tiers, a Financial Statement Version (FSV) layer handles report presentation. This is separate from the CoA structure itself.

```
Operative CoA (fixed structure, accounts never change)
    ↓
Financial Statement Version (flexible presentation layer)
    FSV 1: Pakistan Statutory — for FBR filing
    FSV 2: Management Reporting — contribution margin format
    FSV 3: Bank Reporting — for loan applications
    FSV 4: IFRS — if needed in future
```

The same underlying GL Entries render differently under each FSV. No data changes. Only the grouping and labeling of accounts in the report changes.

### When Each Tier Is Actually Needed

| Tier | When Needed | SME Default |
|---|---|---|
| Operative CoA | Always, from day one | One instance, created at setup |
| Group CoA | When holding company structure exists | Leave null, schema ready |
| Country CoA | When operating in multiple tax jurisdictions | Leave null, schema ready |
| FSV | Always — needed to render Balance Sheet and P&L | Create two: Statutory and Management |

### Account Number Convention

SpotLedger uses four-digit account codes. Human readable, scales to 9999 accounts per group, tree structure implicit in number range. This is cleaner than SAP's ten-digit codes for SME context while retaining full structural clarity.

```
1000-1999    Non-Current Assets
    1100-1199    Tangible Fixed Assets
    1200-1299    Intangible Fixed Assets
    1300-1399    Long-Term Investments
    1400-1499    Capital Work in Progress

2000-2999    Current Assets
    2100-2199    Inventory
    2200-2299    Trade Receivables (Reconciliation Account, Type D)
    2300-2399    Other Receivables
    2400-2499    Advances and Prepayments
    2500-2599    Cash and Bank
        2510         Bank Accounts (Type S)
        2520         Cash in Hand (Type S)

3000-3999    Equity
    3100-3199    Share Capital
    3200-3299    Retained Earnings
    3300-3399    Other Reserves

4000-4999    Non-Current Liabilities
    4100-4199    Long-Term Loans
    4200-4299    Deferred Tax Liabilities

5000-5999    Current Liabilities
    5100-5199    Trade Payables (Reconciliation Account, Type K)
    5200-5299    Other Payables
    5300-5399    Tax Liabilities
    5400-5499    Accrued Liabilities
    5500-5599    Short-Term Borrowings

6000-6999    Revenue
    6100-6199    Sales Revenue
    6200-6299    Service Revenue
    6300-6399    Other Income

7000-7999    Cost of Sales
    7100-7199    Cost of Goods Sold
    7200-7299    Direct Labour
    7300-7399    Manufacturing Overhead

8000-8999    Operating Expenses
    8100-8199    Selling and Distribution
    8200-8299    General and Administrative
    8300-8399    Depreciation and Amortisation

9000-9999    Financial Items
    9100-9199    Finance Costs
    9200-9299    Finance Income
    9300-9399    Tax Expense
```

---

## Section 3 — DocType Specifications

### 3.1 DocType: Currency

**Purpose:** Master list of all currencies used in the system. Required before any other financial DocType can be created.

**Fields:**

| Field | Type | Rules |
|---|---|---|
| `code` | string | Mandatory, unique. ISO 4217. Example: PKR, USD, AED |
| `name` | string | Mandatory. Full name. Example: Pakistani Rupee |
| `symbol` | string | Mandatory. Example: ₨, $, AED |
| `is_base` | boolean | Default false. Only one currency can be base per deployment |
| `decimal_places` | integer | Default 2. JPY uses 0 |
| `enabled` | boolean | Default true |

**Constraints:**
- Only one record can have `is_base = true` across the entire table. Enforce with unique partial index.
- `code` must be uppercase, exactly 3 characters.
- Cannot delete a currency that has been used in any GL Entry.

**Notes:** Simple master DocType. No complex controller logic. Standard list view showing code, name, symbol, enabled status.

---

### 3.2 DocType: Chart of Accounts

**Purpose:** A named container defining a CoA structure. Not the accounts themselves. Think of it as the scheme under which accounts are organized.

**Fields:**

| Field | Type | Rules |
|---|---|---|
| `name` | string | Mandatory, unique. Example: SpotLedger Standard PKR |
| `code` | string | Mandatory, unique. Short identifier. Example: STD-PKR |
| `coa_type` | select | Mandatory. Options: Operative, Group, Country. Default: Operative |
| `parent_coa` | link → Chart of Accounts | Optional. For Group and Country CoAs, points to the Operative CoA they map from. Operative CoA has no parent |
| `root_currency` | link → Currency | Mandatory |
| `description` | text | Optional |
| `is_default` | boolean | Default false. The default Operative CoA used when creating new companies |

**Constraints:**
- Only one record can have `is_default = true`.
- Only one record of `coa_type = Operative` should exist per deployment for SME use.
- `parent_coa` is mandatory when `coa_type` is Group or Country.
- `parent_coa` must reference an Operative type CoA.
- Cannot delete a CoA that has accounts under it.

**Notes:** Almost no user interaction after initial setup. Created once during company setup wizard.

---

### 3.3 DocType: Account Group

**Purpose:** Groups accounts within a CoA for number range management and behavioral defaults. SAP's Kontengruppe concept. Controls which account numbers are valid and what the default behavior is for accounts in each group.

**Fields — Identity:**

| Field | Type | Rules |
|---|---|---|
| `name` | string | Mandatory, unique within CoA |
| `code` | string | Mandatory. Unique within CoA |
| `chart_of_accounts` | link → Chart of Accounts | Mandatory |
| `description` | text | Optional |

**Fields — Classification (shown to user):**

| Field | Type | Rules |
|---|---|---|
| `account_category` | select | Mandatory. This is what the user sees and selects. Options listed below |

**account_category options and their system mappings:**

| User Sees (account_category) | Internal Type (account_type_internal) | fs_type | is_reconciliation | party_required |
|---|---|---|---|---|
| Fixed Asset | A | Asset | true | false |
| Current Asset | S | Asset | false | false |
| Cash and Bank | S | Asset | false | false |
| Trade Receivable | D | Asset | true | true |
| Inventory | M | Asset | true | false |
| Equity | S | Equity | false | false |
| Long Term Liability | S | Liability | false | false |
| Trade Payable | K | Liability | true | true |
| Current Liability | S | Liability | false | false |
| Revenue | G | Income | false | false |
| Cost of Sales | G | Expense | false | false |
| Expense | G | Expense | false | false |
| Tax Account | S | Liability | false | false |

**Fields — Derived (hidden from user, set by controller):**

| Field | Type | Rules |
|---|---|---|
| `account_type_internal` | string | Hidden. SAP codes: S, G, A, D, K, M. Derived from account_category |
| `fs_type` | string | Hidden. Values: Asset, Liability, Equity, Income, Expense. Derived |
| `is_reconciliation_group` | boolean | Hidden. Derived. True for D, K, A, M categories |
| `party_required` | boolean | Hidden. Derived. True for D and K categories |

**Fields — Number Range:**

| Field | Type | Rules |
|---|---|---|
| `number_from` | string | Mandatory. Start of valid account number range |
| `number_to` | string | Mandatory. End of valid account number range |

**Constraints:**
- Number ranges cannot overlap within the same Chart of Accounts.
- `number_from` must be numerically less than `number_to`.
- Cannot delete an Account Group that has accounts under it.
- `code` unique within a Chart of Accounts.
- Derive all hidden fields in the controller `before_save` event, not in the database.

**Indexes:**
- Unique index on `chart_of_accounts + code`
- Index on `number_from, number_to` for range validation queries

---

### 3.4 DocType: Account

**Purpose:** The individual account in the Chart of Accounts. The most important DocType in the system. Every financial transaction ultimately posts to an Account. Follows SAP's account master Chart of Accounts segment — defined once, shared across all companies.

**Fields — Identity:**

| Field | Type | Rules |
|---|---|---|
| `account_number` | string | Mandatory, unique within CoA. Must fall within Account Group number range |
| `account_name` | string | Mandatory |
| `chart_of_accounts` | link → Chart of Accounts | Mandatory |
| `account_group` | link → Account Group | Mandatory. Drives all derived classification fields |

**Fields — Classification (all derived from Account Group, hidden from user):**

| Field | Type | Rules |
|---|---|---|
| `account_type_internal` | string | Hidden. Derived from account_group. SAP codes: S, G, A, D, K, M |
| `account_category` | string | Read-only display. Derived from account_group. What user sees |
| `fs_type` | string | Hidden. Derived. Values: Asset, Liability, Equity, Income, Expense |
| `is_reconciliation_account` | boolean | Hidden. Derived. When true, only designated source doctypes can post |
| `party_required` | boolean | Hidden. Derived. Every GL Entry against this account must have a party |

**Fields — Tree Structure:**

| Field | Type | Rules |
|---|---|---|
| `parent_account` | link → Account | Optional. Self-referential |
| `is_group` | boolean | Default false. Group accounts cannot receive direct GL postings |
| `lft` | integer | System managed. Left value for nested set traversal |
| `rgt` | integer | System managed. Right value for nested set traversal |

**Fields — P&L Specific:**

| Field | Type | Rules |
|---|---|---|
| `retained_earnings_account` | link → Account | Mandatory when account_type_internal is G and is_group is false. Points to the Retained Earnings balance sheet account where this account's balance transfers during year-end closing |

**Fields — Consolidation (exist day one, populated in later stages):**

| Field | Type | Rules |
|---|---|---|
| `group_account` | link → Account | Optional. Maps to equivalent account in Group CoA |
| `country_account` | link → Account | Optional. Maps to equivalent account in Country CoA |

**Fields — Tax:**

| Field | Type | Rules |
|---|---|---|
| `tax_category` | select | Optional. Options: Input Tax, Output Tax, None |

**Constraints:**
- `account_number` must fall within the `number_from` and `number_to` range of the assigned `account_group`. Validate in controller.
- Group accounts cannot have GL Entries posted against them. Enforced as database event on gl_entry.
- Reconciliation accounts can only receive postings from these source doctypes: Invoice, Payment, Stock Entry, Asset Entry, and system-generated Journal Entries. Enforce as database event.
- `retained_earnings_account` is mandatory for all non-group P&L accounts (account_type_internal = G). Enforce in controller on submit.
- Cannot change `account_type_internal` after any GL Entry has been posted. Validate in controller before_save.
- Cannot delete an account that has GL Entries.
- `group_account` must reference an account in a Group type CoA.
- `country_account` must reference an account in a Country type CoA.

**Auto-Extension Behavior:**
When a new Account is created, the system automatically creates an Account Company record for every existing Company using this Chart of Accounts. This is invisible to the user. The Account Company record inherits the Company's default currency and derives sensible defaults. Implemented as a SurrealDB event on the account table after insert.

**Indexes:**
- Unique index on `chart_of_accounts + account_number`
- Index on `parent_account` for tree traversal
- Index on `lft, rgt` for nested set queries
- Index on `account_type_internal` for reporting queries
- Index on `fs_type` for financial statement queries

---

### 3.5 DocType: Account Company

**Purpose:** The company code segment of an account. SAP terminology: extending an account to a company code. Stores company-specific behavior for each account. Entirely system-managed. No list view, no form view exposed to users.

**Fields:**

| Field | Type | Rules |
|---|---|---|
| `account` | link → Account | Mandatory |
| `company` | link → Company | Mandatory |
| `currency` | link → Currency | Mandatory. Defaults to Company's base currency |
| `is_active` | boolean | Default true. Inactive accounts block posting for this company |
| `open_item_management` | boolean | Derived. True when account_type_internal is D or K |
| `line_item_display` | boolean | Default true |
| `field_status` | select | Options: Standard, Cost Center Required, Party Required. Default: Standard |
| `local_account_code` | string | Optional. For statutory reporting in country-specific CoA context |
| `is_intercompany` | boolean | Default false. Marks account as intercompany for this company |

**Constraints:**
- Unique constraint on `account + company`.
- Cannot be deleted manually. Only deactivated via `is_active`.
- `open_item_management` must be true for all accounts where `account_type_internal` is D or K.

**Auto-Creation Rules:**
1. When a new Company is created: create Account Company records for all accounts in the assigned Chart of Accounts using company's currency and derived defaults.
2. When a new Account is created: create Account Company records for all existing companies using the same Chart of Accounts.

---

### 3.6 DocType: Financial Statement Version (FSV)

**Purpose:** Defines how accounts are grouped and presented in financial reports. Separates report presentation from account structure. Multiple FSVs can exist against the same Chart of Accounts. This is the missing piece that allows management reporting, statutory filing, and bank reporting to coexist without structural changes to the CoA.

**Fields — Header:**

| Field | Type | Rules |
|---|---|---|
| `name` | string | Mandatory, unique. Example: Pakistan Statutory 2025, Management P&L |
| `chart_of_accounts` | link → Chart of Accounts | Mandatory. Which CoA this FSV presents |
| `purpose` | select | Mandatory. Options: Statutory, Management, Consolidation, Tax, Bank Reporting |
| `description` | text | Optional |
| `is_default` | boolean | Default false. The FSV used for standard financial statement rendering |

---

### 3.7 DocType: FSV Node

**Purpose:** A node in the Financial Statement Version tree. Defines the hierarchy of line items in the rendered financial statement. This is a child structure of FSV but implemented as a separate DocType with self-referential parent for tree depth.

**Fields:**

| Field | Type | Rules |
|---|---|---|
| `fsv` | link → Financial Statement Version | Mandatory |
| `node_name` | string | Mandatory. Example: Non-Current Assets, Revenue, Gross Profit |
| `parent_node` | link → FSV Node | Optional. Self-referential. Null for root nodes |
| `sequence` | integer | Mandatory. Display order within parent |
| `is_group` | boolean | Default false. Group nodes are subtotal lines |
| `report_type` | select | Mandatory. Options: Balance Sheet, Profit and Loss |
| `sign` | select | Options: Normal, Reverse. Reverse displays credit balances as positive for liability/equity lines |
| `bold` | boolean | Default false. For formatting subtotal and total lines |

---

### 3.8 DocType: FSV Account Mapping

**Purpose:** Links actual Chart of Accounts accounts (or ranges) to FSV nodes. Determines which accounts contribute to which line in the financial statement.

**Fields:**

| Field | Type | Rules |
|---|---|---|
| `fsv_node` | link → FSV Node | Mandatory |
| `chart_of_accounts` | link → Chart of Accounts | Mandatory |
| `account_from` | string | Mandatory. Start of account number range. Example: 6100 |
| `account_to` | string | Mandatory. End of account number range. Example: 6999 |

**Notes:** A single FSV node can have multiple mapping records to accommodate non-contiguous account ranges. For example, Revenue node might map 6100-6299 and 6500-6599 while 6300-6499 maps to a different node.

---

### 3.9 DocType: Company

**Purpose:** A legal entity maintaining independent books of account. All financial transactions belong to a Company.

**Fields — Identity:**

| Field | Type | Rules |
|---|---|---|
| `name` | string | Mandatory, unique. Legal name |
| `code` | string | Mandatory, unique. Max 4 characters, uppercase. Example: PK01, AE01 |
| `currency` | link → Currency | Mandatory. Functional currency of this company |
| `country` | string | Mandatory |
| `chart_of_accounts` | link → Chart of Accounts | Mandatory. Must be Operative type |

**Fields — Group Structure:**

| Field | Type | Rules |
|---|---|---|
| `ledger_type` | select | Mandatory. Options: Operational, Consolidation, Reporting. Default: Operational |
| `parent_company` | link → Company | Optional. Parent must have `is_group = true` |
| `is_group` | boolean | Default false. Group companies consolidate subsidiary books |

**Fields — Fiscal Year:**

| Field | Type | Rules |
|---|---|---|
| `fiscal_year_start_month` | integer | Mandatory. 1 through 12. Pakistan: 7 (July). Calendar year: 1 |

**Fields — Default Accounts:**

| Field | Type | Rules |
|---|---|---|
| `retained_earnings_account` | link → Account | Mandatory. For year-end closing entries |
| `default_receivable_account` | link → Account | Mandatory. Default AR control account. Must be Type D |
| `default_payable_account` | link → Account | Mandatory. Default AP control account. Must be Type K |
| `round_off_account` | link → Account | Optional. For rounding differences |
| `intercompany_receivable_account` | link → Account | Optional. Activated in Stage 5 |
| `intercompany_payable_account` | link → Account | Optional. Activated in Stage 5 |

**Constraints:**
- `code` must be uppercase, no spaces, max 4 characters.
- `parent_company` cannot reference self.
- A Consolidation company must have a `parent_company`.
- Cannot change `chart_of_accounts` after any GL Entry has been posted.
- Cannot change `currency` after any GL Entry has been posted.
- `default_receivable_account` must have `account_type_internal = D`.
- `default_payable_account` must have `account_type_internal = K`.

**On Create Behavior:**
1. Create Account Company records for all accounts in assigned Chart of Accounts.
2. Create first Fiscal Year based on `fiscal_year_start_month`.
3. Create 12 Accounting Period records for that Fiscal Year.

---

### 3.10 DocType: Fiscal Year

**Purpose:** Defines a financial year for a company. All GL Entries belong to a Fiscal Year.

**Fields:**

| Field | Type | Rules |
|---|---|---|
| `name` | string | Unique per company. Example: FY2024-25 |
| `company` | link → Company | Mandatory |
| `start_date` | date | Mandatory |
| `end_date` | date | Mandatory |
| `is_closed` | boolean | Default false. Closed years block all posting |
| `closed_on` | datetime | System set |
| `closed_by` | link → User | System set |

**Constraints:**
- Fiscal year periods cannot overlap for the same company.
- `end_date` must be after `start_date`.
- Maximum 366 days between start and end.
- Cannot reopen a closed fiscal year without superuser action.
- Cannot delete a fiscal year that has GL Entries.

---

### 3.11 DocType: Accounting Period

**Purpose:** A subdivision of a Fiscal Year, typically monthly. Period locking prevents backdated entries after month-end close. All GL Entries belong to an Accounting Period.

**Fields:**

| Field | Type | Rules |
|---|---|---|
| `name` | string | Example: Jul 2024, Aug 2024 |
| `company` | link → Company | Mandatory |
| `fiscal_year` | link → Fiscal Year | Mandatory |
| `start_date` | date | Mandatory |
| `end_date` | date | Mandatory |
| `period_number` | integer | 1 through 12 |
| `is_closed` | boolean | Default false |
| `closed_on` | datetime | System set |
| `closed_by` | link → User | System set |

**Constraints:**
- Periods cannot overlap within the same company and fiscal year.
- A period can only be closed after all prior periods in the fiscal year are closed (sequential closing).
- Cannot post to a closed period under any circumstance. Enforced as database event on gl_entry, not at application level.

---

### 3.12 DocType: Cost Center

**Purpose:** Dimension for branch-wise, department-wise, and project-wise reporting. This is how branches are handled — not as separate companies but as cost center dimensions on GL Entries.

**Fields:**

| Field | Type | Rules |
|---|---|---|
| `name` | string | Mandatory, unique within company |
| `code` | string | Mandatory. Short identifier. Example: CC-LHR, CC-KHI |
| `company` | link → Company | Mandatory |
| `parent_cost_center` | link → Cost Center | Optional. Self-referential for hierarchy |
| `is_group` | boolean | Default false |
| `lft` | integer | System managed |
| `rgt` | integer | System managed |
| `is_active` | boolean | Default true |

---

## Section 4 — Business Partner Module

### 4.1 DocType: Party

**Purpose:** The Business Partner master. Replaces ERPNext's separate Customer and Supplier doctypes. Any entity your business has a financial relationship with is a Party. A Party can have multiple roles simultaneously. Follows SAP S/4HANA's Business Partner concept.

**Design Decision Rationale:** ERPNext uses separate Customer and Supplier doctypes. This creates duplication when a business entity is both customer and supplier — extremely common in Pakistani SME context where traders, distributors, and manufacturers frequently buy from and sell to each other. SpotLedger uses a single Party master with roles. This is the correct model in SAP S/4HANA, Oracle Financials, and Dynamics 365.

**Fields — Identity:**

| Field | Type | Rules |
|---|---|---|
| `party_name` | string | Mandatory, unique. Legal name |
| `short_name` | string | Optional. Trade name or display name |
| `party_type` | select | Mandatory. Options: Individual, Company |
| `roles` | set | At least one mandatory. Values: Customer, Supplier, Employee, Shareholder, Director, Franchisee, Bank, Tax Authority, Government Office |
| `is_active` | boolean | Default true |

**Fields — Tax Identity:**

| Field | Type | Rules |
|---|---|---|
| `tax_id` | string | Optional but strongly recommended. NTN for companies, CNIC for individuals |
| `tax_id_type` | select | Options: NTN, CNIC, VAT Number, Tax Registration Number, Other |
| `is_tax_registered` | boolean | Default false |
| `sales_tax_number` | string | Optional. STRN in Pakistan context |

**Fields — Contact:**

| Field | Type | Rules |
|---|---|---|
| `primary_email` | string | Optional |
| `primary_phone` | string | Optional |
| `website` | string | Optional |
| `currency` | link → Currency | Mandatory. Default transaction currency |
| `country` | string | Mandatory |

**Fields — Billing Address:**

| Field | Type | Rules |
|---|---|---|
| `billing_address_line1` | string | Optional |
| `billing_address_line2` | string | Optional |
| `billing_city` | string | Optional |
| `billing_state` | string | Optional |
| `billing_postal_code` | string | Optional |
| `billing_country` | string | Optional |

**Constraints:**
- `tax_id` must be unique across all party records when provided.
- At least one role must be assigned.
- Cannot delete a party that has GL Entries.
- Cannot remove a role if GL Entries exist for that party in that role.

**All Supported Roles and Their Financial Behavior:**

| Role | GL Behavior | Subledger | Notes |
|---|---|---|---|
| Customer | AR postings | Type D account | Invoices raised against them |
| Supplier | AP postings | Type K account | Invoices received from them |
| Employee | Salary and expense | Type K account | Payroll and expense claims |
| Shareholder | Dividend and capital | Equity accounts | Capital transactions |
| Director | Loans and remuneration | Type K account | Director current accounts |
| Franchisee | Royalty receivable | Type D account | Management reporting only |
| Bank | Loan transactions | Loan accounts | Bank as a lender |
| Tax Authority | Tax payments | Tax liability accounts | FBR, SRB, PRA etc |
| Government Office | Fee and levy payments | Expense accounts | SECP, municipality etc |

---

### 4.2 DocType: Party Role Config

**Purpose:** Role-specific financial settings for a party. One record per party per role per company. Created automatically when a role is added to a Party. Contains AR/AP account linkages, credit terms, and limits specific to how this party behaves in each role. This is a child table within the Party form — not a standalone DocType visible in navigation.

**Fields:**

| Field | Type | Rules |
|---|---|---|
| `party` | link → Party | Mandatory. Parent |
| `role` | select | Mandatory. Same options as Party roles |
| `company` | link → Company | Mandatory. Config is per company |
| `receivable_account` | link → Account | Conditional mandatory when role is Customer or Franchisee. Must be Type D |
| `payable_account` | link → Account | Conditional mandatory when role is Supplier, Employee, or Director. Must be Type K |
| `payment_terms` | link → Payment Terms | Optional. Default payment terms |
| `credit_limit` | decimal | Optional. Maximum outstanding for Customer role |
| `credit_days` | integer | Optional. Days before invoice is overdue |
| `tax_category` | string | Optional. Determines applicable tax rules |
| `withholding_tax_applicable` | boolean | Default false |
| `withholding_tax_rate` | decimal | Conditional mandatory when withholding_tax_applicable is true |

**Constraints:**
- Unique constraint on `party + role + company`.
- `receivable_account` must have `account_type_internal = D`.
- `payable_account` must have `account_type_internal = K`.
- Defaults to Company's `default_receivable_account` and `default_payable_account` when not specified.

---

### 4.3 DocType: Payment Terms

**Purpose:** Standard payment terms applied to invoices and party role configs.

**Fields:**

| Field | Type | Rules |
|---|---|---|
| `name` | string | Mandatory, unique. Example: Net 30, Net 45, Advance |
| `payment_days` | integer | Mandatory. Number of days from invoice date until due |
| `discount_days` | integer | Optional. Days within which early payment discount applies |
| `discount_percent` | decimal | Optional. Early payment discount percentage |
| `description` | text | Optional |

---

## Section 5 — GL Entry Module

### 5.1 DocType: GL Entry

**Purpose:** The atomic unit of accounting. Every financial transaction produces GL Entries. GL Entry is never created manually by users. Always created by a controller function of a parent transaction DocType. Never deleted. Never updated after creation. Reversals create new entries with opposite signs.

**This DocType has no New button in the UI. It is read-only for all users.**

**Fields — Core:**

| Field | Type | Rules |
|---|---|---|
| `company` | link → Company | Mandatory |
| `account` | link → Account | Mandatory. Must be non-group, must be active in Account Company for this company |
| `posting_date` | date | Mandatory. Must fall within an open Accounting Period |
| `fiscal_year` | link → Fiscal Year | Mandatory. Derived automatically from posting_date and company |
| `accounting_period` | link → Accounting Period | Mandatory. Derived automatically |

**Fields — Amounts:**

| Field | Type | Rules |
|---|---|---|
| `debit` | decimal | Default 0. Amount in transaction currency |
| `credit` | decimal | Default 0. Amount in transaction currency |
| `debit_base` | decimal | Default 0. debit × exchange_rate. Amount in company base currency |
| `credit_base` | decimal | Default 0. credit × exchange_rate. Amount in company base currency |
| `currency` | link → Currency | Mandatory. Transaction currency |
| `exchange_rate` | decimal | Default 1 |

**Fields — Party:**

| Field | Type | Rules |
|---|---|---|
| `party` | link → Party | Conditional mandatory. Required when account.party_required is true |
| `party_role` | select | Conditional mandatory. Required when party is set. Options: Customer, Supplier, Employee, Shareholder, Director, Franchisee, Bank, Tax Authority, Government Office |

**Fields — Source Document:**

| Field | Type | Rules |
|---|---|---|
| `source_doctype` | string | Mandatory. The DocType that created this entry |
| `source_name` | string | Mandatory. The name/ID of the source document |
| `voucher_number` | string | Mandatory. Human-readable reference from source document |

**Fields — Classification:**

| Field | Type | Rules |
|---|---|---|
| `cost_center` | link → Cost Center | Conditional. Mandatory if account field_status is Cost Center Required |
| `project` | link → Project | Optional |
| `remarks` | text | Optional |

**Fields — Intercompany (schema exists day one, logic activated in Stage 5):**

| Field | Type | Rules |
|---|---|---|
| `trading_partner` | link → Company | Optional. Mandatory when account is_intercompany is true for this company |
| `intercompany_reference` | string | Optional. Links to mirror GL Entry in trading partner's books |

**Fields — Reversal:**

| Field | Type | Rules |
|---|---|---|
| `is_reversal` | boolean | Default false |
| `reversal_of` | link → GL Entry | Optional. Points to original entry being reversed |

**Fields — Audit:**

| Field | Type | Rules |
|---|---|---|
| `is_system_generated` | boolean | Default true |
| `created_by` | link → User | System set |
| `creation` | datetime | System set |

**Database Events (enforced at database level, not application level):**

**Event 1 — Block closed period posting:**
When a new GL Entry is created, if the accounting_period linked to the posting_date has `is_closed = true`, throw: "Accounting period is closed. Cannot post."

**Event 2 — Enforce party on reconciliation accounts:**
When a new GL Entry is created, if the account's `party_required` is true and `party` is null, throw: "Party is mandatory for this account."

**Event 3 — Enforce trading partner on intercompany accounts:**
When a new GL Entry is created, if the account's `is_intercompany` is true for this company and `trading_partner` is null, throw: "Trading partner is mandatory for intercompany accounts."

**Event 4 — Block direct posting to reconciliation accounts:**
When a new GL Entry is created, if the account's `is_reconciliation_account` is true and `source_doctype` is not in the list (Invoice, Payment, Stock Entry, Asset Entry, Journal Entry where is_system_generated is true), throw: "Cannot post directly to reconciliation account."

**Event 5 — Block posting to group accounts:**
When a new GL Entry is created, if the account's `is_group` is true, throw: "Cannot post to a group account."

**Event 6 — Block posting to inactive accounts:**
When a new GL Entry is created, if the Account Company record for this account and company has `is_active = false`, throw: "Account is inactive for this company."

**Event 7 — Block all updates and deletes:**
GL Entry table has no UPDATE or DELETE permissions for any role including Administrator. This is enforced as a table-level permission, not an event.

**Indexes:**
- Index on `company + posting_date`
- Index on `account + posting_date`
- Index on `party + party_role + posting_date`
- Index on `source_doctype + source_name`
- Index on `accounting_period`
- Index on `trading_partner`
- Index on `cost_center + posting_date`
- Index on `fiscal_year`

---

### 5.2 SurrealQL Function: fn::post_gl_entries

**Purpose:** The single entry point for all GL posting. Every DocType controller that posts to the GL calls this function. Validates, derives fiscal year and period, creates entries atomically, verifies books balance before committing.

**Parameters:**
- `source_doctype` — string
- `source_name` — string
- `voucher_number` — string
- `company` — record reference to Company
- `posting_date` — date
- `entries` — array of objects each containing: account, debit, credit, currency, exchange_rate, party, party_role, cost_center, trading_partner, remarks

**Logic Steps (all within a single transaction, in order):**

1. Find the open Accounting Period for the given company and posting_date. If none found, throw: "No open accounting period found for this posting date."
2. Derive the Fiscal Year from the Accounting Period.
3. For each entry in the entries array, create a GL Entry record with all fields populated. Calculate `debit_base = debit × exchange_rate` and `credit_base = credit × exchange_rate`.
4. After all entries are created, sum `debit_base` and `credit_base` for this `source_doctype` and `source_name` combination.
5. If `total_debit_base - total_credit_base ≠ 0`, throw: "GL entries do not balance. Difference: [amount]."
6. Commit transaction.

---

### 5.3 SurrealQL Function: fn::reverse_gl_entries

**Purpose:** Creates reversal GL Entries for a given source document. Called when a submitted document is cancelled. Never deletes original entries.

**Parameters:**
- `source_doctype` — string
- `source_name` — string
- `reversal_date` — date
- `company` — record reference

**Logic Steps:**

1. Find all GL Entries where `source_doctype` and `source_name` match.
2. For each found entry create a new GL Entry with debit and credit swapped, `is_reversal = true`, `reversal_of` pointing to original, `posting_date` set to reversal_date, `source_doctype` suffixed with `-Cancel`.
3. Validate the reversal entries balance (they always will if originals balanced, but validate anyway).
4. Commit atomically.

---

### 5.4 DocType: Journal Entry

**Purpose:** The only DocType that allows manual GL posting by users. Used for opening balances, adjustments, depreciation, accruals, corrections, and foreign exchange revaluation.

**Fields — Header:**

| Field | Type | Rules |
|---|---|---|
| `name` | string | Auto-generated. Series: JV-YYYY-NNNNN |
| `company` | link → Company | Mandatory |
| `posting_date` | date | Mandatory. Defaults to today |
| `entry_type` | select | Mandatory. Options: Journal Entry, Opening Entry, Depreciation Entry, Foreign Exchange Revaluation, Period Closing Entry, Reversal Entry. Default: Journal Entry |
| `title` | string | Optional |
| `remarks` | text | Optional |
| `docstatus` | integer | 0 = Draft, 1 = Submitted, 2 = Cancelled |
| `is_system_generated` | boolean | Default false. System entries can post to reconciliation accounts |
| `reversal_of` | link → Journal Entry | Optional. For Reversal Entry type |
| `reversal_date` | date | Optional |

**Child DocType: Journal Entry Line**

| Field | Type | Rules |
|---|---|---|
| `account` | link → Account | Mandatory |
| `debit` | decimal | Default 0 |
| `credit` | decimal | Default 0 |
| `currency` | link → Currency | Mandatory. Defaults to company currency |
| `exchange_rate` | decimal | Default 1 |
| `party` | link → Party | Conditional. Required when account.party_required is true |
| `party_role` | select | Conditional. Required when party is set |
| `cost_center` | link → Cost Center | Optional |
| `trading_partner` | link → Company | Optional |
| `remarks` | string | Optional. Line-level narrative |

**Controller Logic:**

On Validate:
1. Total debit in base currency must equal total credit in base currency. Show validation error if not balanced. Block submission.
2. Each line's account must be active for the selected company.
3. Party must be provided where account requires it.

On Submit:
1. Call `fn::post_gl_entries` with all lines.
2. Set `docstatus = 1`.

On Cancel:
1. Call `fn::reverse_gl_entries`.
2. Set `docstatus = 2`.
3. If `entry_type` is not Reversal Entry, automatically create a linked Reversal Entry Journal.

**Constraints:**
- Cannot amend a cancelled Journal Entry. Create new.
- Opening entries only allowed before any other transaction in that fiscal year.
- System-generated entries (`is_system_generated = true`) cannot be cancelled manually.

---

## Section 6 — ERPNext Compatibility

### What to Import Directly from ERPNext

| Component | Import Approach |
|---|---|
| GL Entry cancellation pattern | Direct port. docstatus 0/1/2 model retained |
| Account balance queries | Direct port. Translate field names per mapping table below |
| Trial Balance report logic | Direct port |
| Party outstanding calculation | Direct port with party_role filter added |
| Fiscal year and period validation | Direct port |
| Exchange rate handling | Direct port |
| ERPNext GL Entry test suite | Port all tests. This is the primary value |
| Edge case handling (backdating, frozen accounts, currency revaluation) | Port via test suite |

### Field Name Translation Map

| ERPNext Field | SpotLedger Field | Notes |
|---|---|---|
| `against` | `source_doc` | Renamed |
| `against_voucher` | `source_name` | Renamed |
| `against_voucher_type` | `source_doctype` | Renamed |
| `party_type` | `party_role` | Renamed. String value same |
| `party` | `party` | Now record reference, not string |
| `debit_in_account_currency` | `debit` | Renamed |
| `credit_in_account_currency` | `credit` | Renamed |
| `debit` (base amount) | `debit_base` | Renamed |
| `credit` (base amount) | `credit_base` | Renamed |
| `voucher_type` | `source_doctype` | Renamed |
| `voucher_no` | `source_name` | Renamed |

### What NOT to Import from ERPNext

| ERPNext Component | Reason Not Imported |
|---|---|
| Customer DocType | Replaced by Party with role Customer |
| Supplier DocType | Replaced by Party with role Supplier |
| Separate account types system | Replaced by account_category + account_type_internal |
| Payment Entry DocType | Collapsed into polymorphic Payment DocType |
| Payment Order DocType | Collapsed into Payment DocType |
| Payment Request DocType | Collapsed into Payment DocType |
| Bank Transaction as separate DocType | Part of Payment DocType |
| Any DocType that is a workflow stage of another DocType | Always collapse into one DocType with state machine |

---

## Section 7 — Implementation Stages

### Stage 1 — Foundation (Implement First)

Implement in this exact order. Everything else depends on this stage being correct and complete.

1. Currency DocType
2. Chart of Accounts DocType
3. Account Group DocType with full derivation logic for all hidden fields
4. Account DocType with all constraints and auto-extension event
5. Company DocType with auto-creation of Account Company records, auto Fiscal Year, auto Periods
6. Account Company DocType — system managed, no UI
7. Fiscal Year DocType with auto-period creation on insert
8. Accounting Period DocType
9. Cost Center DocType
10. GL Entry DocType with all six database events
11. `fn::post_gl_entries` SurrealQL function
12. `fn::reverse_gl_entries` SurrealQL function
13. Journal Entry DocType with Journal Entry Line child table

**Stage 1 Deliverable:** System can maintain a chart of accounts, create companies, manage fiscal periods, enforce period locking, post manual journal entries, enforce GL balance invariant, and produce a Trial Balance query.

### Stage 2 — Financial Statement Reporting

1. Financial Statement Version DocType
2. FSV Node DocType
3. FSV Account Mapping DocType
4. Balance Sheet rendering function using FSV
5. Profit and Loss rendering function using FSV
6. Create default FSVs: Pakistan Statutory and Management Reporting
7. Trial Balance report

**Stage 2 Deliverable:** System can produce proper Balance Sheet, P&L, and Trial Balance against any FSV configuration.

### Stage 3 — Business Partner

1. Payment Terms DocType
2. Party DocType
3. Party Role Config child table
4. Update GL Entry party field to properly reference Party
5. AR outstanding query function (party + role Customer)
6. AP outstanding query function (party + role Supplier)
7. Net position query (same party in both roles)

**Stage 3 Deliverable:** System can maintain a business partner master and track party-wise outstanding balances with full role separation.

### Stage 4 — Transaction Doctypes

1. Invoice DocType — polymorphic, `invoice_type: Sales | Purchase`
2. Invoice Line child table
3. Payment DocType — polymorphic, `payment_type: Receive | Pay | Internal`
4. Payment Reconciliation (matching invoices to payments, open item management)
5. Tax Rule DocType
6. Tax calculation engine

**Stage 4 Deliverable:** Complete AR/AP module. Full invoice-to-payment cycle with tax.

### Stage 5 — Inventory (if applicable)

1. Item DocType
2. Warehouse DocType
3. Stock Entry DocType with `entry_type: Receipt | Issue | Transfer | Adjustment`
4. Stock Ledger Entry — child, system managed
5. Perpetual inventory GL posting (COGS debit, Inventory credit on issue)
6. AVCO/FIFO valuation (port from ERPNext stock_ledger.py)

**Stage 5 Deliverable:** Perpetual inventory with automatic GL posting.

### Stage 6 — Multi-Company Intercompany

1. Activate intercompany account flags on Account Company
2. Trading partner enforcement events go live
3. Auto-mirror entry creation when intercompany account is posted to
4. Intercompany reconciliation report
5. Group CoA setup and operative-to-group account mapping

**Stage 6 Deliverable:** Automated intercompany posting with mirror entries and reconciliation.

### Stage 7 — Legal Consolidation

1. Consolidation company ledger type activation
2. Currency translation entries
3. Intercompany elimination journal generation
4. Minority interest calculation
5. Consolidated financial statements using Group CoA FSV

**Stage 7 Deliverable:** Full statutory consolidated financial statements.

### Stage 8 — Franchise Management Reporting

1. Franchisee role activation on Party
2. Royalty invoice template
3. Performance data import for franchisee reporting
4. Management consolidation report (not statutory consolidation)
5. Franchisee benchmarking dashboard

**Stage 8 Deliverable:** Complete franchise network management reporting without statutory consolidation overhead.

---

## Section 8 — Critical Invariants

These must never be violated under any circumstance. Each is enforced at database level, not application level.

**Invariant 1 — Books Always Balance:**
Sum of `debit_base` equals sum of `credit_base` for any given `source_doctype` and `source_name` combination. Enforced inside `fn::post_gl_entries` as the final check before commit. If violated, the entire transaction rolls back.

**Invariant 2 — No Closed Period Posting:**
No GL Entry has a `posting_date` that falls in a closed Accounting Period. Enforced as database event on gl_entry table on insert.

**Invariant 3 — Reconciliation Account Integrity:**
No GL Entry posts directly to a reconciliation account except through designated source doctypes. Enforced as database event on gl_entry table on insert.

**Invariant 4 — No Group Account Posting:**
No GL Entry posts to a group account (is_group = true). Enforced as database event on gl_entry table on insert.

**Invariant 5 — GL Entry Immutability:**
No GL Entry is ever updated or deleted after creation. Corrections are always reversals. Enforced by removing UPDATE and DELETE permissions on gl_entry table for all roles including Administrator. This is a table-level permission setting, not a soft constraint.

**Invariant 6 — Party Completeness:**
Every GL Entry against a party_required account has both `party` and `party_role` populated. No orphaned or partial party references. Enforced as database event on gl_entry table on insert.

**Invariant 7 — Intercompany Completeness:**
Every GL Entry against an intercompany account has `trading_partner` populated. Enforced as database event on gl_entry table on insert. Logic activated in Stage 6 when intercompany accounts are designated.

---

## Section 9 — Design Decisions Reference

This section records key design decisions for future reference and justification.

| Decision | Choice Made | Alternative Considered | Reason |
|---|---|---|---|
| Party model | Single Party with roles | Separate Customer/Supplier | Same entity often buys and sells. Duplication causes reconciliation problems. SAP S/4HANA made same choice |
| Account classification | Two fields: account_category (user) + account_type_internal (system) | Single user-visible field | Users need simple labels. System needs SAP precision. LLMs benefit from self-documenting category names |
| CoA scope | Single master Operative CoA shared across companies | Per-company CoA | Branch/subsidiary reality in Pakistani SME market. SAP, Oracle, D365 all use shared master CoA |
| Company-account relationship | account_company override table, system-managed | Embed company settings in account | Allows per-company currency and behavior without polluting the master account |
| Branch handling | Cost Centers not Companies | Separate company per branch | Branches are the same legal entity. Cost Centers are the universally correct solution |
| Franchise handling | Party with Franchisee role not Company | Separate company per franchisee | Franchisees are legally independent. No ownership relationship. Management reporting not statutory consolidation |
| Account number format | 4-digit codes | SAP 10-digit codes | SAP codes are too long for SME context. 4-digit retains tree structure clarity and fits Pakistani SME scale |
| FSV | Separate DocType layer | Embed presentation in CoA | Same data must render in statutory, management, and bank formats without data changes. Separation is mandatory |
| GL Entry creation | SurrealQL function fn::post_gl_entries | Application-layer posting | Atomic balance enforcement. Zero database round trips. Accounting brain in database not application |
| GL Entry deletion | Never allowed | Soft delete | Audit integrity. ERPNext and every serious ERP takes same position |

---

*End of SpotLedger Finance Module Implementation Plan v1.0*
*This document covers Stages 1 through 3 in full specification detail.*
*Stages 4 through 8 are specified at design level pending Stage 3 completion.*
