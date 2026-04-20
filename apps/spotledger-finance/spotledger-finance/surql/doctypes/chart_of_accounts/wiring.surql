-- Chart of Accounts — pipeline wiring

RETURN fn::pipeline::wire_generic("Chart of Accounts", "tabChart_of_Accounts", false);

IF (SELECT id FROM pipeline_node:coa_validate_parent_required LIMIT 1) = [] {
    UPSERT pipeline_node:coa_validate_parent_required CONTENT {
        name:    "coa_validate_parent_required",
        fn_name: "fn::validate::coa_parent_required"
    };
};

IF (SELECT id FROM pipeline_node:coa_validate_parent_type LIMIT 1) = [] {
    UPSERT pipeline_node:coa_validate_parent_type CONTENT {
        name:    "coa_validate_parent_type",
        fn_name: "fn::validate::coa_parent_type"
    };
};

IF (SELECT id FROM pipeline_node:coa_validate_single_default LIMIT 1) = [] {
    UPSERT pipeline_node:coa_validate_single_default CONTENT {
        name:    "coa_validate_single_default",
        fn_name: "fn::validate::coa_single_default"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabChart_of_Accounts_save_validate AND out = pipeline_node:coa_validate_parent_required LIMIT 1) = [] {
    RELATE pipeline_stage:tabChart_of_Accounts_save_validate -> has_node -> pipeline_node:coa_validate_parent_required CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabChart_of_Accounts_save_validate AND out = pipeline_node:coa_validate_parent_type LIMIT 1) = [] {
    RELATE pipeline_stage:tabChart_of_Accounts_save_validate -> has_node -> pipeline_node:coa_validate_parent_type CONTENT { ord: 10, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabChart_of_Accounts_save_validate AND out = pipeline_node:coa_validate_single_default LIMIT 1) = [] {
    RELATE pipeline_stage:tabChart_of_Accounts_save_validate -> has_node -> pipeline_node:coa_validate_single_default CONTENT { ord: 15, config: {} };
};

