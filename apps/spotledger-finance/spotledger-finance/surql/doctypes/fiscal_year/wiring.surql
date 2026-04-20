-- Fiscal Year — pipeline wiring

RETURN fn::pipeline::wire_generic("Fiscal Year", "tabFiscal_Year", false);

IF (SELECT id FROM pipeline_node:fiscal_year_validate_dates LIMIT 1) = [] {
    UPSERT pipeline_node:fiscal_year_validate_dates CONTENT {
        name:    "fiscal_year_validate_dates",
        fn_name: "fn::validate::fiscal_year_dates"
    };
};

IF (SELECT id FROM pipeline_node:fiscal_year_validate_no_overlap LIMIT 1) = [] {
    UPSERT pipeline_node:fiscal_year_validate_no_overlap CONTENT {
        name:    "fiscal_year_validate_no_overlap",
        fn_name: "fn::validate::fiscal_year_no_overlap"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabFiscal_Year_save_validate AND out = pipeline_node:fiscal_year_validate_dates LIMIT 1) = [] {
    RELATE pipeline_stage:tabFiscal_Year_save_validate -> has_node -> pipeline_node:fiscal_year_validate_dates CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabFiscal_Year_save_validate AND out = pipeline_node:fiscal_year_validate_no_overlap LIMIT 1) = [] {
    RELATE pipeline_stage:tabFiscal_Year_save_validate -> has_node -> pipeline_node:fiscal_year_validate_no_overlap CONTENT { ord: 10, config: {} };
};

