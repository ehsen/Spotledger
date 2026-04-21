-- Payment — domain functions
-- Covers 5 GL posting paths: Expense Payment, Income Receipt,
-- Party Payment, Party Receipt, Internal Transfer.

-- Direction/type consistency matrix:
--   Outgoing  → Expense Payment | Party Payment
--   Incoming  → Income Receipt  | Party Receipt
--   Internal  → Internal Transfer

DEFINE FUNCTION OVERWRITE fn::validate::payment_type_direction(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT payment_direction, payment_type FROM $doc_id)[0];

    IF $doc.payment_direction = NONE OR $doc.payment_type = NONE {
        RETURN { ok: true };
    };

    LET $valid = {
        "Outgoing": ["Expense Payment", "Party Payment"],
        "Incoming": ["Income Receipt",  "Party Receipt"],
        "Internal": ["Internal Transfer"]
    };

    LET $allowed = $valid[$doc.payment_direction];

    IF $allowed = NONE {
        RETURN { error: string::concat("Unknown payment direction: ", <string>$doc.payment_direction) };
    };

    IF $doc.payment_type NOT IN $allowed {
        RETURN { error: string::concat("Payment Type '", <string>$doc.payment_type, "' is not valid for direction '", <string>$doc.payment_direction, "'") };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::payment_line_accounts(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT payment_type, expense_income_lines FROM $doc_id)[0];

    IF $doc.payment_type != "Expense Payment" AND $doc.payment_type != "Income Receipt" {
        RETURN { ok: true }; -- line account validation only for expense/income types
    };

    IF $doc.expense_income_lines = NONE OR array::len($doc.expense_income_lines) = 0 {
        RETURN { error: "At least one line is required for Expense Payment and Income Receipt" };
    };

    LET $expected_fs = IF $doc.payment_type = "Expense Payment" { "Expense" } ELSE { "Income" };

    FOR $line IN $doc.expense_income_lines {
        IF $line.account = NONE OR $line.account = "" {
            RETURN { error: "All payment lines must have an Account selected" };
        };

        LET $acct = (SELECT fs_type FROM tabAccount WHERE name = $line.account LIMIT 1)[0];

        IF $acct = NONE {
            RETURN { error: string::concat("Account '", <string>$line.account, "' not found") };
        };

        IF $acct.fs_type != $expected_fs {
            RETURN { error: string::concat("Account '", <string>$line.account, "' is not an ", $expected_fs, " account (fs_type = ", <string>$acct.fs_type, ")") };
        };

        IF $line.amount = NONE OR $line.amount <= 0 {
            RETURN { error: string::concat("Amount must be greater than zero on line with account '", <string>$line.account, "'") };
        };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::payment_party_role(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT payment_type, payment_direction, party_role FROM $doc_id)[0];

    IF $doc.payment_type != "Party Payment" AND $doc.payment_type != "Party Receipt" {
        RETURN { ok: true };
    };

    IF $doc.party_role = NONE OR $doc.party_role = "" {
        RETURN { error: "Party Role is required for Party Payment and Party Receipt" };
    };

    -- Supplier → Outgoing; Customer → Incoming
    IF $doc.payment_direction = "Outgoing" AND $doc.party_role != "Supplier" AND $doc.party_role != "Employee" {
        RETURN { error: string::concat("Party Role '", <string>$doc.party_role, "' is not valid for Outgoing Party Payment. Use Supplier or Employee.") };
    };

    IF $doc.payment_direction = "Incoming" AND $doc.party_role != "Customer" {
        RETURN { error: string::concat("Party Role '", <string>$doc.party_role, "' is not valid for Incoming Party Receipt. Use Customer.") };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::payment_transfer_accounts(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT payment_type, from_account, to_account FROM $doc_id)[0];

    IF $doc.payment_type != "Internal Transfer" {
        RETURN { ok: true };
    };

    IF $doc.from_account = NONE OR $doc.from_account = "" {
        RETURN { error: "From Account is required for Internal Transfer" };
    };

    IF $doc.to_account = NONE OR $doc.to_account = "" {
        RETURN { error: "To Account is required for Internal Transfer" };
    };

    IF $doc.from_account = $doc.to_account {
        RETURN { error: "From Account and To Account cannot be the same" };
    };

    -- Both must be Cash and Bank accounts
    FOR $acct_name IN [$doc.from_account, $doc.to_account] {
        LET $acct = (SELECT account_category FROM tabAccount WHERE name = $acct_name LIMIT 1)[0];

        IF $acct = NONE {
            RETURN { error: string::concat("Account '", <string>$acct_name, "' not found") };
        };

        IF $acct.account_category != "Cash and Bank" {
            RETURN { error: string::concat("Account '", <string>$acct_name, "' must be a Cash and Bank account for Internal Transfer") };
        };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::payment_paid_accounts(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT payment_direction, payment_type, paid_from, paid_into FROM $doc_id)[0];

    IF $doc.payment_type = "Internal Transfer" {
        RETURN { ok: true }; -- handled by transfer_accounts validator
    };

    IF $doc.payment_direction = "Outgoing" OR $doc.payment_direction = "Internal" {
        IF $doc.paid_from = NONE OR $doc.paid_from = "" {
            RETURN { error: "Paid From account is required for Outgoing payments" };
        };
        LET $acct = (SELECT account_category FROM tabAccount WHERE name = $doc.paid_from LIMIT 1)[0];
        IF $acct != NONE AND $acct.account_category != "Cash and Bank" {
            RETURN { error: "Paid From must be a Cash and Bank account" };
        };
    };

    IF $doc.payment_direction = "Incoming" OR $doc.payment_direction = "Internal" {
        IF $doc.paid_into = NONE OR $doc.paid_into = "" {
            RETURN { error: "Paid Into account is required for Incoming payments" };
        };
        LET $acct = (SELECT account_category FROM tabAccount WHERE name = $doc.paid_into LIMIT 1)[0];
        IF $acct != NONE AND $acct.account_category != "Cash and Bank" {
            RETURN { error: "Paid Into must be a Cash and Bank account" };
        };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::payment_casual_party_scope(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT payment_type, casual_party FROM $doc_id)[0];

    IF $doc.casual_party = NONE OR $doc.casual_party = "" {
        RETURN { ok: true };
    };

    IF $doc.payment_type = "Party Payment" OR $doc.payment_type = "Party Receipt" {
        RETURN { error: "Casual Party cannot be used for Party Payment or Party Receipt. Use a full Party with a subledger account." };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::payment_invoice_allocation(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT payment_type, against_invoices FROM $doc_id)[0];

    IF $doc.payment_type != "Party Payment" AND $doc.payment_type != "Party Receipt" {
        RETURN { ok: true };
    };

    IF $doc.against_invoices = NONE OR array::len($doc.against_invoices) = 0 {
        RETURN { ok: true }; -- advance payment — no invoice allocation required
    };

    FOR $alloc IN $doc.against_invoices {
        IF $alloc.invoice = NONE OR $alloc.invoice = "" {
            RETURN { error: "Invoice is required on each allocation line" };
        };

        IF $alloc.allocated_amount = NONE OR $alloc.allocated_amount <= 0 {
            RETURN { error: string::concat("Allocated amount must be greater than zero for invoice '", <string>$alloc.invoice, "'") };
        };

        -- TODO: Phase transactions — check allocated_amount <= invoice outstanding balance
        -- Deferred until Sales Invoice / Purchase Invoice DocTypes exist.
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::compute::payment_totals(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT payment_type, amount, expense_income_lines, exchange_rate FROM $doc_id)[0];

    LET $rate = $doc.exchange_rate OR 1;
    LET $total = IF $doc.payment_type = "Expense Payment" OR $doc.payment_type = "Income Receipt" {
        math::sum($doc.expense_income_lines[*].amount ?? [0])
    } ELSE {
        $doc.amount OR 0
    };

    UPDATE $doc_id SET
        total_amount      = $total,
        total_amount_base = $total * $rate;

    RETURN { ok: true };
};

-- GL posting helpers

DEFINE FUNCTION OVERWRITE fn::payment::post_expense(
    $doc_id: record,
    $doc_name: string,
    $doc: object
) {
    -- Expense Payment: debit each expense account line, credit paid_from (Cash/Bank)
    LET $total = math::sum($doc.expense_income_lines[*].amount ?? [0]);

    FOR $line IN $doc.expense_income_lines {
        INSERT INTO tabGL_Entry {
            name:         string::concat($doc_name, "-EXP-", <string>($line._pk ?? $line.idx ?? 0)),
            doctype:      "GL Entry",
            owner:        $doc.owner,
            modified_by:  $doc.modified_by,
            company:      $doc.company,
            account:      $line.account,
            posting_date: $doc.date,
            debit:        $line.amount OR 0,
            credit:       0,
            cost_center:  $line.cost_center,
            voucher_type: "Payment",
            voucher_no:   $doc_name,
            remarks:      $doc.narration,
            docstatus:    1,
            is_cancelled: 0
        };
    };

    INSERT INTO tabGL_Entry {
        name:         string::concat($doc_name, "-BANK"),
        doctype:      "GL Entry",
        owner:        $doc.owner,
        modified_by:  $doc.modified_by,
        company:      $doc.company,
        account:      $doc.paid_from,
        posting_date: $doc.date,
        debit:        0,
        credit:       $total,
        voucher_type: "Payment",
        voucher_no:   $doc_name,
        remarks:      $doc.narration,
        docstatus:    1,
        is_cancelled: 0
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::payment::post_income(
    $doc_id: record,
    $doc_name: string,
    $doc: object
) {
    -- Income Receipt: credit each income account line, debit paid_into (Cash/Bank)
    LET $total = math::sum($doc.expense_income_lines[*].amount ?? [0]);

    INSERT INTO tabGL_Entry {
        name:         string::concat($doc_name, "-BANK"),
        doctype:      "GL Entry",
        owner:        $doc.owner,
        modified_by:  $doc.modified_by,
        company:      $doc.company,
        account:      $doc.paid_into,
        posting_date: $doc.date,
        debit:        $total,
        credit:       0,
        voucher_type: "Payment",
        voucher_no:   $doc_name,
        remarks:      $doc.narration,
        docstatus:    1,
        is_cancelled: 0
    };

    FOR $line IN $doc.expense_income_lines {
        INSERT INTO tabGL_Entry {
            name:         string::concat($doc_name, "-INC-", <string>($line._pk ?? $line.idx ?? 0)),
            doctype:      "GL Entry",
            owner:        $doc.owner,
            modified_by:  $doc.modified_by,
            company:      $doc.company,
            account:      $line.account,
            posting_date: $doc.date,
            debit:        0,
            credit:       $line.amount OR 0,
            cost_center:  $line.cost_center,
            voucher_type: "Payment",
            voucher_no:   $doc_name,
            remarks:      $doc.narration,
            docstatus:    1,
            is_cancelled: 0
        };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::payment::post_party_payment(
    $doc_id: record,
    $doc_name: string,
    $doc: object
) {
    -- Party Payment (Outgoing): debit subledger account (party payable), credit paid_from
    LET $amount = $doc.amount OR 0;

    -- Get the payable account for this party + company
    LET $prc = (
        SELECT payable_account FROM tabParty_Role_Config
        WHERE party   = $doc.party
          AND role    = $doc.party_role
          AND company = $doc.company
        LIMIT 1
    )[0];

    IF $prc = NONE OR ($prc.payable_account = NONE OR $prc.payable_account = "") {
        RETURN { error: string::concat("No payable account configured for party '", <string>$doc.party, "' role '", <string>$doc.party_role, "' in company '", <string>$doc.company, "'") };
    };

    INSERT INTO tabGL_Entry {
        name:         string::concat($doc_name, "-PAY"),
        doctype:      "GL Entry",
        owner:        $doc.owner,
        modified_by:  $doc.modified_by,
        company:      $doc.company,
        account:      $prc.payable_account,
        posting_date: $doc.date,
        debit:        $amount,
        credit:       0,
        party:        $doc.party,
        party_role:   $doc.party_role,
        voucher_type: "Payment",
        voucher_no:   $doc_name,
        remarks:      $doc.narration,
        docstatus:    1,
        is_cancelled: 0
    };

    INSERT INTO tabGL_Entry {
        name:         string::concat($doc_name, "-BANK"),
        doctype:      "GL Entry",
        owner:        $doc.owner,
        modified_by:  $doc.modified_by,
        company:      $doc.company,
        account:      $doc.paid_from,
        posting_date: $doc.date,
        debit:        0,
        credit:       $amount,
        voucher_type: "Payment",
        voucher_no:   $doc_name,
        remarks:      $doc.narration,
        docstatus:    1,
        is_cancelled: 0
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::payment::post_party_receipt(
    $doc_id: record,
    $doc_name: string,
    $doc: object
) {
    -- Party Receipt (Incoming): debit paid_into, credit subledger account (party receivable)
    LET $amount = $doc.amount OR 0;

    LET $prc = (
        SELECT receivable_account FROM tabParty_Role_Config
        WHERE party   = $doc.party
          AND role    = $doc.party_role
          AND company = $doc.company
        LIMIT 1
    )[0];

    IF $prc = NONE OR ($prc.receivable_account = NONE OR $prc.receivable_account = "") {
        RETURN { error: string::concat("No receivable account configured for party '", <string>$doc.party, "' role '", <string>$doc.party_role, "' in company '", <string>$doc.company, "'") };
    };

    INSERT INTO tabGL_Entry {
        name:         string::concat($doc_name, "-BANK"),
        doctype:      "GL Entry",
        owner:        $doc.owner,
        modified_by:  $doc.modified_by,
        company:      $doc.company,
        account:      $doc.paid_into,
        posting_date: $doc.date,
        debit:        $amount,
        credit:       0,
        voucher_type: "Payment",
        voucher_no:   $doc_name,
        remarks:      $doc.narration,
        docstatus:    1,
        is_cancelled: 0
    };

    INSERT INTO tabGL_Entry {
        name:         string::concat($doc_name, "-REC"),
        doctype:      "GL Entry",
        owner:        $doc.owner,
        modified_by:  $doc.modified_by,
        company:      $doc.company,
        account:      $prc.receivable_account,
        posting_date: $doc.date,
        debit:        0,
        credit:       $amount,
        party:        $doc.party,
        party_role:   $doc.party_role,
        voucher_type: "Payment",
        voucher_no:   $doc_name,
        remarks:      $doc.narration,
        docstatus:    1,
        is_cancelled: 0
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::payment::post_transfer(
    $doc_id: record,
    $doc_name: string,
    $doc: object
) {
    -- Internal Transfer: debit to_account, credit from_account
    LET $amount = $doc.amount OR 0;

    INSERT INTO tabGL_Entry {
        name:         string::concat($doc_name, "-TO"),
        doctype:      "GL Entry",
        owner:        $doc.owner,
        modified_by:  $doc.modified_by,
        company:      $doc.company,
        account:      $doc.to_account,
        posting_date: $doc.date,
        debit:        $amount,
        credit:       0,
        voucher_type: "Payment",
        voucher_no:   $doc_name,
        remarks:      $doc.narration,
        docstatus:    1,
        is_cancelled: 0
    };

    INSERT INTO tabGL_Entry {
        name:         string::concat($doc_name, "-FROM"),
        doctype:      "GL Entry",
        owner:        $doc.owner,
        modified_by:  $doc.modified_by,
        company:      $doc.company,
        account:      $doc.from_account,
        posting_date: $doc.date,
        debit:        0,
        credit:       $amount,
        voucher_type: "Payment",
        voucher_no:   $doc_name,
        remarks:      $doc.narration,
        docstatus:    1,
        is_cancelled: 0
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::on_submit::payment_post_gl(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT * FROM $doc_id)[0];
    LET $doc_name = <string>record::id($doc_id);

    IF $doc.payment_type = "Expense Payment" {
        RETURN fn::payment::post_expense($doc_id, $doc_name, $doc);
    };

    IF $doc.payment_type = "Income Receipt" {
        RETURN fn::payment::post_income($doc_id, $doc_name, $doc);
    };

    IF $doc.payment_type = "Party Payment" {
        RETURN fn::payment::post_party_payment($doc_id, $doc_name, $doc);
    };

    IF $doc.payment_type = "Party Receipt" {
        RETURN fn::payment::post_party_receipt($doc_id, $doc_name, $doc);
    };

    IF $doc.payment_type = "Internal Transfer" {
        RETURN fn::payment::post_transfer($doc_id, $doc_name, $doc);
    };

    RETURN { error: string::concat("Unknown payment type: ", <string>$doc.payment_type) };
};

DEFINE FUNCTION OVERWRITE fn::on_cancel::payment_reverse_gl(
    $doc_id: record,
    $config: object
) {
    LET $doc_name = <string>record::id($doc_id);

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
            is_cancelled = 1,
            docstatus                  = 2;
    };

    RETURN { ok: true };
};
