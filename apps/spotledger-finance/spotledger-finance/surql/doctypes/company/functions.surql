-- Company — domain validation functions
-- Account-related validations deferred to Phase 4 when Account + CoA exist.

DEFINE FUNCTION OVERWRITE fn::validate::company_abbr_format(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT company_name, abbr FROM $doc_id)[0];

    IF $doc.abbr = NONE OR $doc.abbr = "" {
        RETURN { error: "Company abbreviation is required" };
    };

    IF string::len($doc.abbr) > 10 {
        RETURN { error: "Company abbreviation must be 10 characters or fewer" };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::company_abbr_unique(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT company_name, abbr FROM $doc_id)[0];

    LET $existing = (
        SELECT name FROM tabCompany
        WHERE abbr = $doc.abbr
          AND name != $doc.company_name
        LIMIT 1
    )[0];

    IF $existing != NONE {
        RETURN { error: string::concat("Abbreviation '", <string>$doc.abbr, "' is already used by company '", <string>$existing.name, "'") };
    };

    RETURN { ok: true };
};

-- TODO: Phase 4 — validate that retained_earnings_account, default_payable_account, and
-- default_receivable_account exist and have the correct account_type_internal values.
