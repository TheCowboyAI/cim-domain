//! Generic event handler trait for domain events

use async_trait::async_trait;

/// Trait for handling specific domain events
#[async_trait]
pub trait EventHandler<E> {
    /// Error type for this handler
    type Error;

    /// Handle a domain event
    async fn handle(&self, event: E) -> Result<(), Self::Error>;
}
