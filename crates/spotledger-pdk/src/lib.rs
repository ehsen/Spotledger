//! SpotledgerCore Plugin Developer Kit — guest-side WASM helpers.
//!
//! Plugin authors import this crate to call host functions without writing
//! raw `extern "C"` declarations.
//!
//! ## Host functions available to plugins
//!
//! | Host function           | Description                                |
//! |-------------------------|--------------------------------------------|
//! | `sl_get_doc`            | Fetch a document from DB                   |
//! | `sl_save_doc`           | Save a document to DB                      |
//! | `sl_delete_doc`         | Delete a document                          |
//! | `sl_make_gl_entries`    | Post GL entries via the accounting engine  |
//! | `sl_get_value`          | Get a single field value                   |
//! | `sl_set_value`          | Set a single field value                   |
//! | `sl_new_name`           | Generate the next document name            |
//! | `sl_log`                | Write to the host trace log                |
//! | `sl_register_doctype`   | Register a DocType at plugin init          |

pub mod host;
