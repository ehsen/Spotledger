-- Cost Center — domain functions

DEFINE FUNCTION OVERWRITE fn::validate::cost_center_code_unique(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT cost_center_name, cost_center_code, company FROM $doc_id)[0];

    IF $doc.cost_center_code = NONE OR $doc.cost_center_code = "" {
        RETURN { error: "Cost Center Code is required" };
    };

    LET $existing = (
        SELECT name FROM tabCost_Center
        WHERE cost_center_code = $doc.cost_center_code
          AND company = $doc.company
          AND name != $doc.cost_center_name
        LIMIT 1
    )[0];

    IF $existing != NONE {
        RETURN { error: string::concat("Cost Center Code '", <string>$doc.cost_center_code, "' is already used in this company") };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::on_save::cost_center_relate_parent(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT cost_center_name, parent_cost_center FROM $doc_id)[0];

    -- Remove existing parent edge
    DELETE (SELECT id FROM child_of WHERE in = $doc_id);

    -- Create new parent edge if parent is set
    IF $doc.parent_cost_center != NONE AND $doc.parent_cost_center != "" {
        LET $parent_ref = type::record("tabCost_Center", $doc.parent_cost_center);
        RELATE $doc_id -> child_of -> $parent_ref;
    };

    RETURN { ok: true };
};
