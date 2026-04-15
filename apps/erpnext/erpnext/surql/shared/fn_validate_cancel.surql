-- Universal node: fn::validate_cancel::no_dependents
--
-- Checks that no allocated/active references to this document exist before
-- allowing cancellation.
-- Config: { check_table: "tabPayment_Entry_Reference", ref_field: "reference_name",
--           doctype_field: "reference_doctype", doctype_value: "Purchase Invoice" }
DEFINE FUNCTION OVERWRITE fn::validate_cancel::no_dependents(
    $doc_id: record,
    $config: object
) {
    -- Default: no config = no external dependency check (allows cancel)
    IF $config = NONE OR $config = {} {
        RETURN { ok: true };
    };

    LET $check_table    = $config.check_table;
    LET $ref_field      = $config.ref_field;
    LET $doctype_field  = $config.doctype_field;
    LET $doctype_value  = $config.doctype_value;
    LET $doc_name       = record::id($doc_id);

    IF $check_table = NONE {
        RETURN { ok: true };
    };

    LET $blocking = (
        SELECT id FROM type::table($check_table)
        WHERE $doc[$ref_field] = $doc_name
          AND $doc[$doctype_field] = $doctype_value
        LIMIT 1
    );

    IF array::len($blocking) > 0 {
        RETURN {
            error: string::concat(
                "Cannot cancel: active references exist in ",
                <string>$check_table,
                ". Remove them first."
            )
        };
    };

    RETURN { ok: true };
};
