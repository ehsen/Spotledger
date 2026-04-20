-- Casual Party — pipeline wiring

RETURN fn::pipeline::wire_generic("Casual Party", "tabCasual_Party", false);

IF (SELECT id FROM pipeline_node:casual_party_validate_not_duplicate LIMIT 1) = [] {
    UPSERT pipeline_node:casual_party_validate_not_duplicate CONTENT {
        name:    "casual_party_validate_not_duplicate",
        fn_name: "fn::validate::casual_party_not_duplicate"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabCasual_Party_save_validate AND out = pipeline_node:casual_party_validate_not_duplicate LIMIT 1) = [] {
    RELATE pipeline_stage:tabCasual_Party_save_validate -> has_node -> pipeline_node:casual_party_validate_not_duplicate CONTENT { ord: 5, config: {} };
};

