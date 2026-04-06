//! Collection utilities — Rust equivalents of misc `frappe.utils.data` helpers.
//!
//! Functions:
//! - `unique(list)` — deduplicate preserving order
//! - `flatten(lists)` — flatten a `Vec<Vec<T>>` to `Vec<T>`
//! - `group_by_field(docs, field)` — group `serde_json::Value` objects by a field
//! - `has_common(a, b)` — true if two slices share any element
//! - `get_common(a, b)` — elements present in both slices
//! - `diff(a, b)` — elements in `a` not in `b`
//! - `chunk(list, size)` — split a Vec into equal-sized chunks

use serde_json::Value;
use std::collections::{HashMap, HashSet};

// ── Deduplication ─────────────────────────────────────────────────────────────

/// Remove duplicate elements from a slice, preserving the first occurrence order.
///
/// Mirrors Frappe's `unique(list)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::data::unique;
/// assert_eq!(unique(&[1, 2, 1, 3, 2]), vec![1, 2, 3]);
/// ```
pub fn unique<T: Eq + std::hash::Hash + Clone>(list: &[T]) -> Vec<T> {
    let mut seen = HashSet::new();
    list.iter()
        .filter(|item| seen.insert(*item))
        .cloned()
        .collect()
}

/// Remove duplicate `String` items, preserving order.
pub fn unique_strings(list: &[String]) -> Vec<String> {
    unique(list)
}

// ── Flatten ────────────────────────────────────────────────────────────────────

/// Flatten a `Vec<Vec<T>>` into a single `Vec<T>`.
///
/// Mirrors Frappe's `flatten(lists)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::data::flatten;
/// assert_eq!(flatten(vec![vec![1, 2], vec![3, 4]]), vec![1, 2, 3, 4]);
/// ```
pub fn flatten<T: Clone>(lists: Vec<Vec<T>>) -> Vec<T> {
    lists.into_iter().flatten().collect()
}

// ── Group by field ────────────────────────────────────────────────────────────

/// Group a list of JSON objects by the value of `field`.
///
/// Objects where `field` is missing or not a string are placed under the empty
/// string key `""`.
///
/// Mirrors Frappe's `group_by_field(list, field)`.
///
/// # Examples
/// ```
/// use serde_json::json;
/// use spotledger_core::utils::data::group_by_field;
///
/// let docs = vec![
///     json!({"company": "Acme", "amount": 100}),
///     json!({"company": "Globex", "amount": 200}),
///     json!({"company": "Acme", "amount": 300}),
/// ];
/// let grouped = group_by_field(&docs, "company");
/// assert_eq!(grouped["Acme"].len(), 2);
/// assert_eq!(grouped["Globex"].len(), 1);
/// ```
pub fn group_by_field<'a>(
    docs: &'a [Value],
    field: &str,
) -> HashMap<String, Vec<&'a Value>> {
    let mut map: HashMap<String, Vec<&Value>> = HashMap::new();
    for doc in docs {
        let key = doc
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        map.entry(key).or_default().push(doc);
    }
    map
}

// ── Set operations ────────────────────────────────────────────────────────────

/// Return `true` if `a` and `b` share at least one element.
///
/// Mirrors Frappe's `has_common(l1, l2)`.
pub fn has_common<T: Eq + std::hash::Hash>(a: &[T], b: &[T]) -> bool {
    let set: HashSet<_> = a.iter().collect();
    b.iter().any(|x| set.contains(x))
}

/// Return elements that appear in **both** `a` and `b` (intersection), order from `a`.
pub fn get_common<T: Eq + std::hash::Hash + Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let set: HashSet<_> = b.iter().collect();
    a.iter()
        .filter(|x| set.contains(*x))
        .cloned()
        .collect()
}

/// Return elements in `a` that are **not** in `b` (set difference).
pub fn diff<T: Eq + std::hash::Hash + Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let set: HashSet<_> = b.iter().collect();
    a.iter()
        .filter(|x| !set.contains(*x))
        .cloned()
        .collect()
}

// ── Chunking ──────────────────────────────────────────────────────────────────

/// Split `list` into chunks of at most `size` elements.
///
/// Mirrors common usage of Python's `itertools.batched` / list slicing in Frappe.
pub fn chunk<T: Clone>(list: Vec<T>, size: usize) -> Vec<Vec<T>> {
    if size == 0 {
        return vec![list];
    }
    list.chunks(size).map(|c| c.to_vec()).collect()
}

// ── Sorting helpers ────────────────────────────────────────────────────────────

/// Sort a Vec of JSON objects by a string field ascending.
///
/// Objects without the field sort to the end.
pub fn sort_by_field(docs: &mut Vec<Value>, field: &str) {
    docs.sort_by(|a, b| {
        let av = a.get(field).and_then(Value::as_str).unwrap_or("");
        let bv = b.get(field).and_then(Value::as_str).unwrap_or("");
        av.cmp(bv)
    });
}

/// Sort a Vec of JSON objects by a numeric field ascending.
///
/// Objects without the field sort to the end (value treated as `f64::MAX`).
pub fn sort_by_numeric_field(docs: &mut Vec<Value>, field: &str) {
    docs.sort_by(|a, b| {
        let av = a
            .get(field)
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::MAX);
        let bv = b
            .get(field)
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::MAX);
        av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_unique() {
        assert_eq!(unique(&[1, 2, 1, 3, 2]), vec![1, 2, 3]);
        assert_eq!(unique(&["a", "b", "a"]), vec!["a", "b"]);
        assert!(unique(&[] as &[i32]).is_empty());
    }

    #[test]
    fn test_flatten() {
        assert_eq!(flatten(vec![vec![1, 2], vec![3, 4]]), vec![1, 2, 3, 4]);
        assert_eq!(flatten::<i32>(vec![]), Vec::<i32>::new());
    }

    #[test]
    fn test_group_by_field() {
        let docs = vec![
            json!({"company": "Acme", "amount": 100}),
            json!({"company": "Globex", "amount": 200}),
            json!({"company": "Acme", "amount": 300}),
        ];
        let grouped = group_by_field(&docs, "company");
        assert_eq!(grouped["Acme"].len(), 2);
        assert_eq!(grouped["Globex"].len(), 1);
    }

    #[test]
    fn test_has_common() {
        assert!(has_common(&[1, 2, 3], &[3, 4, 5]));
        assert!(!has_common(&[1, 2], &[3, 4]));
    }

    #[test]
    fn test_get_common() {
        assert_eq!(get_common(&[1, 2, 3], &[2, 3, 4]), vec![2, 3]);
    }

    #[test]
    fn test_diff() {
        assert_eq!(diff(&[1, 2, 3], &[2, 3, 4]), vec![1]);
    }

    #[test]
    fn test_chunk() {
        assert_eq!(chunk(vec![1, 2, 3, 4, 5], 2), vec![vec![1, 2], vec![3, 4], vec![5]]);
        assert_eq!(chunk(vec![1, 2, 3], 10), vec![vec![1, 2, 3]]);
        assert_eq!(chunk::<i32>(vec![], 2), Vec::<Vec<i32>>::new());
    }

    #[test]
    fn test_sort_by_field() {
        let mut docs = vec![
            json!({"name": "Charlie"}),
            json!({"name": "Alice"}),
            json!({"name": "Bob"}),
        ];
        sort_by_field(&mut docs, "name");
        assert_eq!(docs[0]["name"], "Alice");
        assert_eq!(docs[1]["name"], "Bob");
        assert_eq!(docs[2]["name"], "Charlie");
    }
}
