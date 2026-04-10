//! SpotledgerCore — Tier 2 optional automation layer.
//!
//! Provides Server Script, Scheduled Job, Webhook, Webhook Log, and
//! Auto Email Report DocTypes.  The `automation` feature flag controls
//! whether this layer is compiled into the binary.
//!
//! Gate all code with `#[cfg(feature = "automation")]`.

pub mod auto_repeat;

/// Returns this crate's name. Used to force-link inventory submissions into the binary.
pub const fn name() -> &'static str { "spotledger-automation" }

#[cfg(feature = "automation")]
pub mod webhook;

#[cfg(feature = "automation")]
pub mod scheduled_job;
