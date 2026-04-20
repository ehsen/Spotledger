-- GL Entry — pipeline wiring (submittable)

RETURN fn::pipeline::wire_generic("GL Entry", "tabGL_Entry", true);

IF (SELECT id FROM pipeline_node:gl_entry_validate_account LIMIT 1) = [] {
    UPSERT pipeline_node:gl_entry_validate_account CONTENT {
        name:    "gl_entry_validate_account",
        fn_name: "fn::validate::gl_entry_account"
    };
};

IF (SELECT id FROM pipeline_node:gl_entry_validate_period_open LIMIT 1) = [] {
    UPSERT pipeline_node:gl_entry_validate_period_open CONTENT {
        name:    "gl_entry_validate_period_open",
        fn_name: "fn::validate::gl_entry_period_open"
    };
};

IF (SELECT id FROM pipeline_node:gl_entry_validate_party_required LIMIT 1) = [] {
    UPSERT pipeline_node:gl_entry_validate_party_required CONTENT {
        name:    "gl_entry_validate_party_required",
        fn_name: "fn::validate::gl_entry_party_required"
    };
};

IF (SELECT id FROM pipeline_node:gl_entry_validate_amounts LIMIT 1) = [] {
    UPSERT pipeline_node:gl_entry_validate_amounts CONTENT {
        name:    "gl_entry_validate_amounts",
        fn_name: "fn::validate::gl_entry_amounts"
    };
};

IF (SELECT id FROM pipeline_node:gl_entry_validate_immutable LIMIT 1) = [] {
    UPSERT pipeline_node:gl_entry_validate_immutable CONTENT {
        name:    "gl_entry_validate_immutable",
        fn_name: "fn::validate::gl_entry_immutable"
    };
};

IF (SELECT id FROM pipeline_node:gl_entry_compute_derive_period LIMIT 1) = [] {
    UPSERT pipeline_node:gl_entry_compute_derive_period CONTENT {
        name:    "gl_entry_compute_derive_period",
        fn_name: "fn::compute::gl_entry_derive_period"
    };
};

IF (SELECT id FROM pipeline_node:gl_entry_on_cancel_reverse LIMIT 1) = [] {
    UPSERT pipeline_node:gl_entry_on_cancel_reverse CONTENT {
        name:    "gl_entry_on_cancel_reverse",
        fn_name: "fn::on_cancel::gl_entry_reverse"
    };
};

-- save/validate
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabGL_Entry_save_validate AND out = pipeline_node:gl_entry_validate_immutable LIMIT 1) = [] {
    RELATE pipeline_stage:tabGL_Entry_save_validate -> has_node -> pipeline_node:gl_entry_validate_immutable CONTENT { ord: 2, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabGL_Entry_save_validate AND out = pipeline_node:gl_entry_validate_account LIMIT 1) = [] {
    RELATE pipeline_stage:tabGL_Entry_save_validate -> has_node -> pipeline_node:gl_entry_validate_account CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabGL_Entry_save_validate AND out = pipeline_node:gl_entry_validate_amounts LIMIT 1) = [] {
    RELATE pipeline_stage:tabGL_Entry_save_validate -> has_node -> pipeline_node:gl_entry_validate_amounts CONTENT { ord: 10, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabGL_Entry_save_validate AND out = pipeline_node:gl_entry_validate_period_open LIMIT 1) = [] {
    RELATE pipeline_stage:tabGL_Entry_save_validate -> has_node -> pipeline_node:gl_entry_validate_period_open CONTENT { ord: 15, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabGL_Entry_save_validate AND out = pipeline_node:gl_entry_validate_party_required LIMIT 1) = [] {
    RELATE pipeline_stage:tabGL_Entry_save_validate -> has_node -> pipeline_node:gl_entry_validate_party_required CONTENT { ord: 20, config: {} };
};

-- save/compute
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabGL_Entry_save_compute AND out = pipeline_node:gl_entry_compute_derive_period LIMIT 1) = [] {
    RELATE pipeline_stage:tabGL_Entry_save_compute -> has_node -> pipeline_node:gl_entry_compute_derive_period CONTENT { ord: 5, config: {} };
};

-- submit/validate (same checks, immutable not needed post-create)
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabGL_Entry_submit_validate AND out = pipeline_node:gl_entry_validate_account LIMIT 1) = [] {
    RELATE pipeline_stage:tabGL_Entry_submit_validate -> has_node -> pipeline_node:gl_entry_validate_account CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabGL_Entry_submit_validate AND out = pipeline_node:gl_entry_validate_amounts LIMIT 1) = [] {
    RELATE pipeline_stage:tabGL_Entry_submit_validate -> has_node -> pipeline_node:gl_entry_validate_amounts CONTENT { ord: 10, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabGL_Entry_submit_validate AND out = pipeline_node:gl_entry_validate_period_open LIMIT 1) = [] {
    RELATE pipeline_stage:tabGL_Entry_submit_validate -> has_node -> pipeline_node:gl_entry_validate_period_open CONTENT { ord: 15, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabGL_Entry_submit_validate AND out = pipeline_node:gl_entry_validate_party_required LIMIT 1) = [] {
    RELATE pipeline_stage:tabGL_Entry_submit_validate -> has_node -> pipeline_node:gl_entry_validate_party_required CONTENT { ord: 20, config: {} };
};

-- cancel/on_cancel: reverse debits/credits before generic mark_cancelled (ord 99)
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabGL_Entry_cancel_on_cancel AND out = pipeline_node:gl_entry_on_cancel_reverse LIMIT 1) = [] {
    RELATE pipeline_stage:tabGL_Entry_cancel_on_cancel -> has_node -> pipeline_node:gl_entry_on_cancel_reverse CONTENT { ord: 10, config: {} };
};

