-- Chart of Accounts — domain functions

DEFINE FUNCTION OVERWRITE fn::validate::coa_parent_required(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT coa_name, coa_type, parent_coa FROM $doc_id)[0];

    IF $doc.coa_type = "Group" OR $doc.coa_type = "Country" {
        IF $doc.parent_coa = NONE OR $doc.parent_coa = "" {
            RETURN { error: string::concat("Parent CoA is required for ", <string>$doc.coa_type, " type Chart of Accounts") };
        };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::coa_parent_type(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT coa_name, coa_type, parent_coa FROM $doc_id)[0];

    IF $doc.parent_coa = NONE OR $doc.parent_coa = "" {
        RETURN { ok: true };
    };

    LET $parent = (SELECT coa_type FROM tabChart_of_Accounts WHERE name = $doc.parent_coa LIMIT 1)[0];

    IF $parent = NONE {
        RETURN { error: string::concat("Parent CoA '", <string>$doc.parent_coa, "' not found") };
    };

    IF $parent.coa_type != "Operative" {
        RETURN { error: "Parent CoA must be of type Operative" };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::coa_single_default(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT coa_name, is_default FROM $doc_id)[0];

    IF $doc.is_default = true OR $doc.is_default = 1 {
        LET $existing = (
            SELECT name FROM tabChart_of_Accounts
            WHERE (is_default = true OR is_default = 1)
              AND name != $doc.coa_name
            LIMIT 1
        )[0];

        IF $existing != NONE {
            RETURN { error: string::concat("Chart of Accounts '", <string>$existing.name, "' is already set as the default") };
        };
    };

    RETURN { ok: true };
};
