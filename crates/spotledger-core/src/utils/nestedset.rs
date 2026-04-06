//! Nested Set Model (NSM) utilities — Rust equivalents of `frappe.utils.nestedset`.
//!
//! The NSM stores tree hierarchies using `lft` and `rgt` integers per node.
//! All functions that query the database accept a `TreeStore` trait object so
//! this crate stays DB-agnostic.  The concrete implementation in `spotledger-db`
//! implements `TreeStore` via SurrealDB.
//!
//! ## Algorithm reference
//! Each node stores:
//! - `lft` — pre-order index when the tree is traversed
//! - `rgt` — index at which the traversal backtracks from this node
//!
//! A node B is a **descendant** of A when `A.lft < B.lft AND B.rgt < A.rgt`.
//! A node B is an **ancestor** of A when `B.lft < A.lft AND A.rgt < B.rgt`.

use async_trait::async_trait;

use crate::error::CoreError;

// ── Node record ────────────────────────────────────────────────────────────────

/// Minimal tree-node record as stored in the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    /// Document name (PK).
    pub name: String,
    /// Nested-set left index.
    pub lft: i64,
    /// Nested-set right index.
    pub rgt: i64,
    /// Parent document name (empty string = root node).
    pub parent: String,
}

// ── Abstract store trait ───────────────────────────────────────────────────────

/// Minimum database operations required by the nested-set algorithms.
///
/// Implement this on `DbAdapter` in `spotledger-db` to satisfy the calls in
/// this module.  All methods are async and return `CoreError` on failure.
#[async_trait]
pub trait TreeStore: Send + Sync {
    /// Return the `(lft, rgt)` bounds for a single node.
    async fn get_node_bounds(
        &self,
        doctype: &str,
        name: &str,
    ) -> Result<(i64, i64), CoreError>;

    /// Return names of all nodes whose `lft < lft_bound AND rgt > rgt_bound`
    /// (ancestors), ordered by `lft DESC` or `ASC`.
    async fn query_ancestors(
        &self,
        doctype: &str,
        lft_bound: i64,
        rgt_bound: i64,
        order_desc: bool,
        limit: Option<usize>,
    ) -> Result<Vec<String>, CoreError>;

    /// Return names of all nodes whose `lft > lft_bound AND rgt < rgt_bound`
    /// (descendants), ordered by `lft`.
    async fn query_descendants(
        &self,
        doctype: &str,
        lft_bound: i64,
        rgt_bound: i64,
        order_desc: bool,
        limit: Option<usize>,
    ) -> Result<Vec<String>, CoreError>;

    /// Return all root nodes (nodes with no parent) for `doctype`.
    async fn get_root_names(&self, doctype: &str) -> Result<Vec<String>, CoreError>;

    /// Return the immediate children of `parent` for `doctype`.
    async fn get_children(
        &self,
        doctype: &str,
        parent: &str,
        parent_field: &str,
    ) -> Result<Vec<String>, CoreError>;

    /// Update `lft` and `rgt` for a single node by name.
    async fn set_node_bounds(
        &self,
        doctype: &str,
        name: &str,
        lft: i64,
        rgt: i64,
    ) -> Result<(), CoreError>;

    /// Shift all nodes where `lft >= threshold` by `+delta` on the `lft` column.
    async fn shift_lft_gte(
        &self,
        doctype: &str,
        threshold: i64,
        delta: i64,
    ) -> Result<(), CoreError>;

    /// Shift all nodes where `rgt >= threshold` by `+delta` on the `rgt` column.
    async fn shift_rgt_gte(
        &self,
        doctype: &str,
        threshold: i64,
        delta: i64,
    ) -> Result<(), CoreError>;

    /// Return `true` if the node identified by `name` appears in the list of
    /// ancestors (used for loop detection).
    async fn is_ancestor_of(
        &self,
        doctype: &str,
        name: &str,
        lft: i64,
        rgt: i64,
    ) -> Result<bool, CoreError>;
}

// ── Public functions ──────────────────────────────────────────────────────────

/// Return the list of ancestor names for a given `doctype`/`name`.
///
/// Results are ordered by `lft DESC` (nearest ancestor first) by default.
///
/// Mirrors Frappe's `get_ancestors_of(doctype, name, order_by, limit)`.
///
/// # Example
/// ```ignore
/// let ancestors = get_ancestors_of(&db, "Account", "Cash - XYZ", None).await?;
/// // ["Bank", "Assets", "Root"]
/// ```
pub async fn get_ancestors_of(
    store: &dyn TreeStore,
    doctype: &str,
    name: &str,
    limit: Option<usize>,
) -> Result<Vec<String>, CoreError> {
    let (lft, rgt) = store.get_node_bounds(doctype, name).await?;
    store
        .query_ancestors(doctype, lft, rgt, true, limit)
        .await
}

/// Return the list of descendant names for a given `doctype`/`name`.
///
/// Returns an empty `Vec` if the node is a leaf node (`rgt - lft == 1`).
///
/// Mirrors Frappe's `get_descendants_of(doctype, name, order_by, limit)`.
pub async fn get_descendants_of(
    store: &dyn TreeStore,
    doctype: &str,
    name: &str,
    limit: Option<usize>,
) -> Result<Vec<String>, CoreError> {
    let (lft, rgt) = store.get_node_bounds(doctype, name).await?;
    if rgt - lft <= 1 {
        return Ok(Vec::new());
    }
    store
        .query_descendants(doctype, lft, rgt, true, limit)
        .await
}

/// Validate that adding `name` as a child of the node at `(lft, rgt)` would
/// not create a cycle (i.e. `name` must not already be an ancestor of that node).
///
/// Mirrors Frappe's `validate_loop(doctype, name, lft, rgt)`.
pub async fn validate_loop(
    store: &dyn TreeStore,
    doctype: &str,
    name: &str,
    target_lft: i64,
    target_rgt: i64,
) -> Result<(), CoreError> {
    let is_ancestor = store
        .is_ancestor_of(doctype, name, target_lft, target_rgt)
        .await?;
    if is_ancestor {
        return Err(CoreError::Validation(format!(
            "{} cannot be added to its own descendants",
            name
        )));
    }
    Ok(())
}

/// Rebuild all `lft` / `rgt` values for a tree DocType.
///
/// Safe to call any time — assigns fresh contiguous values starting from 1
/// based on a depth-first traversal of the parent-child relationships.
///
/// Mirrors Frappe's `rebuild_tree(doctype)`.
pub async fn rebuild_tree(
    store: &dyn TreeStore,
    doctype: &str,
    parent_field: &str,
) -> Result<(), CoreError> {
    let roots = store.get_root_names(doctype).await?;
    let mut counter: i64 = 1;
    for root in roots {
        counter = rebuild_node(store, doctype, &root, counter, parent_field).await?;
    }
    Ok(())
}

/// Recursively assign `lft` / `rgt` for `name` and all its children.
/// Returns the next available index after this subtree.
async fn rebuild_node(
    store: &dyn TreeStore,
    doctype: &str,
    name: &str,
    left: i64,
    parent_field: &str,
) -> Result<i64, CoreError> {
    let right = left + 1;
    let children = store.get_children(doctype, name, parent_field).await?;

    let mut right = right;
    for child in children {
        right = Box::pin(rebuild_node(store, doctype, &child, right, parent_field)).await?;
    }

    // right is now the first index after all children → this node spans [left, right]
    store.set_node_bounds(doctype, name, left, right).await?;
    Ok(right + 1)
}

// ── Pure (no-DB) nested-set utilities ─────────────────────────────────────────

/// Determine whether `node` is an ancestor of `target` given pre-fetched bounds.
///
/// A node is an ancestor when its `lft < target.lft AND rgt > target.rgt`.
pub fn is_ancestor(node: &TreeNode, target: &TreeNode) -> bool {
    node.lft < target.lft && node.rgt > target.rgt
}

/// Determine whether `node` is a descendant of `ancestor` given pre-fetched bounds.
pub fn is_descendant(node: &TreeNode, ancestor: &TreeNode) -> bool {
    is_ancestor(ancestor, node)
}

/// Filter a flat slice of `TreeNode`s to those that are ancestors of `target`.
///
/// Useful when you have already loaded the entire tree in memory.
pub fn ancestors_from_slice<'a>(nodes: &'a [TreeNode], target: &TreeNode) -> Vec<&'a TreeNode> {
    nodes
        .iter()
        .filter(|n| is_ancestor(n, target))
        .collect()
}

/// Filter a flat slice of `TreeNode`s to those that are descendants of `ancestor`.
pub fn descendants_from_slice<'a>(
    nodes: &'a [TreeNode],
    ancestor: &TreeNode,
) -> Vec<&'a TreeNode> {
    nodes
        .iter()
        .filter(|n| is_descendant(n, ancestor))
        .collect()
}

/// Compute the depth of a node in the tree.
///
/// Depth is 0-based (root nodes have depth 0).
/// Calculated as the number of ancestors in `nodes`.
pub fn depth_of(nodes: &[TreeNode], target: &TreeNode) -> usize {
    ancestors_from_slice(nodes, target).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, lft: i64, rgt: i64, parent: &str) -> TreeNode {
        TreeNode {
            name: name.to_owned(),
            lft,
            rgt,
            parent: parent.to_owned(),
        }
    }

    #[test]
    fn test_is_ancestor() {
        //  Root(1..10)
        //    Parent(2..7)
        //      Child(3..4)
        //    Sibling(8..9)
        let root = node("Root", 1, 10, "");
        let parent = node("Parent", 2, 7, "Root");
        let child = node("Child", 3, 4, "Parent");
        let sibling = node("Sibling", 8, 9, "Root");

        assert!(is_ancestor(&root, &child));
        assert!(is_ancestor(&parent, &child));
        assert!(!is_ancestor(&child, &parent));
        assert!(!is_ancestor(&sibling, &child));
    }

    #[test]
    fn test_ancestors_from_slice() {
        let root = node("Root", 1, 10, "");
        let parent = node("Parent", 2, 7, "Root");
        let child = node("Child", 3, 4, "Parent");
        let sibling = node("Sibling", 8, 9, "Root");
        let nodes = vec![root.clone(), parent.clone(), child.clone(), sibling.clone()];

        let ancestors = ancestors_from_slice(&nodes, &child);
        let names: Vec<&str> = ancestors.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Root"));
        assert!(names.contains(&"Parent"));
    }

    #[test]
    fn test_descendants_from_slice() {
        let root = node("Root", 1, 10, "");
        let parent = node("Parent", 2, 7, "Root");
        let child = node("Child", 3, 4, "Parent");
        let sibling = node("Sibling", 8, 9, "Root");
        let nodes = vec![root.clone(), parent.clone(), child.clone(), sibling.clone()];

        let descendants = descendants_from_slice(&nodes, &root);
        assert_eq!(descendants.len(), 3);
    }

    #[test]
    fn test_depth() {
        let root = node("Root", 1, 10, "");
        let parent = node("Parent", 2, 7, "Root");
        let child = node("Child", 3, 4, "Parent");
        let nodes = vec![root.clone(), parent.clone(), child.clone()];

        assert_eq!(depth_of(&nodes, &root), 0);
        assert_eq!(depth_of(&nodes, &parent), 1);
        assert_eq!(depth_of(&nodes, &child), 2);
    }
}
