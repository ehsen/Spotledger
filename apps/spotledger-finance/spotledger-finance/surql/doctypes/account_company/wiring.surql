-- Account Company — pipeline wiring

RETURN fn::pipeline::wire_generic("Account Company", "tabAccount_Company", false);

IF (SELECT id FROM pipeline_node:account_company_validate_unique LIMIT 1) = [] {
    UPSERT pipeline_node:account_company_validate_unique CONTENT {
        name:    "account_company_validate_unique",
        fn_name: "fn::validate::account_company_unique"
    };
};

IF (SELECT id FROM pipeline_node:account_company_compute_open_item LIMIT 1) = [] {
    UPSERT pipeline_node:account_company_compute_open_item CONTENT {
        name:    "account_company_compute_open_item",
        fn_name: "fn::compute::account_company_derive_open_item"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccount_Company_save_validate AND out = pipeline_node:account_company_validate_unique LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccount_Company_save_validate -> has_node -> pipeline_node:account_company_validate_unique CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabAccount_Company_save_compute AND out = pipeline_node:account_company_compute_open_item LIMIT 1) = [] {
    RELATE pipeline_stage:tabAccount_Company_save_compute -> has_node -> pipeline_node:account_company_compute_open_item CONTENT { ord: 5, config: {} };
};

