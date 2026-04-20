# SpotLedger — Payment & Receipt Module Specification
## Design Reference Document

---

## Design Philosophy

Payments and Receipts are the most frequently used screens in any accounting system for an SME. The design goal is that a data entry person with no accounting knowledge can record any real-world money movement in under 30 seconds without ever touching a Journal Entry.

Three principles drive every decision in this module:

**Principle 1 — No free text payees.** Every party receiving or sending money must be registered in the system, even minimally. Free text breaks reporting, tax compliance, and analytics.

**Principle 2 — No invoice required.** Payments and receipts exist independently of invoices. Linking to an invoice is optional, not mandatory.

**Principle 3 — One DocType, two directions.** Payment and Receipt are the same financial event from opposite perspectives. One DocType with direction handles both.

---

## Real-World Scenarios Covered

### Outgoing Payments

| Scenario | Type | Invoice Needed |
|---|---|---|
| Paid electricity bill at bank | Expense Payment | No |
| Paid rent in cash | Expense Payment | No |
| Paid petrol, got receipt | Expense Payment | No |
| Paid supplier against invoice | Party Payment | Optional |
| Paid supplier advance | Party Payment | No |
| Paid casual daily-wage worker | Expense Payment | No |
| Transferred HBL to MCB | Internal Transfer | No |
| Topped up petty cash from bank | Internal Transfer | No |

### Incoming Receipts

| Scenario | Type | Invoice Needed |
|---|---|---|
| Received rental income | Income Receipt | No |
| Received commission | Income Receipt | No |
| Received from customer against invoice | Party Receipt | Optional |
| Received customer advance | Party Receipt | No |
| Received cash sales | Income Receipt | No |
| Transferred MCB to HBL | Internal Transfer | No |

---

## Party Registration — Two Tiers

Free text payees are not allowed. However forcing full party setup before every payment creates friction that kills adoption. The solution is two tiers of party registration.

### Tier 1 — Full Party (Business Partner)

Complete record with full subledger, credit terms, tax details, payment terms. Used for regular suppliers, customers, employees, and anyone transacted with repeatedly.

Created via the Business Partner module before transacting. Takes 2-3 minutes to set up properly.

### Tier 2 — Casual Party

Minimal record created inline from the payment screen in under 10 seconds. Three mandatory fields only. Posts to a shared One-Time account rather than an individual subledger account. Can be promoted to Full Party at any time.

Used for: one-time vendors, casual labour, misc payees, one-time buyers.

---

## DocType: Casual Party

**Purpose:** Lightweight party record for one-time or infrequent payees and receivers. Created inline from the Payment screen without navigating away.

### Fields

| Field | Type | Rules |
|---|---|---|
| `name` | string | Mandatory. Legal or trade name |
| `party_type` | select | Mandatory. Options: Individual, Company |
| `phone` | string | Optional |
| `address` | string | Optional. Single line, unstructured |
| `default_account` | link → Account | System set. One-Time Payable for outgoing, One-Time Receivable for incoming. Derived from first transaction direction |
| `can_promote` | boolean | Default true |
| `promoted_to` | link → Party | Set when promoted to Full Party |
| `is_promoted` | boolean | Default false |

### Promotion to Full Party

A Promote button appears on the Casual Party form. When clicked:
1. Opens a pre-filled Party form with available data copied across.
2. User completes missing fields and saves.
3. System sets `promoted_to` and `is_promoted = true` on Casual Party.
4. All historical GL Entries referencing the Casual Party are relinked to the new Full Party record.
5. Casual Party is deactivated.

### One-Time Accounts

Two dedicated accounts must exist in the Chart of Accounts for Casual Party transactions:

- `One-Time Payable` — Type K, reconciliation account, for outgoing casual payments
- `One-Time Receivable` — Type D, reconciliation account, for incoming casual receipts

These aggregate all casual party transactions. Individual transactions remain identifiable by the casual party reference on each GL Entry. When a Casual Party is promoted, their balance moves from the One-Time account to their new proper subledger account.

---

## DocType: Payment

**Purpose:** Records all money movements — outgoing payments, incoming receipts, and internal transfers. One DocType with direction and type fields covering every real-world scenario. Never requires a Journal Entry from the user.

### Header Fields (always visible)

| Field | Type | Rules |
|---|---|---|
| `name` | string | Auto-generated series. PAY-YYYY-NNNNN for outgoing, REC-YYYY-NNNNN for incoming, TRF-YYYY-NNNNN for internal |
| `company` | link → Company | Mandatory |
| `payment_direction` | select | Mandatory. Options: Outgoing, Incoming, Internal. Drives which fields appear |
| `payment_type` | select | Mandatory. Options filtered by direction — see matrix below |
| `date` | date | Mandatory. Defaults to today |
| `docstatus` | integer | 0 = Draft, 1 = Submitted, 2 = Cancelled |

### Payment Type Matrix

| payment_direction | payment_type options |
|---|---|
| Outgoing | Expense Payment, Party Payment |
| Incoming | Income Receipt, Party Receipt |
| Internal | Internal Transfer |

### Conditional Section: Expense Payment and Income Receipt

Shown when `payment_type` is Expense Payment or Income Receipt.

| Field | Type | Rules |
|---|---|---|
| `party` | link → Party or Casual Party | Mandatory. Inline quick-add available for Casual Party creation |
| `expense_income_lines` | child table | Mandatory. At least one line required |

**Child Table: Payment Line**

| Field | Type | Rules |
|---|---|---|
| `account` | link → Account | Mandatory. Expense account for Expense Payment, Income account for Income Receipt. Must match fs_type |
| `amount` | decimal | Mandatory |
| `cost_center` | link → Cost Center | Optional |
| `description` | string | Optional. Line-level note |

### Conditional Section: Party Payment and Party Receipt

Shown when `payment_type` is Party Payment or Party Receipt.

| Field | Type | Rules |
|---|---|---|
| `party` | link → Party | Mandatory. Full Party only. Casual Party not allowed for subledger transactions |
| `party_role` | select | Mandatory. Customer or Supplier |
| `amount` | decimal | Mandatory |
| `against_invoices` | child table | Optional. Leave empty for advance payments |

**Child Table: Invoice Allocation**

| Field | Type | Rules |
|---|---|---|
| `invoice` | link → Invoice | Mandatory per line |
| `allocated_amount` | decimal | Mandatory. Cannot exceed invoice outstanding |

**Advance Logic:**
When `against_invoices` is empty the system automatically posts to the advance account defined on Party Role Config rather than the normal trade receivable or payable account. No user action required. This is invisible to the user — they simply leave the invoice table empty.

### Conditional Section: Internal Transfer

Shown when `payment_type` is Internal Transfer.

| Field | Type | Rules |
|---|---|---|
| `from_account` | link → Account | Mandatory. Must be Cash and Bank category |
| `to_account` | link → Account | Mandatory. Must be Cash and Bank category. Cannot equal from_account |
| `amount` | decimal | Mandatory |

### Payment Details Section (always visible)

| Field | Type | Rules |
|---|---|---|
| `payment_mode` | select | Mandatory. Options: Cash, Bank Transfer, Cheque, Online Transfer, Mobile Wallet |
| `paid_from` | link → Account | Mandatory for Outgoing and Internal. Must be Cash and Bank category |
| `paid_into` | link → Account | Mandatory for Incoming and Internal. Must be Cash and Bank category |
| `reference_number` | string | Optional. Cheque number, transaction ID, receipt number |
| `narration` | string | Optional but strongly encouraged. What was this payment for |
| `currency` | link → Currency | Mandatory. Defaults to company currency |
| `exchange_rate` | decimal | Default 1. Shown only when currency differs from company currency |

### System Computed Fields

| Field | Type | Notes |
|---|---|---|
| `total_amount` | decimal | Computed from lines or amount field |
| `total_amount_base` | decimal | total_amount × exchange_rate |

---

## GL Posting Logic

The controller calls `fn::post_gl_entries` on submit. The entries generated depend on payment_type.

### Expense Payment

```
Example: Electricity bill PKR 45,000 paid from HBL

Dr  Electricity Expense     45,000    (from expense_income_lines)
Cr  HBL Current Account     45,000    (paid_from account)
```

Multiple lines are supported:

```
Example: Mixed utility payment PKR 80,000

Dr  Electricity Expense     45,000
Dr  Gas Expense             35,000
Cr  HBL Current Account     80,000
```

### Income Receipt

```
Example: Rental income PKR 80,000 received into HBL

Dr  HBL Current Account     80,000    (paid_into account)
Cr  Rental Income            80,000    (from expense_income_lines)
```

### Party Payment — Against Invoice

```
Example: Paid supplier Rehman Traders PKR 200,000 against invoice

Dr  Trade Payable            200,000   (party: Rehman Traders, role: Supplier)
Cr  HBL Current Account      200,000
```

### Party Payment — Advance (no invoice linked)

```
Example: Paid supplier advance PKR 100,000

Dr  Supplier Advance         100,000   (advance_account from Party Role Config)
Cr  HBL Current Account      100,000
```

When invoice arrives later and advance is matched:

```
Dr  Trade Payable            100,000
Cr  Supplier Advance         100,000
```

### Party Receipt — Against Invoice

```
Example: Received from customer Ahmed Traders PKR 500,000

Dr  HBL Current Account      500,000
Cr  Trade Receivable          500,000   (party: Ahmed Traders, role: Customer)
```

### Party Receipt — Advance

```
Example: Received customer advance PKR 300,000

Dr  HBL Current Account      300,000
Cr  Customer Advance          300,000   (advance_account from Party Role Config)
```

### Internal Transfer

```
Example: Transfer PKR 50,000 from HBL to Petty Cash

Dr  Petty Cash               50,000
Cr  HBL Current Account      50,000
```

### Expense Payment — Casual Party

```
Example: Paid casual electrician PKR 2,000 cash

Dr  Repair and Maintenance    2,000
Cr  Cash in Hand              2,000

Party on GL Entry: CasualParty:electrician-oct-2025
Account used: One-Time Payable (reconciliation)
Note: The expense account is posted directly.
      One-Time Payable tracks the party obligation separately
      only if payment was on credit. For immediate cash payment
      no subledger entry is needed.
```

---

## Screen Design — Progressive Disclosure

The payment screen reveals fields progressively based on user selections. The goal is that at any point the user only sees what is relevant to their specific payment.

### Step 1 — Direction and Type Selection

The first thing the user sees is a clean card-based selector:

```
┌─────────────────────────────────────────────────────┐
│  New Payment                                         │
│                                                      │
│  ┌─────────────────┐  ┌─────────────────┐           │
│  │  💸 Outgoing    │  │  💰 Incoming    │           │
│  │     Payment     │  │     Receipt     │           │
│  └─────────────────┘  └─────────────────┘           │
│           ┌─────────────────┐                        │
│           │  🔄 Internal    │                        │
│           │    Transfer     │                        │
│           └─────────────────┘                        │
└─────────────────────────────────────────────────────┘
```

After selecting Outgoing:

```
┌─────────────────────────────────────────────────────┐
│  Outgoing Payment                                    │
│                                                      │
│  ┌──────────────────────┐  ┌──────────────────────┐ │
│  │  📋 Expense Payment  │  │  🏢 Supplier Payment │ │
│  │  Bills, utilities,   │  │  Against invoice or  │ │
│  │  rent, petrol        │  │  advance             │ │
│  └──────────────────────┘  └──────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

### Step 2 — Expense Payment Form

```
┌─────────────────────────────────────────────────────┐
│  Expense Payment                          [← Back]  │
│                                                      │
│  Date          [ 19 Oct 2025          ]              │
│                                                      │
│  Paid To       [ Search parties...    ] [+ Quick Add]│
│                                                      │
│  Paid Via      ○ Cash  ● Bank  ○ Cheque  ○ Online   │
│  From Account  [ HBL Current Account  ▼ ]           │
│                                                      │
│  What did you pay for?                               │
│  ┌───────────────────────────────────┬──────────┐   │
│  │ Electricity Expense           ▼   │  45,000  │   │
│  ├───────────────────────────────────┼──────────┤   │
│  │ + Add another line                │          │   │
│  └───────────────────────────────────┴──────────┘   │
│                                    Total:  45,000    │
│                                                      │
│  Reference     [ LESCO Oct 2025              ]       │
│  Note          [ Monthly electricity bill    ]       │
│                                                      │
│  [Save Draft]                    [Submit & Post ▶]   │
└─────────────────────────────────────────────────────┘
```

### Step 3 — Quick Add Casual Party (inline, no navigation)

```
  Paid To  [ _____________________ ] [+ Quick Add]
                ┌─────────────────────────────┐
                │  Quick Add Party            │
                │                             │
                │  Name  [________________]   │
                │  Type  ○ Person  ○ Company  │
                │  Phone [________________]   │
                │                             │
                │  [Cancel]        [Add →]    │
                └─────────────────────────────┘
```

### Step 2 — Supplier Payment Form

```
┌─────────────────────────────────────────────────────┐
│  Supplier Payment                         [← Back]  │
│                                                      │
│  Date          [ 19 Oct 2025          ]              │
│  Supplier      [ Search suppliers...  ]              │
│                                                      │
│  Paid Via      ○ Cash  ● Bank  ○ Cheque  ○ Online   │
│  From Account  [ HBL Current Account  ▼ ]           │
│  Amount        [ ______________________ ]            │
│                                                      │
│  Against Invoices    (leave empty for advance)       │
│  ┌────────────────────────────┬────────────────┐    │
│  │ INV-2025-00123  Oct 15    │    200,000     │    │
│  │ INV-2025-00089  Sep 28    │     85,000     │    │
│  ├────────────────────────────┼────────────────┤    │
│  │ + Link Invoice             │                │    │
│  └────────────────────────────┴────────────────┘    │
│                     Allocated:        285,000        │
│                                                      │
│  Reference     [ CHQ-004521               ]          │
│  Note          [ Oct payment run          ]          │
│                                                      │
│  [Save Draft]                    [Submit & Post ▶]   │
└─────────────────────────────────────────────────────┘
```

---

## Controller Logic Summary

### On Validate

1. Verify `payment_type` is consistent with `payment_direction`.
2. For Expense Payment and Income Receipt: verify all line accounts match expected `fs_type` (expense accounts for payments, income accounts for receipts).
3. For Party Payment and Party Receipt: verify `party_role` is consistent with direction (Supplier for outgoing, Customer for incoming typically).
4. For Internal Transfer: verify `from_account` and `to_account` are both Cash and Bank category. Verify they are not the same account.
5. Verify `paid_from` is set for Outgoing and Internal.
6. Verify `paid_into` is set for Incoming and Internal.
7. Compute `total_amount` from lines or amount field.
8. For Invoice Allocation: verify `allocated_amount` does not exceed outstanding on each invoice.

### On Submit

1. Determine GL entries based on `payment_type` as described in GL Posting Logic section.
2. Determine whether advance account or trade account is used for Party payments based on whether `against_invoices` is empty.
3. Call `fn::post_gl_entries` with computed entries.
4. If `against_invoices` is populated, mark each linked invoice line as reconciled for the allocated amount.
5. Set `docstatus = 1`.

### On Cancel

1. Call `fn::reverse_gl_entries`.
2. If invoices were reconciled, unreconcile them — restore outstanding amounts.
3. Set `docstatus = 2`.

---

## Accounts Required in Chart of Accounts

The following accounts must exist for this module to function. They should be created as part of the default CoA template.

| Account Name | Category | Type | Purpose |
|---|---|---|---|
| One-Time Payable | Trade Payable | K | Casual party outgoing payments |
| One-Time Receivable | Trade Receivable | D | Casual party incoming receipts |
| Supplier Advance | Current Asset | S | Advance payments to suppliers |
| Customer Advance | Current Liability | S | Advance receipts from customers |

The Supplier Advance and Customer Advance accounts are defined on Party Role Config per party. These defaults are used when no invoice is linked to a Party Payment or Party Receipt.

---

## Constraints Summary

| Rule | Enforcement Point |
|---|---|
| No free text payees | party field is always a record link |
| Casual Party allowed only for Expense and Income types | Controller validation |
| Full Party required for Party Payment and Party Receipt | Controller validation |
| Advance auto-detection when invoice table is empty | Controller on_submit |
| from_account and to_account cannot be same | Controller validation |
| Allocated amount cannot exceed invoice outstanding | Controller validation |
| Closed period blocks submission | Database event on GL Entry |
| Cannot cancel a submitted payment without reversal | docstatus pattern |
| Cannot manually edit GL entries created by payment | GL Entry immutability |

---

## What Is Explicitly Not in This Module

| Excluded | Reason | Where It Lives |
|---|---|---|
| Journal Entry for users | Complexity, error-prone, not needed | Accounts module, restricted access |
| Cheque printing | Separate concern | Reports module |
| Bank reconciliation | Separate process | Accounts module |
| Payment approval workflow | Stage 4 feature | Workflow module |
| Bulk payment runs | Stage 4 feature | Transactions module |
| Foreign currency revaluation | Separate process | Accounts module |

---

*End of Payment and Receipt Module Specification v1.0*
