//! Date / datetime utilities — Rust equivalents of `frappe.utils.data` date functions.
//!
//! All functions use `chrono` types. Dates are represented as `chrono::NaiveDate`
//! and datetimes as `chrono::NaiveDateTime`. The canonical string storage format
//! matches Frappe: `yyyy-mm-dd` for dates, `yyyy-mm-dd HH:MM:SS.ffffff` for datetimes.
//!
//! Key functions:
//! - `now()` / `today()` — current datetime / date strings
//! - `add_days()`, `add_months()`, `add_years()`, `add_to_date()`
//! - `date_diff()` — days between two dates
//! - `get_first_day()` / `get_last_day()` — month boundaries
//! - `get_first_day_of_week()` / `get_last_day_of_week()`
//! - `get_quarter_start()` / `get_quarter_ending()`
//! - `get_year_start()` / `get_year_ending()`

use chrono::{Datelike, Duration, Local, Months, NaiveDate, NaiveDateTime};

pub const DATE_FORMAT: &str = "%Y-%m-%d";
pub const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.6f";

// ── Current time ───────────────────────────────────────────────────────────────

/// Return the current date+time as a `NaiveDateTime` (local wall clock).
///
/// Mirrors Frappe's `now_datetime()`.
pub fn now_datetime() -> NaiveDateTime {
    Local::now().naive_local()
}

/// Return the current datetime as `"yyyy-mm-dd HH:MM:SS.ffffff"`.
///
/// Mirrors Frappe's `now()`.
pub fn now() -> String {
    now_datetime().format(DATETIME_FORMAT).to_string()
}

/// Return today's date as `"yyyy-mm-dd"`.
///
/// Mirrors Frappe's `today()` / `nowdate()`.
pub fn today() -> String {
    Local::now().format(DATE_FORMAT).to_string()
}

/// Return the current time as `"HH:MM:SS.ffffff"`.
///
/// Mirrors Frappe's `nowtime()`.
pub fn nowtime() -> String {
    Local::now().format("%H:%M:%S%.6f").to_string()
}

// ── Parsing helpers ────────────────────────────────────────────────────────────

/// Parse a date string (`"yyyy-mm-dd"`) to `NaiveDate`.
/// Returns `None` for empty / invalid input.
pub fn getdate(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    if s.is_empty() || s.starts_with("0000") || s.starts_with("0001-01-01") {
        return None;
    }
    // Try ISO first (fast path), then fallback
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(s, "%d-%m-%Y"))
        .or_else(|_| NaiveDate::parse_from_str(s, "%m/%d/%Y"))
        .ok()
}

/// Parse a datetime string to `NaiveDateTime`.
/// Returns `None` for empty / invalid input.
pub fn get_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f"))
        .ok()
        .or_else(|| getdate(s).map(|d| d.and_hms_opt(0, 0, 0).unwrap()))
}

// ── Date arithmetic ────────────────────────────────────────────────────────────

/// Parameters for `add_to_date`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DateDelta {
    pub years: i32,
    pub months: i32,
    pub weeks: i32,
    pub days: i32,
    pub hours: i32,
    pub minutes: i32,
    pub seconds: i32,
}

/// Add an arbitrary delta (years, months, weeks, days, hours, minutes, seconds) to a date.
///
/// Mirrors Frappe's `add_to_date(date, years, months, weeks, days, ...)`.
pub fn add_to_date(start: NaiveDate, delta: DateDelta) -> NaiveDate {
    let mut d = start;

    // Handle years by converting to months
    let total_months = delta.years * 12 + delta.months;
    if total_months > 0 {
        d = d + Months::new(total_months as u32);
    } else if total_months < 0 {
        d = d - Months::new((-total_months) as u32);
    }

    // Weeks + days + times (time parts ignored for pure date arithmetic)
    let day_delta = delta.weeks * 7 + delta.days;
    d + Duration::days(day_delta as i64)
}

/// Add a number of days to a date.
///
/// Mirrors Frappe's `add_days(date, days)`.
pub fn add_days(date: NaiveDate, days: i64) -> NaiveDate {
    date + Duration::days(days)
}

/// Add a number of months to a date.
///
/// Mirrors Frappe's `add_months(date, months)`.
pub fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    if months >= 0 {
        date + Months::new(months as u32)
    } else {
        date - Months::new((-months) as u32)
    }
}

/// Add a number of years to a date.
///
/// Mirrors Frappe's `add_years(date, years)`.
pub fn add_years(date: NaiveDate, years: i32) -> NaiveDate {
    add_months(date, years * 12)
}

// ── Difference ────────────────────────────────────────────────────────────────

/// Return the difference in days between two date strings (`end_date - start_date`).
///
/// Mirrors Frappe's `date_diff(string_ed_date, string_st_date)`.
/// Returns `0` if either date is invalid.
pub fn date_diff(end_date: &str, start_date: &str) -> i64 {
    match (getdate(end_date), getdate(start_date)) {
        (Some(e), Some(s)) => (e - s).num_days(),
        _ => 0,
    }
}

/// Return the difference in months between two dates (end_month - start_month, inclusive).
///
/// Mirrors Frappe's `month_diff`.
pub fn month_diff(end_date: &str, start_date: &str) -> i32 {
    match (getdate(end_date), getdate(start_date)) {
        (Some(e), Some(s)) => {
            (e.year() - s.year()) * 12 + (e.month() as i32 - s.month() as i32) + 1
        }
        _ => 0,
    }
}

// ── Month boundaries ──────────────────────────────────────────────────────────

/// Return the first day of the month for `date`, optionally shifted by `d_years` and `d_months`.
///
/// Mirrors Frappe's `get_first_day(dt, d_years, d_months)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::dates::get_first_day;
/// use chrono::NaiveDate;
/// let d = NaiveDate::from_ymd_opt(2026, 4, 15).unwrap();
/// assert_eq!(get_first_day(d, 0, 0).to_string(), "2026-04-01");
/// assert_eq!(get_first_day(d, 0, 1).to_string(), "2026-05-01");
/// ```
pub fn get_first_day(date: NaiveDate, d_years: i32, d_months: i32) -> NaiveDate {
    let total_months = date.year() * 12 + (date.month() as i32 - 1) + d_years * 12 + d_months;
    let year = total_months / 12;
    let month = (total_months % 12 + 1) as u32;
    NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(date)
}

/// Return the last day of the month for `date`.
///
/// Mirrors Frappe's `get_last_day(dt)`.
pub fn get_last_day(date: NaiveDate) -> NaiveDate {
    // First day of *next* month − 1 day
    get_first_day(date, 0, 1) - Duration::days(1)
}

/// Return `true` if `date` is the last day of its month.
pub fn is_last_day_of_month(date: NaiveDate) -> bool {
    date == get_last_day(date)
}

// ── Week boundaries ───────────────────────────────────────────────────────────

/// Return the Monday of the week containing `date` (ISO week start).
///
/// Mirrors Frappe's `get_first_day_of_week(dt)` (defaulting to Monday).
pub fn get_first_day_of_week(date: NaiveDate) -> NaiveDate {
    let days_since_monday = date.weekday().num_days_from_monday();
    date - Duration::days(days_since_monday as i64)
}

/// Return the Sunday of the week containing `date` (ISO week end = Monday + 6).
///
/// Mirrors Frappe's `get_last_day_of_week(dt)`.
pub fn get_last_day_of_week(date: NaiveDate) -> NaiveDate {
    get_first_day_of_week(date) + Duration::days(6)
}

// ── Quarter boundaries ────────────────────────────────────────────────────────

/// Return the first day of the quarter containing `date`.
///
/// Mirrors Frappe's `get_quarter_start(dt)`.
pub fn get_quarter_start(date: NaiveDate) -> NaiveDate {
    let quarter = (date.month() - 1) / 3; // 0-based quarter index
    let first_month = quarter * 3 + 1;
    NaiveDate::from_ymd_opt(date.year(), first_month, 1).unwrap()
}

/// Return the last day of the quarter containing `date`.
///
/// Mirrors Frappe's `get_quarter_ending(date)`.
pub fn get_quarter_ending(date: NaiveDate) -> NaiveDate {
    // Last quarter-end month ≥ date's month
    for &month in &[3u32, 6, 9, 12] {
        let end_month = NaiveDate::from_ymd_opt(date.year(), month, 1).unwrap();
        let end = get_last_day(end_month);
        if date <= end {
            return end;
        }
    }
    // Should not reach here; fallback
    get_last_day(NaiveDate::from_ymd_opt(date.year(), 12, 1).unwrap())
}

// ── Year boundaries ────────────────────────────────────────────────────────────

/// Return January 1 of the year containing `date`.
///
/// Mirrors Frappe's `get_year_start(dt)`.
pub fn get_year_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), 1, 1).unwrap()
}

/// Return December 31 of the year containing `date`.
///
/// Mirrors Frappe's `get_year_ending(date)`.
pub fn get_year_ending(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), 12, 31).unwrap()
}

// ── Timestamp ─────────────────────────────────────────────────────────────────

/// Return a Unix timestamp (seconds since 1970-01-01) for the given date.
///
/// Mirrors Frappe's `get_timestamp(date)`.
pub fn get_timestamp(date: NaiveDate) -> i64 {
    date.and_hms_opt(0, 0, 0)
        .and_then(|dt| dt.and_utc().timestamp().into())
        .unwrap_or_else(|| date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp())
}

// ── Formatting helpers ─────────────────────────────────────────────────────────

/// Render a `NaiveDate` as `"yyyy-mm-dd"`.
pub fn date_to_str(date: NaiveDate) -> String {
    date.format(DATE_FORMAT).to_string()
}

/// Render a `NaiveDateTime` as `"yyyy-mm-dd HH:MM:SS"`.
pub fn datetime_to_str(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        getdate(s).expect(s)
    }

    #[test]
    fn test_getdate() {
        assert_eq!(d("2026-04-06"), NaiveDate::from_ymd_opt(2026, 4, 6).unwrap());
        assert!(getdate("").is_none());
        assert!(getdate("0000-00-00").is_none());
    }

    #[test]
    fn test_add_days() {
        assert_eq!(add_days(d("2026-04-06"), 5), d("2026-04-11"));
        assert_eq!(add_days(d("2026-04-06"), -6), d("2026-03-31"));
    }

    #[test]
    fn test_add_months() {
        assert_eq!(add_months(d("2026-01-31"), 1), d("2026-02-28"));
        assert_eq!(add_months(d("2026-03-31"), -1), d("2026-02-28"));
    }

    #[test]
    fn test_date_diff() {
        assert_eq!(date_diff("2026-04-11", "2026-04-06"), 5);
        assert_eq!(date_diff("2026-01-01", "2026-01-01"), 0);
    }

    #[test]
    fn test_get_first_day() {
        assert_eq!(get_first_day(d("2026-04-15"), 0, 0), d("2026-04-01"));
        assert_eq!(get_first_day(d("2026-04-15"), 0, 1), d("2026-05-01"));
        assert_eq!(get_first_day(d("2026-04-15"), 0, -1), d("2026-03-01"));
    }

    #[test]
    fn test_get_last_day() {
        assert_eq!(get_last_day(d("2026-02-01")), d("2026-02-28"));
        assert_eq!(get_last_day(d("2024-02-01")), d("2024-02-29")); // leap year
        assert_eq!(get_last_day(d("2026-04-01")), d("2026-04-30"));
    }

    #[test]
    fn test_get_first_day_of_week() {
        // 2026-04-06 is a Monday
        assert_eq!(get_first_day_of_week(d("2026-04-06")), d("2026-04-06"));
        // 2026-04-10 is a Friday
        assert_eq!(get_first_day_of_week(d("2026-04-10")), d("2026-04-06"));
    }

    #[test]
    fn test_get_last_day_of_week() {
        assert_eq!(get_last_day_of_week(d("2026-04-06")), d("2026-04-12"));
    }

    #[test]
    fn test_quarter_boundaries() {
        assert_eq!(get_quarter_start(d("2026-04-15")), d("2026-04-01"));
        assert_eq!(get_quarter_ending(d("2026-04-15")), d("2026-06-30"));
        assert_eq!(get_quarter_start(d("2026-01-01")), d("2026-01-01"));
        assert_eq!(get_quarter_ending(d("2026-12-31")), d("2026-12-31"));
    }

    #[test]
    fn test_year_boundaries() {
        assert_eq!(get_year_start(d("2026-07-15")), d("2026-01-01"));
        assert_eq!(get_year_ending(d("2026-07-15")), d("2026-12-31"));
    }
}
