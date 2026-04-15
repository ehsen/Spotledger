-- Journal Entry pipeline wiring
--
-- Equivalent to journal_entry.py: validate() + on_submit()
--
-- Stage layout:
--   save   / validate       → mandatory_fields(1), validate_accounts(20), validate_balance(21)
--   save   / persist
--   submit / validate       → mandatory_fields(1), validate_accounts(20), validate_balance(21)
--   submit / compute
--   submit / persist
--   submit / on_submit      → journal_gl(1)
--   cancel / validate_cancel → no_dependents(1)
--   cancel / on_cancel       → mark_cancelled(99)

-- ── 1. Generic stage scaffold ─────────────────────────────────────────────────
RETURN fn::pipeline::wire_generic("Journal Entry", "journal_entry", true);

-- ── 2. Journal Entry specific pipeline nodes ──────────────────────────────────
IF (SELECT id FROM pipeline_node:journal_validate_accounts LIMIT 1) = [] {
    UPSERT pipeline_node:journal_validate_accounts CONTENT {
        name: "journal_validate_accounts", fn_name: "fn::validate::journal_entry_accounts"
    };
};
IF (SELECT id FROM pipeline_node:journal_validate_balance LIMIT 1) = [] {
    UPSERT pipeline_node:journal_validate_balance CONTENT {
        name: "journal_validate_balance", fn_name: "fn::validate::journal_entry_balance"
    };
};
IF (SELECT id FROM pipeline_node:journal_gl LIMIT 1) = [] {
    UPSERT pipeline_node:journal_gl CONTENT {
        name: "journal_gl", fn_name: "fn::on_submit::journal_entry_gl"
    };
};

-- ── 3. Wire validators into save/validate and submit/validate ─────────────────
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:journal_entry_save_validate
          AND out = pipeline_node:journal_validate_accounts
        LIMIT 1) = [] {
    RELATE pipeline_stage:journal_entry_save_validate -> has_node -> pipeline_node:journal_validate_accounts
        CONTENT { ord: 20, config: {} };
};
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:journal_entry_save_validate
          AND out = pipeline_node:journal_validate_balance
        LIMIT 1) = [] {
    RELATE pipeline_stage:journal_entry_save_validate -> has_node -> pipeline_node:journal_validate_balance
        CONTENT { ord: 21, config: {} };
};

IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:journal_entry_submit_validate
          AND out = pipeline_node:journal_validate_accounts
        LIMIT 1) = [] {
    RELATE pipeline_stage:journal_entry_submit_validate -> has_node -> pipeline_node:journal_validate_accounts
        CONTENT { ord: 20, config: {} };
};
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:journal_entry_submit_validate
          AND out = pipeline_node:journal_validate_balance
        LIMIT 1) = [] {
    RELATE pipeline_stage:journal_entry_submit_validate -> has_node -> pipeline_node:journal_validate_balance
        CONTENT { ord: 21, config: {} };
};

-- ── 4. Wire journal_gl into submit/on_submit ──────────────────────────────────
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:journal_entry_submit_on_submit
          AND out = pipeline_node:journal_gl
        LIMIT 1) = [] {
    RELATE pipeline_stage:journal_entry_submit_on_submit -> has_node -> pipeline_node:journal_gl
        CONTENT { ord: 1, config: {} };
};
