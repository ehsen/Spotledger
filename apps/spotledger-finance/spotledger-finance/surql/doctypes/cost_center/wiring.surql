-- Cost Center — pipeline wiring

RETURN fn::pipeline::wire_generic("Cost Center", "tabCost_Center", false);

IF (SELECT id FROM pipeline_node:cost_center_validate_code_unique LIMIT 1) = [] {
    UPSERT pipeline_node:cost_center_validate_code_unique CONTENT {
        name:    "cost_center_validate_code_unique",
        fn_name: "fn::validate::cost_center_code_unique"
    };
};

IF (SELECT id FROM pipeline_node:cost_center_relate_parent LIMIT 1) = [] {
    UPSERT pipeline_node:cost_center_relate_parent CONTENT {
        name:    "cost_center_relate_parent",
        fn_name: "fn::on_save::cost_center_relate_parent"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabCost_Center_save_validate AND out = pipeline_node:cost_center_validate_code_unique LIMIT 1) = [] {
    RELATE pipeline_stage:tabCost_Center_save_validate -> has_node -> pipeline_node:cost_center_validate_code_unique CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabCost_Center_save_persist AND out = pipeline_node:cost_center_relate_parent LIMIT 1) = [] {
    RELATE pipeline_stage:tabCost_Center_save_persist -> has_node -> pipeline_node:cost_center_relate_parent CONTENT { ord: 90, config: {} };
};

