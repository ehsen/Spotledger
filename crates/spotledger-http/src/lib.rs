//! SpotledgerCore — Axum HTTP layer.
//!
//! This crate will contain:
//! - `AppState` / `SiteState` (shared Axum state)
//! - Axum router setup (`build_router`)
//! - Middleware (`site_middleware`, auth)  
//! - Route handlers (`/api/resource/`, `/api/method/`, `/api/get-doc/`)
//! - Method registry (built-in and app-installed handlers)
//!
//! Phase 0 stub — code extracted from `spotledger` binary in Phase 1.

// Phase 1 will add: pub mod middleware; pub mod routes; pub mod state; pub mod methods;
