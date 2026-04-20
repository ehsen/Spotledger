-- Journal Entry — domain functions
-- Handles balance validation, GL entry creation on submit, and GL reversal on cancel.

DEFINE FUNCTION OVERWRITE fn::validate::je_has_accounts(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT accounts FROM $doc_id)[0];

    IF $doc.accounts = NONE OR array::len($doc.accounts) < 2 {
        RETURN { error: "Journal Entry must have at least 2 account lines" };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::je_account_validity(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT accounts FROM $doc_id)[0];

    IF $doc.accounts = NONE {
        RETURN { ok: true };
    };

    FOR $row IN $doc.accounts {
        IF $row.account = NONE OR $row.account = "" {
            RETURN { error: "All account lines must have an Account selected" };
        };

        LET $acct = (
            SELECT is_group, disabled FROM tabAccount
            WHERE name = $row.account
            LIMIT 1
        )[0];

        IF $acct = NONE {
            RETURN { error: string::concat("Account '", <string>$row.account, "' not found") };
        };

        IF $acct.disabled = true OR $acct.disabled = 1 {
            RETURN { error: string::concat("Account '", <string>$row.account, "' is disabled") };
        };

        IF $acct.is_group = true OR $acct.is_group = 1 {
            RETURN { error: string::concat("Account '", <string>$row.account, "' is a group account and cannot be used in Journal Entries") };
        };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::je_balance(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT total_debit, total_credit FROM $doc_id)[0];

    LET $debit  = $doc.total_debit  OR 0;
    LET $credit = $doc.total_credit OR 0;

    IF $debit != $credit {
        RETURN { error: string::concat("Journal Entry is not balanced. Total Debit: ", <string>$debit, ", Total Credit: ", <string>$credit) };
    };

    IF $debit = 0 {
        RETURN { error: "Journal Entry total cannot be zero" };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::je_period_open(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT company, posting_date FROM $doc_id)[0];

    IF $doc.company = NONE OR $doc.posting_date = NONE {
        RETURN { ok: true };
    };

    LET $period = (
        SELECT name, is_closed FROM tabAccounting_Period
        WHERE company    = $doc.company
          AND start_date <= $doc.posting_date
          AND end_date   >= $doc.posting_date
        LIMIT 1
    )[0];

    IF $period = NONE {
        RETURN { error: "No Accounting Period found for the posting date and company" };
    };

    IF $period.is_closed = true OR $period.is_closed = 1 {
        RETURN { error: string::concat("Accounting Period '", <string>$period.name, "' is closed") };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::compute::je_totals(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT accounts FROM $doc_id)[0];

    IF $doc.accounts = NONE {
        UPDATE $doc_id SET total_debit = 0, total_credit = 0, difference = 0;
        RETURN { ok: true };
    };

    LET $total_debit  = math::sum($doc.accounts[*].debit  ?? [0]);
    LET $total_credit = math::sum($doc.accounts[*].credit ?? [0]);

    UPDATE $doc_id SET
        total_debit  = $total_debit,
        total_credit = $total_credit,
        difference   = $total_debit - $total_credit;

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::on_submit::je_create_gl_entries(
    $doc_id: record,
    $config: object
) {
    LET $doc = (
        SELECT accounts, posting_date, company, voucher_type, user_remark
        FROM $doc_id
    )[0];

    LET $doc_name = <string>record::id($doc_id);

    IF $doc.accounts = NONE OR array::len($doc.accounts) = 0 {
        RETURN { error: "No account lines found on Journal Entry" };
    };

    FOR $row IN $doc.accounts {
        LET $idx = $row.idx OR 0;
        LET $gl_name = string::concat($doc_name, "-GL-", <string>$idx);

        INSERT INTO tabGL_Entry {
            name:                       $gl_name,
            company:                    $doc.company,
            account:                    $row.account,
            posting_date:               $doc.posting_date,
            debit:                      $row.debit  OR 0,
            credit:                     $row.credit OR 0,
            debit_in_account_currency:  $row.debit_in_account_currency  OR 0,
            credit_in_account_currency: $row.credit_in_account_currency OR 0,
            account_currency:           $row.account_currency,
            exchange_rate:              $row.exchange_rate OR 1,
            party:                      $row.party,
            party_role:                 $row.party_role,
            cost_center:                $row.cost_center,
            voucher_type:               $doc.voucher_type OR "Journal Entry",
            voucher_no:                 $doc_name,
            remarks:                    $doc.user_remark,
            docstatus:                  1,
            is_cancelled:               false
        };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::on_cancel::je_cancel_gl_entries(
    $doc_id: record,
    $config: object
) {
    LET $doc_name = <string>record::id($doc_id);

    -- Mark all GL entries for this voucher as cancelled and reverse amounts
    LET $gl_entries = (
        SELECT id, debit, credit, debit_in_account_currency, credit_in_account_currency
        FROM tabGL_Entry
        WHERE voucher_no = $doc_name
          AND (is_cancelled = false OR is_cancelled = 0)
    );

    FOR $gl IN $gl_entries {
        UPDATE $gl.id SET
            debit                      = $gl.credit,
            credit                     = $gl.debit,
            debit_in_account_currency  = $gl.credit_in_account_currency,
            credit_in_account_currency = $gl.debit_in_account_currency,
            is_cancelled               = true,
            docstatus                  = 2;
    };

    RETURN { ok: true };
};
