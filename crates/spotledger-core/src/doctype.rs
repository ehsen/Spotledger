//! `DocType` trait — the lifecycle contract every compiled DocType implements.
//!
//! All hook methods have no-op defaults so implementors only override what
//! they need.

use async_trait::async_trait;

use crate::document::Document;
use crate::error::CoreError;

/// Every compiled DocType implements this trait.
#[async_trait]
pub trait DocType: Send + Sync + 'static {
    fn doctype_name() -> &'static str where Self: Sized;
    fn doc(&self)         -> &Document;
    fn doc_mut(&mut self) -> &mut Document;

    async fn validate(&self)           -> Result<(), CoreError> { Ok(()) }
    async fn before_insert(&mut self)  -> Result<(), CoreError> { Ok(()) }
    async fn after_insert(&self)       -> Result<(), CoreError> { Ok(()) }
    async fn before_save(&mut self)    -> Result<(), CoreError> { Ok(()) }
    async fn after_save(&self)         -> Result<(), CoreError> { Ok(()) }
    async fn before_submit(&mut self)  -> Result<(), CoreError> { Ok(()) }
    async fn on_submit(&mut self)      -> Result<(), CoreError> { Ok(()) }
    async fn before_cancel(&mut self)  -> Result<(), CoreError> { Ok(()) }
    async fn on_cancel(&mut self)      -> Result<(), CoreError> { Ok(()) }
    async fn on_trash(&self)           -> Result<(), CoreError> { Ok(()) }
}
