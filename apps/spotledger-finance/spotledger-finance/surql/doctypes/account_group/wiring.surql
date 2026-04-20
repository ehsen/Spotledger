-- Account Group — pipeline wiring

RETURN fn::pipeline::wire_generic("Account Group", "tabAccount_Group", false);

IF (SELECT id FROM pipeline_node:account_group_compute_classification LIMIT 1) = [] {
    UPSERT pipeline_node:account_group_compute_classification CONTENT {
        name:    "account_group_compute_classification",
        fn_name: "fn::compute::account_group_derive_classification"
    };
};

IF (SELECT id FROM pipeline_node:account_group_validate_number_range LIMIT 1) = [] {
    UPSERT pipeline_node:account_group_validate_number_range CONTENT {
        name:    "account_group_validate_number_range",
        fn_name: "fn::validate::account_group_number_range"
    };
};

IF (SELECT id FROM pipeline_node:account_group_validate_code_unique LIMIT 1) = [] {
    UPSERT pipeline_node:account_group_validate_code_unique CONTENT {
        name:    "account_group_validate_code_unique",
        fn_name: "fn::validate::account_group_code_unique"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccount_Group_save_validate AND out = pipeline_node:account_group_validate_number_range LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccount_Group_save_validate -> has_node -> pipeline_node:account_group_validate_number_range CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccount_Group_save_validate AND out = pipeline_node:account_group_validate_code_unique LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccount_Group_save_validate -> has_node -> pipeline_node:account_group_validate_code_unique CONTENT { ord: 10, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccount_Group_save_compute AND out = pipeline_node:account_group_compute_classification LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccount_Group_save_compute -> has_node -> pipeline_node:account_group_compute_classification CONTENT { ord: 5, config: {} };
};

