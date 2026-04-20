-- Financial Statement Version — domain functions

DEFINE FUNCTION OVERWRITE fn::validate::fsv_single_default_per_coa(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT fsv_name, chart_of_accounts, is_default FROM $doc_id)[0];

    IF $doc.is_default = true OR $doc.is_default = 1 {
        LET $existing = (
            SELECT name FROM tabFinancial_Statement_Version
            WHERE chart_of_accounts = $doc.chart_of_accounts
              AND (is_default = true OR is_default = 1)
              AND name != $doc.fsv_name
            LIMIT 1
        )[0];

        IF $existing != NONE {
            RETURN { error: string::concat("FSV '", <string>$existing.name, "' is already set as the default for this Chart of Accounts") };
        };
    };

    RETURN { ok: true };
};
