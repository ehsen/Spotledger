-- Party — pipeline wiring

RETURN fn::pipeline::wire_generic("Party", "tabParty", false);

IF (SELECT id FROM pipeline_node:party_validate_roles LIMIT 1) = [] {
    UPSERT pipeline_node:party_validate_roles CONTENT {
        name:    "party_validate_roles",
        fn_name: "fn::validate::party_roles"
    };
};

IF (SELECT id FROM pipeline_node:party_validate_tax_id_unique LIMIT 1) = [] {
    UPSERT pipeline_node:party_validate_tax_id_unique CONTENT {
        name:    "party_validate_tax_id_unique",
        fn_name: "fn::validate::party_tax_id_unique"
    };
};

IF (SELECT id FROM pipeline_node:party_role_config_autocreate LIMIT 1) = [] {
    UPSERT pipeline_node:party_role_config_autocreate CONTENT {
        name:    "party_role_config_autocreate",
        fn_name: "fn::on_save::party_role_config_autocreate"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabParty_save_validate AND out = pipeline_node:party_validate_roles LIMIT 1) = [] {
    RELATE pipeline_stage:tabParty_save_validate -> has_node -> pipeline_node:party_validate_roles CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabParty_save_validate AND out = pipeline_node:party_validate_tax_id_unique LIMIT 1) = [] {
    RELATE pipeline_stage:tabParty_save_validate -> has_node -> pipeline_node:party_validate_tax_id_unique CONTENT { ord: 10, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabParty_save_persist AND out = pipeline_node:party_role_config_autocreate LIMIT 1) = [] {
    RELATE pipeline_stage:tabParty_save_persist -> has_node -> pipeline_node:party_role_config_autocreate CONTENT { ord: 90, config: {} };
};

