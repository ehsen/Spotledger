-- Financial Statement Version — pipeline wiring

RETURN fn::pipeline::wire_generic("Financial Statement Version", "tabFinancial_Statement_Version", false);

IF (SELECT id FROM pipeline_node:fsv_validate_single_default LIMIT 1) = [] {
    UPSERT pipeline_node:fsv_validate_single_default CONTENT {
        name:    "fsv_validate_single_default",
        fn_name: "fn::validate::fsv_single_default_per_coa"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabFinancial_Statement_Version_save_validate AND out = pipeline_node:fsv_validate_single_default LIMIT 1) = [] {
    RELATE pipeline_stage:tabFinancial_Statement_Version_save_validate -> has_node -> pipeline_node:fsv_validate_single_default CONTENT { ord: 5, config: {} };
};

