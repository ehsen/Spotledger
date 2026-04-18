-- Pipeline Compute Framework — Runner
-- Applied LAST (after schema, shared fn::, domain fn::, and registry).
-- Requires fn::registry::dispatch to already be defined.

-- ── fn::pipeline::run ─────────────────────────────────────────────────────────
--
-- Entry point for every document operation.  Rust calls this after writing
-- the document content to the DB.
--
-- Returns:
--   { status: "skipped", reason: "no_pipeline" }  → doctype not registered; Rust keeps saved result
--   { status: "skipped", reason: "no_stages"  }  → doctype registered but no stages for this action
--   { status: "ok" }                              → pipeline ran successfully
--   THROW "..."                                   → validation/logic failure; caller should clean up
--
-- NOTE: Pipeline mutations (compute field updates, GL entry inserts, etc.) execute
-- inside SurrealDB's function transaction boundary.  A THROW rolls back ALL of
-- them atomically.  This provides implicit transaction safety with no extra syntax.
DEFINE FUNCTION OVERWRITE fn::pipeline::run(
    $doc_id:  record,
    $doctype: string,
    $action:  string
) {
    -- All seeded doctypes participate in the pipeline; read flags from tabDocType.
    LET $meta = (SELECT issubmittable FROM tabDocType WHERE name = $doctype LIMIT 1)[0];

    IF $meta = NONE {
        RETURN { status: "skipped", reason: "no_pipeline" };
    };

    -- Guard: submit/cancel/amend require issubmittable
    IF $action IN ["submit", "cancel", "amend"] AND !$meta.issubmittable {
        RETURN { status: "error", message: string::concat($doctype, " is not submittable. Action '", $action, "' is not allowed.") };
    };

    -- Fetch stages for this (doctype, action) pair, ordered by ord
    LET $stages = (
        SELECT id, name, ord FROM pipeline_stage
        WHERE doctype = $doctype AND action = $action
        ORDER BY ord ASC
    );

    IF array::len($stages) = 0 {
        RETURN { status: "skipped", reason: "no_stages" };
    };

    -- Walk stages in order
    FOR $stage IN $stages {
        -- Query has_node relation table directly (avoids graph traversal ordering issues)
        LET $edges = (
            SELECT out, ord, config FROM has_node
            WHERE in = $stage.id
            ORDER BY ord ASC
        );

        FOR $edge IN $edges {
            LET $node   = (SELECT fn_name FROM $edge.out)[0];
            LET $result = fn::registry::dispatch($node.fn_name, $doc_id, $edge.config OR {});

            -- Fail-fast: first error halts all remaining nodes
            IF type::is_object($result) AND $result.error != NONE {
                RETURN { status: "error", message: string::concat("[", $node.fn_name, "] ", $result.error) };
            };
        };
    };

    -- After cancel stages complete: cascade to dependent documents
    IF $action = "cancel" {
        fn::pipeline::cascade_cancel($doc_id, $doctype);
    };

    RETURN { status: "ok" };
};

-- ── fn::pipeline::cascade_cancel ─────────────────────────────────────────────
--
-- Called automatically after cancel stages complete.
-- Finds dependent documents via cascade_rule and cancels them recursively.
-- Each child runs its own full cancel pipeline (depth-first, fail-fast).
DEFINE FUNCTION OVERWRITE fn::pipeline::cascade_cancel(
    $doc_id:  record,
    $doctype: string
) {
    LET $rules = (SELECT * FROM cascade_rule WHERE parent_doctype = $doctype);

    FOR $rule IN $rules {
        LET $dependents = (
            SELECT id FROM type::table($rule.child_doctype)
            WHERE record::id(type::field($rule.find_by_field)) = record::id($doc_id)
              AND docstatus != 2
        );

        FOR $dep IN $dependents {
            fn::pipeline::run($dep.id, $rule.child_doctype, "cancel");
        };
    };
};
