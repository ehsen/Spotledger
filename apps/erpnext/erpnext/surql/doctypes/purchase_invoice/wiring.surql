-- Purchase Invoice pipeline wiring
--
-- Equivalent to purchase_invoice.py: validate() + calculate_taxes_and_totals()
--                                    + on_submit()
--
-- Stage layout:
--   save   / validate       → mandatory_fields(1), duplicate_check(30)
--   save   / compute        → totals(1), due_date(2)
--   save   / persist
--   submit / validate       → mandatory_fields(1), duplicate_check(30)
--   submit / compute        → totals(1), due_date(2)
--   submit / persist
--   submit / on_submit      → purchase_invoice_gl(1)
--   cancel / validate_cancel → no_dependents(1)
--   cancel / on_cancel       → mark_cancelled(99)

-- ── 1. Generic stage scaffold ─────────────────────────────────────────────────
RETURN fn::pipeline::wire_generic("Purchase Invoice", "purchase_invoice", true);

-- ── 2. Purchase Invoice specific nodes ───────────────────────────────────────
IF (SELECT id FROM pipeline_node:purchase_invoice_duplicate LIMIT 1) = [] {
    UPSERT pipeline_node:purchase_invoice_duplicate CONTENT {
        name: "purchase_invoice_duplicate", fn_name: "fn::validate::purchase_invoice_duplicate"
    };
};
IF (SELECT id FROM pipeline_node:purchase_invoice_totals LIMIT 1) = [] {
    UPSERT pipeline_node:purchase_invoice_totals CONTENT {
        name: "purchase_invoice_totals", fn_name: "fn::compute::purchase_invoice_totals"
    };
};
IF (SELECT id FROM pipeline_node:purchase_invoice_due_date LIMIT 1) = [] {
    UPSERT pipeline_node:purchase_invoice_due_date CONTENT {
        name: "purchase_invoice_due_date", fn_name: "fn::compute::purchase_invoice_due_date"
    };
};
IF (SELECT id FROM pipeline_node:purchase_invoice_gl LIMIT 1) = [] {
    UPSERT pipeline_node:purchase_invoice_gl CONTENT {
        name: "purchase_invoice_gl", fn_name: "fn::on_submit::purchase_invoice_gl"
    };
};

-- ── 3. Also wire party_check for supplier field ───────────────────────────────
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:purchase_invoice_save_validate
          AND out = pipeline_node:validate_party_check
        LIMIT 1) = [] {
    RELATE pipeline_stage:purchase_invoice_save_validate -> has_node -> pipeline_node:validate_party_check
        CONTENT { ord: 10, config: { field: "supplier", party_type: "Supplier" } };
};
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:purchase_invoice_submit_validate
          AND out = pipeline_node:validate_party_check
        LIMIT 1) = [] {
    RELATE pipeline_stage:purchase_invoice_submit_validate -> has_node -> pipeline_node:validate_party_check
        CONTENT { ord: 10, config: { field: "supplier", party_type: "Supplier" } };
};

-- ── 4. Wire duplicate_check into save/validate and submit/validate ────────────
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:purchase_invoice_save_validate
          AND out = pipeline_node:purchase_invoice_duplicate
        LIMIT 1) = [] {
    RELATE pipeline_stage:purchase_invoice_save_validate -> has_node -> pipeline_node:purchase_invoice_duplicate
        CONTENT { ord: 30, config: {} };
};
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:purchase_invoice_submit_validate
          AND out = pipeline_node:purchase_invoice_duplicate
        LIMIT 1) = [] {
    RELATE pipeline_stage:purchase_invoice_submit_validate -> has_node -> pipeline_node:purchase_invoice_duplicate
        CONTENT { ord: 30, config: {} };
};

-- ── 5. Wire totals + due_date into save/compute and submit/compute ────────────
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:purchase_invoice_save_compute
          AND out = pipeline_node:purchase_invoice_totals
        LIMIT 1) = [] {
    RELATE pipeline_stage:purchase_invoice_save_compute -> has_node -> pipeline_node:purchase_invoice_totals
        CONTENT { ord: 1, config: {} };
};
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:purchase_invoice_save_compute
          AND out = pipeline_node:purchase_invoice_due_date
        LIMIT 1) = [] {
    RELATE pipeline_stage:purchase_invoice_save_compute -> has_node -> pipeline_node:purchase_invoice_due_date
        CONTENT { ord: 2, config: {} };
};
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:purchase_invoice_submit_compute
          AND out = pipeline_node:purchase_invoice_totals
        LIMIT 1) = [] {
    RELATE pipeline_stage:purchase_invoice_submit_compute -> has_node -> pipeline_node:purchase_invoice_totals
        CONTENT { ord: 1, config: {} };
};
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:purchase_invoice_submit_compute
          AND out = pipeline_node:purchase_invoice_due_date
        LIMIT 1) = [] {
    RELATE pipeline_stage:purchase_invoice_submit_compute -> has_node -> pipeline_node:purchase_invoice_due_date
        CONTENT { ord: 2, config: {} };
};

-- ── 6. Wire GL posting into submit/on_submit ──────────────────────────────────
IF (SELECT id FROM has_node
        WHERE in  = pipeline_stage:purchase_invoice_submit_on_submit
          AND out = pipeline_node:purchase_invoice_gl
        LIMIT 1) = [] {
    RELATE pipeline_stage:purchase_invoice_submit_on_submit -> has_node -> pipeline_node:purchase_invoice_gl
        CONTENT { ord: 1, config: {} };
};
