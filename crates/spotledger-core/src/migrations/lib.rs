//! Tier 0 migrations — SurrealQL DDL for core Tier 0 tables.
//!
//! These migrations run at startup via `ensure_all_schemas` and create all
//! Tier 0 tables in SurrealDB.

pub const TIER0_SCHEMA: &str = include_str!("tier0.surql");
