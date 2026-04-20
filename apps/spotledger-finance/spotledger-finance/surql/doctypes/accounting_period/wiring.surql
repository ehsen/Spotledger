-- Accounting Period — pipeline wiring

RETURN fn::pipeline::wire_generic("Accounting Period", "tabAccounting_Period", false);

IF (SELECT id FROM pipeline_node:accounting_period_validate_dates LIMIT 1) = [] {
    UPSERT pipeline_node:accounting_period_validate_dates CONTENT {
        name:    "accounting_period_validate_dates",
        fn_name: "fn::validate::accounting_period_dates"
    };
};

IF (SELECT id FROM pipeline_node:accounting_period_validate_in_fy LIMIT 1) = [] {
    UPSERT pipeline_node:accounting_period_validate_in_fy CONTENT {
        name:    "accounting_period_validate_in_fy",
        fn_name: "fn::validate::accounting_period_in_fiscal_year"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccounting_Period_save_validate AND out = pipeline_node:accounting_period_validate_dates LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccounting_Period_save_validate -> has_node -> pipeline_node:accounting_period_validate_dates CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccounting_Period_save_validate AND out = pipeline_node:accounting_period_validate_in_fy LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccounting_Period_save_validate -> has_node -> pipeline_node:accounting_period_validate_in_fy CONTENT { ord: 10, config: {} };
};

