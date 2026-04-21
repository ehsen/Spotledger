-- Universal pipeline nodes
--
-- These nodes are shared across all doctypes.  They are declared once here
-- so that wiring.surql files can reference them without repeating the UPSERT.
-- Every statement is idempotent — safe to re-apply on every upgrade.

IF NOT (SELECT id FROM pipeline_node:validate_mandatory_fields LIMIT 1) THEN
    UPSERT pipeline_node:validate_mandatory_fields CONTENT {
        name:    "validate_mandatory_fields",
        fn_name: "fn::validate::mandatory_fields"
    };
END;

IF NOT (SELECT id FROM pipeline_node:validate_party_check LIMIT 1) THEN
    UPSERT pipeline_node:validate_party_check CONTENT {
        name:    "validate_party_check",
        fn_name: "fn::validate::party_check"
    };
END;

IF NOT (SELECT id FROM pipeline_node:validate_cancel_no_dependents LIMIT 1) THEN
    UPSERT pipeline_node:validate_cancel_no_dependents CONTENT {
        name:    "validate_cancel_no_dependents",
        fn_name: "fn::validate_cancel::no_dependents"
    };
END;

IF NOT (SELECT id FROM pipeline_node:on_cancel_mark_cancelled LIMIT 1) THEN
    UPSERT pipeline_node:on_cancel_mark_cancelled CONTENT {
        name:    "on_cancel_mark_cancelled",
        fn_name: "fn::on_cancel::mark_cancelled"
    };
END;

IF NOT (SELECT id FROM pipeline_node:on_submit_mark_submitted LIMIT 1) THEN
    UPSERT pipeline_node:on_submit_mark_submitted CONTENT {
        name:    "on_submit_mark_submitted",
        fn_name: "fn::on_submit::mark_submitted"
    };
END;
