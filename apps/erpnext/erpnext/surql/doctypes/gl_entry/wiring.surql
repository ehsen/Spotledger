-- GL Entry pipeline wiring
--
-- GL Entry is submittable (its parent creates and cancels it).
-- Cancel does NOT use the generic mark_cancelled — instead it reverses
-- the debit/credit amounts so the ledger stays balanced.
--
-- Stage layout:
--   save   / validate       → mandatory_fields(1), gl_entry_validate_accounts(5)
--   save   / persist
--   submit / validate       → mandatory_fields(1)
--   submit / compute
--   submit / persist
--   submit / on_submit
--   cancel / validate_cancel → no_dependents(1)
--   cancel / on_cancel       → gl_entry_on_cancel_reverse(1)  ← replaces mark_cancelled
--
-- Cascade rules: Journal Entry, Purchase Invoice, Payment Entry all cascade
-- their cancel to this doctype via voucher_no.

-- ── 1. Generic stage scaffold ─────────────────────────────────────────────────
RETURN fn::pipeline::wire_generic("GL Entry", "gl_entry", true);

-- ── 2. GL Entry specific pipeline nodes ──────────────────────────────────────
IF (SELECT id FROM pipeline_node:gl_entry_validate_accounts LIMIT 1) = [] {
    UPSERT pipeline_node:gl_entry_validate_accounts CONTENT {
        name: "gl_entry_validate_accounts", fn_name: "fn::validate::gl_entry_accounts"
    };
};
IF (SELECT id FROM pipeline_node:gl_entry_on_cancel_reverse LIMIT 1) = [] {
    UPSERT pipeline_node:gl_entry_on_cancel_reverse CONTENT {
        name: "gl_entry_on_cancel_reverse", fn_name: "fn::on_cancel::gl_entry_reverse"
    };
};

-- ── 3. Wire validate_accounts into save/validate ──────────────────────────────
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:gl_entry_save_validate
          AND out = pipeline_node:gl_entry_validate_accounts
        LIMIT 1) = [] {
    RELATE pipeline_stage:gl_entry_save_validate -> has_node -> pipeline_node:gl_entry_validate_accounts
        CONTENT { ord: 5, config: {} };
};

-- ── 4. Replace generic mark_cancelled with GL-specific reversal ───────────────
-- The generic mark_cancelled (applied by wire_generic at ord=99) must be removed.
-- gl_entry_on_cancel_reverse sets is_cancelled=true AND reverses amounts.
DELETE has_node
    WHERE in  = pipeline_stage:gl_entry_cancel_on_cancel
      AND out = pipeline_node:on_cancel_mark_cancelled;

IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:gl_entry_cancel_on_cancel
          AND out = pipeline_node:gl_entry_on_cancel_reverse
        LIMIT 1) = [] {
    RELATE pipeline_stage:gl_entry_cancel_on_cancel -> has_node -> pipeline_node:gl_entry_on_cancel_reverse
        CONTENT { ord: 1, config: {} };
};

-- ── 5. Cascade rules ──────────────────────────────────────────────────────────
-- When any of these parent doctypes are cancelled, matching GL Entry rows are
-- cancelled automatically by fn::pipeline::cascade_cancel.
UPSERT cascade_rule:journal_entry_gl_entry CONTENT {
    parent_doctype: "Journal Entry",   child_doctype: "GL Entry", find_by_field: "voucher_no"
};
UPSERT cascade_rule:purchase_invoice_gl_entry CONTENT {
    parent_doctype: "Purchase Invoice", child_doctype: "GL Entry", find_by_field: "voucher_no"
};
UPSERT cascade_rule:payment_entry_gl_entry CONTENT {
    parent_doctype: "Payment Entry",   child_doctype: "GL Entry", find_by_field: "voucher_no"
};
