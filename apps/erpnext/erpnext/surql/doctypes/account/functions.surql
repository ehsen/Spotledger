-- Account — domain functions
-- Replaces account.py validate() methods in the pipeline.
-- Every function returns { ok: true } or { error: "message" }. Never throws.

-- ── fn::validate::account_parent_check ───────────────────────────────────────
-- Validates parent account relationship (account.py::validate_parent):
--   1. Parent must exist
--   2. Parent cannot be the account itself
--   3. Parent must be a group account
--   4. Parent root_type must match (when both are set)
--   5. Parent must belong to the same company
DEFINE FUNCTION OVERWRITE fn::validate::account_parent_check(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT name, parent_account, root_type, company FROM $doc_id)[0];

    IF $doc.parent_account = NONE OR $doc.parent_account = "" {
        RETURN { ok: true };
    };

    -- An account cannot be its own parent
    IF $doc.parent_account = $doc.name {
        RETURN { error: string::concat("Account '", <string>$doc.name, "' cannot be its own parent") };
    };

    LET $parent = (
        SELECT name, root_type, is_group, company
        FROM tabAccount
        WHERE name = $doc.parent_account
        LIMIT 1
    )[0];

    IF $parent = NONE {
        RETURN { error: string::concat("Parent account does not exist: ", <string>$doc.parent_account) };
    };

    -- Parent must be a group (is_group stored as 0/1 integer or bool)
    IF ($parent.is_group = 0 OR $parent.is_group = false OR $parent.is_group = NONE) {
        RETURN { error: string::concat("Parent account '", <string>$doc.parent_account, "' must be a group account") };
    };

    -- Root type must agree when both sides have one set
    IF $doc.root_type != NONE AND $doc.root_type != ""
       AND $parent.root_type != NONE AND $parent.root_type != ""
       AND $parent.root_type != $doc.root_type {
        RETURN {
            error: string::concat(
                "Root type '", <string>$doc.root_type,
                "' does not match parent root type '", <string>$parent.root_type, "'"
            )
        };
    };

    IF $parent.company != $doc.company {
        RETURN { error: string::concat("Parent account '", <string>$doc.parent_account, "' belongs to a different company") };
    };

    RETURN { ok: true };
};

-- ── fn::validate::account_root_details ───────────────────────────────────────
-- Validates root account constraints (account.py::validate_root_details — partial):
--   Root accounts (no parent) must always be group accounts.
DEFINE FUNCTION OVERWRITE fn::validate::account_root_details(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT name, parent_account, is_group FROM $doc_id)[0];

    -- A root account (no parent) must be a group
    IF ($doc.parent_account = NONE OR $doc.parent_account = "")
       AND ($doc.is_group = 0 OR $doc.is_group = false OR $doc.is_group = NONE) {
        RETURN {
            error: string::concat("Root account '", <string>$doc.name, "' must be a group account")
        };
    };

    RETURN { ok: true };
};

-- ── fn::validate::account_number_unique ──────────────────────────────────────
-- Prevents duplicate account_number within the same company
-- (account.py::validate_account_number).
DEFINE FUNCTION OVERWRITE fn::validate::account_number_unique(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT account_number, company FROM $doc_id)[0];

    IF $doc.account_number = NONE OR $doc.account_number = "" {
        RETURN { ok: true };
    };

    LET $duplicate = (
        SELECT id FROM tabAccount
        WHERE account_number = $doc.account_number
          AND company        = $doc.company
          AND id             != $doc_id
          AND docstatus      != 2
        LIMIT 1
    )[0];

    IF $duplicate != NONE {
        RETURN {
            error: string::concat(
                "Account number '", <string>$doc.account_number,
                "' is already used by another account in this company"
            )
        };
    };

    RETURN { ok: true };
};

-- ── fn::validate::account_parent_child_type ──────────────────────────────────
-- Certain account types are exclusive — a child account cannot share the same
-- account_type as its direct parent (account.py::validate_parent_child_account_type).
-- Applies to: Direct/Indirect Income, Direct/Indirect Expense, Current Asset/Liability.
DEFINE FUNCTION OVERWRITE fn::validate::account_parent_child_type(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT parent_account, account_type FROM $doc_id)[0];

    IF $doc.parent_account = NONE OR $doc.parent_account = ""
       OR $doc.account_type = NONE OR $doc.account_type = "" {
        RETURN { ok: true };
    };

    LET $exclusive = [
        "Direct Income", "Indirect Income",
        "Current Asset", "Current Liability",
        "Direct Expense", "Indirect Expense"
    ];

    IF !($doc.account_type IN $exclusive) {
        RETURN { ok: true };
    };

    LET $parent_type = (
        SELECT account_type FROM tabAccount
        WHERE name = $doc.parent_account
        LIMIT 1
    )[0].account_type;

    IF $parent_type = $doc.account_type {
        RETURN {
            error: string::concat(
                "Only the parent account can be of type '", <string>$doc.account_type,
                "'. A child account cannot repeat the same exclusive type as its parent."
            )
        };
    };

    RETURN { ok: true };
};

-- ── fn::validate::account_group_or_ledger ────────────────────────────────────
-- A ledger account (is_group=0) cannot have child accounts
-- (account.py::validate_group_or_ledger — invariant portion).
DEFINE FUNCTION OVERWRITE fn::validate::account_group_or_ledger(
    $doc_id: record,
    $config: object
) {
    LET $doc  = (SELECT name, is_group FROM $doc_id)[0];

    -- Only check ledger accounts
    IF $doc.is_group = 1 OR $doc.is_group = true {
        RETURN { ok: true };
    };

    LET $child_count = (
        SELECT count() AS cnt FROM tabAccount
        WHERE parent_account = $doc.name
          AND docstatus != 2
    )[0].cnt OR 0;

    IF $child_count > 0 {
        RETURN {
            error: string::concat(
                "Account '", <string>$doc.name,
                "' has child accounts and cannot be a ledger account. Convert it to a group first."
            )
        };
    };

    RETURN { ok: true };
};

-- ── fn::validate::account_currency ───────────────────────────────────────────
-- Prevents changing account_currency after GL entries have been posted with a
-- different currency (account.py::validate_account_currency).
DEFINE FUNCTION OVERWRITE fn::validate::account_currency(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT name, account_currency FROM $doc_id)[0];

    IF $doc.account_currency = NONE OR $doc.account_currency = "" {
        RETURN { ok: true };
    };

    -- Check for non-cancelled GL entries posted with a different currency
    LET $conflict = (
        SELECT id FROM tabGL_Entry
        WHERE account          = $doc.name
          AND is_cancelled     = false
          AND account_currency != $doc.account_currency
        LIMIT 1
    )[0];

    IF $conflict != NONE {
        RETURN {
            error: "Account currency cannot be changed after GL entries have been posted with a different currency"
        };
    };

    RETURN { ok: true };
};

-- ── fn::compute::account_root_type ───────────────────────────────────────────
-- Derives report_type from root_type (account.py::set_root_and_report_type).
-- Also derives balance_must_be from root_type when not explicitly set.
-- For group accounts, propagates root_type + report_type to all descendant
-- accounts up to 5 levels deep. Full unlimited propagation requires the
-- child_of graph relation (see GRAPH_COMPUTE_PLAN.md Phase 6).
DEFINE FUNCTION OVERWRITE fn::compute::account_root_type(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT name, root_type, report_type, balance_must_be, is_group FROM $doc_id)[0];
    LET $rt  = $doc.root_type;

    -- ── Derive report_type for this account ───────────────────────────────────
    LET $rpt =
        IF $doc.report_type != NONE AND $doc.report_type != "" { $doc.report_type }
        ELSE IF $rt IN ["Asset", "Liability", "Equity"]        { "Balance Sheet" }
        ELSE IF $rt IN ["Income", "Expense"]                   { "Profit and Loss" }
        ELSE                                                   { NONE };

    IF $rpt != NONE AND ($doc.report_type = NONE OR $doc.report_type = "") {
        UPDATE $doc_id SET report_type = $rpt;
    };

    -- ── Derive balance_must_be if not explicitly set ──────────────────────────
    IF $doc.balance_must_be = NONE OR $doc.balance_must_be = "" {
        LET $bal =
            IF $rt IN ["Asset", "Expense"]            { "Debit" }
            ELSE IF $rt IN ["Liability", "Income", "Equity"] { "Credit" }
            ELSE                                      { NONE };

        IF $bal != NONE {
            UPDATE $doc_id SET balance_must_be = $bal;
        };
    };

    -- ── Propagate root_type + report_type to descendants (group accounts only) ─
    -- Uses up to 5 levels of parent_account traversal.  The child_of graph
    -- relation (planned) will replace this with proper unbounded recursion.
    IF ($doc.is_group = 1 OR $doc.is_group = true) AND $rt != NONE AND $rt != "" AND $rpt != NONE {
        LET $l1 = (SELECT name FROM tabAccount WHERE parent_account = $doc.name).name;
        IF array::len($l1) > 0 {
            LET $l2 = (SELECT name FROM tabAccount WHERE parent_account IN $l1).name;
            IF array::len($l2) > 0 {
                LET $l3 = (SELECT name FROM tabAccount WHERE parent_account IN $l2).name;
                IF array::len($l3) > 0 {
                    LET $l4 = (SELECT name FROM tabAccount WHERE parent_account IN $l3).name;
                    IF array::len($l4) > 0 {
                        LET $l5 = (SELECT name FROM tabAccount WHERE parent_account IN $l4).name;
                        LET $all = array::flatten([$l1, $l2, $l3, $l4, $l5]);
                        UPDATE tabAccount SET root_type = $rt, report_type = $rpt WHERE name IN $all;
                    } ELSE {
                        LET $all = array::flatten([$l1, $l2, $l3, $l4]);
                        UPDATE tabAccount SET root_type = $rt, report_type = $rpt WHERE name IN $all;
                    };
                } ELSE {
                    LET $all = array::flatten([$l1, $l2, $l3]);
                    UPDATE tabAccount SET root_type = $rt, report_type = $rpt WHERE name IN $all;
                };
            } ELSE {
                LET $all = array::flatten([$l1, $l2]);
                UPDATE tabAccount SET root_type = $rt, report_type = $rpt WHERE name IN $all;
            };
        };
    };

    RETURN { ok: true };
};
