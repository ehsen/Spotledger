# SurrealERP — DocType Designer & Lifecycle Engine
## Comprehensive Feature Plan

> **Stack context:** Rust backend · SurrealDB as source of truth · Frappe-compatible metadata schema · Web-rendered UI

---

## The Vision
The vision is to have a UI in our spotledger-ui, which allows you to create UI via designer, or via adding fields in some tables, both generate the same metadata, just like frappe. But our UI is far more better then frappe, because it not only design it ll contain the logic, permissions(ofcourse like frappe), functions applied on
a system, in an intuitive UI, So basically we are redesignng frappes new doctype screen on steriods. 
## 1. Overview

SurrealERP reimplements the Frappe framework on a Rust + SurrealDB foundation. The DocType system — schema definition, form layout, and document logic — is the core of the platform. This document specifies the full feature set for the **DocType Designer** and **Lifecycle Function Engine**: the two tools through which developers and power users define every aspect of how a document behaves, without leaving the browser.

Three principles drive every design decision in this spec:

- **SurrealDB is the source of truth.** All schema, events, permissions, and functions are expressed as SurrealQL `DEFINE` statements. The UI generates and manages these; users never hand-edit the generated output.
- **No detail is hidden from the power user.** Every field, every function, every validation expression is inspectable and editable in-context. The UI is a structured editor, not an abstraction layer that hides the query language.
- **Familiarity where it counts.** The Frappe mental model — DocType, Child Table, Section, lifecycle hooks — is preserved exactly. Users migrating from Frappe should feel at home within minutes.

---

## 2. Application Shell

### 2.1 Three-tab structure

The designer lives under a single URL per DocType (`/designer/sales_invoice`) with three persistent tabs:

| Tab | Purpose |
|---|---|
| **Doc Design** | Field layout, sections, columns, child table fields |
| **Doc Logic** | Lifecycle stages and their attached SurrealQL functions |
| **SurrealQL** | Read-only generated definition — the ground truth |

Switching tabs preserves all unsaved state. A dirty indicator (dot on the Save button) shows when there are unapplied changes.

### 2.2 Shared sidebar

A single left sidebar serves all three tabs. Its content switches automatically:

- On **Doc Design**: field type palette grouped by category (Link/Relation, Text, Number, Date/Time, Layout)
- On **Doc Logic**: SurrealQL function palette grouped by category (Traversal, Math, String, Logic/Validate)

A search box filters palette items live. Palette items are draggable onto the canvas or into the function list.

### 2.3 Doctype selector

A persistent selector at the top of the Doc Design canvas allows switching between the **main DocType** and any of its **child DocTypes** without navigating away. A color-coded badge ("Main DocType" in purple, "Child DocType" in teal) makes the current context unambiguous at a glance.

---

## 3. Doc Design — Field Layout

### 3.1 Canvas model

The Doc Design canvas renders a document layout as a vertical stack of **sections**. Each section contains one or more **columns**. Each column contains an ordered list of **fields**. This maps directly to Frappe's Section Break / Column Break / field model and generates corresponding `DEFINE FIELD` statements in SurrealQL.

### 3.2 Drag-and-drop field placement

Fields are added by dragging from the left sidebar palette onto any column drop zone. Drop zones highlight on hover. Fields within a column can be reordered by drag-and-drop using the `⠿` grip handle on the left of each field row. Fields can be moved between columns by drag-and-drop.

Dropping a field from the palette creates it with a default name (`new_field`) and immediately selects it, opening its properties in the right panel for immediate rename and configuration.

### 3.3 Section management

- Sections are added via the **+ Add section** button at the bottom of the canvas.
- Sections can be renamed inline by clicking the section label.
- Columns can be added or removed per section using a **+ col** control in the section header. Supported layouts: 1-column (full width), 2-column (left/right), 3-column.
- Sections can be collapsed to a header-only view to manage long forms.
- Sections are reorderable by drag-and-drop.

### 3.4 Child table — inline editing

When a field of type **Table** is present in the canvas, its referenced child DocType's fields are rendered **inline directly below it** in a collapsible panel. This allows adding, reordering, and configuring child table columns without leaving the parent DocType canvas.

The inline child table panel shows:

- All existing fields of the child DocType as compact rows
- A **+ Add field to child table** action at the bottom
- An **Open full editor** button that switches the doctype selector to the child DocType for full section/column editing

This mirrors the Frappe workflow where child table fields are the most frequently edited secondary structure.

### 3.5 Child DocType full editor

Selecting a child DocType via the doctype selector shows its own full canvas — same sections/columns/fields model, same palette, same property panel. A banner identifies it as a child DocType and names its parent. All the same drag-and-drop and section management features apply.

### 3.6 Field property panel

Clicking any field row opens its properties in the right panel. The panel is always visible; it does not appear as a modal. Properties are grouped into three sections:

**Identity**
- Field name (the SurrealDB field key, snake_case)
- Label (display name shown in the rendered form)
- Type (select from all supported types — see §3.7)
- Options / target (for Link: target DocType; for Select: comma-separated option list; for Table: child DocType name)

**Validation**
- ASSERT expression — a raw SurrealQL expression that is embedded as `DEFINE FIELD ... ASSERT <expr>`. The input uses monospace font. Examples: `$value != NONE`, `$value > 0`, `$value IN ["Draft","Submitted"]`.
- Default value — used in `DEFINE FIELD ... DEFAULT <value>`

**Flags** (toggleable pills)
- Required
- Read only
- In list view
- Hidden
- Searchable (creates a SurrealDB index on this field)
- Bold (display hint)

Changes in the property panel are live — the SurrealQL tab reflects them immediately.

### 3.7 Supported field types

| Category | Types |
|---|---|
| Link / Relation | Link (`record(target)`), Table (`array<record(child)>`) |
| Text | Data (`string`), Text (`string`), Small Text, Long Text, Code (`string`) |
| Number | Int, Float, Currency, Percent |
| Date / Time | Date (`datetime`), Datetime, Time |
| Select | Select (`string` with ASSERT enum), Check (`bool`) |
| Layout | Section Break, Column Break, HTML |

---

## 4. Doc Design — SurrealQL Generation

### 4.1 What is generated

The **SurrealQL tab** shows the complete generated definition for the current DocType and all its children. It is a read-only `<textarea>`. A **"View only"** badge makes this explicit. An **Explain ↗** button sends the content to the Claude chat for breakdown.

The generated output includes:

- `DEFINE TABLE <name> SCHEMAFULL;`
- One `DEFINE FIELD` per field, with TYPE, ASSERT, DEFAULT, and VALUE clauses as configured
- All child DocType table and field definitions
- All lifecycle events (from Doc Logic — see §5)
- PERMISSIONS block

### 4.2 Sync model

The generated SurrealQL is never the input — it is always the output. Changes flow in one direction: UI → SurrealQL. The generated definition is applied to SurrealDB via the **Save** button in the topbar, which compiles the current UI state into SurrealQL and executes it against the database using `DEFINE ... OVERWRITE`.

### 4.3 Computed field values

Fields marked as computed (via a `VALUE` expression in the property panel) generate `DEFINE FIELD ... VALUE <expr>` rather than an ASSERT. Example: `amount` on a child item row generates `VALUE $this.qty * $this.rate`.

---

## 5. Doc Logic — Lifecycle Function Engine

### 5.1 Mental model

A DocType has a fixed set of **lifecycle stages**. Each stage corresponds to a moment in the document's state machine. At each stage, an ordered list of **named SurrealQL functions** is executed. Functions are defined as `DEFINE FUNCTION fn::<doctype>::<name>($this: record) { ... }` and are attached to stages via SurrealDB `DEFINE EVENT` handlers.

This is the direct equivalent of Frappe's `validate`, `before_save`, `on_submit`, etc. controller methods — except that each individual function is a first-class, named, inspectable artifact rather than a method on a Python class.

### 5.2 Lifecycle stages

Stages are fixed and ordered. They are grouped semantically:

**Before action** (blocking — throw an error to abort)
- **Validate** — runs before the document is persisted; the right place for all validation logic
- **Before Save** — runs after validation, before the write; the right place for computing derived fields

**After action** (non-blocking — document is already written)
- **On Save** — runs after every successful save (create or update)
- **On Submit** — runs when `status` transitions from `Draft` to `Submitted`; the right place for GL entries, stock ledger, etc.
- **On Cancel** — runs when a submitted document is cancelled; must reverse all On Submit effects
- **On Amend** — runs when a cancelled document is amended; sets up the amendment chain

**Scheduled**
- **Scheduled** — cron-style functions; each function carries a cron expression alongside its code

The sidebar shows all stages with a live function count badge. Stages with zero functions are still visible — this is intentional, so users can see the full lifecycle at a glance and identify gaps.

### 5.3 Function list view

Selecting a stage shows its function list. Each function card shows:

- A sequential execution-order number badge (1, 2, 3…)
- The full function name: `fn::sales_invoice::validate_totals`
- A plain-English description (editable in the code editor)
- Category tags (e.g. `compute`, `accounting`, `validate`)
- A 3–4 line code preview with a fade, showing the function body
- Line count and "Runs:" metadata
- On hover: **Edit** and **Remove** action buttons

**Execution order is explicit and meaningful.** Functions within a stage run in the order shown. The list is reorderable by drag-and-drop using the `⠿` grip. For example, `compute_item_amounts` must appear before `compute_totals`, which must appear before `validate_totals`. The UI enforces this is the user's explicit choice.

### 5.4 Code editor panel

Clicking any function card — or its code preview — opens the function's full SurrealQL in a **persistent right-side code editor panel**. This is not a modal. The function list remains visible and scrollable on the left while the editor is open.

The editor includes:

**Header**
- Full function name in monospace (`fn::sales_invoice::validate_totals`)
- Stage label with stage color dot (e.g. blue dot + "Validate")
- Short description of what the function does

**Toolbar**
- **Format** — normalizes indentation and trims trailing whitespace
- **Review ↗** — sends the current code to Claude for review and improvement suggestions
- **Snippet ↗** — asks Claude for a relevant SurrealQL snippet based on context
- Available context variables shown inline: `$this · $event · $auth · $session`

**Editor area**
- Line numbers gutter (syncs on every keystroke)
- Monospace textarea with `Tab` key support (2-space indent)
- Full SurrealQL function body, including the `DEFINE FUNCTION` wrapper
- Syntax is not highlighted in v1 (planned for v2 with a lightweight tokenizer)

**Footer**
- Live line count status
- **Discard** — reverts to last applied state
- **Apply** — saves the edit into the in-memory definition and updates the code preview in the list

Changes are not written to SurrealDB until the topbar **Save** button is pressed.

### 5.5 Adding and removing functions

**Adding:** The **+ Add function** button in the topbar (and the inline link in empty stages) creates a new function with a placeholder name and body, immediately opens it in the editor, and focuses the function name for renaming.

**Removing:** The **Remove** button on hover in the function list removes the function from the stage after a confirmation. If the function is currently open in the editor, the editor returns to the empty state.

### 5.6 Function naming convention

All functions follow the convention `fn::<doctype>::<action>`. Examples:

- `fn::sales_invoice::validate_totals`
- `fn::sales_invoice::compute_item_amounts`
- `fn::sales_invoice::create_gl_entries`
- `fn::sales_invoice::reverse_gl_entries`
- `fn::purchase_order::validate_supplier_credit`

This namespace isolation ensures no collision between DocTypes and makes the provenance of every function immediately clear in SurrealDB's function registry.

### 5.7 Standard function library

The platform ships a set of standard functions per DocType category that users can add from the palette (drag from sidebar → function list):

**Accounting documents** (Sales Invoice, Purchase Invoice, Payment Entry)
- `fn::accounting::create_gl_entries` — generic GL entry creator, parameterized by account mapping
- `fn::accounting::reverse_gl_entries` — generic GL reversal
- `fn::accounting::update_outstanding` — recalculates outstanding amount from payment references

**Inventory documents** (Delivery Note, Purchase Receipt, Stock Entry)
- `fn::stock::update_stock_ledger` — creates SLE entries
- `fn::stock::validate_stock_availability` — checks warehouse stock before submission

**All documents**
- `fn::common::set_title` — computes the `title` field from a template string
- `fn::common::validate_mandatory` — validates all required fields are populated
- `fn::common::set_status` — transitions status based on a rule map

These are editable after being added — they serve as a starting point, not a black box.

---

## 6. SurrealQL tab — generated definition

The SurrealQL tab is the read-only ground truth view. It shows:

```
-- DocType: Sales Invoice  (auto-generated — edit via UI only)

DEFINE TABLE sales_invoice SCHEMAFULL;

DEFINE FIELD customer ON sales_invoice TYPE record(customer)
  ASSERT $value != NONE;
...

DEFINE EVENT validate_totals ON TABLE sales_invoice
  WHEN $event = "UPDATE" THEN {
    fn::sales_invoice::validate_totals($this);
    fn::sales_invoice::validate_customer($this);
  };
...

DEFINE TABLE sales_invoice_item SCHEMAFULL;
...

DEFINE TABLE sales_invoice PERMISSIONS
  FOR select WHERE $auth.role IN [...]
  FOR create, update WHERE ...
  FOR delete WHERE ...;
```

The tab is not editable. A prominent badge reads **"View only — edit via Doc Design and Doc Logic."** An **Explain ↗** button sends the entire definition to Claude for explanation.

---

## 7. Permissions model

Permissions are configured through a dedicated section in the Doc Design right panel when no field is selected (the "Doc" level). Configuration options:

- Per-role access matrix: select, create, update, delete
- Field-level permissions (hide or read-only a specific field for a role)
- Submit / cancel permission flags (separate from CRUD)
- Owner-only flag (restrict update/delete to `$auth.id = $this.owner`)

These generate the `DEFINE TABLE ... PERMISSIONS` block in the SurrealQL output.

---

## 8. Submittable documents

A DocType can be marked **Submittable** via a toggle in the Doc settings panel (accessible from the topbar). When enabled:

- A `status` field of type Select with values `["Draft", "Submitted", "Cancelled"]` is automatically added and locked from deletion
- An `amended_from` Link field is added (nullable)
- The **On Submit**, **On Cancel**, and **On Amend** lifecycle stages become active in Doc Logic
- Submit and Cancel actions are exposed in the rendered document UI
- The generated SurrealQL includes status transition ASSERT guards on the relevant events

---

## 9. Versioning and audit

Every `DEFINE FUNCTION` and `DEFINE EVENT` change is tracked with a version timestamp and author. The function list shows a "last modified" timestamp per function. A version history drawer (accessible via a clock icon on the function card) shows the last 20 versions of a function's code, with a diff view and a **Restore** action.

This is implemented by storing function versions as records in a `_fn_version` table in SurrealDB, separate from the live function definition.

---

## 10. Keyboard shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl/Cmd + S` | Save (applies to SurrealDB) |
| `Tab` | 2-space indent in code editor |
| `Escape` | Discard unsaved code edit |
| `Ctrl/Cmd + Enter` | Apply code edit |
| `Ctrl/Cmd + Z` | Undo in code editor |
| `Ctrl/Cmd + /` | Toggle comment in code editor |

---

## 11. Phased delivery

### Phase 1 — Doc Design core
- Field palette and drag-and-drop onto sections/columns
- Section and column management (add, reorder, delete)
- Field property panel (identity, validation, flags)
- Inline child table field editing
- Child DocType full editor via doctype selector
- SurrealQL generation and read-only view
- Save to SurrealDB

### Phase 2 — Doc Logic core
- Lifecycle sidebar with all 7 stages
- Function list per stage with execution order
- Drag-to-reorder within a stage
- Code editor panel (persistent, not modal)
- Add / remove functions
- Apply / Discard per function
- Claude Review ↗ and Snippet ↗ integration

### Phase 3 — Standard library and submittable docs
- Standard function palette (accounting, stock, common)
- Submittable DocType toggle and automatic field injection
- Permissions configuration UI
- PERMISSIONS block generation

### Phase 4 — Versioning and polish
- Function version history with diff view and restore
- Keyboard shortcuts
- Format button (SurrealQL normalizer)
- Syntax tokenizer for basic keyword highlighting in code editor
- Scheduled functions with cron expression input

---

## 12. Out of scope (this document)

The following are related but specified separately:

- The rendered document form UI (the end-user-facing form that renders a DocType)
- The List View and Report builder
- Workflow engine (state machine beyond the basic lifecycle)
- Print Format designer
- Role and permission management UI (separate from per-DocType permissions)
- Data import / export tooling
