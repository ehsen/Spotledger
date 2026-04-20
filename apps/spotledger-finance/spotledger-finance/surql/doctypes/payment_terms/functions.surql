-- Payment Terms — domain validation functions

DEFINE FUNCTION OVERWRITE fn::validate::payment_terms_days(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT payment_days FROM $doc_id)[0];

    IF $doc.payment_days = NONE {
        RETURN { error: "Payment Days is required" };
    };

    IF $doc.payment_days < 0 {
        RETURN { error: "Payment Days cannot be negative" };
    };

    RETURN { ok: true };
};
