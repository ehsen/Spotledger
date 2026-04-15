-- Universal node: fn::validate::mandatory_fields
--
-- Checks that all fields marked reqd=1 in tabDocField are present and non-empty.
-- No per-doctype config required — the function derives the doctype from the
-- record table name (strips the "tab" prefix) and queries the live field graph.
--
-- This is the "DB owns the schema" principle: the required field list lives in
-- tabDocField (seeded from DocType JSON at install time), not in wiring edges.
DEFINE FUNCTION OVERWRITE fn::validate::mandatory_fields(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT * FROM $doc_id)[0];

    -- Derive Frappe doctype name from SurrealDB table name (strip "tab" prefix).
    -- e.g. meta::tb(tabAccount:ACC-001) → "tabAccount" → "Account"
    LET $table   = <string>meta::tb($doc_id);
    LET $doctype = string::slice($table, 3, 999);

    -- Query required fields from the live field graph.
    -- tabDocField.parent = Frappe doctype name, reqd = 1 means mandatory.
    LET $fields = (
        SELECT fieldname FROM tabDocField
        WHERE parent = $doctype AND reqd = 1
    ).fieldname;

    FOR $f IN $fields {
        LET $val = $doc[$f];
        IF $val = NONE OR $val = NULL OR $val = "" OR $val = [] {
            RETURN {
                error: string::concat(
                    string::uppercase(string::slice(<string>$f, 0, 1)),
                    string::slice(<string>$f, 1, 999),
                    " is mandatory"
                )
            };
        };
    };

    RETURN { ok: true };
};
