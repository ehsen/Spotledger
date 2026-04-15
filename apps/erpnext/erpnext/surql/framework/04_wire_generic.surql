-- Generic pipeline wiring helper
--
-- fn::pipeline::wire_generic($doctype, $table, $is_submittable)
--
-- Creates the factory-default stage graph for any doctype and wires the two
-- universal nodes that run on every document:
--
--   save/validate    → [1] fn::validate::mandatory_fields
--   save/persist     (empty — Rust UPSERT step)
--
-- For submittable doctypes, also creates:
--   submit/validate  → [1] fn::validate::mandatory_fields
--   submit/compute
--   submit/persist
--   submit/on_submit
--   cancel/validate_cancel → [1] fn::validate_cancel::no_dependents
--   cancel/on_cancel       → [99] fn::on_cancel::mark_cancelled
--
-- Stage record IDs follow the convention used throughout:
--   {table}_{action}_{stage_name}
--   e.g. "account_save_validate", "purchase_invoice_cancel_on_cancel"
--
-- All statements are idempotent — safe to call on every upgrade or re-wire.
-- Doctype-specific wiring.surql files call this first, then add their own nodes.
DEFINE FUNCTION OVERWRITE fn::pipeline::wire_generic(
    $doctype:        string,
    $table:          string,
    $is_submittable: bool
) {
    -- ── 1. Register in doctype_meta (opt-in flag for fn::pipeline::run) ───────
    LET $existing_meta = (SELECT id FROM doctype_meta WHERE name = $doctype LIMIT 1)[0];
    IF $existing_meta = NONE {
        UPSERT type::thing("doctype_meta", $doctype) CONTENT {
            name:           $doctype,
            is_submittable: $is_submittable
        };
    };

    -- ── 2. Stage IDs (save) ───────────────────────────────────────────────────
    LET $sv = string::concat($table, "_save_validate");
    LET $sp = string::concat($table, "_save_persist");

    IF (SELECT id FROM type::thing("pipeline_stage", $sv) LIMIT 1) = [] {
        UPSERT type::thing("pipeline_stage", $sv) CONTENT {
            name: "validate", ord: 1, doctype: $doctype, action: "save"
        };
    };
    IF (SELECT id FROM type::thing("pipeline_stage", $sp) LIMIT 1) = [] {
        UPSERT type::thing("pipeline_stage", $sp) CONTENT {
            name: "persist", ord: 2, doctype: $doctype, action: "save"
        };
    };

    -- ── 3. Wire mandatory_fields into save/validate ───────────────────────────
    IF (SELECT id FROM has_node
            WHERE in  = type::thing("pipeline_stage", $sv)
              AND out = pipeline_node:validate_mandatory_fields
            LIMIT 1) = [] {
        RELATE type::thing("pipeline_stage", $sv)
            -> has_node
            -> pipeline_node:validate_mandatory_fields
            CONTENT { ord: 1, config: {} };
    };

    -- ── 4. Submittable-only stages ────────────────────────────────────────────
    IF $is_submittable {
        LET $subv = string::concat($table, "_submit_validate");
        LET $subc = string::concat($table, "_submit_compute");
        LET $subp = string::concat($table, "_submit_persist");
        LET $subo = string::concat($table, "_submit_on_submit");
        LET $canv = string::concat($table, "_cancel_validate_cancel");
        LET $cano = string::concat($table, "_cancel_on_cancel");

        IF (SELECT id FROM type::thing("pipeline_stage", $subv) LIMIT 1) = [] {
            UPSERT type::thing("pipeline_stage", $subv) CONTENT {
                name: "validate",       ord: 1, doctype: $doctype, action: "submit"
            };
        };
        IF (SELECT id FROM type::thing("pipeline_stage", $subc) LIMIT 1) = [] {
            UPSERT type::thing("pipeline_stage", $subc) CONTENT {
                name: "compute",        ord: 2, doctype: $doctype, action: "submit"
            };
        };
        IF (SELECT id FROM type::thing("pipeline_stage", $subp) LIMIT 1) = [] {
            UPSERT type::thing("pipeline_stage", $subp) CONTENT {
                name: "persist",        ord: 3, doctype: $doctype, action: "submit"
            };
        };
        IF (SELECT id FROM type::thing("pipeline_stage", $subo) LIMIT 1) = [] {
            UPSERT type::thing("pipeline_stage", $subo) CONTENT {
                name: "on_submit",      ord: 4, doctype: $doctype, action: "submit"
            };
        };
        IF (SELECT id FROM type::thing("pipeline_stage", $canv) LIMIT 1) = [] {
            UPSERT type::thing("pipeline_stage", $canv) CONTENT {
                name: "validate_cancel", ord: 1, doctype: $doctype, action: "cancel"
            };
        };
        IF (SELECT id FROM type::thing("pipeline_stage", $cano) LIMIT 1) = [] {
            UPSERT type::thing("pipeline_stage", $cano) CONTENT {
                name: "on_cancel",      ord: 2, doctype: $doctype, action: "cancel"
            };
        };

        -- Wire mandatory_fields into submit/validate
        IF (SELECT id FROM has_node
                WHERE in  = type::thing("pipeline_stage", $subv)
                  AND out = pipeline_node:validate_mandatory_fields
                LIMIT 1) = [] {
            RELATE type::thing("pipeline_stage", $subv)
                -> has_node
                -> pipeline_node:validate_mandatory_fields
                CONTENT { ord: 1, config: {} };
        };

        -- Wire no_dependents into cancel/validate_cancel
        IF (SELECT id FROM has_node
                WHERE in  = type::thing("pipeline_stage", $canv)
                  AND out = pipeline_node:validate_cancel_no_dependents
                LIMIT 1) = [] {
            RELATE type::thing("pipeline_stage", $canv)
                -> has_node
                -> pipeline_node:validate_cancel_no_dependents
                CONTENT { ord: 1, config: {} };
        };

        -- Wire mark_cancelled into cancel/on_cancel (last node, ord=99)
        IF (SELECT id FROM has_node
                WHERE in  = type::thing("pipeline_stage", $cano)
                  AND out = pipeline_node:on_cancel_mark_cancelled
                LIMIT 1) = [] {
            RELATE type::thing("pipeline_stage", $cano)
                -> has_node
                -> pipeline_node:on_cancel_mark_cancelled
                CONTENT { ord: 99, config: {} };
        };
    };

    RETURN { ok: true };
};
