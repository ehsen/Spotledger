-- FSV Node — pipeline wiring

RETURN fn::pipeline::wire_generic("FSV Node", "tabFSV_Node", false);

IF (SELECT id FROM pipeline_node:fsv_node_validate_sequence_unique LIMIT 1) = [] {
    UPSERT pipeline_node:fsv_node_validate_sequence_unique CONTENT {
        name:    "fsv_node_validate_sequence_unique",
        fn_name: "fn::validate::fsv_node_sequence_unique"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabFSV_Node_save_validate AND out = pipeline_node:fsv_node_validate_sequence_unique LIMIT 1) = [] {
    RELATE pipeline_stage:tabFSV_Node_save_validate -> has_node -> pipeline_node:fsv_node_validate_sequence_unique CONTENT { ord: 5, config: {} };
};

