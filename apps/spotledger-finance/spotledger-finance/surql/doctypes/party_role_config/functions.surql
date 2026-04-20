-- Party Role Config — domain functions

DEFINE FUNCTION OVERWRITE fn::validate::party_role_config_unique(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT name, party, role, company FROM $doc_id)[0];

    LET $existing = (
        SELECT name FROM tabParty_Role_Config
        WHERE party   = $doc.party
          AND role    = $doc.role
          AND company = $doc.company
          AND name   != $doc.name
        LIMIT 1
    )[0];

    IF $existing != NONE {
        RETURN { error: string::concat("A Party Role Config already exists for party '", <string>$doc.party, "', role '", <string>$doc.role, "', company '", <string>$doc.company, "'") };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::party_role_config_account_types(
    $doc_id: record,
    $config: object
) {
    -- TODO: Phase 4 — Validate that receivable_account is account_type_internal D
    -- and payable_account is account_type_internal K. Deferred because Account + Account Group
    -- tables do not exist until Phase 4.
    RETURN { ok: true };
};
