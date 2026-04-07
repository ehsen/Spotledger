//! SpotledgerCore — Axum HTTP layer.
//!
//! Provides the complete HTTP server stack:
//! - `AppState` / `SiteState` (shared Axum state)
//! - Axum router and middleware
//! - Route handlers (`/api/resource/`, `/api/method/`, auth)
//! - Method registry (built-in Tier 1 + app-installed handlers)
//! - Server startup and site loading
//!
//! Extracted from spotledger binary in Phase 1.

pub mod middleware;
pub mod methods;
pub mod routes;
pub mod server;
pub mod state;

// Re-export commonly used types
pub use middleware::{site_middleware, CurrentUser};
pub use server::{serve, build_app, register_site_from_config};
pub use state::{AppState, SiteState};
pub use methods::{MethodRegistry, build_registry};
