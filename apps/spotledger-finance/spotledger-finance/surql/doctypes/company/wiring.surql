-- Company — pipeline wiring

RETURN fn::pipeline::wire_generic("Company", "tabCompany", false);

IF (SELECT id FROM pipeline_node:company_validate_abbr_format LIMIT 1) = [] {
    UPSERT pipeline_node:company_validate_abbr_format CONTENT {
        name:    "company_validate_abbr_format",
        fn_name: "fn::validate::company_abbr_format"
    };
};

IF (SELECT id FROM pipeline_node:company_validate_abbr_unique LIMIT 1) = [] {
    UPSERT pipeline_node:company_validate_abbr_unique CONTENT {
        name:    "company_validate_abbr_unique",
        fn_name: "fn::validate::company_abbr_unique"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabCompany_save_validate AND out = pipeline_node:company_validate_abbr_format LIMIT 1) = [] {
    RELATE pipeline_stage:tabCompany_save_validate -> has_node -> pipeline_node:company_validate_abbr_format CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabCompany_save_validate AND out = pipeline_node:company_validate_abbr_unique LIMIT 1) = [] {
    RELATE pipeline_stage:tabCompany_save_validate -> has_node -> pipeline_node:company_validate_abbr_unique CONTENT { ord: 10, config: {} };
};

