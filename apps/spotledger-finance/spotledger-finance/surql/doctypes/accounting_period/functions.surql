-- Accounting Period — domain validation functions

DEFINE FUNCTION OVERWRITE fn::validate::accounting_period_dates(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT start_date, end_date FROM $doc_id)[0];

    IF $doc.start_date = NONE {
        RETURN { error: "Start Date is required" };
    };

    IF $doc.end_date = NONE {
        RETURN { error: "End Date is required" };
    };

    IF $doc.end_date <= $doc.start_date {
        RETURN { error: "End Date must be after Start Date" };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::accounting_period_in_fiscal_year(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT period_name, company, start_date, end_date FROM $doc_id)[0];

    IF $doc.company = NONE OR $doc.company = "" {
        RETURN { ok: true }; -- company not set yet, skip
    };

    -- The period dates must fall within at least one Fiscal Year for this company
    LET $fy = (
        SELECT name FROM tabFiscal_Year
        WHERE company = $doc.company
          AND year_start_date <= $doc.start_date
          AND year_end_date   >= $doc.end_date
        LIMIT 1
    )[0];

    IF $fy = NONE {
        RETURN { error: "Accounting Period dates must fall within an existing Fiscal Year for this company" };
    };

    -- Check for overlap with another period in the same company
    LET $overlap = (
        SELECT name FROM tabAccounting_Period
        WHERE company = $doc.company
          AND name != $doc.period_name
          AND start_date <= $doc.end_date
          AND end_date   >= $doc.start_date
        LIMIT 1
    )[0];

    IF $overlap != NONE {
        RETURN { error: string::concat("Accounting Period '", <string>$overlap.name, "' overlaps with the dates entered") };
    };

    RETURN { ok: true };
};
