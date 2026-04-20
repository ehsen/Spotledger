-- Casual Party — domain functions

DEFINE FUNCTION OVERWRITE fn::validate::casual_party_not_duplicate(
    $doc_id: record,
    $config: object
) {
    -- Soft warning: check if a similar name exists (not a hard block).
    -- Returns ok: true always but includes a warning hint in the message when a match is found.
    -- The UI can choose to surface this as a toast/warning.
    LET $doc = (SELECT name, party_name FROM $doc_id)[0];

    IF $doc.party_name = NONE OR $doc.party_name = "" {
        RETURN { ok: true };
    };

    -- Look for existing Casual Party with very similar name (case-insensitive exact match)
    LET $similar = (
        SELECT name FROM tabCasual_Party
        WHERE string::lowercase(party_name) = string::lowercase($doc.party_name)
          AND name != $doc.name
          AND (is_promoted = false OR is_promoted = 0)
        LIMIT 1
    )[0];

    IF $similar != NONE {
        RETURN { ok: true, warning: string::concat("A Casual Party with the name '", <string>$doc.party_name, "' already exists (", <string>$similar.name, ")") };
    };

    RETURN { ok: true };
};
