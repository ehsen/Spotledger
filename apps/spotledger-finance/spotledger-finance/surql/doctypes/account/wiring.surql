-- Account — pipeline wiring

RETURN fn::pipeline::wire_generic("Account", "tabAccount", false);

IF (SELECT id FROM pipeline_node:account_compute_from_group LIMIT 1) = [] {
    UPSERT pipeline_node:account_compute_from_group CONTENT {
        name:    "account_compute_from_group",
        fn_name: "fn::compute::account_derive_from_group"
    };
};

IF (SELECT id FROM pipeline_node:account_validate_number_in_range LIMIT 1) = [] {
    UPSERT pipeline_node:account_validate_number_in_range CONTENT {
        name:    "account_validate_number_in_range",
        fn_name: "fn::validate::account_number_in_group_range"
    };
};

IF (SELECT id FROM pipeline_node:account_validate_group_matches_coa LIMIT 1) = [] {
    UPSERT pipeline_node:account_validate_group_matches_coa CONTENT {
        name:    "account_validate_group_matches_coa",
        fn_name: "fn::validate::account_group_matches_coa"
    };
};

IF (SELECT id FROM pipeline_node:account_validate_retained_earnings LIMIT 1) = [] {
    UPSERT pipeline_node:account_validate_retained_earnings CONTENT {
        name:    "account_validate_retained_earnings",
        fn_name: "fn::validate::account_retained_earnings_required"
    };
};

IF (SELECT id FROM pipeline_node:account_validate_type_immutable LIMIT 1) = [] {
    UPSERT pipeline_node:account_validate_type_immutable CONTENT {
        name:    "account_validate_type_immutable",
        fn_name: "fn::validate::account_type_immutable"
    };
};

IF (SELECT id FROM pipeline_node:account_relate_parent LIMIT 1) = [] {
    UPSERT pipeline_node:account_relate_parent CONTENT {
        name:    "account_relate_parent",
        fn_name: "fn::on_save::account_relate_parent"
    };
};

IF (SELECT id FROM pipeline_node:account_extend_companies LIMIT 1) = [] {
    UPSERT pipeline_node:account_extend_companies CONTENT {
        name:    "account_extend_companies",
        fn_name: "fn::on_save::account_extend_companies"
    };
};

-- save/validate: structural checks that don't depend on computed fields
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccount_save_validate AND out = pipeline_node:account_validate_group_matches_coa LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccount_save_validate -> has_node -> pipeline_node:account_validate_group_matches_coa CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccount_save_validate AND out = pipeline_node:account_validate_type_immutable LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccount_save_validate -> has_node -> pipeline_node:account_validate_type_immutable CONTENT { ord: 10, config: {} };
};

-- save/compute: derive from group, then validate derived values
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccount_save_compute AND out = pipeline_node:account_compute_from_group LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccount_save_compute -> has_node -> pipeline_node:account_compute_from_group CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccount_save_compute AND out = pipeline_node:account_validate_number_in_range LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccount_save_compute -> has_node -> pipeline_node:account_validate_number_in_range CONTENT { ord: 10, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccount_save_compute AND out = pipeline_node:account_validate_retained_earnings LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccount_save_compute -> has_node -> pipeline_node:account_validate_retained_earnings CONTENT { ord: 15, config: {} };
};

-- save/persist: graph edges and Account Company records
IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccount_save_persist AND out = pipeline_node:account_relate_parent LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccount_save_persist -> has_node -> pipeline_node:account_relate_parent CONTENT { ord: 85, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccount_save_persist AND out = pipeline_node:account_extend_companies LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccount_save_persist -> has_node -> pipeline_node:account_extend_companies CONTENT { ord: 90, config: {} };
};

