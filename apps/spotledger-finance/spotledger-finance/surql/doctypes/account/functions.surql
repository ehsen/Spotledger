-- Account — domain functions

DEFINE FUNCTION OVERWRITE fn::compute::account_derive_from_group(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT account_group FROM $doc_id)[0];

    IF $doc.account_group = NONE OR $doc.account_group = "" {
        RETURN { ok: true };
    };

    LET $grp = (
        SELECT account_type_internal, account_category, fs_type, is_reconciliation_group, party_required
        FROM tabAccount_Group
        WHERE name = $doc.account_group
        LIMIT 1
    )[0];

    IF $grp = NONE {
        RETURN { error: string::concat("Account Group not found: ", <string>$doc.account_group) };
    };

    UPDATE $doc_id SET
        account_type_internal      = $grp.account_type_internal,
        account_category           = $grp.account_category,
        fs_type                    = $grp.fs_type,
        is_reconciliation_account  = $grp.is_reconciliation_group,
        party_required             = $grp.party_required;

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::account_number_in_group_range(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT account_number, account_group FROM $doc_id)[0];

    IF $doc.account_group = NONE OR $doc.account_group = "" {
        RETURN { ok: true };
    };

    LET $grp = (
        SELECT number_from, number_to FROM tabAccount_Group
        WHERE name = $doc.account_group
        LIMIT 1
    )[0];

    IF $grp = NONE {
        RETURN { ok: true }; -- group not found, will be caught by mandatory check
    };

    IF $doc.account_number < $grp.number_from OR $doc.account_number > $grp.number_to {
        RETURN { error: string::concat("Account Number '", <string>$doc.account_number, "' is outside the range [", <string>$grp.number_from, " – ", <string>$grp.number_to, "] of Account Group '", <string>$doc.account_group, "'") };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::account_group_matches_coa(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT chart_of_accounts, account_group FROM $doc_id)[0];

    IF $doc.account_group = NONE OR $doc.account_group = "" {
        RETURN { ok: true };
    };

    LET $grp = (
        SELECT chart_of_accounts FROM tabAccount_Group
        WHERE name = $doc.account_group
        LIMIT 1
    )[0];

    IF $grp = NONE {
        RETURN { ok: true };
    };

    IF $grp.chart_of_accounts != $doc.chart_of_accounts {
        RETURN { error: string::concat("Account Group '", <string>$doc.account_group, "' belongs to a different Chart of Accounts") };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::account_retained_earnings_required(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT fs_type, is_group, retained_earnings_account FROM $doc_id)[0];

    IF $doc.is_group = true OR $doc.is_group = 1 {
        RETURN { ok: true }; -- group accounts don't need retained earnings
    };

    IF ($doc.fs_type = "Income" OR $doc.fs_type = "Expense") {
        IF $doc.retained_earnings_account = NONE OR $doc.retained_earnings_account = "" {
            RETURN { error: "Retained Earnings Account is required for Income and Expense accounts" };
        };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::account_type_immutable(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT account_number, account_type_internal FROM $doc_id)[0];

    -- Check if any GL entries exist for this account
    LET $gl = (
        SELECT id FROM tabGL_Entry
        WHERE account = $doc.account_number
          AND (is_cancelled = false OR is_cancelled = 0)
        LIMIT 1
    )[0];

    IF $gl != NONE {
        -- Retrieve what was stored before this save
        LET $stored = (SELECT account_type_internal FROM tabAccount WHERE name = $doc.account_number LIMIT 1)[0];

        IF $stored != NONE AND $stored.account_type_internal != $doc.account_type_internal {
            RETURN { error: "Cannot change Account Type when GL Entries exist for this account" };
        };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::on_save::account_relate_parent(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT account_number, parent_account FROM $doc_id)[0];

    -- Remove old parent edge
    DELETE (SELECT id FROM child_of WHERE in = $doc_id);

    -- Create new parent edge if parent is set
    IF $doc.parent_account != NONE AND $doc.parent_account != "" {
        LET $parent_ref = type::record("tabAccount", $doc.parent_account);
        RELATE $doc_id -> child_of -> $parent_ref;
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::on_save::account_extend_companies(
    $doc_id: record,
    $config: object
) {
    -- For every Company that uses this CoA, ensure an Account Company record exists.
    LET $doc = (SELECT account_number, chart_of_accounts FROM $doc_id)[0];

    LET $companies = (
        SELECT name FROM tabCompany
        WHERE chart_of_accounts = $doc.chart_of_accounts
        LIMIT 100
    );

    FOR $company IN $companies {
        LET $co_name = $company.name;
        LET $exists = (
            SELECT id FROM tabAccount_Company
            WHERE account = $doc.account_number
              AND company = $co_name
            LIMIT 1
        )[0];

        IF $exists = NONE {
            INSERT INTO tabAccount_Company {
                account: $doc.account_number,
                company: $co_name
            };
        };
    };

    RETURN { ok: true };
};
