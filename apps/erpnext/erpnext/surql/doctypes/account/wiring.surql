-- Account pipeline wiring
--
-- Equivalent to account.py: validate() + set_root_and_report_type()
--
-- Stage layout after this file runs:
--   save / validate  (ord=1) → mandatory_fields(1), parent_check(20),
--                               root_details(21), number_unique(22),
--                               parent_child_type(23), group_or_ledger(24),
--                               currency(25)
--   save / compute   (ord=2) → account_root_type(1)
--   save / persist   (ord=3)
--
-- Account is not submittable — no submit/cancel stages.

-- ── 1. Generic stage scaffold + mandatory_fields wiring ──────────────────────
RETURN fn::pipeline::wire_generic("Account", "account", false);

-- ── 2. Account-specific pipeline nodes ───────────────────────────────────────
IF (SELECT id FROM pipeline_node:account_parent_check LIMIT 1) = [] {
    UPSERT pipeline_node:account_parent_check CONTENT {
        name: "account_parent_check", fn_name: "fn::validate::account_parent_check"
    };
};
IF (SELECT id FROM pipeline_node:account_root_details LIMIT 1) = [] {
    UPSERT pipeline_node:account_root_details CONTENT {
        name: "account_root_details", fn_name: "fn::validate::account_root_details"
    };
};
IF (SELECT id FROM pipeline_node:account_number_unique LIMIT 1) = [] {
    UPSERT pipeline_node:account_number_unique CONTENT {
        name: "account_number_unique", fn_name: "fn::validate::account_number_unique"
    };
};
IF (SELECT id FROM pipeline_node:account_parent_child_type LIMIT 1) = [] {
    UPSERT pipeline_node:account_parent_child_type CONTENT {
        name: "account_parent_child_type", fn_name: "fn::validate::account_parent_child_type"
    };
};
IF (SELECT id FROM pipeline_node:account_group_or_ledger LIMIT 1) = [] {
    UPSERT pipeline_node:account_group_or_ledger CONTENT {
        name: "account_group_or_ledger", fn_name: "fn::validate::account_group_or_ledger"
    };
};
IF (SELECT id FROM pipeline_node:account_currency LIMIT 1) = [] {
    UPSERT pipeline_node:account_currency CONTENT {
        name: "account_currency", fn_name: "fn::validate::account_currency"
    };
};
IF (SELECT id FROM pipeline_node:account_root_type LIMIT 1) = [] {
    UPSERT pipeline_node:account_root_type CONTENT {
        name: "account_root_type", fn_name: "fn::compute::account_root_type"
    };
};

-- ── 3. Insert compute stage between validate (ord=1) and persist ──────────────
-- wire_generic created persist at ord=2; shift it to ord=3 to make room.
IF (SELECT id FROM pipeline_stage:account_save_compute LIMIT 1) = [] {
    UPSERT pipeline_stage:account_save_compute CONTENT {
        name: "compute", ord: 2, doctype: "Account", action: "save"
    };
};
UPDATE pipeline_stage SET ord = 3
    WHERE doctype = "Account" AND action = "save" AND name = "persist";

-- ── 4. Wire validators into save/validate ────────────────────────────────────
-- Ordinals continue from mandatory_fields (ord=1); 20-25 reserved for account.
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:account_save_validate
          AND out = pipeline_node:account_parent_check
        LIMIT 1) = [] {
    RELATE pipeline_stage:account_save_validate -> has_node -> pipeline_node:account_parent_check
        CONTENT { ord: 20, config: {} };
};
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:account_save_validate
          AND out = pipeline_node:account_root_details
        LIMIT 1) = [] {
    RELATE pipeline_stage:account_save_validate -> has_node -> pipeline_node:account_root_details
        CONTENT { ord: 21, config: {} };
};
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:account_save_validate
          AND out = pipeline_node:account_number_unique
        LIMIT 1) = [] {
    RELATE pipeline_stage:account_save_validate -> has_node -> pipeline_node:account_number_unique
        CONTENT { ord: 22, config: {} };
};
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:account_save_validate
          AND out = pipeline_node:account_parent_child_type
        LIMIT 1) = [] {
    RELATE pipeline_stage:account_save_validate -> has_node -> pipeline_node:account_parent_child_type
        CONTENT { ord: 23, config: {} };
};
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:account_save_validate
          AND out = pipeline_node:account_group_or_ledger
        LIMIT 1) = [] {
    RELATE pipeline_stage:account_save_validate -> has_node -> pipeline_node:account_group_or_ledger
        CONTENT { ord: 24, config: {} };
};
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:account_save_validate
          AND out = pipeline_node:account_currency
        LIMIT 1) = [] {
    RELATE pipeline_stage:account_save_validate -> has_node -> pipeline_node:account_currency
        CONTENT { ord: 25, config: {} };
};

-- ── 5. Wire account_root_type into save/compute ───────────────────────────────
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:account_save_compute
          AND out = pipeline_node:account_root_type
        LIMIT 1) = [] {
    RELATE pipeline_stage:account_save_compute -> has_node -> pipeline_node:account_root_type
        CONTENT { ord: 1, config: {} };
};
