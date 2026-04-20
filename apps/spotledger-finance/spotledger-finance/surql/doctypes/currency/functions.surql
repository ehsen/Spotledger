-- Currency — domain validation functions

DEFINE FUNCTION OVERWRITE fn::validate::currency_code_format(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT code FROM $doc_id)[0];

    IF $doc.code = NONE OR $doc.code = "" {
        RETURN { error: "Currency code is required" };
    };

    IF string::len($doc.code) != 3 {
        RETURN { error: "Currency code must be exactly 3 uppercase characters (ISO 4217)" };
    };

    IF $doc.code != string::uppercase($doc.code) {
        RETURN { error: string::concat("Currency code must be uppercase. Got: ", $doc.code) };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::currency_single_base(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT code, is_base FROM $doc_id)[0];

    IF $doc.is_base = true OR $doc.is_base = 1 {
        LET $existing = (
            SELECT name FROM tabCurrency
            WHERE (is_base = true OR is_base = 1)
              AND name != $doc.code
            LIMIT 1
        )[0];

        IF $existing != NONE {
            RETURN { error: string::concat("Currency ", <string>$existing.name, " is already set as the base currency. Only one base currency is allowed.") };
        };
    };

    RETURN { ok: true };
};
