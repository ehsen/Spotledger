-- Party Role Config — pipeline wiring

RETURN fn::pipeline::wire_generic("Party Role Config", "tabParty_Role_Config", false);

IF (SELECT id FROM pipeline_node:party_role_config_validate_unique LIMIT 1) = [] {
    UPSERT pipeline_node:party_role_config_validate_unique CONTENT {
        name:    "party_role_config_validate_unique",
        fn_name: "fn::validate::party_role_config_unique"
    };
};

IF (SELECT id FROM pipeline_node:party_role_config_validate_account_types LIMIT 1) = [] {
    UPSERT pipeline_node:party_role_config_validate_account_types CONTENT {
        name:    "party_role_config_validate_account_types",
        fn_name: "fn::validate::party_role_config_account_types"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabParty_Role_Config_save_validate AND out = pipeline_node:party_role_config_validate_unique LIMIT 1) = [] {
    RELATE pipeline_stage:tabParty_Role_Config_save_validate -> has_node -> pipeline_node:party_role_config_validate_unique CONTENT { ord: 5, config: {} };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabParty_Role_Config_save_validate AND out = pipeline_node:party_role_config_validate_account_types LIMIT 1) = [] {
    RELATE pipeline_stage:tabParty_Role_Config_save_validate -> has_node -> pipeline_node:party_role_config_validate_account_types CONTENT { ord: 10, config: {} };
};

