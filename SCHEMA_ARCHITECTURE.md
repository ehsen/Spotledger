# Schema Architecture — Rust-Driven, SurrealDB-Derived

## Overview

The core principle of this architecture is simple:

> **Rust is the single source of truth. SurrealDB is a derived artifact of it.**

No schema is ever written directly in SurrealDB. No `.surql` file is ever hand-edited.
Everything flows in one direction — from Rust definitions, into the database.

---

## Syncfusion License Key
For UI, You need to register Syncfusion license key for version 33.xx
this is Essential Studio Enterprise Edition. 
key is : Ngo9BigBOggjHTQxAR8/V1JHaF5cWWdCf1FpRmJGdld5fUVHYVZUTXxaS00DNHVRdkdlWXtfcXRWRmldWUJ0WkRWYEo=

## The Problem This Solves

Traditional metadata-driven systems (like Frappe) store schema in the database itself.
This is convenient but brittle — schema lives in a place with no compiler, no type
checking, and no guarantee that what you wrote is structurally valid until something
breaks at runtime.

The alternative — defining everything in application code — is more reliable but
creates a second maintenance burden when the database needs to reflect those definitions.

This architecture eliminates that tension. You get:

- The reliability of statically defined, compiler-verified schema
- The queryability and graph traversal of a live database
- Zero duplication — the database is always generated, never independently maintained

---

## Layers of the System

```
┌─────────────────────────────────────────┐
│         Rust DocType Definitions        │  ← Source of truth. Compiler verified.
│         (src/doctypes/)                 │    Never wrong if it compiles.
└────────────────────┬────────────────────┘
                     │  code generation
                     ▼
┌──────────────────────────��──────────────┐
│         SurrealQL Generator             │  ← Walks Rust definitions, emits
│         (internal, automatic)           │    valid SurrealQL. Never hand-written.
└────────────────────┬────────────────────┘
                     │  upsert / apply
                     ▼
┌─────────────────────────────────────────┐
│         SurrealDB Graph                 │  ← Derived. Always in sync.
│                                         │    Queryable, traversable, graph-native.
└─────────────────────────────────────────┘
```

---

## What Lives in Rust

Every DocType is defined as a plain Rust data structure. It describes:

- The DocType name and module
- Every field — its name, type, label, position (idx), and constraints
- Field-level metadata — mandatory, read-only, hidden, default value
- Relationships — Link fields declare their target DocType explicitly

All DocType definitions are registered in a single central module. Adding a new
DocType means creating one file and adding one line to the registry. Removing a
DocType means deleting that file and removing that line. The compiler enforces
that nothing is left broken.

Field types are a closed enum in Rust. You cannot invent a new field type by
accident. A typo in a field type is a compile error, not a runtime surprise.

---

## What Lives in SurrealDB

SurrealDB holds two categories of data:

**Structural schema (fixed, defined once)**
The core table definitions — `doctype`, `docfield`, and the relation tables
`has_field` and `links_to`. These are the permanent scaffolding of the graph.
They are defined in a schema preamble that runs once and never changes unless
a fundamentally new core concept is introduced.

**Instance data (generated from Rust)**
The actual DocType nodes, DocField nodes, and the edges between them. These
are fully generated from Rust definitions and applied via idempotent upserts.
Every sync is safe to run multiple times — it converges to the correct state.

---

## The Graph Model

```
[doctype:SalesOrder] ──has_field {idx:0}──► [docfield:customer]
                     ──has_field {idx:1}──► [docfield:grand_total]
                     ──has_field {idx:2}──► [docfield:status]

[docfield:customer] ──links_to──► [doctype:Customer]
```

**Nodes:**
- `doctype` — represents a DocType (e.g. SalesOrder, Customer, ToDo)
- `docfield` — represents a field definition (e.g. subject, status, grand_total)

**Edges:**
- `has_field` — connects a DocType to its fields. Carries `idx` (ordering)
  and field-level overrides like `mandatory` and `read_only`
- `links_to` — connects a Link-type DocField to the DocType it points at.
  This makes DocType dependencies a first-class graph query, not a string match.

**Why idx lives on the edge, not the node:**
A DocField is a reusable definition. Its position (idx) is meaningful only
within the context of a specific DocType. Two DocTypes could share a field
definition but place it at different positions. The edge is the right place
for that information.

---

## Migration Strategy

There are no traditional migrations in this system. There is only sync.

Because all upserts are idempotent, running the sync at any time converges
the database to exactly what the Rust definitions describe. You do not need
migration files, version numbers, or up/down scripts.

**What sync does:**
1. Applies the structural preamble (table and relation definitions) — safe to re-run
2. Upserts every DocType node
3. Upserts every DocField node
4. Upserts every `has_field` edge with correct idx and constraints
5. Upserts every `links_to` edge for Link-type fields

**What sync does not do:**
It does not delete data. If you remove a field from a DocType in Rust, the
old DocField node and its edges remain in the database until explicitly
cleaned up. This is intentional — it prevents accidental data loss. A
separate explicit cleanup command handles removal.

---

## Developer Flow

### Adding a new DocType

1. Create a new file in `src/doctypes/`
2. Define the DocType — name, module, and all fields in order
3. Register it in `src/doctypes/mod.rs` with one line
4. Run `cargo build` — the compiler validates the definition
5. Run `cargo run -- sync` — the DB is updated

The DocType is now live in SurrealDB as a proper graph with all nodes and edges.

---

### Changing a field

1. Open the relevant DocType file in `src/doctypes/`
2. Edit the field — change its type, label, idx, constraints, or options
3. Run `cargo build` — invalid field types or missing required properties are caught here
4. Run `cargo run -- sync` — the upsert updates the existing node and edges in place

No migration file needed. The sync is idempotent.

---

### Removing a field

1. Delete the `DocFieldDef` entry from the DocType definition
2. Run `cargo build`
3. Run `cargo run -- sync` — the field is no longer connected via edges
4. Run `cargo run -- cleanup` — explicitly removes orphaned DocField nodes

The cleanup step is intentionally separate to prevent accidental deletion.

---

### Adding a new FieldType

1. Add a new variant to the `FieldType` enum in `src/meta/types.rs`
2. Add its SurrealDB type mapping in the `to_surreal_type()` method
3. Run `cargo build` — the compiler will flag every match statement that
   does not yet handle the new variant. Fix each one.
4. Define a DocField using the new type in any DocType
5. Sync as normal

The compiler forces you to handle the new type everywhere it matters before
you can ship anything.

---

### Reviewing what will be applied before syncing

Run `cargo run -- emit`

This writes the fully generated SurrealQL to `generated/schema.surql` without
touching the database. You can read it, diff it against a previous version,
include it in a pull request for review, or run it manually.

This file is never hand-edited. It exists purely for visibility and auditability.

---

### Startup behaviour

When the server starts normally (no subcommand), the sync runs automatically
before the server begins accepting requests. This means:

- A fresh environment (new machine, new container) always comes up with the
  correct schema without any manual steps
- A redeployment that includes DocType changes applies those changes
  automatically on restart
- The sync is fast — all operations are upserts and SurrealDB handles
  idempotency efficiently

---

## AI-Assisted Schema Changes

Because Rust is the source of truth, AI edits only Rust files. This is the
correct place for AI to operate because:

**The compiler acts as an automatic reviewer.**
If the AI makes a structurally invalid change — wrong type, missing required
field, invalid enum variant — the build fails. The AI sees the compiler output,
corrects the mistake, and tries again. This feedback loop requires no human
intervention for structural errors.

**The change is visible and reviewable.**
A schema change is a diff on a Rust file in git. It is as readable as any
other code change, can be reviewed in a pull request, and is permanently
part of the version history.

**The database is never touched directly by AI.**
The AI does not write SurrealQL. It does not run queries against the DB. It
edits Rust, the build validates, and the sync applies the result. The DB is
always downstream.

**The flow for an AI-driven change:**

```
Instruction: "Add a mandatory Currency field called grand_total at position 4 in SalesOrder"
      │
      ▼
AI edits src/doctypes/sales_order.rs
      │
      ▼
cargo build  →  success or compiler error (AI fixes and retries)
      │
      ▼
cargo run -- emit  →  review generated/schema.surql if desired
      │
      ▼
cargo run -- sync  →  DB updated
```

---

## What This System Does Not Support

**Runtime schema customisation by end users.**
Non-technical users cannot add fields through a UI. This is an intentional
tradeoff. The system is developer-owned and AI-assisted, not end-user
configurable. Stability and correctness are prioritised over flexibility.

**Automatic destructive migrations.**
Removing a field or DocType does not automatically delete data. This requires
an explicit cleanup command. The system will never silently drop data.

**Independent `.surql` authoring.**
Writing schema directly in SurrealDB, whether through the CLI, a dashboard,
or a manually written `.surql` file, is outside this system and will be
overwritten on the next sync. SurrealDB is treated as an output, not an input.

---

## Summary

| Concern | Where it lives |
|---|---|
| DocType and field definitions | Rust source files |
| Field type validity | Rust enum — compiler enforced |
| Schema generation | Automatic — never hand-written |
| Schema application | Idempotent sync — runs on startup |
| Schema review and audit | `generated/schema.surql` — git tracked |
| Graph queries and traversal | SurrealDB — fully operational |
| DocType dependency graph | `links_to` edges — queryable natively |
| AI schema changes | Rust files only — compiler validates first |
| Destructive changes | Explicit cleanup command — never automatic |