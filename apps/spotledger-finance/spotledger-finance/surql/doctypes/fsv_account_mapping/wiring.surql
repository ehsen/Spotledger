-- FSV Account Mapping — pipeline wiring

RETURN fn::pipeline::wire_generic("FSV Account Mapping", "tabFSV_Account_Mapping", false);

IF (SELECT id FROM pipeline_node:fsv_mapping_validate_range LIMIT 1) = [] {
    UPSERT pipeline_node:fsv_mapping_validate_range CONTENT {
        name:    "fsv_mapping_validate_range",
        fn_name: "fn::validate::fsv_mapping_range"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabFSV_Account_Mapping_save_validate AND out = pipeline_node:fsv_mapping_validate_range LIMIT 1) = [] {
    RELATE pipeline_stage:tabFSV_Account_Mapping_save_validate -> has_node -> pipeline_node:fsv_mapping_validate_range CONTENT { ord: 5, config: {} };
};

