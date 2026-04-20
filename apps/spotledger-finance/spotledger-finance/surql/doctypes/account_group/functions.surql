-- Account Group — domain functions

-- Derivation mapping: account_category → (account_type_internal, fs_type, is_reconciliation_group, party_required)
DEFINE FUNCTION OVERWRITE fn::compute::account_group_derive_classification(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT account_category FROM $doc_id)[0];

    IF $doc.account_category = NONE OR $doc.account_category = "" {
        RETURN { ok: true }; -- nothing to derive yet
    };

    LET $cat = $doc.account_category;

    LET $mapping = {
        "Fixed Asset":          { type: "A", fs: "Asset",     recon: true,  party: false },
        "Current Asset":        { type: "S", fs: "Asset",     recon: false, party: false },
        "Cash and Bank":        { type: "S", fs: "Asset",     recon: false, party: false },
        "Trade Receivable":     { type: "D", fs: "Asset",     recon: true,  party: true  },
        "Inventory":            { type: "M", fs: "Asset",     recon: true,  party: false },
        "Equity":               { type: "S", fs: "Equity",    recon: false, party: false },
        "Long Term Liability":  { type: "S", fs: "Liability", recon: false, party: false },
        "Trade Payable":        { type: "K", fs: "Liability", recon: true,  party: true  },
        "Current Liability":    { type: "S", fs: "Liability", recon: false, party: false },
        "Revenue":              { type: "G", fs: "Income",    recon: false, party: false },
        "Cost of Sales":        { type: "G", fs: "Expense",   recon: false, party: false },
        "Expense":              { type: "G", fs: "Expense",   recon: false, party: false },
        "Tax Account":          { type: "S", fs: "Liability", recon: false, party: false }
    };

    LET $entry = $mapping[$cat];

    IF $entry = NONE {
        RETURN { error: string::concat("Unknown account_category: ", $cat) };
    };

    UPDATE $doc_id SET
        account_type_internal    = $entry.type,
        fs_type                  = $entry.fs,
        is_reconciliation_group  = $entry.recon,
        party_required           = $entry.party;

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::account_group_number_range(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT group_name, chart_of_accounts, number_from, number_to FROM $doc_id)[0];

    IF $doc.number_from = NONE OR $doc.number_from = "" {
        RETURN { error: "Number From is required" };
    };

    IF $doc.number_to = NONE OR $doc.number_to = "" {
        RETURN { error: "Number To is required" };
    };

    IF $doc.number_from >= $doc.number_to {
        RETURN { error: "Number From must be less than Number To" };
    };

    -- Check for overlap with other groups in the same CoA
    LET $overlap = (
        SELECT name FROM tabAccount_Group
        WHERE chart_of_accounts = $doc.chart_of_accounts
          AND name != $doc.group_name
          AND number_from <= $doc.number_to
          AND number_to   >= $doc.number_from
        LIMIT 1
    )[0];

    IF $overlap != NONE {
        RETURN { error: string::concat("Number range overlaps with Account Group '", <string>$overlap.name, "'") };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::account_group_code_unique(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT group_name, code, chart_of_accounts FROM $doc_id)[0];

    LET $existing = (
        SELECT name FROM tabAccount_Group
        WHERE code = $doc.code
          AND chart_of_accounts = $doc.chart_of_accounts
          AND name != $doc.group_name
        LIMIT 1
    )[0];

    IF $existing != NONE {
        RETURN { error: string::concat("Code '", <string>$doc.code, "' is already used by group '", <string>$existing.name, "' in this Chart of Accounts") };
    };

    RETURN { ok: true };
};
