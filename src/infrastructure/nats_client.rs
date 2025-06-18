//! NATS client for event store integration

use async_nats::{Client, ConnectOptions};
use async_nats::jetstream::{self, Context as JetStreamContext};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur when working with NATS
#[derive(Debug, Error)]
pub enum NatsError {
    /// Failed to establish connection to NATS server
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Error occurred in JetStream operations
    #[error("JetStream error: {0}")]
    JetStreamError(String),

    /// Invalid configuration provided
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Authentication credentials were rejected
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
}

impl From<async_nats::Error> for NatsError {
    fn from(err: async_nats::Error) -> Self {
        NatsError::ConnectionFailed(err.to_string())
    }
}

impl From<async_nats::jetstream::Error> for NatsError {
    fn from(err: async_nats::jetstream::Error) -> Self {
        NatsError::JetStreamError(err.to_string())
    }
}

/// Configuration for NATS client connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    /// NATS server URL (e.g., "nats://localhost:4222")
    pub url: String,

    /// Optional username for authentication
    pub user: Option<String>,

    /// Optional password for authentication
    pub password: Option<String>,

    /// Whether TLS is required
    pub tls_required: bool,

    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,

    /// Reconnect interval in seconds
    pub reconnect_interval_secs: u64,

    /// Maximum reconnect attempts (0 = infinite)
    pub max_reconnects: usize,

    /// JetStream domain (optional)
    pub jetstream_domain: Option<String>,

    /// JetStream prefix (optional)
    pub jetstream_prefix: Option<String>,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".to_string(),
            user: None,
            password: None,
            tls_required: false,
            connection_timeout_secs: 10,
            reconnect_interval_secs: 5,
            max_reconnects: 0, // Infinite reconnects
            jetstream_domain: None,
            jetstream_prefix: None,
        }
    }
}

/// NATS client wrapper with JetStream support
#[derive(Debug)]
pub struct NatsClient {
    /// The underlying NATS client
    client: Client,
    /// JetStream context for persistent messaging
    jetstream: JetStreamContext,
    /// Configuration used to establish the connection
    config: NatsConfig,
}

impl NatsClient {
    /// Connect to NATS server with the provided configuration
    pub async fn connect(config: NatsConfig) -> Result<Self, NatsError> {
        let mut options = ConnectOptions::new()
            .connection_timeout(Duration::from_secs(config.connection_timeout_secs))
            .reconnect_delay_callback(move |attempts| {
                let delay = Duration::from_secs(config.reconnect_interval_secs);
                if config.max_reconnects > 0 && attempts >= config.max_reconnects {
                    // Stop reconnecting after max attempts
                    Duration::from_secs(0)
                } else {
                    delay
                }
            })
            .event_callback(|event| async move {
                match event {
                    async_nats::Event::Disconnected => eprintln!("NATS disconnected"),
                    async_nats::Event::Connected => eprintln!("NATS connected"),
                    async_nats::Event::ClientError(err) => eprintln!("NATS client error: {err}"),
                    _ => {}
                }
            });

        // Add authentication if provided
        if let (Some(user), Some(password)) = (&config.user, &config.password) {
            options = options.user_and_password(user.clone(), password.clone());
        }

        // Add TLS if required
        if config.tls_required {
            options = options.require_tls(true);
        }

        // Connect to NATS
        let client = options
            .connect(&config.url)
            .await
            .map_err(|e| NatsError::ConnectionFailed(format!("Failed to connect to {}: {}", config.url, e)))?;

                // Create JetStream context
        let jetstream = jetstream::new(client.clone());

        // Note: domain and prefix configuration would be done at the stream level
        // The new async-nats API doesn't support these on the context itself

        Ok(Self {
            client,
            jetstream,
            config,
        })
    }

    /// Get the underlying NATS client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get the JetStream context
    pub fn jetstream(&self) -> &JetStreamContext {
        &self.jetstream
    }

    /// Get the configuration
    pub fn config(&self) -> &NatsConfig {
        &self.config
    }

    /// Check if the client is connected
    pub async fn is_connected(&self) -> bool {
        // Try to flush to check connection
        self.client.flush().await.is_ok()
    }

    /// Reconnect to NATS (useful after connection loss)
    pub async fn reconnect(&mut self) -> Result<(), NatsError> {
        *self = Self::connect(self.config.clone()).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NatsConfig::default();
        assert_eq!(config.url, "nats://localhost:4222");
        assert_eq!(config.connection_timeout_secs, 10);
        assert_eq!(config.max_reconnects, 0);
    }

    #[test]
    fn test_config_with_auth() {
        let config = NatsConfig {
            user: Some("testuser".to_string()),
            password: Some("testpass".to_string()),
            ..Default::default()
        };
        assert_eq!(config.user, Some("testuser".to_string()));
        assert_eq!(config.password, Some("testpass".to_string()));
    }
}
