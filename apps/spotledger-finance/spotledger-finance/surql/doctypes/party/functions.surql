-- Party — domain functions

DEFINE FUNCTION OVERWRITE fn::validate::party_roles(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT party_name, roles FROM $doc_id)[0];

    IF $doc.roles = NONE OR array::len($doc.roles) = 0 {
        RETURN { error: "Party must have at least one role assigned" };
    };

    LET $valid_roles = ["Customer", "Supplier", "Employee", "Shareholder", "Director", "Franchisee", "Bank", "Tax Authority", "Government Office"];

    FOR $role_obj IN $doc.roles {
        LET $role = <string>$role_obj.role;
        IF $role NOT IN $valid_roles {
            RETURN { error: string::concat("Invalid role: '", $role, "'. Valid roles: Customer, Supplier, Employee, Shareholder, Director, Franchisee, Bank, Tax Authority, Government Office") };
        };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::validate::party_tax_id_unique(
    $doc_id: record,
    $config: object
) {
    LET $doc = (SELECT party_name, tax_id FROM $doc_id)[0];

    IF $doc.tax_id = NONE OR $doc.tax_id = "" {
        RETURN { ok: true }; -- tax_id is optional
    };

    LET $existing = (
        SELECT name FROM tabParty
        WHERE tax_id = $doc.tax_id
          AND name != $doc.party_name
        LIMIT 1
    )[0];

    IF $existing != NONE {
        RETURN { error: string::concat("Tax ID '", <string>$doc.tax_id, "' is already registered under party '", <string>$existing.name, "'") };
    };

    RETURN { ok: true };
};

DEFINE FUNCTION OVERWRITE fn::on_save::party_role_config_autocreate(
    $doc_id: record,
    $config: object
) {
    -- For each role on this party, ensure a Party Role Config record exists per company.
    -- We only auto-create for companies that don't have one yet.
    -- The PRC record is created with minimal data; the user can fill in account details later.
    LET $doc = (SELECT party_name, roles FROM $doc_id)[0];

    IF $doc.roles = NONE OR array::len($doc.roles) = 0 {
        RETURN { ok: true };
    };

    LET $companies = (SELECT name FROM tabCompany LIMIT 100);

    FOR $role_obj IN $doc.roles {
        LET $role = <string>$role_obj.role;
        FOR $company IN $companies {
            LET $co_name = $company.name;
            LET $exists = (
                SELECT id FROM tabParty_Role_Config
                WHERE party = $doc.party_name
                  AND role  = $role
                  AND company = $co_name
                LIMIT 1
            )[0];

            IF $exists = NONE {
                INSERT INTO tabParty_Role_Config {
                    party:   $doc.party_name,
                    role:    $role,
                    company: $co_name
                };
            };
        };
    };

    RETURN { ok: true };
};
