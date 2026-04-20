-- Fiscal Year — domain validation functions

DEFINE FUNCTION OVERWRITE fn::validate::fiscal_year_dates(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT year_start_date, year_end_date FROM $doc_id)[0];

    IF $doc.year_start_date = NONE {
        RETURN { error: "Year Start Date is required" };
    };

    IF $doc.year_end_date = NONE {
        RETURN { error: "Year End Date is required" };
    };

    IF $doc.year_end_date <= $doc.year_start_date {
        RETURN { error: "Year End Date must be after Year Start Date" };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::fiscal_year_no_overlap(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT year_name, company, year_start_date, year_end_date FROM $doc_id)[0];

    -- Overlapping = another FY for same company whose range intersects ours
    LET $overlap = (
        SELECT name FROM tabFiscal_Year
        WHERE company = $doc.company
          AND name != $doc.year_name
          AND year_start_date <= $doc.year_end_date
          AND year_end_date   >= $doc.year_start_date
        LIMIT 1
    )[0];

    IF $overlap != NONE {
        RETURN { error: string::concat("Fiscal Year '", <string>$overlap.name, "' overlaps with the dates entered") };
    };

    RETURN { ok: true };
};
