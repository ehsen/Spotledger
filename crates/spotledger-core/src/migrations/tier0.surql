-- Tier 0 SurrealQL Migrations
-- Core metadata and user management tables for Spotledger

-- ============================================================================
-- DocType Management (System DocTypes that define schema)
-- ============================================================================

DEFINE TABLE DocType SCHEMAFULL;
DEFINE FIELD name ON DocType TYPE string ASSERT $value != NONE;
DEFINE FIELD module ON DocType TYPE string;
DEFINE FIELD issingle ON DocType TYPE int;
DEFINE FIELD istree ON DocType TYPE int;
DEFINE FIELD issubmittable ON DocType TYPE int;
DEFINE FIELD custom ON DocType TYPE int;
DEFINE FIELD is_child_table ON DocType TYPE int;
DEFINE FIELD track_changes ON DocType TYPE int;
DEFINE FIELD title_field ON DocType TYPE option<string>;
DEFINE FIELD search_fields ON DocType TYPE option<string>;
DEFINE FIELD sort_field ON DocType TYPE option<string>;
DEFINE FIELD sort_order ON DocType TYPE option<string>;
DEFINE FIELD autoname ON DocType TYPE option<string>;
DEFINE FIELD naming_series ON DocType TYPE option<string>;
DEFINE FIELD fields ON DocType TYPE array<object>;
DEFINE FIELD permissions ON DocType TYPE array<object>;
DEFINE FIELD owner ON DocType TYPE option<string>;
DEFINE FIELD creation ON DocType TYPE option<datetime>;
DEFINE FIELD modified ON DocType TYPE option<datetime>;
DEFINE FIELD modified_by ON DocType TYPE option<string>;

CREATE UNIQUE INDEX idx_doctype_name ON DocType (name);

DEFINE TABLE DocField SCHEMAFULL;
DEFINE FIELD parent ON DocField TYPE string;
DEFINE FIELD parenttype ON DocField TYPE string;
DEFINE FIELD parentfield ON DocField TYPE string;
DEFINE FIELD idx ON DocField TYPE int;
DEFINE FIELD fieldname ON DocField TYPE string ASSERT $value != NONE;
DEFINE FIELD label ON DocField TYPE string ASSERT $value != NONE;
DEFINE FIELD fieldtype ON DocField TYPE string;
DEFINE FIELD options ON DocField TYPE option<string>;
DEFINE FIELD reqd ON DocField TYPE int;
DEFINE FIELD unique ON DocField TYPE int;
DEFINE FIELD read_only ON DocField TYPE int;
DEFINE FIELD hidden ON DocField TYPE int;
DEFINE FIELD in_list_view ON DocField TYPE int;
DEFINE FIELD in_standard_filter ON DocField TYPE int;
DEFINE FIELD bold ON DocField TYPE int;
DEFINE FIELD default_value ON DocField TYPE option<string>;
DEFINE FIELD description ON DocField TYPE option<string>;
DEFINE FIELD set_only_once ON DocField TYPE int;
DEFINE FIELD allow_on_submit ON DocField TYPE int;
DEFINE FIELD ignore_xss_filter ON DocField TYPE int;
DEFINE FIELD permlevel ON DocField TYPE int;
DEFINE FIELD fetch_from ON DocField TYPE option<string>;
DEFINE FIELD fetch_if_empty ON DocField TYPE int;
DEFINE FIELD precision ON DocField TYPE option<int>;
DEFINE FIELD length ON DocField TYPE option<int>;

CREATE INDEX idx_docfield_parent ON DocField (parent);

DEFINE TABLE DocPerm SCHEMAFULL;
DEFINE FIELD parent ON DocPerm TYPE string;
DEFINE FIELD parenttype ON DocPerm TYPE string;
DEFINE FIELD parentfield ON DocPerm TYPE string;
DEFINE FIELD idx ON DocPerm TYPE int;
DEFINE FIELD role ON DocPerm TYPE string ASSERT $value != NONE;
DEFINE FIELD permlevel ON DocPerm TYPE int;
DEFINE FIELD read ON DocPerm TYPE int;
DEFINE FIELD write ON DocPerm TYPE int;
DEFINE FIELD create ON DocPerm TYPE int;
DEFINE FIELD delete ON DocPerm TYPE int;
DEFINE FIELD submit ON DocPerm TYPE int;
DEFINE FIELD cancel ON DocPerm TYPE int;
DEFINE FIELD amend ON DocPerm TYPE int;
DEFINE FIELD report ON DocPerm TYPE int;
DEFINE FIELD import ON DocPerm TYPE int;
DEFINE FIELD export ON DocPerm TYPE int;
DEFINE FIELD print ON DocPerm TYPE int;
DEFINE FIELD email ON DocPerm TYPE int;
DEFINE FIELD share ON DocPerm TYPE int;
DEFINE FIELD if_owner ON DocPerm TYPE int;

CREATE INDEX idx_docperm_parent ON DocPerm (parent);

DEFINE TABLE CustomField SCHEMAFULL;
DEFINE FIELD dt ON CustomField TYPE string ASSERT $value != NONE;
DEFINE FIELD fieldname ON CustomField TYPE string ASSERT $value != NONE;
DEFINE FIELD label ON CustomField TYPE option<string>;
DEFINE FIELD fieldtype ON CustomField TYPE option<string>;
DEFINE FIELD options ON CustomField TYPE option<string>;
DEFINE FIELD reqd ON CustomField TYPE int;
DEFINE FIELD unique ON CustomField TYPE int;
DEFINE FIELD read_only ON CustomField TYPE int;
DEFINE FIELD hidden ON CustomField TYPE int;
DEFINE FIELD in_list_view ON CustomField TYPE int;
DEFINE FIELD default_value ON CustomField TYPE option<string>;
DEFINE FIELD description ON CustomField TYPE option<string>;
DEFINE FIELD owner ON CustomField TYPE option<string>;
DEFINE FIELD creation ON CustomField TYPE option<datetime>;
DEFINE FIELD modified ON CustomField TYPE option<datetime>;
DEFINE FIELD modified_by ON CustomField TYPE option<string>;

CREATE INDEX idx_customfield_dt ON CustomField (dt);

DEFINE TABLE PropertySetter SCHEMAFULL;
DEFINE FIELD doctype_or_field ON PropertySetter TYPE string;
DEFINE FIELD doc_type ON PropertySetter TYPE string ASSERT $value != NONE;
DEFINE FIELD field_name ON PropertySetter TYPE option<string>;
DEFINE FIELD property ON PropertySetter TYPE string ASSERT $value != NONE;
DEFINE FIELD value ON PropertySetter TYPE string ASSERT $value != NONE;
DEFINE FIELD old_value ON PropertySetter TYPE option<string>;
DEFINE FIELD owner ON PropertySetter TYPE option<string>;
DEFINE FIELD creation ON PropertySetter TYPE option<datetime>;
DEFINE FIELD modified ON PropertySetter TYPE option<datetime>;
DEFINE FIELD modified_by ON PropertySetter TYPE option<string>;

CREATE INDEX idx_propertysetter_doc_type ON PropertySetter (doc_type);

-- ============================================================================
-- User Management
-- ============================================================================

DEFINE TABLE Role SCHEMAFULL;
DEFINE FIELD role_name ON Role TYPE string ASSERT $value != NONE;
DEFINE FIELD disabled ON Role TYPE int;
DEFINE FIELD desk_access ON Role TYPE int;
DEFINE FIELD description ON Role TYPE option<string>;
DEFINE FIELD role_type ON Role TYPE option<string>;
DEFINE FIELD owner ON Role TYPE option<string>;
DEFINE FIELD creation ON Role TYPE option<datetime>;
DEFINE FIELD modified ON Role TYPE option<datetime>;
DEFINE FIELD modified_by ON Role TYPE option<string>;

CREATE UNIQUE INDEX idx_role_name ON Role (role_name);

DEFINE TABLE UserType SCHEMAFULL;
DEFINE FIELD name ON UserType TYPE string ASSERT $value != NONE;
DEFINE FIELD description ON UserType TYPE option<string>;
DEFINE FIELD user_type_name ON UserType TYPE string ASSERT $value != NONE;
DEFINE FIELD owner ON UserType TYPE option<string>;
DEFINE FIELD creation ON UserType TYPE option<datetime>;
DEFINE FIELD modified ON UserType TYPE option<datetime>;
DEFINE FIELD modified_by ON UserType TYPE option<string>;

CREATE UNIQUE INDEX idx_usertype_name ON UserType (name);

DEFINE TABLE User SCHEMAFULL;
DEFINE FIELD name ON User TYPE string ASSERT $value != NONE;
DEFINE FIELD email ON User TYPE string ASSERT $value != NONE;
DEFINE FIELD first_name ON User TYPE option<string>;
DEFINE FIELD last_name ON User TYPE option<string>;
DEFINE FIELD full_name ON User TYPE option<string>;
DEFINE FIELD user_type ON User TYPE option<string>;
DEFINE FIELD roles ON User TYPE array<object>;
DEFINE FIELD permissions ON User TYPE array<object>;
DEFINE FIELD enabled ON User TYPE int DEFAULT 1;
DEFINE FIELD password ON User TYPE option<string>;
DEFINE FIELD phone ON User TYPE option<string>;
DEFINE FIELD mobile_no ON User TYPE option<string>;
DEFINE FIELD bio ON User TYPE option<string>;
DEFINE FIELD mute_sounds ON User TYPE int;
DEFINE FIELD send_welcome_email ON User TYPE int;
DEFINE FIELD owner ON User TYPE option<string>;
DEFINE FIELD creation ON User TYPE option<datetime>;
DEFINE FIELD modified ON User TYPE option<datetime>;
DEFINE FIELD modified_by ON User TYPE option<string>;

CREATE UNIQUE INDEX idx_user_email ON User (email);
CREATE INDEX idx_user_name ON User (name);

DEFINE TABLE HasRole SCHEMAFULL;
DEFINE FIELD parent ON HasRole TYPE string;
DEFINE FIELD parenttype ON HasRole TYPE string;
DEFINE FIELD parentfield ON HasRole TYPE string;
DEFINE FIELD idx ON HasRole TYPE int;
DEFINE FIELD role ON HasRole TYPE string ASSERT $value != NONE;

CREATE INDEX idx_hasrole_parent ON HasRole (parent);

DEFINE TABLE UserPermission SCHEMAFULL;
DEFINE FIELD parent ON UserPermission TYPE string;
DEFINE FIELD parenttype ON UserPermission TYPE string;
DEFINE FIELD parentfield ON UserPermission TYPE string;
DEFINE FIELD idx ON UserPermission TYPE int;
DEFINE FIELD user ON UserPermission TYPE string ASSERT $value != NONE;
DEFINE FIELD ptype ON UserPermission TYPE string ASSERT $value != NONE;
DEFINE FIELD doc ON UserPermission TYPE string ASSERT $value != NONE;
DEFINE FIELD when_shared ON UserPermission TYPE option<datetime>;

CREATE INDEX idx_userpermission_parent ON UserPermission (parent);
CREATE INDEX idx_userpermission_user ON UserPermission (user);

DEFINE TABLE UserGroup SCHEMAFULL;
DEFINE FIELD name ON UserGroup TYPE string ASSERT $value != NONE;
DEFINE FIELD description ON UserGroup TYPE option<string>;
DEFINE FIELD users ON UserGroup TYPE array<object>;
DEFINE FIELD owner ON UserGroup TYPE option<string>;
DEFINE FIELD creation ON UserGroup TYPE option<datetime>;
DEFINE FIELD modified ON UserGroup TYPE option<datetime>;
DEFINE FIELD modified_by ON UserGroup TYPE option<string>;

CREATE UNIQUE INDEX idx_usergroup_name ON UserGroup (name);

DEFINE TABLE UserGroupMember SCHEMAFULL;
DEFINE FIELD parent ON UserGroupMember TYPE string;
DEFINE FIELD parenttype ON UserGroupMember TYPE string;
DEFINE FIELD parentfield ON UserGroupMember TYPE string;
DEFINE FIELD idx ON UserGroupMember TYPE int;
DEFINE FIELD user ON UserGroupMember TYPE string ASSERT $value != NONE;

CREATE INDEX idx_usergroupmember_parent ON UserGroupMember (parent);

-- ============================================================================
-- System Configuration
-- ============================================================================

DEFINE TABLE SystemSettings SCHEMAFULL;
DEFINE FIELD name ON SystemSettings TYPE string ASSERT $value != NONE;
DEFINE FIELD setup_complete ON SystemSettings TYPE int;
DEFINE FIELD system_default_locale ON SystemSettings TYPE option<string>;
DEFINE FIELD country ON SystemSettings TYPE option<string>;
DEFINE FIELD timezone ON SystemSettings TYPE option<string> DEFAULT "UTC";
DEFINE FIELD enable_password_user_creation ON SystemSettings TYPE int;
DEFINE FIELD disable_user_import ON SystemSettings TYPE int;
DEFINE FIELD allow_on_submit ON SystemSettings TYPE int;
DEFINE FIELD incomplete_days ON SystemSettings TYPE option<int>;
DEFINE FIELD setup_wizard_redirect ON SystemSettings TYPE int;
DEFINE FIELD mail_server ON SystemSettings TYPE option<string>;
DEFINE FIELD mail_port ON SystemSettings TYPE option<int>;
DEFINE FIELD use_ssl ON SystemSettings TYPE int;
DEFINE FIELD mail_login ON SystemSettings TYPE option<string>;
DEFINE FIELD mail_password ON SystemSettings TYPE option<string>;
DEFINE FIELD auto_email_id ON SystemSettings TYPE option<string>;
DEFINE FIELD default_company ON SystemSettings TYPE option<string>;
DEFINE FIELD default_currency ON SystemSettings TYPE option<string>;
DEFINE FIELD default_doc_title ON SystemSettings TYPE option<string>;
DEFINE FIELD enable_two_factor_auth ON SystemSettings TYPE int;
DEFINE FIELD file_upload_size_limit ON SystemSettings TYPE option<int>;
DEFINE FIELD session_expiry_timeout ON SystemSettings TYPE option<int>;
DEFINE FIELD owner ON SystemSettings TYPE option<string>;
DEFINE FIELD creation ON SystemSettings TYPE option<datetime>;
DEFINE FIELD modified ON SystemSettings TYPE option<datetime>;
DEFINE FIELD modified_by ON SystemSettings TYPE option<string>;

CREATE UNIQUE INDEX idx_systemsettings_name ON SystemSettings (name);

DEFINE TABLE DefaultValue SCHEMAFULL;
DEFINE FIELD parenttype ON DefaultValue TYPE string ASSERT $value != NONE;
DEFINE FIELD fieldname ON DefaultValue TYPE string ASSERT $value != NONE;
DEFINE FIELD value ON DefaultValue TYPE option<string>;
DEFINE FIELD defvalue ON DefaultValue TYPE option<string>;
DEFINE FIELD user ON DefaultValue TYPE option<string>;
DEFINE FIELD is_standard ON DefaultValue TYPE int;
DEFINE FIELD owner ON DefaultValue TYPE option<string>;
DEFINE FIELD creation ON DefaultValue TYPE option<datetime>;
DEFINE FIELD modified ON DefaultValue TYPE option<datetime>;
DEFINE FIELD modified_by ON DefaultValue TYPE option<string>;

CREATE INDEX idx_defaultvalue_parenttype ON DefaultValue (parenttype);

-- ============================================================================
-- Document Naming
-- ============================================================================

DEFINE TABLE DocumentNamingRule SCHEMAFULL;
DEFINE FIELD name ON DocumentNamingRule TYPE string ASSERT $value != NONE;
DEFINE FIELD doctype ON DocumentNamingRule TYPE string ASSERT $value != NONE;
DEFINE FIELD rule_type ON DocumentNamingRule TYPE string ASSERT $value != NONE;
DEFINE FIELD field_value ON DocumentNamingRule TYPE option<string>;
DEFINE FIELD expression ON DocumentNamingRule TYPE option<string>;
DEFINE FIELD series_pattern ON DocumentNamingRule TYPE option<string>;
DEFINE FIELD next_id ON DocumentNamingRule TYPE int;
DEFINE FIELD is_default ON DocumentNamingRule TYPE int;
DEFINE FIELD is_standard ON DocumentNamingRule TYPE int;
DEFINE FIELD docstatus ON DocumentNamingRule TYPE int DEFAULT 0;
DEFINE FIELD owner ON DocumentNamingRule TYPE option<string>;
DEFINE FIELD creation ON DocumentNamingRule TYPE option<datetime>;
DEFINE FIELD modified ON DocumentNamingRule TYPE option<datetime>;
DEFINE FIELD modified_by ON DocumentNamingRule TYPE option<string>;

CREATE INDEX idx_docnamingrule_doctype ON DocumentNamingRule (doctype);

DEFINE TABLE DocumentNamingSettings SCHEMAFULL;
DEFINE FIELD name ON DocumentNamingSettings TYPE string ASSERT $value != NONE;
DEFINE FIELD prefix_series_based_on_document_field ON DocumentNamingSettings TYPE int;
DEFINE FIELD apply_series_on_submit_only ON DocumentNamingSettings TYPE int;
DEFINE FIELD allow_custom_naming ON DocumentNamingSettings TYPE int;
DEFINE FIELD allow_duplicate_series ON DocumentNamingSettings TYPE int;
DEFINE FIELD naming_rules ON DocumentNamingSettings TYPE array<object>;
DEFINE FIELD owner ON DocumentNamingSettings TYPE option<string>;
DEFINE FIELD creation ON DocumentNamingSettings TYPE option<datetime>;
DEFINE FIELD modified ON DocumentNamingSettings TYPE option<datetime>;
DEFINE FIELD modified_by ON DocumentNamingSettings TYPE option<string>;

CREATE UNIQUE INDEX idx_docnamingsettings_name ON DocumentNamingSettings (name);
