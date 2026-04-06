//! Numeric utilities — Rust equivalents of `frappe.utils.data` numeric helpers.
//!
//! Functions:
//! - `rounded(num, precision, method)` — rounding with multiple strategies
//! - `floor(s)` / `ceil(s)` — integer rounding
//! - `money_in_words(amount, currency, fraction_currency)` — amount to English text
//! - `in_words(integer)` — integer to English words
//! - `safe_div(a, b, precision)` — division with zero-denominator guard
//! - `remainder(a, b, precision)` — modulo with float precision

// ── Rounding ───────────────────────────────────────────────────────────────────

/// Rounding strategy, matching Frappe's SystemSettings options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoundingMethod {
    /// "Banker's Rounding (legacy)" — Frappe's historical default.
    /// At exactly 0.5, rounds **up**.
    #[default]
    BankersLegacy,
    /// "Banker's Rounding" — true round-half-to-even (IEEE 754 default).
    Bankers,
    /// "Commercial Rounding" — round half away from zero.
    Commercial,
}

/// Round `num` to `precision` decimal places using the specified method.
///
/// Mirrors Frappe's `rounded(num, precision, rounding_method)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::numbers::{rounded, RoundingMethod};
/// assert_eq!(rounded(43.5, 0, RoundingMethod::BankersLegacy), 44.0);
/// assert_eq!(rounded(42.5, 0, RoundingMethod::BankersLegacy), 43.0); // legacy rounds up
/// assert_eq!(rounded(42.5, 0, RoundingMethod::Bankers), 42.0);       // round-to-even
/// assert_eq!(rounded(10_500.566_6, 2, RoundingMethod::BankersLegacy), 10_500.57);
/// ```
pub fn rounded(num: f64, precision: i32, method: RoundingMethod) -> f64 {
    match method {
        RoundingMethod::BankersLegacy => bankers_rounding_legacy(num, precision),
        RoundingMethod::Bankers => bankers_rounding(num, precision),
        RoundingMethod::Commercial => round_away_from_zero(num, precision),
    }
}

fn bankers_rounding_legacy(num: f64, precision: i32) -> f64 {
    let multiplier = 10f64.powi(precision);
    let scaled = (num * multiplier * 1e8).round() / 1e8; // avoid float jitter
    let floor_val = scaled.floor();
    let decimal = scaled - floor_val;

    let rounded_scaled = if precision == 0 {
        if (decimal - 0.5).abs() < f64::EPSILON {
            floor_val + 1.0
        } else {
            scaled.round()
        }
    } else {
        if (decimal - 0.5).abs() < 1e-9 {
            floor_val + 1.0
        } else {
            scaled.round()
        }
    };

    rounded_scaled / multiplier
}

fn bankers_rounding(num: f64, precision: i32) -> f64 {
    let multiplier = 10f64.powi(precision);
    let scaled = (num * multiplier * 1e12_f64.powi(1)).round() / 1e12;
    if scaled == 0.0 {
        return 0.0;
    }
    let floor_val = scaled.floor();
    let decimal = scaled - floor_val;

    // Epsilon based on magnitude
    let epsilon = if num.abs() > 0.0 {
        2f64.powf(num.abs().log2() - 52.0)
    } else {
        f64::EPSILON
    };

    let rounded_scaled = if (decimal - 0.5).abs() < epsilon {
        // round to even
        if floor_val as i64 % 2 == 0 {
            floor_val
        } else {
            floor_val + 1.0
        }
    } else {
        scaled.round()
    };

    rounded_scaled / multiplier
}

fn round_away_from_zero(num: f64, precision: i32) -> f64 {
    if num == 0.0 {
        return 0.0;
    }
    let epsilon = if num.abs() > 0.0 {
        2f64.powf(num.abs().log2() - 52.0)
    } else {
        f64::EPSILON
    };
    let sign = if num > 0.0 { 1.0 } else { -1.0 };
    (num + sign * epsilon).round_ties_even_at_precision(precision)
}

trait RoundTiesEven {
    fn round_ties_even_at_precision(self, precision: i32) -> f64;
}

impl RoundTiesEven for f64 {
    fn round_ties_even_at_precision(self, precision: i32) -> f64 {
        let factor = 10f64.powi(precision);
        (self * factor).round() / factor
    }
}

// ── Floor / Ceil ──────────────────────────────────────────────────────────────

/// Return the largest integer ≤ `num` (floor).  
/// Mirrors Frappe's `floor(s)`.
pub fn floor(num: f64) -> i64 {
    num.floor() as i64
}

/// Return the smallest integer ≥ `num` (ceiling).  
/// Mirrors Frappe's `ceil(s)`.
pub fn ceil(num: f64) -> i64 {
    num.ceil() as i64
}

// ── Safe arithmetic ────────────────────────────────────────────────────────────

/// Division returning `0.0` when `denominator == 0`.
///
/// Mirrors Frappe's `safe_div(a, b, precision)`.
pub fn safe_div(numerator: f64, denominator: f64, precision: i32) -> f64 {
    if denominator == 0.0 {
        0.0
    } else {
        rounded(numerator / denominator, precision, RoundingMethod::BankersLegacy)
    }
}

/// Return `numerator % denominator` at the given float `precision`.
///
/// Mirrors Frappe's `remainder(a, b, precision)`.
pub fn remainder(numerator: f64, denominator: f64, precision: i32) -> f64 {
    let factor = 10f64.powi(precision);
    let r = if precision > 0 {
        ((numerator * factor) % (denominator * factor)) / factor
    } else {
        numerator % denominator
    };
    rounded(r, precision, RoundingMethod::BankersLegacy)
}

// ── Money in words ─────────────────────────────────────────────────────────────

/// Convert a monetary amount to English words.
///
/// Mirrors Frappe's `money_in_words(number, main_currency, fraction_currency)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::numbers::money_in_words;
/// let s = money_in_words(1001.50, "USD", "Cents");
/// assert!(s.contains("One Thousand"), "got: {s}");
/// assert!(s.contains("Fifty"), "got: {s}");
/// ```
pub fn money_in_words(amount: f64, main_currency: &str, fraction_currency: &str) -> String {
    if amount < 0.0 {
        return String::new();
    }

    let n = format!("{:.2}", amount);
    let parts: Vec<&str> = n.split('.').collect();
    let main_str = parts[0];
    let fraction_str = parts.get(1).copied().unwrap_or("00");

    let main_val: i64 = main_str.parse().unwrap_or(0);
    let frac_val: i64 = fraction_str.parse().unwrap_or(0);

    if main_val == 0 && frac_val == 0 {
        return format!("{main_currency} Zero");
    }

    let mut out = String::new();

    if main_val > 0 {
        out.push_str(main_currency);
        out.push(' ');
        out.push_str(&title_case(&in_words(main_val)));
    }

    if frac_val > 0 {
        if !out.is_empty() {
            out.push_str(" and ");
        }
        out.push_str(&title_case(&in_words(frac_val)));
        out.push(' ');
        out.push_str(fraction_currency);
    }

    format!("{out} only.")
}

/// Convert a non-negative integer to English words.
///
/// Mirrors Frappe's `in_words(integer)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::numbers::in_words;
/// assert_eq!(in_words(0), "zero");
/// assert_eq!(in_words(1), "one");
/// assert_eq!(in_words(21), "twenty one");
/// assert_eq!(in_words(1000), "one thousand");
/// assert_eq!(in_words(1_000_001), "one million one");
/// ```
pub fn in_words(n: i64) -> String {
    if n < 0 {
        return format!("negative {}", in_words(-n));
    }
    match n {
        0 => "zero".to_owned(),
        1..=19 => ONES[n as usize].to_owned(),
        20..=99 => {
            let tens = TENS[(n / 10) as usize];
            let ones = n % 10;
            if ones == 0 {
                tens.to_owned()
            } else {
                format!("{tens} {}", ONES[ones as usize])
            }
        }
        100..=999 => {
            let h = n / 100;
            let rem = n % 100;
            if rem == 0 {
                format!("{} hundred", ONES[h as usize])
            } else {
                format!("{} hundred {}", ONES[h as usize], in_words(rem))
            }
        }
        1_000..=999_999 => {
            let thousands = n / 1_000;
            let rem = n % 1_000;
            if rem == 0 {
                format!("{} thousand", in_words(thousands))
            } else {
                format!("{} thousand {}", in_words(thousands), in_words(rem))
            }
        }
        1_000_000..=999_999_999 => {
            let millions = n / 1_000_000;
            let rem = n % 1_000_000;
            if rem == 0 {
                format!("{} million", in_words(millions))
            } else {
                format!("{} million {}", in_words(millions), in_words(rem))
            }
        }
        1_000_000_000..=999_999_999_999 => {
            let billions = n / 1_000_000_000;
            let rem = n % 1_000_000_000;
            if rem == 0 {
                format!("{} billion", in_words(billions))
            } else {
                format!("{} billion {}", in_words(billions), in_words(rem))
            }
        }
        _ => {
            let trillions = n / 1_000_000_000_000;
            let rem = n % 1_000_000_000_000;
            if rem == 0 {
                format!("{} trillion", in_words(trillions))
            } else {
                format!("{} trillion {}", in_words(trillions), in_words(rem))
            }
        }
    }
}

const ONES: [&str; 20] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen",
    "nineteen",
];

const TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rounded_bankers_legacy() {
        assert_eq!(rounded(43.5, 0, RoundingMethod::BankersLegacy), 44.0);
        assert_eq!(rounded(42.5, 0, RoundingMethod::BankersLegacy), 43.0);
        assert_eq!(rounded(10_500.566_6, 2, RoundingMethod::BankersLegacy), 10_500.57);
        assert_eq!(rounded(1.005, 2, RoundingMethod::BankersLegacy), 1.01);
    }

    #[test]
    fn test_rounded_bankers() {
        // round-to-even: 0.5 → 0 (even), 1.5 → 2 (even)
        assert_eq!(rounded(0.5, 0, RoundingMethod::Bankers), 0.0);
        assert_eq!(rounded(1.5, 0, RoundingMethod::Bankers), 2.0);
        assert_eq!(rounded(2.5, 0, RoundingMethod::Bankers), 2.0);
    }

    #[test]
    fn test_in_words() {
        assert_eq!(in_words(0), "zero");
        assert_eq!(in_words(1), "one");
        assert_eq!(in_words(21), "twenty one");
        assert_eq!(in_words(100), "one hundred");
        assert_eq!(in_words(115), "one hundred fifteen");
        assert_eq!(in_words(1_000), "one thousand");
        assert_eq!(in_words(1_001), "one thousand one");
        assert_eq!(in_words(1_000_000), "one million");
        assert_eq!(in_words(1_000_001), "one million one");
    }

    #[test]
    fn test_money_in_words() {
        let s = money_in_words(1001.50, "USD", "Cents");
        assert!(s.contains("One Thousand"), "got: {s}");
        assert!(s.contains("Fifty"), "got: {s}");
        assert!(s.contains("only"), "got: {s}");

        let zero = money_in_words(0.0, "USD", "Cents");
        assert_eq!(zero, "USD Zero");
    }

    #[test]
    fn test_floor_ceil() {
        assert_eq!(floor(3.9), 3);
        assert_eq!(ceil(3.1), 4);
        assert_eq!(floor(-3.1), -4);
        assert_eq!(ceil(-3.9), -3);
    }

    #[test]
    fn test_safe_div() {
        assert_eq!(safe_div(10.0, 3.0, 2), 3.33);
        assert_eq!(safe_div(10.0, 0.0, 2), 0.0);
    }
}
