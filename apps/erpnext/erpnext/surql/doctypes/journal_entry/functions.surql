-- Journal Entry — domain functions

-- ── fn::validate::journal_entry_accounts ─────────────────────────────────────
-- Validates the accounts table (multi-currency, balance, account existence).
DEFINE FUNCTION OVERWRITE fn::validate::journal_entry_accounts(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT accounts, company FROM $doc_id)[0];

    IF $doc.accounts = NONE OR array::len($doc.accounts) = 0 {
        RETURN { error: "Journal Entry must have at least one account row" };
    };

    FOR $row IN $doc.accounts {
        IF $row.account = NONE OR $row.account = "" {
            RETURN { error: "Account is required on each row" };
        };

        LET $acc = (SELECT is_group, disabled FROM tabAccount WHERE name = $row.account LIMIT 1)[0];

        IF $acc = NONE {
            RETURN { error: string::concat("Account does not exist: ", <string>$row.account) };
        };

        IF $acc.disabled = true OR $acc.disabled = 1 {
            RETURN { error: string::concat("Account is disabled: ", <string>$row.account) };
        };

        IF $acc.is_group = true OR $acc.is_group = 1 {
            RETURN { error: string::concat("Cannot post to a group account: ", <string>$row.account) };
        };
    };

    RETURN { ok: true };
};

-- ── fn::validate::journal_entry_balance ──────────────────────────────────────
-- Total debits must equal total credits (in company currency).
DEFINE FUNCTION OVERWRITE fn::validate::journal_entry_balance(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT accounts FROM $doc_id)[0];

    LET $total_debit  = math::sum($doc.accounts[*].debit  OR 0);
    LET $total_credit = math::sum($doc.accounts[*].credit OR 0);

    IF math::abs($total_debit - $total_credit) > 0.005 {
        RETURN {
            error: string::concat(
                "Debits and credits must balance. Debit: ",
                <string>math::round($total_debit * 100) / 100,
                ", Credit: ",
                <string>math::round($total_credit * 100) / 100
            )
        };
    };

    RETURN { ok: true };
};

-- ── fn::on_submit::journal_entry_gl ──────────────────────────────────────────
-- Creates GL entries for each account row on Journal Entry submit.
-- Each GL entry runs through its own save pipeline (if wired) for validation.
DEFINE FUNCTION OVERWRITE fn::on_submit::journal_entry_gl(
    $doc_id: record,
    $config: object
) {
    LET $doc = (
        SELECT accounts, posting_date, company, voucher_type, remarks, finance_book
        FROM $doc_id
    )[0];

    LET $voucher_type = $doc.voucher_type OR "Journal Entry";
    LET $doc_name     = record::id($doc_id);

    FOR $row IN $doc.accounts {
        INSERT INTO tabGL_Entry {
            name:         string::concat($doc_name, "-gl-", <string>$row.idx OR 0),
            account:      $row.account,
            debit:        $row.debit  OR 0,
            credit:       $row.credit OR 0,
            debit_in_account_currency:  $row.debit_in_account_currency  OR 0,
            credit_in_account_currency: $row.credit_in_account_currency OR 0,
            voucher_type:      $voucher_type,
            voucher_no:        $doc_name,
            posting_date:      $doc.posting_date,
            company:           $doc.company,
            remarks:           $doc.remarks,
            finance_book:      $doc.finance_book,
            party:             $row.party,
            party_type:        $row.party_type,
            cost_center:       $row.cost_center,
            project:           $row.project,
            against_voucher:   $row.reference_name,
            against_voucher_type: $row.reference_type,
            docstatus:         1,
            is_opening:        $row.is_opening OR "No",
            fiscal_year:       string::slice(<string>$doc.posting_date, 0, 4)
        };
    };

    -- Mark the Journal Entry as submitted
    UPDATE $doc_id SET docstatus = 1;

    RETURN { ok: true };
};
