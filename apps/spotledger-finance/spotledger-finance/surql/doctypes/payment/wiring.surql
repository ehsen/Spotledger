-- Payment — pipeline wiring (submittable)

RETURN fn::pipeline::wire_generic("Payment", "tabPayment", true);

IF (SELECT id FROM pipeline_node:payment_validate_type_direction LIMIT 1) = [] {
    UPSERT pipeline_node:payment_validate_type_direction CONTENT {
        name:    "payment_validate_type_direction",
        fn_name: "fn::validate::payment_type_direction"
    };
};

IF (SELECT id FROM pipeline_node:payment_validate_line_accounts LIMIT 1) = [] {
    UPSERT pipeline_node:payment_validate_line_accounts CONTENT {
        name:    "payment_validate_line_accounts",
        fn_name: "fn::validate::payment_line_accounts"
    };
};

IF (SELECT id FROM pipeline_node:payment_validate_party_role LIMIT 1) = [] {
    UPSERT pipeline_node:payment_validate_party_role CONTENT {
        name:    "payment_validate_party_role",
        fn_name: "fn::validate::payment_party_role"
    };
};

IF (SELECT id FROM pipeline_node:payment_validate_transfer_accounts LIMIT 1) = [] {
    UPSERT pipeline_node:payment_validate_transfer_accounts CONTENT {
        name:    "payment_validate_transfer_accounts",
        fn_name: "fn::validate::payment_transfer_accounts"
    };
};

IF (SELECT id FROM pipeline_node:payment_validate_paid_accounts LIMIT 1) = [] {
    UPSERT pipeline_node:payment_validate_paid_accounts CONTENT {
        name:    "payment_validate_paid_accounts",
        fn_name: "fn::validate::payment_paid_accounts"
    };
};

IF (SELECT id FROM pipeline_node:payment_validate_casual_party_scope LIMIT 1) = [] {
    UPSERT pipeline_node:payment_validate_casual_party_scope CONTENT {
        name:    "payment_validate_casual_party_scope",
        fn_name: "fn::validate::payment_casual_party_scope"
    };
};

IF (SELECT id FROM pipeline_node:payment_validate_invoice_allocation LIMIT 1) = [] {
    UPSERT pipeline_node:payment_validate_invoice_allocation CONTENT {
        name:    "payment_validate_invoice_allocation",
        fn_name: "fn::validate::payment_invoice_allocation"
    };
};

IF (SELECT id FROM pipeline_node:payment_compute_totals LIMIT 1) = [] {
    UPSERT pipeline_node:payment_compute_totals CONTENT {
        name:    "payment_compute_totals",
        fn_name: "fn::compute::payment_totals"
    };
};

IF (SELECT id FROM pipeline_node:payment_on_submit_post_gl LIMIT 1) = [] {
    UPSERT pipeline_node:payment_on_submit_post_gl CONTENT {
        name:    "payment_on_submit_post_gl",
        fn_name: "fn::on_submit::payment_post_gl"
    };
};

IF (SELECT id FROM pipeline_node:payment_on_cancel_reverse_gl LIMIT 1) = [] {
    UPSERT pipeline_node:payment_on_cancel_reverse_gl CONTENT {
        name:    "payment_on_cancel_reverse_gl",
        fn_name: "fn::on_cancel::payment_reverse_gl"
    };
};

-- save/validate
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_save_validate AND out = pipeline_node:payment_validate_type_direction LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_save_validate -> has_node -> pipeline_node:payment_validate_type_direction CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_save_validate AND out = pipeline_node:payment_validate_casual_party_scope LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_save_validate -> has_node -> pipeline_node:payment_validate_casual_party_scope CONTENT { ord: 8, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_save_validate AND out = pipeline_node:payment_validate_line_accounts LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_save_validate -> has_node -> pipeline_node:payment_validate_line_accounts CONTENT { ord: 10, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_save_validate AND out = pipeline_node:payment_validate_party_role LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_save_validate -> has_node -> pipeline_node:payment_validate_party_role CONTENT { ord: 15, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_save_validate AND out = pipeline_node:payment_validate_transfer_accounts LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_save_validate -> has_node -> pipeline_node:payment_validate_transfer_accounts CONTENT { ord: 20, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_save_validate AND out = pipeline_node:payment_validate_paid_accounts LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_save_validate -> has_node -> pipeline_node:payment_validate_paid_accounts CONTENT { ord: 25, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_save_validate AND out = pipeline_node:payment_validate_invoice_allocation LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_save_validate -> has_node -> pipeline_node:payment_validate_invoice_allocation CONTENT { ord: 30, config: {} };
};

-- save/compute
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_save_compute AND out = pipeline_node:payment_compute_totals LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_save_compute -> has_node -> pipeline_node:payment_compute_totals CONTENT { ord: 5, config: {} };
};

-- submit/validate
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_submit_validate AND out = pipeline_node:payment_validate_type_direction LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_submit_validate -> has_node -> pipeline_node:payment_validate_type_direction CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_submit_validate AND out = pipeline_node:payment_validate_casual_party_scope LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_submit_validate -> has_node -> pipeline_node:payment_validate_casual_party_scope CONTENT { ord: 8, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_submit_validate AND out = pipeline_node:payment_validate_line_accounts LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_submit_validate -> has_node -> pipeline_node:payment_validate_line_accounts CONTENT { ord: 10, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_submit_validate AND out = pipeline_node:payment_validate_party_role LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_submit_validate -> has_node -> pipeline_node:payment_validate_party_role CONTENT { ord: 15, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_submit_validate AND out = pipeline_node:payment_validate_transfer_accounts LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_submit_validate -> has_node -> pipeline_node:payment_validate_transfer_accounts CONTENT { ord: 20, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_submit_validate AND out = pipeline_node:payment_validate_paid_accounts LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_submit_validate -> has_node -> pipeline_node:payment_validate_paid_accounts CONTENT { ord: 25, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_submit_validate AND out = pipeline_node:payment_validate_invoice_allocation LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_submit_validate -> has_node -> pipeline_node:payment_validate_invoice_allocation CONTENT { ord: 30, config: {} };
};

-- submit/on_submit: post GL entries
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_submit_on_submit AND out = pipeline_node:payment_on_submit_post_gl LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_submit_on_submit -> has_node -> pipeline_node:payment_on_submit_post_gl CONTENT { ord: 5, config: {} };
};

-- cancel/on_cancel: reverse GL entries
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_cancel_on_cancel AND out = pipeline_node:payment_on_cancel_reverse_gl LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_cancel_on_cancel -> has_node -> pipeline_node:payment_on_cancel_reverse_gl CONTENT { ord: 10, config: {} };
};

