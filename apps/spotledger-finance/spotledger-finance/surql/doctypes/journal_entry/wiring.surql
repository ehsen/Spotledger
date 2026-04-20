-- Journal Entry — pipeline wiring (submittable)

RETURN fn::pipeline::wire_generic("Journal Entry", "tabJournal_Entry", true);

IF (SELECT id FROM pipeline_node:je_validate_has_accounts LIMIT 1) = [] {
    UPSERT pipeline_node:je_validate_has_accounts CONTENT {
        name:    "je_validate_has_accounts",
        fn_name: "fn::validate::je_has_accounts"
    };
};

IF (SELECT id FROM pipeline_node:je_validate_account_validity LIMIT 1) = [] {
    UPSERT pipeline_node:je_validate_account_validity CONTENT {
        name:    "je_validate_account_validity",
        fn_name: "fn::validate::je_account_validity"
    };
};

IF (SELECT id FROM pipeline_node:je_validate_period_open LIMIT 1) = [] {
    UPSERT pipeline_node:je_validate_period_open CONTENT {
        name:    "je_validate_period_open",
        fn_name: "fn::validate::je_period_open"
    };
};

IF (SELECT id FROM pipeline_node:je_validate_balance LIMIT 1) = [] {
    UPSERT pipeline_node:je_validate_balance CONTENT {
        name:    "je_validate_balance",
        fn_name: "fn::validate::je_balance"
    };
};

IF (SELECT id FROM pipeline_node:je_compute_totals LIMIT 1) = [] {
    UPSERT pipeline_node:je_compute_totals CONTENT {
        name:    "je_compute_totals",
        fn_name: "fn::compute::je_totals"
    };
};

IF (SELECT id FROM pipeline_node:je_on_submit_create_gl LIMIT 1) = [] {
    UPSERT pipeline_node:je_on_submit_create_gl CONTENT {
        name:    "je_on_submit_create_gl",
        fn_name: "fn::on_submit::je_create_gl_entries"
    };
};

IF (SELECT id FROM pipeline_node:je_on_cancel_gl_entries LIMIT 1) = [] {
    UPSERT pipeline_node:je_on_cancel_gl_entries CONTENT {
        name:    "je_on_cancel_gl_entries",
        fn_name: "fn::on_cancel::je_cancel_gl_entries"
    };
};

-- save/validate
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabJournal_Entry_save_validate AND out = pipeline_node:je_validate_has_accounts LIMIT 1) = [] {
    RELATE pipeline_stage:tabJournal_Entry_save_validate -> has_node -> pipeline_node:je_validate_has_accounts CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabJournal_Entry_save_validate AND out = pipeline_node:je_validate_account_validity LIMIT 1) = [] {
    RELATE pipeline_stage:tabJournal_Entry_save_validate -> has_node -> pipeline_node:je_validate_account_validity CONTENT { ord: 10, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabJournal_Entry_save_validate AND out = pipeline_node:je_validate_period_open LIMIT 1) = [] {
    RELATE pipeline_stage:tabJournal_Entry_save_validate -> has_node -> pipeline_node:je_validate_period_open CONTENT { ord: 15, config: {} };
};

-- save/compute
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabJournal_Entry_save_compute AND out = pipeline_node:je_compute_totals LIMIT 1) = [] {
    RELATE pipeline_stage:tabJournal_Entry_save_compute -> has_node -> pipeline_node:je_compute_totals CONTENT { ord: 5, config: {} };
};

-- submit/validate: full balance check required
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabJournal_Entry_submit_validate AND out = pipeline_node:je_validate_has_accounts LIMIT 1) = [] {
    RELATE pipeline_stage:tabJournal_Entry_submit_validate -> has_node -> pipeline_node:je_validate_has_accounts CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabJournal_Entry_submit_validate AND out = pipeline_node:je_validate_account_validity LIMIT 1) = [] {
    RELATE pipeline_stage:tabJournal_Entry_submit_validate -> has_node -> pipeline_node:je_validate_account_validity CONTENT { ord: 10, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabJournal_Entry_submit_validate AND out = pipeline_node:je_validate_period_open LIMIT 1) = [] {
    RELATE pipeline_stage:tabJournal_Entry_submit_validate -> has_node -> pipeline_node:je_validate_period_open CONTENT { ord: 15, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabJournal_Entry_submit_validate AND out = pipeline_node:je_validate_balance LIMIT 1) = [] {
    RELATE pipeline_stage:tabJournal_Entry_submit_validate -> has_node -> pipeline_node:je_validate_balance CONTENT { ord: 20, config: {} };
};

-- submit/on_submit: post GL entries
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabJournal_Entry_submit_on_submit AND out = pipeline_node:je_on_submit_create_gl LIMIT 1) = [] {
    RELATE pipeline_stage:tabJournal_Entry_submit_on_submit -> has_node -> pipeline_node:je_on_submit_create_gl CONTENT { ord: 5, config: {} };
};

-- cancel/on_cancel: reverse GL entries before generic mark_cancelled (ord 99)
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabJournal_Entry_cancel_on_cancel AND out = pipeline_node:je_on_cancel_gl_entries LIMIT 1) = [] {
    RELATE pipeline_stage:tabJournal_Entry_cancel_on_cancel -> has_node -> pipeline_node:je_on_cancel_gl_entries CONTENT { ord: 10, config: {} };
};

