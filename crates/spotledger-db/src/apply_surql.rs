//! Apply `.surql` files to a SurrealDB database.
//!
//! These helpers are used by `install_app` to apply the pipeline framework
//! schema, universal function definitions, and domain-function files.
//!
//! ## Design
//!
//! - `apply_surql_file` reads one `.surql` file and fires all statements.
//! - `apply_surql_dir` walks a directory (sorted by filename) and calls
//!   `apply_surql_file` for each `.surql` file found, including one level
//!   of subdirectories (for `doctypes/*/functions.surql` layouts).
//! - `collect_fn_names` scans surql source text for `DEFINE FUNCTION` lines
//!   and extracts the `fn::` names.  Used to auto-generate the registry.

use std::fmt::Write as FmtWrite;
use std::path::Path;

use crate::adapter::DbAdapter;
use crate::error::DbError;

// ── apply_surql_file ──────────────────────────────────────────────────────────

/// Read a `.surql` file and execute all its statements against the database.
/// All statements in the file are sent as a single multi-statement query.
pub async fn apply_surql_file(adapter: &DbAdapter, path: &Path) -> Result<(), DbError> {
    let sql = tokio::fs::read_to_string(path).await.map_err(|e| {
        DbError::Other(format!("apply_surql_file: cannot read {}: {}", path.display(), e))
    })?;

    if sql.trim().is_empty() {
        return Ok(());
    }

    adapter.execute(&sql, vec![]).await.map_err(|e| {
        DbError::Other(format!(
            "apply_surql_file: error applying {}: {}",
            path.display(),
            e
        ))
    })
}

// ── apply_surql_dir ───────────────────────────────────────────────────────────

/// Walk a directory and apply every `.surql` file found, sorted by filename.
///
/// **Recursion**: descends one level into immediate subdirectories (i.e.
/// `doctypes/account/functions.surql` is discovered when `dir` is `doctypes/`).
/// Deeper nesting is not walked.
///
/// Returns the total number of files applied.
pub async fn apply_surql_dir(adapter: &DbAdapter, dir: &Path) -> Result<usize, DbError> {
    apply_surql_dir_named(adapter, dir, None).await
}

/// Like `apply_surql_dir` but only applies files whose filename (without path)
/// matches `only_name`.  Pass `None` to apply all `.surql` files.
///
/// Example: `apply_surql_dir_named(adapter, &doctypes_dir, Some("wiring.surql"))`
/// applies only the `wiring.surql` file inside each doctype subdirectory.
pub async fn apply_surql_dir_named(
    adapter:   &DbAdapter,
    dir:       &Path,
    only_name: Option<&str>,
) -> Result<usize, DbError> {
    if !dir.exists() {
        return Ok(0);
    }

    let mut files: Vec<std::path::PathBuf> = vec![];
    collect_surql_files_filtered(dir, only_name, &mut files)
        .map_err(|e| DbError::Other(e.to_string()))?;
    files.sort();

    for path in &files {
        apply_surql_file(adapter, path).await?;
    }

    Ok(files.len())
}

/// Collect `.surql` files from `dir`, including one level of subdirectories.
/// If `only_name` is Some, only collect files whose filename matches exactly.
fn collect_surql_files_filtered(
    dir:       &Path,
    only_name: Option<&str>,
    out:       &mut Vec<std::path::PathBuf>,
) -> std::io::Result<()> {
    let matches = |path: &std::path::Path| -> bool {
        if path.extension().and_then(|e| e.to_str()) != Some("surql") {
            return false;
        }
        match only_name {
            Some(name) => path.file_name().and_then(|n| n.to_str()) == Some(name),
            None => true,
        }
    };
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            for sub in std::fs::read_dir(&path)? {
                let sub_path = sub?.path();
                if matches(&sub_path) {
                    out.push(sub_path);
                }
            }
        } else if matches(&path) {
            out.push(path);
        }
    }
    Ok(())
}

// ── collect_fn_names ─────────────────────────────────────────────────────────

/// Scan surql source text for `DEFINE FUNCTION` declarations.
///
/// Handles both `DEFINE FUNCTION fn::foo` and `DEFINE FUNCTION OVERWRITE fn::foo`.
/// Returns the sorted, deduplicated list of `fn::` names found.
pub fn collect_fn_names(sql: &str) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();

    for line in sql.lines() {
        let trimmed = line.trim();
        // Skip comments
        if trimmed.starts_with("--") {
            continue;
        }
        let Some(after_define) = trimmed
            .strip_prefix("DEFINE FUNCTION")
            .or_else(|| trimmed.strip_prefix("define function"))
        else {
            continue;
        };
        let after_define = after_define.trim();
        // Skip OVERWRITE keyword if present
        let after_kw = after_define
            .strip_prefix("OVERWRITE")
            .or_else(|| after_define.strip_prefix("overwrite"))
            .map(str::trim)
            .unwrap_or(after_define);

        if let Some(paren) = after_kw.find('(') {
            let name = after_kw[..paren].trim();
            if name.starts_with("fn::") {
                names.insert(name.to_owned());
            }
        }
    }

    names.into_iter().collect()
}

// ── generate_registry_surql ──────────────────────────────────────────────────

/// Generate the `fn::registry::dispatch` function body from a list of fn names.
///
/// The registry is an `IF / ELSE IF` chain that maps a string fn_name to an
/// actual function call.  SurrealDB cannot call functions by dynamic string name,
/// so this generated dispatcher is the only supported mechanism.
///
/// The generated SQL is applied to the database — it is never written to disk.
pub fn generate_registry_surql(fn_names: &[String]) -> String {
    let mut sql = String::from(
        "DEFINE FUNCTION OVERWRITE fn::registry::dispatch(\n\
         \t$fn_name: string,\n\
         \t$doc_id:  record,\n\
         \t$config:  object\n\
         ) {\n",
    );

    for (i, name) in fn_names.iter().enumerate() {
        let kw = if i == 0 { "IF" } else { "ELSE IF" };
        write!(
            sql,
            "\t{kw} $fn_name = \"{name}\" {{\n\t\tRETURN {name}($doc_id, $config);\n\t}}\n"
        )
        .unwrap();
    }

    if fn_names.is_empty() {
        sql.push_str("\tRETURN { ok: true };\n");
    } else {
        sql.push_str(
            "\tELSE {\n\t\tTHROW string::concat(\"Unknown pipeline node: \", $fn_name);\n\t};\n",
        );
    }

    sql.push_str("};");
    sql
}

/// Apply the generated registry to the database immediately.
pub async fn apply_registry(adapter: &DbAdapter, fn_names: &[String]) -> Result<(), DbError> {
    let sql = generate_registry_surql(fn_names);
    adapter.execute(&sql, vec![]).await.map_err(|e| {
        DbError::Other(format!("apply_registry: failed to apply registry: {e}"))
    })
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_fn_names_basic() {
        let sql = r#"
-- A comment
DEFINE FUNCTION OVERWRITE fn::validate::party_check($doc: record, $cfg: object) { };
DEFINE FUNCTION fn::compute::totals($doc: record, $cfg: object) { };
        "#;
        let names = collect_fn_names(sql);
        assert_eq!(
            names,
            vec!["fn::compute::totals", "fn::validate::party_check"]
        );
    }

    #[test]
    fn generate_registry_roundtrip() {
        let names = vec![
            "fn::validate::mandatory_fields".to_owned(),
            "fn::validate::party_check".to_owned(),
        ];
        let sql = generate_registry_surql(&names);
        assert!(sql.contains("fn::registry::dispatch"));
        assert!(sql.contains("fn::validate::mandatory_fields"));
        assert!(sql.contains("fn::validate::party_check"));
        assert!(sql.contains("Unknown pipeline node"));
    }
}
