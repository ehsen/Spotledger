# Spotledger Feature Parity Scope
**Date**: April 4, 2026  
**Basis**: 271 Frappe test files + 268 Python controllers + current Spotledger codebase analysis

---

## Executive Summary

The original project was API-level compatibility (Frappe REST API served by Axum/SurrealDB).  
The **real scope** is full feature-to-feature parity: every DocType, every controller method, every `frappe.*` namespace call, and a **PyO3 bridge** so existing Python apps can call into the Rust engine without modification.

This document catalogs every area. Items marked ✅ exist, 🔧 are stubbed/partial, ❌ are not started.

---

## 1. Test File Inventory (271 files)

### 1a. Core Framework Tests (`frappe/tests/` — 78 files)
| Test File | What It Covers | Priority |
|-----------|---------------|----------|
| `test_document.py` | `frappe.get_doc`, insert/save/update/delete, defaults, child tables, events | P0 |
| `test_base_document.py` | `BaseDocument` validation, fieldtype coercion, `as_dict()` | P0 |
| `test_db.py` | All `frappe.db.*` methods (1510 lines) | P0 |
| `test_db_query.py` | `frappe.db.sql`, `DatabaseQuery`, filters, joins (1456 lines) | P0 |
| `test_db_surrealdb.py` | SurrealDB-specific `frappe.database.surrealdb` layer | P0 |
| `test_api.py` | REST `/api/resource/*`, `/api/method/*`, auth cycle | P0 |
| `test_api_v2.py` | `/api/v2/document/*` (CRUD + run doc method) | P0 |
| `test_permissions.py` | Full permission matrix — role, user, DocPerm (780 lines) | P0 |
| `test_naming.py` | Naming series, autoname, make_autoname, parse_naming_series | P0 |
| `test_hooks.py` | Hook dispatching, wildcard, app-level hooks | P0 |
| `test_child_table.py` | Child table save/load/delete, row ordering | P0 |
| `test_auth.py` | Login, logout, guest, two-factor, session | P0 |
| `test_client.py` | `frappe.client.*` full suite | P0 |
| `test_docstatus.py` | Draft/Submit/Cancel/Amend lifecycle | P0 |
| `test_db_update.py` | Schema migration, `updatedb`, `add_column` | P1 |
| `test_rename_doc.py` | `frappe.rename_doc` — cascade FK updates | P1 |
| `test_query_builder.py` | `frappe.qb` (PyPika-based query builder) | P1 |
| `test_query.py` | `frappe.db.get_list` advanced filters | P1 |
| `test_defaults.py` | `frappe.db.set_default`, `get_default`, `DefaultValue` | P1 |
| `test_model_utils.py` | `meta.get_link_fields`, `is_single`, etc. | P1 |
| `test_helpers.py` | `frappe.utils.*` helper functions | P2 |
| `test_utils.py` | Large utils coverage (cint, flt, formatdate, etc.) | P2 |
| `test_boot.py` | `/api/method/frappe.utils.boot.get_boot_info` payload | P1 |
| `test_form_load.py` | `getdoc`, `getdoctype` response shape | P1 |
| `test_reportview.py` | `frappe.desk.reportview.*` | P1 |
| `test_listview.py` | List view settings, group by | P2 |
| `test_dynamic_links.py` | Dynamic link fields, `get_dynamic_link_map` | P2 |
| `test_linked_with.py` | `get_linked_docs`, submitted linked docs | P2 |
| `test_assign.py` | `frappe.desk.form.assign_to.*` | P2 |
| `test_seen.py` | Mark as seen, view logs | P2 |
| `test_document_locks.py` | `doc.lock()`, `doc.unlock()` | P2 |
| `test_nestedset.py` | Nested set trees (lft/rgt rebuild) | P2 |
| `test_scheduler.py` | Scheduled job dispatch | P3 |
| `test_background_jobs.py` | `frappe.enqueue`, RQ jobs | P3 |
| `test_email.py` | `frappe.sendmail`, email queue | P3 |
| `test_safe_exec.py` | Safe Python eval (server scripts) | P3 |
| `test_rate_limiter.py` | Rate limiting middleware | P3 |
| `test_oauth20.py` | OAuth2 authorization flow | P3 |
| `test_translate.py` | `frappe.translate`, `_()` function | P2 |
| `test_password.py` | Password hash, check, update | P1 |
| `test_search.py` | `frappe.desk.search.*`, awesomebar | P2 |
| `test_global_search.py` | Full-text global search | P3 |
| `test_sqlite_search.py` | SQLite FTS search backend | P3 |
| `test_caching.py` | `frappe.cache`, Redis cache layer | P2 |
| `test_recorder.py` | SQL recorder | P3 |
| `test_pdf.py` | PDF generation | P3 |
| `test_printview.py` | Print format rendering | P3 |
| `test_formatter.py` | Field value formatting | P2 |
| `test_permissions.py` | Full permissions (already listed) | P0 |
| `test_virtual_doctype.py` | Virtual doctype (no DB table) | P2 |
| `test_deferred_insert.py` | Deferred/batched inserts | P3 |
| `test_patches.py` | Patch runner | P3 |
| `test_monitor.py` | Performance monitoring | P3 |

### 1b. DocType-specific Tests (193 files across modules)
Grouped below by module area.

---

## 2. Framework API Surface — PyO3 Requirement

**Every method below** is called from Python app code. They MUST be implemented in Rust and **exposed via PyO3** so `import frappe; frappe.get_doc(...)` works against the Rust/SurrealDB backend.

### 2a. Top-level `frappe.*` namespace (frappe/__init__.py)
```
frappe.get_doc(doctype, name=None)          → fetch or new Document
frappe.new_doc(doctype)                     → new Document with defaults
frappe.get_all(doctype, ...)                → list query
frappe.get_list(doctype, ...)               → list query (alias)
frappe.get_value(doctype, name, fieldname)  → single field fetch
frappe.set_value(doctype, name, fieldname, value)
frappe.delete_doc(doctype, name)
frappe.rename_doc(doctype, old, new)
frappe.get_meta(doctype)                    → DocType meta object
frappe.get_last_doc(doctype, filters)
frappe.get_lazy_doc(doctype, name)          → lazy-loaded doc proxy
frappe.get_cached_doc(doctype, name)        → doc from cache
frappe.clear_cache(doctype=None)
frappe.delete_doc_if_exists(doctype, name)
frappe.has_permission(doctype, ptype, doc)
frappe.only_for(roles)
frappe.set_user(user)
frappe.get_user()
frappe.get_roles(user=None)
frappe.generate_hash(txt, length)
frappe.scrub(txt)
frappe.unscrub(txt)
frappe.get_module(modulename)
frappe.get_hooks(hook, app=None)
frappe.call(fn, *args, **kwargs)            → whitelisted method dispatch
frappe.whitelist(fn)                        → decorator to whitelist
frappe.enqueue(method, **kwargs)            → background job
frappe.sendmail(recipients, subject, message, ...)
frappe.share.add(doctype, name, user, ...)
frappe.share.get_users(doctype, name)
frappe.parse_json(json_str)
frappe.as_json(obj)
frappe.session                              → session object (user, sid)
frappe.flags                                → request flags dict
frappe.form_dict                            → request params
frappe.response                             → response object
frappe.local                                → thread-local state
frappe.conf                                 → site config
frappe._dict(d)                             → dot-access dict
frappe.ping()
frappe.mock(doctype)
frappe.safe_decode(s)
frappe.safe_eval(code, eval_globals)
frappe.get_app_path(app, *joins)
frappe.get_site_path(*joins)
frappe.get_module_path(module, *joins)
frappe.reload_doctype(doctype)
frappe.reload_doc(module, dt, dn)
frappe.init(site, sites_path)
frappe.connect(site)
frappe.destroy()
```

### 2b. `frappe.db.*` (Database API — all called from app code)
```
frappe.db.sql(query, values, as_dict)
frappe.db.get_value(doctype, filters, fieldname)
frappe.db.get_values(doctype, filters, fieldnames)
frappe.db.get_all(doctype, ...)
frappe.db.get_list(doctype, ...)
frappe.db.exists(doctype, filters)
frappe.db.count(doctype, filters)
frappe.db.set_value(doctype, name, field, value)
frappe.db.set_single_value(doctype, field, value)
frappe.db.get_single_value(doctype, field)
frappe.db.set_default(key, value, parent)
frappe.db.get_default(key, parent)
frappe.db.delete(doctype, filters)
frappe.db.truncate(doctype)
frappe.db.commit()
frappe.db.rollback()
frappe.db.savepoint(name)
frappe.db.after_commit(fn)
frappe.db.before_rollback(fn)
frappe.db.after_rollback(fn)
frappe.db.get_table_columns(doctype)
frappe.db.get_column_type(doctype, column)
frappe.db.get_table_columns_description(doctype)
frappe.db.get_tables()
frappe.db.rename_table(old, new)
frappe.db.updatedb(doctype)
frappe.db.add_index(doctype, fields)
frappe.db.bulk_update(doctype, conditions, items)
frappe.db.describe(doctype)
frappe.db.db_type                           → "surrealdb"
frappe.db.transaction_writes
frappe.db.last_query
frappe.db.get_next_sequence_val(seq)
frappe.db.create_sequence(name)
frappe.db.set_next_sequence_val(name, val)
frappe.db.connect()
frappe.db.sql_ddl(query)
frappe.db.change_column_type(doctype, col, type)
frappe.db.set_execution_timeout(val)
```

### 2c. `frappe.model.Document` object API
```
doc.insert(ignore_permissions=False)
doc.save(ignore_permissions=False)
doc.submit()
doc.cancel()
doc.delete()
doc.reload()
doc.get(fieldname)
doc.set(fieldname, value)
doc.append(fieldname, row)
doc.get_doc_before_save()
doc.validate()                  → override hook
doc.before_insert()             → override hook
doc.after_insert()              → override hook
doc.before_save()               → override hook
doc.on_update()                 → override hook
doc.before_submit()             → override hook
doc.on_submit()                 → override hook
doc.before_cancel()             → override hook
doc.on_cancel()                 → override hook
doc.on_trash()                  → override hook
doc.after_delete()              → override hook
doc.as_dict()
doc.get_value(fieldname)
doc.db_set(fieldname, value)    → write single field without full save
doc.db_get(fieldname)
doc.lock()
doc.unlock()
doc.check_permission(ptype)
doc.has_permission(ptype)
doc.run_method(methodname, *args)
doc.get_url()
doc.get_doc_before_save()
```

### 2d. `frappe.client.*` (Whitelist methods — already partially done)
```
get_list / get_count / get / get_value / get_single_value ✅
save / insert / insert_many / set_value / delete ✅
submit / cancel ✅
rename_doc ❌
bulk_update 🔧
has_permission ❌
get_doc_permissions ❌
get_password ❌
get_time_zone ❌
attach_file ❌
is_document_amended ❌
validate_link ❌
```

---

## 3. DocType Controllers — 268 Python files to Reimplement

All of these have a `class <DocType>(Document):` that overrides lifecycle hooks or exposes `@frappe.whitelist` methods. They must be **reimplemented as Rust structs** that call lifecycle hooks AND expose the whitelisted methods through the method registry.

### 3a. Automation (9 controllers)
| File | Key Methods/Hooks |
|------|------------------|
| `assignment_rule.py` | `validate`, `on_update`, `bulk_apply` (whitelist) |
| `assignment_rule_day.py` | minimal |
| `assignment_rule_user.py` | minimal |
| `auto_repeat.py` | `validate`, `before_submit`, `on_submit`, `before_cancel`, `on_cancel`, `make_auto_repeat` (whitelist) |
| `auto_repeat_day.py` | minimal |
| `auto_repeat_user.py` | minimal |
| `milestone.py` | `on_update`, `on_cancel` |
| `milestone_tracker.py` | `validate`, `on_update` |
| `reminder.py` | `validate`, `create_new_reminder` (whitelist) |

### 3b. Contacts (7 controllers)
| File | Key Methods/Hooks |
|------|------------------|
| `address.py` | `validate`, `on_update`, `get_address_display` (whitelist), `get_default_address` |
| `address_template.py` | `validate`, `standard_template_exists` |
| `contact.py` | `validate`, `on_update`, `on_trash`, `get_contact_details` (whitelist) |
| `contact_email.py` | minimal |
| `contact_phone.py` | minimal |
| `gender.py` | minimal |
| `salutation.py` | minimal |

### 3c. Core — DocType/Schema (14 controllers)
| File | Key Methods/Hooks |
|------|------------------|
| `doctype.py` | `validate`, `on_update`, `on_trash`, `get_row_size_utilization`, schema sync | **CRITICAL** |
| `docfield.py` | `validate`, `before_save` |
| `docperm.py` | minimal |
| `custom_docperm.py` | `validate`, `on_update` |
| `property_setter.py` | `validate`, `on_update` |
| `custom_field.py` | `validate`, `on_update`, `on_trash` |
| `customize_form.py` | `save_customization`, `reset_to_defaults` (whitelist) — massive |
| `customize_form_field.py` | minimal |
| `doctype_layout.py` | `validate` |
| `document_naming_rule.py` | `validate`, `apply` |
| `document_naming_settings.py` | `get_amended_name_examples` (whitelist) |
| `version.py` | `get_diff`, tracking write |
| `audit_trail.py` | `on_update`, `on_cancel` |
| `deleted_document.py` | `restore` (whitelist) |

### 3d. Core — User/Role/Session (15 controllers)
| File | Key Methods/Hooks |
|------|------------------|
| `user.py` | `validate`, `on_update`, `on_trash`, `get_all_roles`, `test_password_strength`, `verify_password`, `switch_theme` (whitelists) — massive | **CRITICAL** |
| `user_type.py` | `validate`, `on_update` |
| `user_permission.py` | `validate`, `on_update`, `on_trash`, `get_user_permissions` (whitelist) | **CRITICAL** |
| `user_group.py` | `validate` |
| `user_group_member.py` | minimal |
| `user_invitation.py` | `validate`, `invite_by_email` (whitelist) |
| `role.py` | `validate`, `before_rename` |
| `role_profile.py` | `validate` |
| `role_replication.py` | `replicate_role` (whitelist only) |
| `custom_role.py` | `validate`, `on_update` |
| `module_profile.py` | `validate` |
| `session_default_settings.py` | `get_session_default_values`, `set_session_default_values` (whitelists) |
| `has_role.py` | minimal |
| `user_role.py` | minimal |
| `user_social_login.py` | minimal |

### 3e. Core — File System (1 controller)
| File | Key Methods/Hooks |
|------|------------------|
| `file.py` | `validate`, `before_insert`, `on_update`, `on_trash`, `get_attached_images`, `create_new_folder`, `move_file`, `unzip_file` (whitelists) — massive file binary storage | **CRITICAL** |

### 3f. Core — Communication/Email (16+ controllers)
| File | Key Methods/Hooks | Priority |
|------|------------------|---------|
| `communication.py` | `validate`, `on_update`, `on_trash`, `make` (whitelist) | P1 |
| `notification.py` | `validate`, `on_update`, `send` | P2 |
| `notification_log.py` | `get_notification_logs`, `mark_as_read`, `mark_all_as_read` (whitelists) | P1 |
| `email_account.py` | `validate`, `on_update`, `set_email_password` (whitelist) | P2 |
| `email_queue.py` | `send_one`, `flush`, `mark_email_as_sent` | P2 |
| `email_unsubscribe.py` | `on_update` | P3 |
| Full list (10 more) | ... | P2-P3 |

### 3g. Desk — Workspace/Desktop (20+ controllers)
| File | Key Methods/Hooks | Priority |
|------|------------------|---------|
| `workspace.py` | `validate`, `get_workspaces`, `new_page`, `save_page` (whitelists) | P1 |
| `workspace_sidebar.py` | `add_sidebar_items` (whitelist) | P1 |
| `todo.py` | `validate`, `on_update`, `on_trash` | P1 |
| `note.py` | `validate`, `mark_as_seen`, `reset_notes` (whitelists) | P2 |
| `kanban_board.py` | `validate`, `get_kanban_boards`, `quick_kanban_board`, `save_settings` | P2 |
| `dashboard.py` | `validate` | P2 |
| `dashboard_chart.py` | `validate`, `get`, `get_charts_for_user` (whitelists) | P2 |
| `number_card.py` | `validate`, `get_result`, `get_cards_for_user` (whitelists) | P2 |
| `event.py` | `validate`, `get_events`, `update_event` (whitelists) | P2 |
| `notification_settings.py` | `set_seen_value` (whitelist) | P1 |
| Full list (10 more) | ... | P2-P3 |

### 3h. Workflow (9 controllers)
| File | Key Methods/Hooks | Priority |
|------|------------------|---------|
| `workflow.py` | `validate`, `on_update`, `get_transitions`, `apply_workflow`, `get_common_transition_actions`, `bulk_workflow_approval`, `can_cancel_document` (whitelists) | P2 |
| `workflow_action.py` | `on_update`, `process_signature_request` | P2 |
| `workflow_state.py` | minimal | P2 |
| Other 6 | minimal | P3 |

### 3i. Website (25+ controllers) — P3
### 3j. Integrations (20+ controllers) — P2-P3
### 3k. Printing (7 controllers) — P2-P3
### 3l. Geo (2 controllers) — P2

---

## 4. HTTP API Endpoints — Current Status

### REST Resource API
| Endpoint | Status |
|----------|--------|
| `GET /api/resource/{doctype}` | 🔧 (get_list, no advanced filter grammar) |
| `GET /api/resource/{doctype}/{name}` | ✅ (get_doc) |
| `POST /api/resource/{doctype}` | ✅ (insert) |
| `PUT /api/resource/{doctype}/{name}` | ✅ (save) |
| `DELETE /api/resource/{doctype}/{name}` | ✅ (delete) |
| `GET /api/v2/document/{doctype}` | ❌ |
| `GET /api/v2/document/{doctype}/{name}` | ❌ |
| `POST /api/v2/document/{doctype}` | ❌ |
| `PATCH /api/v2/document/{doctype}/{name}` | ❌ |
| `DELETE /api/v2/document/{doctype}/{name}` | ❌ |
| `POST /api/v2/document/{doctype}/{name}/run_doc_method` | ❌ |
| `GET /api/v2/meta/{doctype}` | ❌ |
| `GET /api/v2/document/{doctype}/count` | ❌ |

### Method API (registered handlers)
| Path | Status |
|------|--------|
| `frappe.auth.get_logged_user` | ✅ |
| `login` / `logout` | ✅ |
| `frappe.client.get_list` | ✅ |
| `frappe.client.get` | ✅ |
| `frappe.client.get_value` | ✅ |
| `frappe.client.get_count` | ✅ |
| `frappe.client.save` | ✅ |
| `frappe.client.insert` | ✅ |
| `frappe.client.set_value` | ✅ |
| `frappe.client.delete` | ✅ |
| `frappe.client.submit` | ✅ |
| `frappe.client.cancel` | ✅ |
| `frappe.client.rename_doc` | ❌ |
| `frappe.client.bulk_update` | 🔧 stub |
| `frappe.client.has_permission` | ❌ |
| `frappe.client.get_doc_permissions` | 🔧 stub |
| `frappe.client.get_password` | ❌ |
| `frappe.client.attach_file` | ❌ |
| `frappe.client.validate_link` | ❌ |
| `frappe.desk.form.load.getdoctype` | ✅ |
| `frappe.desk.form.load.getdoc` | ✅ |
| `frappe.desk.form.load.get_docinfo` | ✅ |
| `frappe.desk.form.save.savedocs` | ✅ |
| `frappe.desk.reportview.get` | ✅ |
| `frappe.desk.reportview.get_count` | ✅ |
| `frappe.desk.search.search_link` | ✅ |
| `frappe.desk.notifications.get_notification_info` | ✅ |
| `frappe.utils.boot.get_boot_info` | ✅ |
| `frappe.model.workflow.*` | 🔧 stubs |
| `frappe.model.rename_doc.update_document_title` | 🔧 stub |
| All `stubs::handle_*` (40+ entries) | 🔧 stubs |

---

## 5. PyO3 Bridge — Not Started

This is the mechanism that makes `import frappe` work against the Rust engine.

### Required PyO3 Modules
```rust
// spotledger-pyo3 crate (new)
#[pymodule]
fn spotledger(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_doc, m)?)?;
    m.add_function(wrap_pyfunction!(new_doc, m)?)?;
    m.add_function(wrap_pyfunction!(get_all, m)?)?;
    m.add_function(wrap_pyfunction!(get_value, m)?)?;
    m.add_function(wrap_pyfunction!(set_value, m)?)?;
    m.add_function(wrap_pyfunction!(delete_doc, m)?)?;
    m.add_function(wrap_pyfunction!(rename_doc, m)?)?;
    m.add_class::<PyDocument>()?;   // doc.save(), doc.submit(), etc.
    m.add_class::<PyDatabase>()?;   // frappe.db
    m.add_class::<PyMeta>()?;       // frappe.get_meta()
    // ...
}
```

### PyDocument wrapper
```rust
#[pyclass]
struct PyDocument {
    inner: Arc<Mutex<Document>>,
    db: Arc<Db>,
}

#[pymethods]
impl PyDocument {
    fn insert(&self, py: Python) -> PyResult<()> { ... }
    fn save(&self, py: Python) -> PyResult<()> { ... }
    fn submit(&self, py: Python) -> PyResult<()> { ... }
    fn cancel(&self, py: Python) -> PyResult<()> { ... }
    fn delete(&self, py: Python) -> PyResult<()> { ... }
    fn get(&self, fieldname: &str) -> PyResult<PyObject> { ... }
    fn set(&self, fieldname: &str, value: PyObject) -> PyResult<()> { ... }
    fn db_set(&self, fieldname: &str, value: PyObject) -> PyResult<()> { ... }
    fn as_dict(&self, py: Python) -> PyResult<PyDict> { ... }
    fn run_method(&self, method: &str, py: Python) -> PyResult<PyObject> { ... }
}
```

### Lifecycle Hook Protocol
Python controllers call lifecycle hooks as Python methods. The Rust engine must:
1. Load the Python controller module for the doctype (e.g. `frappe.desk.doctype.todo.todo.Todo`)
2. Call the lifecycle method on it via PyO3 (`before_insert`, `validate`, `on_update`, etc.)
3. Fall back to Rust no-op if the method doesn't exist
4. Catch Python exceptions and convert to `SpotError`

---

## 6. Infrastructure Gaps

### 6a. Background Jobs (frappe.enqueue)
- Current: ❌ not implemented
- Required: Rust equivalent of Redis Queue (RQ) — either:
  - Implement a Tokio-based job queue with SurrealDB as store
  - Embed a Python RQ worker that calls PyO3 for DB ops
- Tests: `test_background_jobs.py`, `test_scheduled_job_type.py`, `rq_job.py`

### 6b. File Storage (frappe.core.doctype.file)
- Current: ❌ not implemented
- Required: Binary file storage, private/public, S3 compatibility
- Tests: `test_file.py`, `test_api.py::test_binary_and_csv_response`

### 6c. Email Engine
- Current: 🔧 stubs only
- Required: SMTP send, inbound IMAP, email queue processing
- Tests: `test_email_account.py`, `test_email_queue.py`, `test_email.py`

### 6d. Scheduler
- Current: ❌ not implemented
- Required: Cron-like dispatch of `scheduled_job_type` records
- Tests: `test_scheduler.py`, `test_scheduled_job_type.py`

### 6e. Translations
- Current: ❌ not implemented
- Required: `frappe._()` / `frappe.translate.*`, translation table lookup
- Tests: `test_translate.py`, `gettext/test_translate.py`

### 6f. Full-text Search
- Current: 🔧 stub (`frappe.utils.global_search.search`)
- Required: SurrealDB FTS or SQLite FTS index
- Tests: `test_full_text_search.py`, `test_sqlite_search.py`

### 6g. Redis Cache
- Current: ❌ (frappe.cache not bridged)
- Required: `frappe.cache.get_value`, `set_value`, `delete_value`, per-site namespacing
- Tests: `test_caching.py`, `test_client_cache.py`

### 6h. Print/PDF
- Current: 🔧 stub
- Required: Print format Jinja rendering → wkhtmltopdf/weasyprint
- Tests: `test_pdf.py`, `test_printview.py`, `test_print_format.py`

### 6i. OAuth2 / Social Login
- Current: ❌ not implemented
- Required: Token endpoint, authorization code flow
- Tests: `test_oauth20.py`, `test_oauth_client.py`

---

## 7. Prioritised Work Plan

### Phase 4 — PyO3 Bridge + Core DB API
**Exit criterion**: `frappe.get_doc("Todo", name).save()` in Python calls Rust, not MariaDB.
- [ ] New crate `spotledger-pyo3`
- [ ] `frappe.db.*` full implementation (sql, get_value, set_value, exists, count, delete, commit, rollback, transactions, defaults)
- [ ] `frappe.get_doc` / `frappe.new_doc` / `frappe.get_all` / `frappe.get_value` / `frappe.set_value` / `frappe.delete_doc`
- [ ] `PyDocument` wrapper (insert, save, submit, cancel, delete, get, set, db_set, as_dict)
- [ ] Single table and child table round-trip via PyO3
- [ ] `frappe.session`, `frappe.flags`, `frappe._dict`, `frappe.local` proxies

### Phase 5 — DocType Controllers (P0 controllers)
**Exit criterion**: Creating a User or ToDo from Python triggers all hooks.
- [ ] Lifecycle hook dispatch (`validate`, `before_insert`, `on_update`, etc.)
- [ ] Python controller loader (import `{app}.{module}.doctype.{dt}.{dt}.{Class}`)
- [ ] User controller: `validate`, `on_update`, password hashing, role management
- [ ] UserPermission controller
- [ ] DocType controller (schema sync on save)
- [ ] File controller (binary upload, move, delete)
- [ ] ToDo, Note — simple examples to validate the pattern

### Phase 6 — naming, permissions, metadata (full parity)
- [ ] `frappe.db.get_next_sequence_val`, sequences
- [ ] Full permissions matrix: role + user permissions, `has_permission` tree
- [ ] `frappe.get_meta` — full DocMeta object with all field accessors
- [ ] `frappe.rename_doc` — cascading FK rewrite
- [ ] `frappe.db_query.DatabaseQuery` full filter grammar
- [ ] `frappe.qb` query builder proxy

### Phase 7 — HTTP API v2 + REST resource API completion
- [ ] `/api/v2/document/*` CRUD
- [ ] `/api/v2/document/{dt}/{name}/run_doc_method`
- [ ] `/api/v2/meta/{dt}`
- [ ] Advanced filter grammar: `[["fieldname", "operator", "value"], ...]`
- [ ] `frappe.client.rename_doc`, `attach_file`, `validate_link`, `bulk_update`

### Phase 8 — Background Jobs + Scheduler
### Phase 9 — Email Engine
### Phase 10 — File Storage
### Phase 11 — Translations, OAuth2, PDF

---

## 8. Numbers

| Category | Count | Status |
|----------|-------|--------|
| Test files | 271 | Catalogued |
| Python controllers | 268 | 0 reimplemented in Rust |
| Registered Axum methods | ~140 | ~90 are real, ~50 are stubs |
| `frappe.*` top-level functions | ~60 | 0 in PyO3 |
| `frappe.db.*` methods | ~35 | 0 in PyO3 |
| `PyDocument` lifecycle hooks | ~15 | 0 in PyO3 |
| Rust source lines (current) | ~9,400 | |
| Estimated lines at parity | ~80,000–120,000 | |

---

## 9. Key Design Decision Required

**How should the PyO3 bridge handle async?**

Rust uses `async/tokio`. Python is synchronous (except `asyncio`). Options:
1. **Block-on-async**: Each PyO3 call spawns a Tokio task and blocks with `block_on()`. Simple, works with synchronous Python app code.
2. **Separate thread with its own runtime**: PyO3 calls cross-thread to a dedicated Tokio runtime via `mpsc` channel. Better isolation.
3. **`pyo3-asyncio`**: Expose async methods to Python as awaitables. Requires apps to use `async/await`. Breaking change.

**Recommendation**: Option 1 (`block_on`) for the first pass — matches frappe's synchronous programming model exactly.
