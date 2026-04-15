# Writing Doctype Pipelines — Authoring Guide

**Audience**: Developers adding new ERPNext/Frappe-equivalent doctype logic to Spotledger.  
**Prerequisite**: Read `GRAPH_COMPUTE_PLAN.md` for the architecture. This document is the practical how-to.

---

## The Core Mental Model

In Frappe, doctype logic lives in a Python class:

```python
class PurchaseInvoice(BuyingController):
    def validate(self): ...
    def before_save(self): ...
    def on_submit(self): ...
    def on_cancel(self): ...
```

In Spotledger, the same logic is split across two artifacts:

| Frappe Python | Spotledger equivalent |
|---|---|
| `validate()` | `fn::validate::*` SurrealQL functions wired into the **validate** stage |
| `before_save()` | `fn::compute::*` functions wired into the **compute** stage |
| `on_submit()` | `fn::on_submit::*` functions wired into the **on_submit** stage |
| `on_cancel()` | `fn::on_cancel::*` functions wired into the **on_cancel** stage |
| `validate()` called again on submit | submit pipeline has its own **validate** stage that re-runs all validators |

The **pipeline graph** (stored as SurrealDB records) is the wiring.  
The **fn:: functions** (defined in `.surql` files) are the logic.

They are separate. Wiring is data; functions are code.

---

## Part 1 — The Universal Document Path

Every document that passes through the system follows one of two paths depending on whether `is_submittable` is true on the `tabDocType` record.

### Path A — Regular (non-submittable, save only)

```
HTTP POST /api/resource/{DocType}
    │
    ├─ [Rust] auth check (Write / Create)
    ├─ [Rust] custom_ prefix enforcement
    ├─ [Rust] naming resolution (naming series, expression, UUID)
    ├─ [Rust] CREATE / UPSERT into SurrealDB
    │
    └─ [SurrealDB] fn::pipeline::run($doc_id, $doctype, "save")
            │
            ├─ stage: validate  (ord 1)
            │       node: fn::validate::mandatory_fields  — config: { fields: [...] }
            │       node: fn::validate::party_check       — config: { field: "...", party_type: "..." }
            │       node: fn::validate::*                 — any doctype-specific validators
            │
            └─ stage: persist  (ord 2)
                    (reserved for final housekeeping, no nodes by default)
```

If the pipeline returns `{ status: "skipped" }` (no stages defined yet), Rust returns the result of the raw UPSERT. The document is saved without pipeline processing. This is the **progressive opt-in** mechanism — a doctype without wiring still saves correctly.

### Path B — Submittable (save / submit / cancel)

```
                ┌── SAVE ──────────────────────────────────────┐
                │  validate → compute → persist                 │
                │  (same as Path A + compute stage)             │
                └───────────────────────────────────────────────┘

                ┌── SUBMIT ─────────────────────────────────────┐
                │  validate → compute → persist → on_submit      │
                │  (re-validates; submit is never trusted blindly)│
                └───────────────────────────────────────────────┘

                ┌── CANCEL ─────────────────────────────────────┐
                │  validate_cancel → on_cancel                   │
                │  → cascade_cancel (recursive depth-first)      │
                └───────────────────────────────────────────────┘
```

**Critical rule**: Submit always re-validates. Time passes between save and submit. The submit pipeline has its own validate stage that runs the same validators — never skip this.

---

## Part 2 — File Layout

Every new doctype's logic lives under:

```
apps/erpnext/erpnext/surql/doctypes/{doctype_snake_case}/functions.surql
```

The framework, shared, and other files are **never touched** when adding a new doctype.

```
apps/erpnext/erpnext/surql/
    framework/
        01_schema.surql     ← DDL for pipeline tables (DO NOT EDIT)
        02_runner.surql     ← fn::pipeline::run (DO NOT EDIT)
    shared/
        fn_validate_mandatory.surql         ← universal, reused by all
        fn_validate_party.surql             ← universal, reused by all
        fn_validate_cancel.surql            ← universal, reused by all
        fn_on_cancel_mark_cancelled.surql   ← universal, reused by all
    doctypes/
        account/functions.surql
        gl_entry/functions.surql
        journal_entry/functions.surql
        purchase_invoice/functions.surql
        your_new_doctype/functions.surql    ← CREATE THIS
```

`registry.surql` is generated automatically by `install_app` by scanning all `.surql` files for `DEFINE FUNCTION fn::` declarations. **Never create or edit it manually.**

---

## Part 3 — The Domain Function Contract

Every `fn::` function **must** conform to this signature and return one of two shapes:

```surql
DEFINE FUNCTION OVERWRITE fn::{namespace}::{name}(
    $doc_id: record,   -- record ID of the document being processed
    $config: object    -- edge-level config from the has_node relation
) {
    -- ... logic ...

    -- Success:
    RETURN { ok: true };

    -- Failure (validation error, business rule violation):
    RETURN { error: "Human-readable message. " };
};
```

### Rules

1. **Never throw** — return `{ error: "..." }` instead. The runner will `THROW` for you.
2. **Never declare side effects** outside of what the function purpose says it does.
3. **Validate-only functions** — read from DB, never write.
4. **Compute functions** — read + `UPDATE $doc_id SET field = value`. Never write to other tables.
5. **Submit/cancel functions** — may `INSERT INTO` other tables (e.g. GL entries) or `UPDATE` related records.
6. The runner is **fail-fast**: the first `{ error: ... }` halts the entire stage and rolls back all mutations in the current function transaction.

### Namespace conventions

| Namespace | Purpose |
|---|---|
| `fn::validate::` | Read-only checks. Return error or ok. |
| `fn::compute::` | Derive/update fields on the document itself. |
| `fn::on_submit::` | Post-submit side effects (GL entries, stock ledger, etc.). |
| `fn::on_cancel::` | Cancel-time cleanup. |
| `fn::validate_cancel::` | Read-only pre-cancel checks. |

---

## Part 4 — Wiring (Stages and Nodes)

Wiring is the graph of `pipeline_stage` records connected to `pipeline_node` records via `has_node` edges. It is **generated at install time** by Rust from the DocType JSON, then lives as mutable data in SurrealDB.

### How the factory-default wiring is seeded

`pipeline.rs` → `seed_doctype_pipelines()` reads every DocType JSON and calls `seed_one_doctype()`, which:

1. Creates pipeline_stage records for each action / phase pair.
2. Wires `fn::validate::mandatory_fields` into all validate stages, carrying the field list as edge config.
3. Wires `fn::validate::party_check` for any field named `supplier`, `customer`, `employee`, etc.
4. Wires `fn::on_cancel::mark_cancelled` into the `on_cancel` stage (submittable only).
5. Wires `fn::validate_cancel::no_dependents` into the `validate_cancel` stage (submittable only).

This covers ~95% of all doctypes with zero hand-written wiring.

### Tier A extra wiring

For core ledger doctypes (account, gl_entry, journal_entry, purchase_invoice), `seed_tier_a_wiring()` adds domain-specific nodes programmatically in Rust. See `crates/spotledger-db/src/pipeline.rs` for the implementation.

### Stage IDs

Stage IDs follow the pattern `{table_snake}_{action}_{phase}`:

```
purchase_invoice_save_validate
purchase_invoice_save_persist
purchase_invoice_submit_validate
purchase_invoice_submit_compute
purchase_invoice_submit_persist
purchase_invoice_submit_on_submit
purchase_invoice_cancel_validate_cancel
purchase_invoice_cancel_on_cancel
```

> The `table` snake form strips the `tab` prefix and lowercases: `tabPurchase_Invoice` → `purchase_invoice`.

### Edge config

The same universal node (e.g. `fn::validate::party_check`) can be wired into many doctypes with different configs. The config is stored **on the `has_node` edge**, not in the node itself:

```surql
-- Purchase Invoice wiring: supplier party check
RELATE pipeline_stage:purchase_invoice_submit_validate
    -> has_node
    -> pipeline_node:validate_party_check
    CONTENT {
        ord:    10,
        config: { field: "supplier", party_type: "Supplier" }
    };

-- Sales Invoice wiring: same node, different config
RELATE pipeline_stage:sales_invoice_submit_validate
    -> has_node
    -> pipeline_node:validate_party_check
    CONTENT {
        ord:    10,
        config: { field: "customer", party_type: "Customer" }
    };
```

Zero code duplication. One function, two doctypes, different behavior.

---

## Part 5 — Step-by-Step: Adding a New Doctype

### Step 1 — Check if factory wiring is sufficient

For most non-financial doctypes, the factory seed covers everything:
- mandatory fields are validated automatically from the JSON's `reqd: 1` markers
- known party links are validated automatically

If the doctype is simple (no computed fields, no GL posting, no cross-document dependencies), **you do not need to write any `.surql` file**. The pipeline handles it.

### Step 2 — Create `functions.surql` for custom logic

If the doctype needs logic beyond what factory wiring provides, create:

```
apps/erpnext/erpnext/surql/doctypes/{snake_name}/functions.surql
```

At the top of the file, add a comment block:

```surql
-- {DocType Name} — domain functions
--
-- Frappe equivalent hooks implemented here:
--   validate()    → fn::validate::*
--   before_save() → fn::compute::*
--   on_submit()   → fn::on_submit::*
--   on_cancel()   → fn::on_cancel::*
```

### Step 3 — Write the functions

See [Part 3](#part-3--the-domain-function-contract) for the contract. See the worked examples below.

### Step 4 — Add Tier A wiring in `pipeline.rs` (if needed)

If your doctype needs non-default wiring (custom stages, specific ordering, extra domain nodes), add a `seed_{doctype}_wiring()` function in `crates/spotledger-db/src/pipeline.rs` and call it from `seed_tier_a_wiring()`.

```rust
pub async fn seed_tier_a_wiring(adapter: &DbAdapter) -> Result<(), DbError> {
    seed_account_wiring(adapter).await?;
    seed_gl_entry_wiring(adapter).await?;
    seed_journal_entry_wiring(adapter).await?;
    seed_purchase_invoice_wiring(adapter).await?;
    seed_your_new_doctype_wiring(adapter).await?;  // ← add here
    Ok(())
}
```

### Step 5 — Build and reinstall

```powershell
cargo xtask build
spotledger install-app erpnext hello_graph --bench .
```

`install_app` will:
- Scan all `functions.surql` files and discover your new `fn::` declarations.
- Regenerate `registry.surql` to include them.
- Apply all surql files in order.
- Re-seed pipeline wiring (upserts, existing wiring is preserved by `IF NOT EXISTS` guards).

---

## Part 6 — Worked Example: Purchase Invoice

The Purchase Invoice is the canonical reference. Here is how its Frappe Python hooks map to the pipeline.

### Frappe `purchase_invoice.py` → Pipeline mapping

| Frappe method | Pipeline stage | SurrealQL function |
|---|---|---|
| `validate()` → `validate_supplier_invoice()` | `submit:validate` | `fn::validate::purchase_invoice_duplicate` |
| `validate()` → `po_required()` / `pr_required()` | `submit:validate` | `fn::validate::mandatory_fields` (via config) |
| `validate()` → `validate_credit_to_acc()` | `submit:validate` | `fn::validate::party_check` (via config) |
| `set_missing_values()` / `set_due_date()` | `save:compute` | `fn::compute::purchase_invoice_due_date` |
| `calculate_taxes_and_totals()` | `save:compute` | `fn::compute::purchase_invoice_totals` |
| `on_submit()` → `make_gl_entries()` | `submit:on_submit` | `fn::on_submit::purchase_invoice_gl` |
| `on_cancel()` → `make_reverse_gl_entries()` | `cancel:on_cancel` | (cascade cancels child gl_entry records) |
| `on_cancel()` → set status = "Cancelled" | `cancel:on_cancel` | `fn::on_cancel::mark_cancelled` |

### The duplicate bill check

Frappe's `validate_supplier_invoice()` prevents duplicate bill numbers. In the pipeline:

```surql
-- apps/erpnext/erpnext/surql/doctypes/purchase_invoice/functions.surql

DEFINE FUNCTION OVERWRITE fn::validate::purchase_invoice_duplicate(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT supplier, bill_no, bill_date FROM $doc_id)[0];

    -- bill_no is optional — skip check if not set
    IF ($doc.bill_no = NONE OR $doc.bill_no = "") OR ($doc.bill_date = NONE) {
        RETURN { ok: true };
    };

    LET $existing = (
        SELECT id FROM tabPurchase_Invoice
        WHERE supplier  = $doc.supplier
          AND bill_no   = $doc.bill_no
          AND bill_date = $doc.bill_date
          AND id        != $doc_id
          AND docstatus != 2   -- not cancelled
        LIMIT 1
    )[0];

    IF $existing != NONE {
        RETURN {
            error: string::concat(
                "Duplicate bill. Invoice ",
                <string>record::id($existing.id),
                " already exists for this supplier, bill number and date."
            )
        };
    };

    RETURN { ok: true };
};
```

### Totals computation

Frappe's `calculate_taxes_and_totals()` runs via `before_save`. In the pipeline, it is a compute node:

```surql
DEFINE FUNCTION OVERWRITE fn::compute::purchase_invoice_totals(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT items, tax_rate, conversion_rate,
                       write_off_amount, discount_amount FROM $doc_id)[0];

    LET $net  = math::sum($doc.items[*].amount OR 0);
    LET $rate = $doc.tax_rate OR 0.0;
    LET $tax  = math::round($net * ($rate / 100.0) * 100.0) / 100.0;
    LET $conv = $doc.conversion_rate OR 1.0;
    LET $grand = $net + $tax
               - ($doc.write_off_amount OR 0.0)
               - ($doc.discount_amount  OR 0.0);

    UPDATE $doc_id SET
        net_total               = $net,
        total_taxes_and_charges = $tax,
        base_net_total          = $net  * $conv,
        grand_total             = $grand,
        base_grand_total        = $grand * $conv,
        outstanding_amount      = $grand;

    RETURN { ok: true };
};
```

### GL posting on submit

Frappe's `on_submit()` → `make_gl_entries()`. The Spotledger version:

```surql
DEFINE FUNCTION OVERWRITE fn::on_submit::purchase_invoice_gl(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT items, taxes_and_charges_account,
                       total_taxes_and_charges, credit_to,
                       supplier, posting_date, company,
                       remarks, base_grand_total, grand_total,
                       conversion_rate FROM $doc_id)[0];

    LET $doc_name = record::id($doc_id);

    -- Debit each expense account line
    FOR $item IN $doc.items {
        INSERT INTO tabGL_Entry {
            name:         string::concat($doc_name, "-gl-exp-", <string>$item.idx OR 0),
            account:      $item.expense_account,
            debit:        $item.base_amount OR $item.amount OR 0,
            credit:       0,
            voucher_type: "Purchase Invoice",
            voucher_no:   $doc_name,
            posting_date: $doc.posting_date,
            company:      $doc.company,
            remarks:      $doc.remarks,
            cost_center:  $item.cost_center,
            docstatus:    1
        };
    };

    -- Credit the payable account
    INSERT INTO tabGL_Entry {
        name:         string::concat($doc_name, "-gl-payable"),
        account:      $doc.credit_to,
        debit:        0,
        credit:       $doc.base_grand_total OR $doc.grand_total OR 0,
        voucher_type: "Purchase Invoice",
        voucher_no:   $doc_name,
        posting_date: $doc.posting_date,
        company:      $doc.company,
        party:        $doc.supplier,
        party_type:   "Supplier",
        docstatus:    1
    };

    -- Verify balance
    LET $entries    = (SELECT debit, credit FROM tabGL_Entry WHERE voucher_no = $doc_name);
    LET $ttl_debit  = math::sum($entries[*].debit  OR 0);
    LET $ttl_credit = math::sum($entries[*].credit OR 0);

    IF math::abs($ttl_debit - $ttl_credit) > 0.01 {
        RETURN {
            error: string::concat(
                "GL imbalance: debit=", <string>$ttl_debit,
                " credit=", <string>$ttl_credit
            )
        };
    };

    UPDATE $doc_id SET
        docstatus          = 1,
        status             = "Unpaid",
        outstanding_amount = $doc.grand_total;

    RETURN { ok: true };
};
```

**Why it works atomically**: The GL entry inserts and the docstatus update all happen inside the SurrealDB function transaction. If the balance check returns an error, the runner throws and the entire transaction rolls back — no partial GL entries, no dangling docstatus change.

---

## Part 7 — Worked Example: GL Entry (Validation as Self-Defence)

GL Entry has no submit action from the user. It is created by parent documents (Journal Entry, Purchase Invoice) during their `on_submit` stage. But it still has a pipeline — for validation:

```surql
DEFINE FUNCTION OVERWRITE fn::validate::gl_entry_accounts(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT account, debit, credit FROM $doc_id)[0];

    IF $doc.account = NONE OR $doc.account = "" {
        RETURN { error: "Account is required on GL Entry" };
    };

    LET $acc = (
        SELECT is_group, disabled
        FROM tabAccount
        WHERE name = $doc.account
        LIMIT 1
    )[0];

    IF $acc = NONE    { RETURN { error: string::concat("Account does not exist: ",    <string>$doc.account) }; };
    IF $acc.disabled  { RETURN { error: string::concat("Account is disabled: ",       <string>$doc.account) }; };
    IF $acc.is_group  { RETURN { error: string::concat("Cannot post to group account:", <string>$doc.account) }; };
    IF ($doc.debit  OR 0) < 0 { RETURN { error: "Debit cannot be negative"  }; };
    IF ($doc.credit OR 0) < 0 { RETURN { error: "Credit cannot be negative" }; };

    RETURN { ok: true };
};
```

This validation fires for **every write** to `tabGL_Entry`, regardless of caller — whether it is the purchase invoice GL function, a journal entry, or a direct DB console command. This is the key difference from Frappe: in Frappe, bypassing the controller bypasses validation. Here, validation is table-level and unconditional.

The cancel behaviour (reversing debits and credits) is a separate function:

```surql
DEFINE FUNCTION OVERWRITE fn::on_cancel::gl_entry_reverse(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT debit, credit FROM $doc_id)[0];

    UPDATE $doc_id SET
        debit        = $doc.credit,
        credit       = $doc.debit,
        docstatus    = 2,
        status       = "Cancelled",
        is_cancelled = true;

    RETURN { ok: true };
};
```

---

## Part 8 — Worked Example: Account (Master Data with Graph Traversal)

The Account doctype replaces Frappe's nested set (`lft`/`rgt`) with a `child_of` relation table, and replaces `account.py::validate_parent()` with a graph-aware SurrealQL function.

### Parent validation

```surql
DEFINE FUNCTION OVERWRITE fn::validate::account_parent_check(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT parent_account, root_type, company FROM $doc_id)[0];

    -- No parent = root account; skip check
    IF $doc.parent_account = NONE OR $doc.parent_account = "" {
        RETURN { ok: true };
    };

    LET $parent = (
        SELECT root_type, is_group, company
        FROM tabAccount
        WHERE name = $doc.parent_account
        LIMIT 1
    )[0];

    IF $parent = NONE {
        RETURN { error: string::concat("Parent does not exist: ", <string>$doc.parent_account) };
    };
    IF !$parent.is_group AND $parent.is_group != 1 {
        RETURN { error: "Parent account must be a group account" };
    };
    IF $parent.root_type != $doc.root_type AND $doc.root_type != NONE AND $doc.root_type != "" {
        RETURN { error: string::concat(
            "Root type '", <string>$doc.root_type,
            "' does not match parent root type '", <string>$parent.root_type, "'"
        )};
    };
    IF $parent.company != $doc.company {
        RETURN { error: "Parent account must belong to the same company" };
    };

    RETURN { ok: true };
};
```

### Derived fields (report_type, balance_must_be)

Frappe's `set_root_and_report_type()` sets computed fields from `root_type`. In the pipeline:

```surql
DEFINE FUNCTION OVERWRITE fn::compute::account_root_type(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT root_type, report_type, balance_must_be FROM $doc_id)[0];
    LET $rt  = $doc.root_type;

    IF $doc.report_type = NONE OR $doc.report_type = "" {
        LET $rtype =
            IF $rt IN ["Asset", "Liability", "Equity"] { "Balance Sheet" }
            ELSE IF $rt IN ["Income", "Expense"] { "Profit and Loss" }
            ELSE { NONE };
        IF $rtype != NONE { UPDATE $doc_id SET report_type = $rtype; };
    };

    IF $doc.balance_must_be = NONE OR $doc.balance_must_be = "" {
        LET $bal =
            IF $rt IN ["Asset", "Expense"]          { "Debit"  }
            ELSE IF $rt IN ["Liability","Income","Equity"] { "Credit" }
            ELSE { NONE };
        IF $bal != NONE { UPDATE $doc_id SET balance_must_be = $bal; };
    };

    RETURN { ok: true };
};
```

---

## Part 9 — Universal Shared Nodes (Reuse These)

These nodes already exist and are wired to most doctypes automatically. You can wire them to custom stages or custom configs via the `has_node` relation.

### `fn::validate::mandatory_fields`

Checks that fields are non-empty. Config: `{ fields: ["field1", "field2"] }`.

Factory-seeded automatically from `reqd: 1` on DocType JSON fields — you rarely need to wire this manually.

### `fn::validate::party_check`

Validates that a linked party (Supplier, Customer, etc.) exists, is not disabled, and is not on hold.  
Config: `{ field: "supplier", party_type: "Supplier" }`.

Factory-seeded automatically for fields named `supplier`, `customer`, `employee`, `lead`, `contact`, `shareholder`.

### `fn::validate_cancel::no_dependents`

Blocks cancel if active references exist in another table.  
Config: `{ check_table: "tabPayment_Entry_Reference", ref_field: "reference_name", doctype_field: "reference_doctype", doctype_value: "Purchase Invoice" }`.

Wired automatically with empty config (no-op) for all submittable doctypes. Override the config to add real dependency checks.

### `fn::on_cancel::mark_cancelled`

Sets `docstatus = 2` and `status = "Cancelled"`. Always the **last** node in an `on_cancel` stage.

Wired automatically for all submittable doctypes at `ord = 99` (highest, runs last).

---

## Part 10 — Cancellation and Cascade

### Cascade rules

When a parent document is cancelled, dependent child documents must also be cancelled. This is declared as a `cascade_rule` record, not hardcoded in any function:

```surql
-- Purchase Invoice cancelled → cancel its GL entries
UPSERT cascade_rule:purchase_invoice_gl_entry CONTENT {
    parent_doctype: "Purchase Invoice",
    child_doctype:  "GL Entry",
    find_by_field:  "voucher_no"
};
```

`fn::pipeline::cascade_cancel` reads these rules and recursively cancels matching child records. Each child runs its own full cancel pipeline (including its own `validate_cancel` stage), so the cascade is safe.

### Cancel must be idempotent

The `on_cancel` stage of any doctype must be safe to run on an already-cancelled document. The `fn::on_cancel::mark_cancelled` node uses `UPDATE` not `INSERT`, so it is inherently idempotent. For custom `on_cancel` nodes, always guard against double-execution:

```surql
DEFINE FUNCTION OVERWRITE fn::on_cancel::my_doctype_undo_something($doc_id: record, $config: object) {
    LET $doc = (SELECT docstatus FROM $doc_id)[0];

    -- Guard: already cancelled
    IF $doc.docstatus = 2 {
        RETURN { ok: true };
    };

    -- ... undo logic ...
    RETURN { ok: true };
};
```

---

## Part 11 — What Rust Still Does (and Must Not Lose)

The Rust `save_proxy.rs` retains exactly two responsibilities. Do not move these to SurrealDB:

### 1. Auth

```rust
let allowed = has_permission(adapter, user, doctype, PermissionType::Write).await?;
if !allowed { return Err(SaveProxyError::PermissionDenied { ... }); }
```

Frappe-style role + doctype + user-permission matrix cannot be expressed in SurrealDB's row-level permission model. Auth stays in Rust forever.

### 2. Naming

```rust
let resolved = resolve_name(adapter, doctype, &doc, None).await?;
```

The pipeline receives a `record` type — the document must already exist (even as a shell) before `fn::pipeline::run` is called. Rust creates the shell, then the pipeline fills it in.

**Sequence for new documents:**
```
Rust: auth → naming → CREATE shell → fn::pipeline::run("save")
SurrealDB: pipeline fetches the shell, validates, computes, updates in place
```

### `custom_` prefix enforcement

```rust
// fieldname rewriting in save_proxy.rs — stays at API boundary
// ensures custom fields never shadow standard fields
if is_custom && !fieldname.starts_with("custom_") { rename(fieldname, "custom_" + fieldname) }
```

---

## Part 12 — Querying the Pipeline Graph

Because the topology is stored as SurrealDB records, you can query it like any other data:

```surql
-- What stages run when Purchase Invoice is submitted?
SELECT name, ord FROM pipeline_stage
WHERE doctype = "Purchase Invoice" AND action = "submit"
ORDER BY ord ASC;

-- What nodes (functions) run in the submit validate stage?
SELECT ->has_node->pipeline_node.fn_name AS fn,
       ->has_node.ord    AS ord,
       ->has_node.config AS config
FROM pipeline_stage
WHERE doctype = "Purchase Invoice" AND action = "submit" AND name = "validate"
ORDER BY ->has_node.ord ASC;

-- Which doctypes use fn::validate::party_check?
SELECT <-has_node<-pipeline_stage.doctype AS doctype,
       <-has_node.config AS config
FROM pipeline_node
WHERE fn_name = "fn::validate::party_check";

-- All failed pipeline runs in the last 24 hours
SELECT doctype, doc_id, action, failed_node, errors, started_at
FROM pipeline_run
WHERE status = "failed" AND started_at > time::now() - 24h
ORDER BY started_at DESC;
```

You can also add or reorder nodes at runtime without touching any Rust or SurrealQL code:

```surql
-- Add a custom validator to an existing stage at runtime
RELATE pipeline_stage:purchase_invoice_submit_validate
    -> has_node
    -> pipeline_node:my_custom_validator
    CONTENT { ord: 50, config: { threshold: 10000.0 } };
```

---

## Part 13 — What Not to Do

### Do not `THROW` from domain functions

```surql
-- WRONG
IF $doc.supplier = NONE { THROW "Supplier is required"; };

-- CORRECT
IF $doc.supplier = NONE { RETURN { error: "Supplier is required" }; };
```

The runner translates `{ error: "..." }` into a `THROW`. If you throw directly, the runner's error message is lost and the audit log receives no `failed_node` value.

### Do not read from `$before` or `$after` in domain functions

Those are `DEFINE EVENT` variables. Domain functions receive `$doc_id` and must fetch the document themselves:

```surql
-- WRONG (event-style, not function-style)
LET $doc = $after;

-- CORRECT
LET $doc = (SELECT * FROM $doc_id)[0];
```

### Do not create stages or nodes by hand in surql files

Stage and node records are generated by Rust. Write only `DEFINE FUNCTION` statements in `functions.surql` files. Wiring (`pipeline_stage`, `pipeline_node`, `has_node`) is seeded by `seed_doctype_pipelines()` and `seed_tier_a_wiring()`.

### Do not write compute logic in validate functions

Validate functions are read-only. If you need to derive a field value, write a separate `fn::compute::` function. This keeps each function's purpose unambiguous and makes the pipeline graph self-documenting.

### Do not add domain functions to shared/

The `shared/` directory is for universal reusable nodes (mandatory_fields, party_check, etc.). Domain-specific logic (purchase_invoice_duplicate, gl_entry_accounts, etc.) belongs in `doctypes/{name}/functions.surql`.

---

## Quick Reference

### Adding a new validate function

1. Add to `apps/erpnext/erpnext/surql/doctypes/{name}/functions.surql`
2. Use namespace `fn::validate::{name}`
3. Read-only: fetch doc, check condition, return `{ ok: true }` or `{ error: "..." }`
4. Add wiring in `seed_{name}_wiring()` in `pipeline.rs`, or it will be picked up automatically if it is a mandatory field or party link

### Adding a new compute function

1. Add to the same `functions.surql`
2. Use namespace `fn::compute::{name}`
3. `UPDATE $doc_id SET field = value` — only update the document itself
4. Return `{ ok: true }` — never return an error shape from a compute function unless input is so bad that the document cannot be saved at all

### Adding a new on_submit function

1. Add to the same `functions.surql`
2. Use namespace `fn::on_submit::{name}`
3. May `INSERT INTO` child tables (GL entries, stock ledger entries, etc.)
4. Verify balance / integrity after inserts; return `{ error: "..." }` on failure
5. The entire function runs inside one SurrealDB transaction — a returned error rolls back all inserts

### Stage execution order (submittable doctype, submit action)

| ord | stage name | canonical purpose |
|-----|--------|----|
| 1 | `validate` | read-only checks (mandatory, party, business rules) |
| 2 | `compute` | derive totals, due dates, other computed fields |
| 3 | `persist` | (reserved; empty by default) |
| 4 | `on_submit` | create GL entries, update outstanding amounts, post stock |
