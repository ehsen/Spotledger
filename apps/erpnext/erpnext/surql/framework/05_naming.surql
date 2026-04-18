-- Naming Functions — Spotledger Framework
--
-- Implements all Frappe naming conventions purely in SurrealDB.
-- Rust becomes a thin caller: it calls fn::naming::resolve($doctype, $doc)
-- and uses the returned string as the record key.
--
-- Naming priority (mirrors Frappe meta.autoname logic):
--   1. Explicit name in doc            → use as-is
--   2. naming_series field in doc      → expand series template + increment counter
--   3. Admin override: tabDocumentNamingRule (document_type = doctype)
--   4. Default: tabDocType.autoname
--   5. UUID fallback
--
-- Supported autoname patterns:
--   "field:fieldname"     → value of doc.fieldname
--   "naming_series:"      → use naming_series field from doc
--   "format:TEMPLATE"     → treat TEMPLATE as series (e.g. "format:AUT-AR-{#####}")
--   "Prompt"/"hash"/"UUID"→ UUID fallback (user-entered or random)
--   "TEMPLATE"            → direct series (e.g. "EV.#####", "SO-.YYYY.-.####")
--
-- Series template tokens:
--   YYYY → 4-digit year    YY → 2-digit year
--   MM   → 2-digit month   DD → 2-digit day
--   .    → separator (stripped from output prefix)
--   #    → one digit position (group of # at end sets zero-pad width)

-- ── fn::naming::uuid_name ─────────────────────────────────────────────────────
-- Returns a random fallback name when no naming rule applies.
DEFINE FUNCTION OVERWRITE fn::naming::uuid_name() {
    RETURN "new-" + <string>rand::uuid::v4();
};

-- ── fn::naming::next_series ───────────────────────────────────────────────────
-- Expands date tokens in a naming series template, then atomically
-- increments the counter in tabSeries and returns the formatted name.
--
-- Examples:
--   fn::naming::next_series("SO-.YYYY.-.####") → "SO-2025-0001"
--   fn::naming::next_series("EMP-.YYYY.-#####") → "EMP-2025-00001"
--   fn::naming::next_series("CUST-####")        → "CUST-0001"
DEFINE FUNCTION OVERWRITE fn::naming::next_series($series: string) {
    LET $now = time::now();

    -- Expand date tokens sequentially
    LET $s1 = string::replace($series, "YYYY", time::format($now, "%Y"));
    LET $s2 = string::replace($s1,     "YY",   time::format($now, "%y"));
    LET $s3 = string::replace($s2,     "MM",   time::format($now, "%m"));
    LET $s4 = string::replace($s3,     "DD",   time::format($now, "%d"));

    -- Split on "#" to extract:
    --   parts[0]       = everything before the first "#" (the prefix template)
    --   len(parts) - 1 = number of "#" characters (zero-pad width)
    -- e.g. "SO-2025-.####" → ["SO-2025-.", "", "", "", ""] → 4 hashes
    LET $parts      = string::split($s4, "#");
    LET $hash_count = IF array::len($parts) > 1 { array::len($parts) - 1 } ELSE { 1 };

    -- Remove dots from prefix (dots are visual separators only in Frappe series)
    LET $prefix = string::replace($parts[0], ".", "");

    -- Atomically increment the counter row; create on first use
    LET $updated = (
        UPDATE tabSeries SET current = current + 1
        WHERE name = $prefix
        RETURN AFTER
    );

    LET $counter = IF array::len($updated) > 0 {
        $updated[0].current
    } ELSE {
        -- First use for this prefix: insert with current = 1
        (INSERT INTO tabSeries (name, current) VALUES ($prefix, 1));
        1
    };

    -- Zero-pad the counter to hash_count width (supports up to 8 digits)
    LET $num_str   = <string>$counter;
    LET $pad_zeros = "00000000";
    LET $pad_len   = IF $hash_count > string::len($num_str) {
        $hash_count - string::len($num_str)
    } ELSE { 0 };
    LET $padding   = string::slice($pad_zeros, 0, $pad_len);

    RETURN $prefix + $padding + $num_str;
};

-- ── fn::naming::apply_rule ────────────────────────────────────────────────────
-- Applies a single autoname rule string to the document and returns a name.
-- $rule — autoname value from tabDocType or tabDocumentNamingRule
-- $doc  — full document object (field values accessible as $doc.fieldname)
DEFINE FUNCTION OVERWRITE fn::naming::apply_rule($rule: string, $doc: object) {
    -- field:fieldname — use the value of a specific field as the document name
    IF string::starts_with($rule, "field:") {
        LET $fieldname = string::slice($rule, 6, 999);
        -- Check the raw value first: <string>(NONE) = "NONE" in SurrealDB, which
        -- would silently be accepted by the != NONE check below.
        LET $raw_val = $doc[$fieldname];
        IF $raw_val != NONE AND <string>$raw_val != "" {
            RETURN <string>$raw_val;
        };
        -- Field is missing or empty — reject with a clear message so the user
        -- knows which field to fill in, rather than silently generating a random name.
        THROW string::concat("Field \"" , $fieldname, "\" is required for naming but is empty or missing.");
    };

    -- naming_series: — the naming series is chosen from the doc itself
    IF string::starts_with($rule, "naming_series:") {
        IF $doc.naming_series != NONE AND $doc.naming_series != "" {
            RETURN fn::naming::next_series(<string>$doc.naming_series);
        };
        RETURN fn::naming::uuid_name();
    };

    -- format:TEMPLATE — expand the template as a naming series
    -- (used by automation rules like "format:AUT-AR-{#####}")
    IF string::starts_with($rule, "format:") {
        LET $template = string::replace(
            string::replace(
                string::slice($rule, 7, 999),
                "{", ""
            ),
            "}", ""
        );
        RETURN fn::naming::next_series($template);
    };

    -- Prompt / hash / UUID — no auto-name; return UUID fallback
    IF $rule IN ["Prompt", "hash", "UUID"] {
        RETURN fn::naming::uuid_name();
    };

    -- Empty rule — UUID fallback
    IF $rule = "" {
        RETURN fn::naming::uuid_name();
    };

    -- Direct series template (e.g. "EV.#####", "INV-.####")
    RETURN fn::naming::next_series($rule);
};

-- ── fn::naming::resolve ───────────────────────────────────────────────────────
-- Resolves the final name for a new document.
-- Rust calls this before INSERT so the record key is known.
--
-- Priority:
--   1. Explicit name already in $doc.name
--   2. naming_series field in $doc
--   3. Admin override from tabDocumentNamingRule
--   4. Default autoname/naming_rule from tabDocType
--   5. UUID fallback
DEFINE FUNCTION OVERWRITE fn::naming::resolve($doctype: string, $doc: object) {
    -- 1. Explicit name provided by the client or already set
    IF $doc.name != NONE AND <string>$doc.name != "" {
        RETURN <string>$doc.name;
    };

    -- 2. naming_series field chosen by the user in the form
    IF $doc.naming_series != NONE AND <string>$doc.naming_series != "" {
        RETURN fn::naming::next_series(<string>$doc.naming_series);
    };

    -- 3. Admin override: tabDocumentNamingRule (highest-priority admin config)
    LET $nr = (
        SELECT autoname FROM tabDocumentNamingRule
        WHERE document_type = $doctype
        LIMIT 1
    )[0];
    IF $nr != NONE AND $nr.autoname != NONE AND <string>$nr.autoname != "" {
        RETURN fn::naming::apply_rule(<string>$nr.autoname, $doc);
    };

    -- 4. Default autoname + naming_rule from tabDocType
    LET $dt = (
        SELECT autoname, naming_rule FROM tabDocType
        WHERE name = $doctype
        LIMIT 1
    )[0];
    IF $dt != NONE {
        -- Frappe v15: naming_rule = "By fieldname" → autoname IS the field name directly
        -- (contrast with old style: autoname = "field:employee_name")
        IF $dt.naming_rule != NONE AND <string>$dt.naming_rule = 'By fieldname' {
            IF $dt.autoname != NONE AND <string>$dt.autoname != "" {
                LET $fieldname = <string>$dt.autoname;
                -- Check raw value before casting: <string>(NONE) = "NONE" in SurrealDB,
                -- which would silently pass the != NONE check and store "NONE" as the name.
                LET $raw_val = $doc[$fieldname];
                IF $raw_val != NONE AND <string>$raw_val != "" { RETURN <string>$raw_val; };
                -- Field is missing or empty — tell the user exactly what to fill in.
                THROW string::concat("Field \"", $fieldname, "\" is required for naming but is empty or missing.");
            };
            -- autoname itself is blank — this is a designer misconfiguration.
            THROW string::concat("DocType \"", $doctype, "\" has naming_rule = 'By fieldname' but autoname is not set.");
        };
        -- Standard apply_rule for all other autoname patterns
        IF $dt.autoname != NONE AND <string>$dt.autoname != "" {
            RETURN fn::naming::apply_rule(<string>$dt.autoname, $doc);
        };
    };

    -- 5. UUID fallback
    RETURN fn::naming::uuid_name();
};
