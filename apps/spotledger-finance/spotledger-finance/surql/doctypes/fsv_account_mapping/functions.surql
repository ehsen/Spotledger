-- FSV Account Mapping — domain functions

DEFINE FUNCTION OVERWRITE fn::validate::fsv_mapping_range(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT name, account_from, account_to FROM $doc_id)[0];

    IF $doc.account_from = NONE OR $doc.account_from = "" {
        RETURN { error: "Account From is required" };
    };

    IF $doc.account_to = NONE OR $doc.account_to = "" {
        RETURN { error: "Account To is required" };
    };

    IF $doc.account_from >= $doc.account_to {
        RETURN { error: "Account From must be less than Account To" };
    };

    RETURN { ok: true };
};
