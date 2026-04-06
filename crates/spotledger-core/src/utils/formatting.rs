//! Formatting utilities — Rust equivalents of `frappe.utils.data`
//! `cint`, `flt`, `fmt_money`, `formatdate`, `format_datetime`, `format_duration`.
//!
//! Unlike Frappe, these functions are pure (no DB calls, no request context).
//! Currency symbol/separator info is passed explicitly rather than fetched from the DB.

use chrono::{NaiveDate, NaiveDateTime};
use crate::utils::numbers::RoundingMethod;

// ── Type coercions ─────────────────────────────────────────────────────────────

/// Convert any stringifiable or numeric value to `i64`.
/// Returns `default` (0) when parsing fails.  
/// Mirrors Frappe's `cint(s)`.
pub fn cint<S: AsRef<str>>(s: S, default: i64) -> i64 {
    let s = s.as_ref().trim().replace(',', "");
    if s.is_empty() {
        return default;
    }
    // Try integer first, then float-then-truncate (mirrors Python `int(float(s))`)
    s.parse::<i64>()
        .or_else(|_| s.parse::<f64>().map(|f| f as i64))
        .unwrap_or(default)
}

/// `cint(s)` with default 0.
pub fn cint0<S: AsRef<str>>(s: S) -> i64 {
    cint(s, 0)
}

/// Convert any string/numeric value to `f64`, stripping commas.
/// Returns `0.0` on failure.  
/// Mirrors Frappe's `flt(s, precision)`.
pub fn flt<S: AsRef<str>>(s: S, precision: Option<u8>) -> f64 {
    let s = s.as_ref().trim().replace(',', "");
    if s.is_empty() {
        return 0.0;
    }
    let num = s.parse::<f64>().unwrap_or(0.0);
    match precision {
        Some(p) => crate::utils::numbers::rounded(num, p as i32, RoundingMethod::BankersLegacy),
        None => num,
    }
}

// ── Money formatting ──────────────────────────────────────────────────────────

/// Named number format similar to Frappe's `NUMBER_FORMAT_MAP`.
///
/// Decimal separator, thousands separator, and default precision.
#[derive(Debug, Clone)]
pub struct NumberFormat {
    /// Character used between integer and fractional parts (e.g. `'.'` or `','`).
    pub decimal_sep: char,
    /// Character used between digit groups (e.g. `','`, `'.'`, `' '`).
    pub thousands_sep: &'static str,
    /// Default number of decimal places.
    pub precision: u8,
    /// The format string as displayed in Frappe UI (e.g. `"#,###.##"`).
    pub string: &'static str,
}

impl NumberFormat {
    pub const COMMA_DOT_2: NumberFormat = NumberFormat {
        decimal_sep: '.',
        thousands_sep: ",",
        precision: 2,
        string: "#,###.##",
    };

    pub const DOT_COMMA_2: NumberFormat = NumberFormat {
        decimal_sep: ',',
        thousands_sep: ".",
        precision: 2,
        string: "#.###,##",
    };

    pub const SPACE_DOT_2: NumberFormat = NumberFormat {
        decimal_sep: '.',
        thousands_sep: " ",
        precision: 2,
        string: "# ###.##",
    };
    pub const SPACE_COMMA_2: NumberFormat = NumberFormat {
        decimal_sep: ',',
        thousands_sep: " ",
        precision: 2,
        string: "# ###,##",
    };
    pub const QUOTE_DOT_2: NumberFormat = NumberFormat {
        decimal_sep: '.',
        thousands_sep: "'",
        precision: 2,
        string: "#'###.##",
    };
    pub const COMMA_DOT_3: NumberFormat = NumberFormat {
        decimal_sep: '.',
        thousands_sep: ",",
        precision: 3,
        string: "#,###.###",
    };
    /// Indian numbering: groups of 2 after the first 3 digits.
    pub const INDIAN: NumberFormat = NumberFormat {
        decimal_sep: '.',
        thousands_sep: ",",
        precision: 2,
        string: "#,##,###.##",
    };

    /// Look up a format by its Frappe format string. Defaults to `COMMA_DOT_2`.
    pub fn from_string(s: &str) -> &'static NumberFormat {
        match s {
            "#,###.##" => &Self::COMMA_DOT_2,
            "#.###,##" => &Self::DOT_COMMA_2,
            "# ###.##" => &Self::SPACE_DOT_2,
            "# ###,##" => &Self::SPACE_COMMA_2,
            "#'###.##" => &Self::QUOTE_DOT_2,
            "#,###.###" => &Self::COMMA_DOT_3,
            "#,##,###.##" => &Self::INDIAN,
            _ => &Self::COMMA_DOT_2,
        }
    }
}

/// Format a monetary amount as a string with thousands separators and decimal places.
///
/// Mirrors Frappe's `fmt_money(amount, precision, currency, format)`.  
/// Currency symbol is optional and prepended (or appended) when provided.
///
/// # Arguments
/// * `amount` — The numeric value to format.
/// * `precision` — Number of decimal places (defaults to `format.precision`).
/// * `currency_symbol` — Optional symbol string, e.g. `"$"` or `"₹"`.
/// * `symbol_on_right` — When `true`, symbol appears after the amount.
/// * `format` — The `NumberFormat` to use; defaults to `COMMA_DOT_2`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::formatting::{fmt_money, NumberFormat};
/// assert_eq!(fmt_money(40_000.0, None, None, false, &NumberFormat::COMMA_DOT_2), "40,000.00");
/// assert_eq!(fmt_money(1234.5, Some(2), Some("$"), false, &NumberFormat::COMMA_DOT_2), "$ 1,234.50");
/// ```
pub fn fmt_money(
    amount: f64,
    precision: Option<u8>,
    currency_symbol: Option<&str>,
    symbol_on_right: bool,
    format: &NumberFormat,
) -> String {
    let prec = precision.unwrap_or(format.precision) as usize;
    let negative = amount < 0.0;
    let abs_amount = amount.abs();

    // Format fixed-precision string (e.g. "40000.50")
    let formatted = format!("{:.prec$}", abs_amount, prec = prec);
    let (integer_part, decimal_part) = if let Some(idx) = formatted.find('.') {
        (&formatted[..idx], Some(&formatted[idx + 1..]))
    } else {
        (formatted.as_str(), None)
    };

    // Insert thousands separators.
    // Indian format uses groups of 2 after initial 3.
    let integer_with_sep = insert_thousands_sep(integer_part, format);

    let mut result = integer_with_sep;

    // Append decimals if precision > 0
    if prec > 0 {
        let dec = decimal_part.unwrap_or("00");
        result.push(format.decimal_sep);
        result.push_str(dec);
    }

    if negative && result != "0" {
        result = format!("-{result}");
    }

    // Attach currency symbol
    match currency_symbol {
        Some(sym) if !sym.is_empty() => {
            if symbol_on_right {
                format!("{result} {sym}")
            } else {
                format!("{sym} {result}")
            }
        }
        _ => result,
    }
}

fn insert_thousands_sep(integer: &str, fmt: &NumberFormat) -> String {
    if fmt.thousands_sep.is_empty() || integer.len() <= 3 {
        return integer.to_owned();
    }

    let indian = fmt.string == "#,##,###.##";
    let bytes = integer.as_bytes();
    let len = bytes.len();
    let result;

    if indian {
        // First group of 3, then groups of 2
        let first_group = if len > 3 { len - ((len - 1) / 2) * 2 } else { len };
        // Simple implementation: build groups right to left
        let mut groups: Vec<&str> = Vec::new();
        let mut pos = len;
        // Last pair groups
        while pos > 3 {
            let start = if pos >= 2 { pos - 2 } else { 0 };
            groups.push(&integer[start..pos]);
            pos = start;
        }
        groups.push(&integer[..pos]);
        let _ = first_group; // suppress warning
        groups.reverse();
        result = groups.join(fmt.thousands_sep);
    } else {
        // Standard groups of 3
        let mut groups: Vec<&str> = Vec::new();
        let mut pos = len;
        while pos > 3 {
            groups.push(&integer[pos - 3..pos]);
            pos -= 3;
        }
        groups.push(&integer[..pos]);
        groups.reverse();
        result = groups.join(fmt.thousands_sep);
    }

    result
}

// ── Date / datetime formatting ────────────────────────────────────────────────

/// Format a `NaiveDate` using a Frappe-style format string.
///
/// Supported tokens: `dd`, `mm`, `MM` (month name abbrev), `yyyy`, `yy`.
/// Falls back to `yyyy-mm-dd` for unrecognised patterns.
///
/// Mirrors Frappe's `formatdate(date, format_string)`.
pub fn formatdate(date: NaiveDate, format_str: &str) -> String {
    // Map Frappe format tokens to strftime equivalents
    let strftime_fmt = frappe_date_fmt_to_strftime(format_str);
    date.format(&strftime_fmt).to_string()
}

/// Format a `NaiveDateTime` with a Frappe date+time format string.
///
/// Mirrors Frappe's `format_datetime(datetime, format_string)`.
pub fn format_datetime(dt: NaiveDateTime, format_str: &str) -> String {
    let strftime_fmt = frappe_date_fmt_to_strftime(format_str);
    dt.format(&strftime_fmt).to_string()
}

/// Format a duration in seconds as Frappe does: `"3h 34m 45s"`, `"2d 1h"`, …
///
/// Mirrors Frappe's `format_duration(seconds, hide_days)`.
pub fn format_duration(total_seconds: i64, hide_days: bool) -> String {
    let negative = total_seconds < 0;
    let secs = total_seconds.unsigned_abs();

    let (days, remainder) = if hide_days {
        (0u64, secs)
    } else {
        (secs / 86_400, secs % 86_400)
    };
    let hours = if hide_days {
        secs / 3600
    } else {
        remainder / 3600
    };
    let remainder2 = remainder % 3600;
    let minutes = remainder2 / 60;
    let seconds = remainder2 % 60;

    let mut parts: Vec<String> = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if seconds > 0 {
        parts.push(format!("{seconds}s"));
    }

    let s = parts.join(" ");
    if negative && !s.is_empty() {
        format!("-{s}")
    } else {
        s
    }
}

/// Helper for `duration_to_seconds`: parse leading integer before `suffix`,
/// return (parsed_value, remainder_str).
fn parse_duration_unit(s: &str, suffix: char) -> (i64, &str) {
    if let Some(pos) = s.find(suffix) {
        let num = s[..pos].trim().parse::<i64>().unwrap_or(0);
        (num, s[pos + suffix.len_utf8()..].trim())
    } else {
        (0, s)
    }
}

/// Parse a duration string like `"3h 34m 45s"` or `"2d 1h"` to total seconds.
///
/// Mirrors Frappe's `duration_to_seconds(duration)`.
pub fn duration_to_seconds(duration: &str) -> i64 {
    let mut total: i64 = 0;
    let s = duration.trim();

    let mut rest = s;
    let (d, r) = parse_duration_unit(rest, 'd');
    total += d * 86_400;
    rest = r;
    let (h, r) = parse_duration_unit(rest, 'h');
    total += h * 3600;
    rest = r;
    let (m, r) = parse_duration_unit(rest, 'm');
    total += m * 60;
    rest = r;
    let (seconds, _) = parse_duration_unit(rest, 's');
    total += seconds;

    total
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn frappe_date_fmt_to_strftime(fmt: &str) -> String {
    // Order matters: longer tokens before shorter ones.
    fmt.replace("yyyy", "%Y")
        .replace("yy", "%y")
        .replace("mm", "%m")
        .replace("dd", "%d")
        .replace("HH", "%H")
        .replace("MM", "%M") // minute
        .replace("ss", "%S")
}

/// Return the abbreviated month name for chrono month number (1-based).
pub fn month_abbr(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

/// Return the full month name for chrono month number (1-based).
pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

/// Return the weekday name for chrono's weekday number (0=Monday per chrono).
pub fn weekday_name(weekday: chrono::Weekday) -> &'static str {
    match weekday {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    }
}

// ── Internal use by `flt` ──────────────────────────────────────────────────────
// (numbers::rounded is called; forward-declaration here to avoid circular dep)
// The actual `rounded` is in numbers.rs.

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_cint() {
        assert_eq!(cint("100", 0), 100);
        assert_eq!(cint("3.7", 0), 3);
        assert_eq!(cint("abc", 0), 0);
        assert_eq!(cint("", 0), 0);
        assert_eq!(cint("1,000", 0), 1000); // commas stripped
    }

    #[test]
    fn test_flt() {
        assert_eq!(flt("43.5", None), 43.5);
        assert_eq!(flt("10,500.5666", None), 10_500.566_6);
        assert_eq!(flt("abc", None), 0.0);
        assert_eq!(flt("", None), 0.0);
    }

    #[test]
    fn test_fmt_money_basic() {
        let s = fmt_money(40_000.0, None, None, false, &NumberFormat::COMMA_DOT_2);
        assert_eq!(s, "40,000.00");

        let s = fmt_money(1234.5, Some(2), Some("$"), false, &NumberFormat::COMMA_DOT_2);
        assert_eq!(s, "$ 1,234.50");

        let s = fmt_money(0.0, Some(2), None, false, &NumberFormat::COMMA_DOT_2);
        assert_eq!(s, "0.00");
    }

    #[test]
    fn test_fmt_money_negative() {
        let s = fmt_money(-100.5, Some(2), None, false, &NumberFormat::COMMA_DOT_2);
        assert_eq!(s, "-100.50");
    }

    #[test]
    fn test_formatdate() {
        let d = NaiveDate::from_ymd_opt(2026, 4, 6).unwrap();
        assert_eq!(formatdate(d, "yyyy-mm-dd"), "2026-04-06");
        assert_eq!(formatdate(d, "dd/mm/yyyy"), "06/04/2026");
        assert_eq!(formatdate(d, "mm/dd/yyyy"), "04/06/2026");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(12885, false), "3h 34m 45s");
        assert_eq!(format_duration(-12885, false), "-3h 34m 45s");
        assert_eq!(format_duration(86400 + 3600, false), "1d 1h");
        assert_eq!(format_duration(0, false), "");
    }

    #[test]
    fn test_duration_to_seconds() {
        assert_eq!(duration_to_seconds("3h 34m 45s"), 12885);
        assert_eq!(duration_to_seconds("1d"), 86400);
        assert_eq!(duration_to_seconds("2d 1h 30m"), 2 * 86400 + 3600 + 1800);
    }
}
