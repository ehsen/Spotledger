-- Pipeline Compute Framework — Schema
-- Idempotent: uses OVERWRITE / IF NOT EXISTS throughout.
-- Applied by install_app before any fn:: or wiring seeds.

-- ── doctype_meta ─────────────────────────────────────────────────────────────
-- Registers doctypes in the pipeline. Presence here is the opt-in flag:
-- fn::pipeline::run returns { status: "skipped" } for any doctype not in this table.
DEFINE TABLE IF NOT EXISTS doctype_meta SCHEMALESS;
DEFINE FIELD OVERWRITE name           ON doctype_meta TYPE string;
DEFINE FIELD OVERWRITE is_submittable ON doctype_meta TYPE bool DEFAULT false;
DEFINE INDEX OVERWRITE idx_dtmeta_name ON doctype_meta FIELDS name UNIQUE;

-- ── pipeline_stage ────────────────────────────────────────────────────────────
-- One record per (doctype, action, stage-name).  The ord field controls
-- execution sequence.  Stages are data — AI can add / reorder / remove them.
DEFINE TABLE IF NOT EXISTS pipeline_stage SCHEMALESS;
DEFINE FIELD OVERWRITE name    ON pipeline_stage TYPE string;
DEFINE FIELD OVERWRITE ord     ON pipeline_stage TYPE int;
DEFINE FIELD OVERWRITE doctype ON pipeline_stage TYPE string;
DEFINE FIELD OVERWRITE action  ON pipeline_stage TYPE string;
DEFINE INDEX OVERWRITE idx_pstage ON pipeline_stage FIELDS doctype, action, name UNIQUE;

-- ── pipeline_node ─────────────────────────────────────────────────────────────
-- Global (doctype-agnostic) registry of fn:: functions.
-- One record per fn_name — shared across doctypes via has_node edges.
DEFINE TABLE IF NOT EXISTS pipeline_node SCHEMALESS;
DEFINE FIELD OVERWRITE name    ON pipeline_node TYPE string;
DEFINE FIELD OVERWRITE fn_name ON pipeline_node TYPE string;
DEFINE INDEX OVERWRITE idx_pnode_fn ON pipeline_node FIELDS fn_name UNIQUE;

-- ── has_node (relation) ───────────────────────────────────────────────────────
-- Edge: pipeline_stage -> has_node -> pipeline_node
-- Carries ord (execution order within stage) and config (edge-level params that
-- let the same universal node behave differently across doctypes).
DEFINE TABLE IF NOT EXISTS has_node SCHEMALESS;
DEFINE FIELD OVERWRITE in     ON has_node TYPE record<pipeline_stage>;
DEFINE FIELD OVERWRITE out    ON has_node TYPE record<pipeline_node>;
DEFINE FIELD OVERWRITE ord    ON has_node TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE config ON has_node TYPE option<object>;

-- ── cascade_rule ──────────────────────────────────────────────────────────────
-- When a submittable doctype is cancelled, which dependent documents must
-- also be cancelled?  Each rule declares one parent → child dependency.
-- field: the fieldname on child that references the parent record ID.
DEFINE TABLE IF NOT EXISTS cascade_rule SCHEMALESS;
DEFINE FIELD OVERWRITE parent_doctype ON cascade_rule TYPE string;
DEFINE FIELD OVERWRITE child_doctype  ON cascade_rule TYPE string;
DEFINE FIELD OVERWRITE find_by_field  ON cascade_rule TYPE string;
DEFINE INDEX OVERWRITE idx_cascade ON cascade_rule FIELDS parent_doctype, child_doctype UNIQUE;

-- ── pipeline_run ──────────────────────────────────────────────────────────────
-- Audit log written by the Rust caller, not by the pipeline function itself
-- (SurrealDB function transactions roll back all mutations on THROW, so audit
-- must live outside the function boundary).
DEFINE TABLE IF NOT EXISTS pipeline_run SCHEMALESS;
DEFINE FIELD OVERWRITE doctype     ON pipeline_run TYPE string;
DEFINE FIELD OVERWRITE doc_id      ON pipeline_run TYPE string;
DEFINE FIELD OVERWRITE action      ON pipeline_run TYPE string;
DEFINE FIELD OVERWRITE status      ON pipeline_run TYPE string;
DEFINE FIELD OVERWRITE failed_node ON pipeline_run TYPE option<string>;
DEFINE FIELD OVERWRITE errors      ON pipeline_run TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE started_at  ON pipeline_run TYPE datetime;
DEFINE FIELD OVERWRITE finished_at ON pipeline_run TYPE option<datetime>;
DEFINE INDEX OVERWRITE idx_prun_doc    ON pipeline_run FIELDS doctype, doc_id;
DEFINE INDEX OVERWRITE idx_prun_status ON pipeline_run FIELDS status, started_at;

-- ── child_of (relation) ───────────────────────────────────────────────────────
-- Graph edge used for all tree-structured doctypes: Account, Cost Center,
-- Chart of Accounts, and future Asset/Item category trees.
-- SCHEMAFULL: only in/out are ever valid fields on a tree edge.
-- Any code writing extra fields to a child_of edge is a bug.
DEFINE TABLE IF NOT EXISTS child_of SCHEMAFULL;
DEFINE FIELD OVERWRITE in  ON child_of TYPE record;
DEFINE FIELD OVERWRITE out ON child_of TYPE record;
DEFINE INDEX OVERWRITE idx_child_of ON child_of FIELDS in, out UNIQUE;
