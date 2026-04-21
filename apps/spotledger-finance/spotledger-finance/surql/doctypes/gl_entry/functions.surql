-- GL Entry — domain functions
-- GL Entries are system-created (on Journal Entry / Payment submit). They are immutable after submit.

DEFINE FUNCTION OVERWRITE fn::validate::gl_entry_account(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT account FROM $doc_id)[0];

    IF $doc.account = NONE OR $doc.account = "" {
        RETURN { error: "Account is required" };
    };

    LET $acct = (
        SELECT account_number, is_group, disabled FROM tabAccount
        WHERE name = $doc.account
        LIMIT 1
    )[0];

    IF $acct = NONE {
        RETURN { error: string::concat("Account '", <string>$doc.account, "' not found") };
    };

    IF $acct.disabled = true OR $acct.disabled = 1 {
        RETURN { error: string::concat("Account '", <string>$doc.account, "' is disabled") };
    };

    IF $acct.is_group = true OR $acct.is_group = 1 {
        RETURN { error: string::concat("Account '", <string>$doc.account, "' is a group account and cannot be used in GL entries") };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::gl_entry_period_open(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT company, posting_date FROM $doc_id)[0];

    IF $doc.company = NONE OR $doc.posting_date = NONE {
        RETURN { ok: true }; -- caught by mandatory check
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

DEFINE FUNCTION OVERWRITE fn::validate::gl_entry_party_required(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT account, party FROM $doc_id)[0];

    IF $doc.account = NONE OR $doc.account = "" {
        RETURN { ok: true };
    };

    LET $acct = (
        SELECT party_required FROM tabAccount
        WHERE name = $doc.account
        LIMIT 1
    )[0];

    IF $acct = NONE {
        RETURN { ok: true };
    };

    IF ($acct.party_required = true OR $acct.party_required = 1) {
        IF $doc.party = NONE OR $doc.party = "" {
            RETURN { error: string::concat("Party is required for account '", <string>$doc.account, "' (subledger account)") };
        };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::gl_entry_amounts(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT debit, credit FROM $doc_id)[0];

    LET $debit  = $doc.debit  OR 0;
    LET $credit = $doc.credit OR 0;

    IF $debit < 0 {
        RETURN { error: "Debit amount cannot be negative" };
    };

    IF $credit < 0 {
        RETURN { error: "Credit amount cannot be negative" };
    };

    IF $debit = 0 AND $credit = 0 {
        RETURN { error: "GL Entry must have a non-zero debit or credit" };
    };

    IF $debit > 0 AND $credit > 0 {
        RETURN { error: "GL Entry cannot have both debit and credit amounts" };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::gl_entry_immutable(
    $doc_id: record,
    $config: object
) {
    -- Once submitted, GL entries cannot be modified (only cancelled)
    LET $doc = (SELECT docstatus FROM $doc_id)[0];

    IF $doc.docstatus = 1 {
        RETURN { error: "Submitted GL Entry cannot be modified. Cancel the parent document to reverse." };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::compute::gl_entry_derive_period(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT company, posting_date FROM $doc_id)[0];

    IF $doc.company = NONE OR $doc.posting_date = NONE {
        RETURN { ok: true };
    };

    LET $fy = (
        SELECT name FROM tabFiscal_Year
        WHERE company        = $doc.company
          AND year_start_date <= $doc.posting_date
          AND year_end_date   >= $doc.posting_date
        LIMIT 1
    )[0];

    LET $period = (
        SELECT name FROM tabAccounting_Period
        WHERE company   = $doc.company
          AND start_date <= $doc.posting_date
          AND end_date   >= $doc.posting_date
        LIMIT 1
    )[0];

    IF $fy = NONE {
        RETURN { error: "No Fiscal Year found for the posting date" };
    };

    UPDATE $doc_id SET
        fiscal_year       = $fy.name,
        accounting_period = $period.name;

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::on_cancel::gl_entry_reverse(
    $doc_id: record,
    $config: object
) {
    -- Reverse the GL entry: swap debit/credit and mark as cancelled.
    LET $doc = (SELECT debit, credit, debit_in_account_currency, credit_in_account_currency FROM $doc_id)[0];

    UPDATE $doc_id SET
        debit                      = $doc.credit,
        credit                     = $doc.debit,
        debit_in_account_currency  = $doc.credit_in_account_currency,
        credit_in_account_currency = $doc.debit_in_account_currency,
        is_cancelled               = 1,
        docstatus                  = 2;

    RETURN { ok: true };
};
