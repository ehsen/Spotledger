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

    IF $doc.accounts = NONE OR array::len($doc.accounts) = 0 {
        UPDATE $doc_id SET total_debit = 0, total_credit = 0, difference = 0;
        RETURN { ok: true };
    };

    -- array::map applies |$v| $v ?? 0 to every element so math::sum never
    -- receives a NULL.  For single-currency JEs the user fills
    -- debit_in_account_currency / credit_in_account_currency; the company-
    -- currency fields (debit / credit) are absent (NONE) until GL posting.
    -- We prefer the company-currency field when present and non-zero
    -- (multi-currency case), otherwise fall back to account-currency amount.
    LET $d_arr = array::map($doc.accounts, |$row| {
        LET $d = IF $row.debit != NONE AND $row.debit != 0
                 THEN $row.debit
                 ELSE $row.debit_in_account_currency END;
        RETURN $d ?? 0;
    });

    LET $c_arr = array::map($doc.accounts, |$row| {
        LET $c = IF $row.credit != NONE AND $row.credit != 0
                 THEN $row.credit
                 ELSE $row.credit_in_account_currency END;
        RETURN $c ?? 0;
    });

    LET $total_debit  = math::sum($d_arr);
    LET $total_credit = math::sum($c_arr);

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
        SELECT accounts, posting_date, company, voucher_type, user_remark, owner, modified_by
        FROM $doc_id
    )[0];

    LET $doc_name = <string>record::id($doc_id);

    -- Remove any GL entries from previous failed submit attempts (idempotency)
    DELETE tabGL_Entry WHERE voucher_no = $doc_name
        OR string::starts_with(name ?? "", $doc_name);

    IF $doc.accounts = NONE OR array::len($doc.accounts) = 0 {
        RETURN { error: "No account lines found on Journal Entry" };
    };

    FOR $row IN $doc.accounts {
        LET $row_key = <string>($row._pk ?? $row.idx ?? 0);
        LET $gl_name = string::concat($doc_name, "-GL-", $row_key);

        INSERT INTO tabGL_Entry {
            name:                       $gl_name,
            doctype:                    "GL Entry",
            owner:                      $doc.owner,
            modified_by:                $doc.modified_by,
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
            is_cancelled:               0
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
            is_cancelled               = 1,
            docstatus                  = 2;
    };

    RETURN { ok: true };
};
