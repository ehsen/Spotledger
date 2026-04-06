//! Core utility functions — Rust equivalents of `frappe.utils.*`.
//!
//! Each sub-module mirrors a Frappe utility file:
//!
//! | Frappe module           | Rust module                     |
//! |-------------------------|---------------------------------|
//! | `frappe.utils.data`     | `formatting`, `strings`, `data` |
//! | `frappe.utils.dateutils`| `dates`                         |
//! | `frappe.utils.number_format`, `data.fmt_money` | `numbers` |
//! | `frappe.utils.data` (`rounded`, `money_in_words`) | `numbers` |
//! | `frappe.utils.data` (validation helpers) | `validation` |
//! | `frappe.utils.password` | `passwords`                     |
//! | `frappe.utils.nestedset`| `nestedset`                     |
//! | `serde_json` wrappers   | `json`                          |

pub mod data;
pub mod dates;
pub mod formatting;
pub mod json;
pub mod nestedset;
pub mod numbers;
pub mod passwords;
pub mod strings;
pub mod validation;

// Flat re-exports so callers can write `use spotledger_core::utils::scrub;`
pub use data::{flatten, group_by_field, unique};
pub use dates::{
    add_days, add_months, add_to_date, date_diff, get_first_day, get_first_day_of_week,
    get_last_day, get_last_day_of_week, get_quarter_ending, get_quarter_start, get_year_ending,
    get_year_start, now, now_datetime, today,
};
pub use formatting::{cint, flt, fmt_money, format_datetime, format_duration, formatdate};
pub use json::{as_json, parse_json};
pub use numbers::{ceil, floor, in_words, money_in_words, rounded};
pub use passwords::{check_password, hash_password};
pub use strings::{cstr, scrub, strip_html, unscrub};
pub use validation::{validate_email_address, validate_phone_number, validate_url};
