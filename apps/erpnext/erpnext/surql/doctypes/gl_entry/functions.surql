-- GL Entry — domain functions
-- gl_entry is a ledger record: it can be created (via on_submit of parent)
-- and it can be cancelled (via cascade from parent cancel).

-- ── fn::validate::gl_entry_accounts ──────────────────────────────────────────
-- Run on every GL entry creation (save/submit of the entry itself).
-- Validates account existence, not disabled, not a group account.
DEFINE FUNCTION OVERWRITE fn::validate::gl_entry_accounts(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT account, debit, credit, voucher_type FROM $doc_id)[0];

    IF $doc.account = NONE OR $doc.account = "" {
        RETURN { error: "Account is required on GL Entry" };
    };

    LET $acc = (
        SELECT is_group, disabled, root_type
        FROM tabAccount
        WHERE name = $doc.account
        LIMIT 1
    )[0];

    IF $acc = NONE {
        RETURN { error: string::concat("Account does not exist: ", <string>$doc.account) };
    };

    IF $acc.disabled = true OR $acc.disabled = 1 {
        RETURN { error: string::concat("Account is disabled: ", <string>$doc.account) };
    };

    IF $acc.is_group = true OR $acc.is_group = 1 {
        RETURN { error: string::concat("Cannot post to a group account: ", <string>$doc.account) };
    };

    -- Debit and credit must be non-negative
    IF ($doc.debit OR 0) < 0 {
        RETURN { error: "Debit amount cannot be negative" };
    };

    IF ($doc.credit OR 0) < 0 {
        RETURN { error: "Credit amount cannot be negative" };
    };

    RETURN { ok: true };
};

-- ── fn::on_cancel::gl_entry_reverse ──────────────────────────────────────────
-- Reverses a GL entry on cancel: swaps debit/credit, marks cancelled.
DEFINE FUNCTION OVERWRITE fn::on_cancel::gl_entry_reverse(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT debit, credit FROM $doc_id)[0];

    UPDATE $doc_id SET
        debit     = $doc.credit,
        credit    = $doc.debit,
        docstatus = 2,
        status    = "Cancelled",
        is_cancelled = true;

    RETURN { ok: true };
};
