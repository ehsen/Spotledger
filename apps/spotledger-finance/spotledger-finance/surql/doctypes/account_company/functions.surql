-- Account Company — domain functions

DEFINE FUNCTION OVERWRITE fn::validate::account_company_unique(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT name, account, company FROM $doc_id)[0];

    LET $existing = (
        SELECT name FROM tabAccount_Company
        WHERE account = $doc.account
          AND company = $doc.company
          AND name   != $doc.name
        LIMIT 1
    )[0];

    IF $existing != NONE {
        RETURN { error: string::concat("Account Company record already exists for account '", <string>$doc.account, "' and company '", <string>$doc.company, "'") };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::compute::account_company_derive_open_item(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT account FROM $doc_id)[0];

    IF $doc.account = NONE OR $doc.account = "" {
        RETURN { ok: true };
    };

    LET $acct = (
        SELECT account_type_internal FROM tabAccount
        WHERE name = $doc.account
        LIMIT 1
    )[0];

    IF $acct = NONE {
        RETURN { ok: true };
    };

    LET $is_open = ($acct.account_type_internal = "D" OR $acct.account_type_internal = "K");

    UPDATE $doc_id SET open_item_management = $is_open;

    RETURN { ok: true };
};
