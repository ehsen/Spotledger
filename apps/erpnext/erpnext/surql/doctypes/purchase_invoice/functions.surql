-- Purchase Invoice — domain functions

-- ── fn::validate::purchase_invoice_duplicate ────────────────────────────────
-- Prevents duplicate supplier bill numbers for the same supplier + date.
DEFINE FUNCTION OVERWRITE fn::validate::purchase_invoice_duplicate(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT supplier, bill_no, bill_date FROM $doc_id)[0];

    IF ($doc.bill_no = NONE OR $doc.bill_no = "") OR ($doc.bill_date = NONE) {
        RETURN { ok: true };
    };

    LET $existing = (
        SELECT id FROM tabPurchase_Invoice
        WHERE supplier  = $doc.supplier
          AND bill_no   = $doc.bill_no
          AND bill_date = $doc.bill_date
          AND id        != $doc_id
          AND docstatus != 2
        LIMIT 1
    )[0];

    IF $existing != NONE {
        RETURN {
            error: string::concat(
                "Duplicate bill. Invoice ",
                <string>record::id($existing.id),
                " already exists for this supplier, bill number and date."
            )
        };
    };

    RETURN { ok: true };
};

-- ── fn::compute::purchase_invoice_totals ─────────────────────────────────────
-- Computes net_total, taxes, grand_total from line items.
DEFINE FUNCTION OVERWRITE fn::compute::purchase_invoice_totals(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT items, tax_rate, conversion_rate, write_off_amount, discount_amount FROM $doc_id)[0];

    LET $net = math::sum($doc.items[*].amount OR 0);
    LET $rate = $doc.tax_rate OR 0.0;
    LET $tax  = math::round($net * ($rate / 100.0) * 100.0) / 100.0;
    LET $conv = $doc.conversion_rate OR 1.0;

    LET $grand = $net
        + $tax
        - ($doc.write_off_amount OR 0.0)
        - ($doc.discount_amount OR 0.0);

    UPDATE $doc_id SET
        net_total               = $net,
        total_taxes_and_charges = $tax,
        base_net_total          = $net * $conv,
        grand_total             = $grand,
        base_grand_total        = $grand * $conv,
        outstanding_amount      = $grand;

    RETURN { ok: true };
};

-- ── fn::compute::purchase_invoice_due_date ───────────────────────────────────
-- Computes due_date from posting_date + payment terms.
DEFINE FUNCTION OVERWRITE fn::compute::purchase_invoice_due_date(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT posting_date, payment_terms_template FROM $doc_id)[0];

    IF $doc.posting_date = NONE {
        RETURN { error: "Posting date is required to compute due date" };
    };

    LET $terms = (
        SELECT credit_days FROM tabPayment_Terms_Template
        WHERE name = $doc.payment_terms_template
        LIMIT 1
    )[0];

    LET $days     = $terms.credit_days OR 30;
    -- SurrealDB v3: add days via string-to-duration conversion
    LET $dur_str  = string::concat(<string> $days, "d");
    LET $due_date = $doc.posting_date + type::duration($dur_str);

    UPDATE $doc_id SET due_date = $due_date;

    RETURN { ok: true };
};

-- ── fn::on_submit::purchase_invoice_gl ───────────────────────────────────────
-- Creates GL entries (debit expense accounts, credit payable account).
-- Called during the on_submit stage of the submit action pipeline.
DEFINE FUNCTION OVERWRITE fn::on_submit::purchase_invoice_gl(
    $doc_id: record,
    $config: object
) {
    LET $doc = (
        SELECT items, taxes_and_charges, total_taxes_and_charges,
               credit_to, supplier, posting_date, company, remarks,
               base_grand_total, grand_total, conversion_rate,
               taxes_and_charges_account
        FROM $doc_id
    )[0];

    LET $doc_name = record::id($doc_id);

    IF $doc.items = NONE OR array::len($doc.items) = 0 {
        RETURN { error: "Cannot create GL entries: no line items" };
    };

    -- Debit each expense account
    FOR $item IN $doc.items {
        INSERT INTO tabGL_Entry {
            name:         string::concat($doc_name, "-gl-exp-", <string>$item.idx OR 0),
            account:      $item.expense_account,
            debit:        $item.base_amount OR $item.amount OR 0,
            credit:       0,
            voucher_type: "Purchase Invoice",
            voucher_no:   $doc_name,
            posting_date: $doc.posting_date,
            company:      $doc.company,
            remarks:      $doc.remarks,
            cost_center:  $item.cost_center,
            project:      $item.project,
            docstatus:    1
        };
    };

    -- Debit tax account if taxes exist
    IF ($doc.total_taxes_and_charges OR 0) > 0 AND $doc.taxes_and_charges_account != NONE {
        INSERT INTO tabGL_Entry {
            name:         string::concat($doc_name, "-gl-tax"),
            account:      $doc.taxes_and_charges_account,
            debit:        $doc.total_taxes_and_charges,
            credit:       0,
            voucher_type: "Purchase Invoice",
            voucher_no:   $doc_name,
            posting_date: $doc.posting_date,
            company:      $doc.company,
            docstatus:    1
        };
    };

    -- Credit payable account
    INSERT INTO tabGL_Entry {
        name:         string::concat($doc_name, "-gl-payable"),
        account:      $doc.credit_to,
        debit:        0,
        credit:       $doc.base_grand_total OR $doc.grand_total OR 0,
        voucher_type: "Purchase Invoice",
        voucher_no:   $doc_name,
        posting_date: $doc.posting_date,
        company:      $doc.company,
        party:        $doc.supplier,
        party_type:   "Supplier",
        docstatus:    1
    };

    -- Verify balance (debit must equal credit within tolerance)
    LET $entries     = (SELECT debit, credit FROM tabGL_Entry WHERE voucher_no = $doc_name AND docstatus = 1);
    LET $ttl_debit   = math::sum($entries[*].debit  OR 0);
    LET $ttl_credit  = math::sum($entries[*].credit OR 0);

    IF math::abs($ttl_debit - $ttl_credit) > 0.01 {
        RETURN {
            error: string::concat(
                "GL imbalance: debit=", <string>$ttl_debit,
                " credit=", <string>$ttl_credit
            )
        };
    };

    -- Mark submitted
    UPDATE $doc_id SET
        docstatus          = 1,
        status             = "Unpaid",
        outstanding_amount = $doc.grand_total;

    RETURN { ok: true };
};
