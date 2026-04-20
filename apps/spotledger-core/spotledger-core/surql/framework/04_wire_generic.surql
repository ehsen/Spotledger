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
        LET $meta_rec = type::record("doctype_meta", $doctype);
        UPSERT $meta_rec CONTENT {
            name:           $doctype,
            is_submittable: $is_submittable
        };
    };

    -- ── 2. Stage IDs (save) ───────────────────────────────────────────────────
    LET $sv = string::concat($table, "_save_validate");
    LET $sc = string::concat($table, "_save_compute");
    LET $sp = string::concat($table, "_save_persist");
    LET $sv_rec = type::record("pipeline_stage", $sv);
    LET $sc_rec = type::record("pipeline_stage", $sc);
    LET $sp_rec = type::record("pipeline_stage", $sp);

    IF (SELECT id FROM $sv_rec LIMIT 1) = [] {
        UPSERT $sv_rec CONTENT {
            name: "validate", ord: 1, doctype: $doctype, action: "save"
        };
    };
    -- save/compute: derives computed fields after validation, before persist.
    -- Doctypes with no compute functions simply leave this stage empty (runner skips it).
    IF (SELECT id FROM $sc_rec LIMIT 1) = [] {
        UPSERT $sc_rec CONTENT {
            name: "compute", ord: 2, doctype: $doctype, action: "save"
        };
    };
    IF (SELECT id FROM $sp_rec LIMIT 1) = [] {
        UPSERT $sp_rec CONTENT {
            name: "persist", ord: 3, doctype: $doctype, action: "save"
        };
    };

    -- ── 3. Wire mandatory_fields into save/validate ───────────────────────────
    IF (SELECT id FROM has_node
            WHERE in  = $sv_rec
              AND out = pipeline_node:validate_mandatory_fields
            LIMIT 1) = [] {
        RELATE $sv_rec -> has_node -> pipeline_node:validate_mandatory_fields
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
        LET $subv_rec = type::record("pipeline_stage", $subv);
        LET $subc_rec = type::record("pipeline_stage", $subc);
        LET $subp_rec = type::record("pipeline_stage", $subp);
        LET $subo_rec = type::record("pipeline_stage", $subo);
        LET $canv_rec = type::record("pipeline_stage", $canv);
        LET $cano_rec = type::record("pipeline_stage", $cano);

        IF (SELECT id FROM $subv_rec LIMIT 1) = [] {
            UPSERT $subv_rec CONTENT {
                name: "validate",       ord: 1, doctype: $doctype, action: "submit"
            };
        };
        IF (SELECT id FROM $subc_rec LIMIT 1) = [] {
            UPSERT $subc_rec CONTENT {
                name: "compute",        ord: 2, doctype: $doctype, action: "submit"
            };
        };
        IF (SELECT id FROM $subp_rec LIMIT 1) = [] {
            UPSERT $subp_rec CONTENT {
                name: "persist",        ord: 3, doctype: $doctype, action: "submit"
            };
        };
        IF (SELECT id FROM $subo_rec LIMIT 1) = [] {
            UPSERT $subo_rec CONTENT {
                name: "on_submit",      ord: 4, doctype: $doctype, action: "submit"
            };
        };
        IF (SELECT id FROM $canv_rec LIMIT 1) = [] {
            UPSERT $canv_rec CONTENT {
                name: "validate_cancel", ord: 1, doctype: $doctype, action: "cancel"
            };
        };
        IF (SELECT id FROM $cano_rec LIMIT 1) = [] {
            UPSERT $cano_rec CONTENT {
                name: "on_cancel",      ord: 2, doctype: $doctype, action: "cancel"
            };
        };

        -- Wire mandatory_fields into submit/validate
        IF (SELECT id FROM has_node
                WHERE in  = $subv_rec
                  AND out = pipeline_node:validate_mandatory_fields
                LIMIT 1) = [] {
            RELATE $subv_rec -> has_node -> pipeline_node:validate_mandatory_fields
                CONTENT { ord: 1, config: {} };
        };

        -- Wire no_dependents into cancel/validate_cancel
        IF (SELECT id FROM has_node
                WHERE in  = $canv_rec
                  AND out = pipeline_node:validate_cancel_no_dependents
                LIMIT 1) = [] {
            RELATE $canv_rec -> has_node -> pipeline_node:validate_cancel_no_dependents
                CONTENT { ord: 1, config: {} };
        };

        -- Wire mark_cancelled into cancel/on_cancel (last node, ord=99)
        IF (SELECT id FROM has_node
                WHERE in  = $cano_rec
                  AND out = pipeline_node:on_cancel_mark_cancelled
                LIMIT 1) = [] {
            RELATE $cano_rec -> has_node -> pipeline_node:on_cancel_mark_cancelled
                CONTENT { ord: 99, config: {} };
        };
    };

    RETURN { ok: true };
};
