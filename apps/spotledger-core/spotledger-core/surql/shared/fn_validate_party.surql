-- Universal node: fn::validate::party_check
--
-- Validates that a linked party (Supplier, Customer, etc.) exists, is not
-- disabled, and is not on hold.
--
-- Config: { field: "supplier", party_type: "Supplier" }
--         { field: "customer", party_type: "Customer" }
-- The same function is wired to Purchase Invoice (supplier) and Sales Invoice
-- (customer) via different edge configs — zero code duplication.
DEFINE FUNCTION OVERWRITE fn::validate::party_check(
    $doc_id: record,
    $config: object
) {
    LET $doc      = (SELECT * FROM $doc_id)[0];
    LET $field    = $config.field;
    LET $pt       = $config.party_type;
    LET $party_id = $doc[$field];

    IF $party_id = NONE OR $party_id = NULL OR $party_id = "" {
        RETURN { error: string::concat(<string>$field, " is required") };
    };

    -- Look up the party in its table (tabSupplier or tabCustomer etc.)
    LET $party_table = string::concat("tab", string::replace(string::replace(<string>$pt, " ", "_"), "-", "_"));
    LET $party = (SELECT disabled, on_hold FROM type::table($party_table) WHERE name = $party_id LIMIT 1)[0];

    IF $party = NONE {
        RETURN { error: string::concat(<string>$pt, " does not exist: ", <string>$party_id) };
    };

    IF $party.disabled = true OR $party.disabled = 1 {
        RETURN { error: string::concat(<string>$pt, " is disabled: ", <string>$party_id) };
    };

    IF $party.on_hold = true OR $party.on_hold = 1 {
        RETURN { error: string::concat(<string>$pt, " is on hold: ", <string>$party_id) };
    };

    RETURN { ok: true };
};
