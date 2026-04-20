-- Currency — pipeline wiring

RETURN fn::pipeline::wire_generic("Currency", "tabCurrency", false);

IF (SELECT id FROM pipeline_node:currency_validate_code_format LIMIT 1) = [] {
    UPSERT pipeline_node:currency_validate_code_format CONTENT {
        name:    "currency_validate_code_format",
        fn_name: "fn::validate::currency_code_format"
    };
};

IF (SELECT id FROM pipeline_node:currency_validate_single_base LIMIT 1) = [] {
    UPSERT pipeline_node:currency_validate_single_base CONTENT {
        name:    "currency_validate_single_base",
        fn_name: "fn::validate::currency_single_base"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabCurrency_save_validate AND out = pipeline_node:currency_validate_code_format LIMIT 1) = [] {
    RELATE pipeline_stage:tabCurrency_save_validate -> has_node -> pipeline_node:currency_validate_code_format CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabCurrency_save_validate AND out = pipeline_node:currency_validate_single_base LIMIT 1) = [] {
    RELATE pipeline_stage:tabCurrency_save_validate -> has_node -> pipeline_node:currency_validate_single_base CONTENT { ord: 10, config: {} };
};

