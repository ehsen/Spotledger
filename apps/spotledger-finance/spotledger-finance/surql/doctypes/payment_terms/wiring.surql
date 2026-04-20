-- Payment Terms — pipeline wiring

RETURN fn::pipeline::wire_generic("Payment Terms", "tabPayment_Terms", false);

IF (SELECT id FROM pipeline_node:payment_terms_validate_days LIMIT 1) = [] {
    UPSERT pipeline_node:payment_terms_validate_days CONTENT {
        name:    "payment_terms_validate_days",
        fn_name: "fn::validate::payment_terms_days"
    };
};

IF (SELECT id FROM has_node WHERE in = pipeline_stage:tabPayment_Terms_save_validate AND out = pipeline_node:payment_terms_validate_days LIMIT 1) = [] {
    RELATE pipeline_stage:tabPayment_Terms_save_validate -> has_node -> pipeline_node:payment_terms_validate_days CONTENT { ord: 5, config: {} };
};

