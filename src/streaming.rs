//! Streaming subscription for audit entries via majra pub/sub.
//!
//! Publishes each appended entry to a configurable topic hierarchy.
//! Subscribers receive entries in real-time using MQTT-style wildcard patterns.
//!
//! Requires the `streaming` feature flag.
//!
//! # Topic hierarchy
//!
//! Entries are published to `libro/{source}/{action}`, enabling subscribers
//! to filter by source and action using wildcards:
//! - `libro/#` — all entries
//! - `libro/daimon/#` — all daimon entries
//! - `libro/*/agent.start` — agent.start from any source
//!
//! # Usage
//!
//! ```rust,ignore
//! use libro::streaming::AuditStream;
//! use libro::{AuditChain, EventSeverity};
//!
//! let stream = AuditStream::new();
//! let mut rx = stream.subscribe("libro/#");
//!
//! let mut chain = AuditChain::new();
//! let entry = chain.append(EventSeverity::Info, "daimon", "agent.start", serde_json::json!({}));
//! stream.publish(entry);
//! ```

use majra::pubsub::{TypedMessage, TypedPubSub};
use tokio::sync::broadcast;
use tracing::debug;

use crate::entry::AuditEntry;

/// A received audit entry from a stream subscription.
pub type StreamMessage = TypedMessage<AuditEntry>;

/// A streaming pub/sub hub for audit entries.
///
/// Wraps majra's [`TypedPubSub`] to publish audit entries as they are appended.
pub struct AuditStream {
    hub: TypedPubSub<AuditEntry>,
    topic_prefix: String,
}

impl AuditStream {
    /// Create a new audit stream with the default topic prefix `"libro"`.
    pub fn new() -> Self {
        Self {
            hub: TypedPubSub::new(),
            topic_prefix: "libro".to_owned(),
        }
    }

    /// Create a new audit stream with a custom topic prefix.
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            hub: TypedPubSub::new(),
            topic_prefix: prefix.into(),
        }
    }

    /// Build the topic string for an entry: `{prefix}/{source}/{action}`.
    fn topic_for(&self, entry: &AuditEntry) -> String {
        format!("{}/{}/{}", self.topic_prefix, entry.source(), entry.action())
    }

    /// Publish an audit entry to the stream.
    ///
    /// The entry is published to `{prefix}/{source}/{action}`.
    pub fn publish(&self, entry: &AuditEntry) {
        let topic = self.topic_for(entry);
        debug!(topic = %topic, hash = entry.hash(), "entry published to stream");
        self.hub.publish(&topic, entry.clone());
    }

    /// Subscribe to entries matching a topic pattern.
    ///
    /// Uses MQTT-style wildcards:
    /// - `libro/#` — all entries
    /// - `libro/daimon/*` — all actions from daimon
    /// - `libro/*/alert` — alerts from any source
    pub fn subscribe(
        &self,
        pattern: &str,
    ) -> broadcast::Receiver<StreamMessage> {
        self.hub.subscribe(pattern)
    }

    /// Number of active subscription patterns.
    pub fn pattern_count(&self) -> usize {
        self.hub.pattern_count()
    }

    /// Total entries published since creation.
    pub fn entries_published(&self) -> u64 {
        self.hub.messages_published()
    }
}

impl Default for AuditStream {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AuditStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditStream")
            .field("prefix", &self.topic_prefix)
            .field("patterns", &self.hub.pattern_count())
            .field("published", &self.hub.messages_published())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::EventSeverity;
    use crate::AuditChain;

    #[tokio::test]
    async fn publish_subscribe_entry() {
        let stream = AuditStream::new();
        let mut rx = stream.subscribe("libro/#");

        let mut chain = AuditChain::new();
        let entry = chain.append(EventSeverity::Info, "daimon", "agent.start", serde_json::json!({}));
        stream.publish(entry);

        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.topic, "libro/daimon/agent.start");
        assert_eq!(msg.payload.hash(), entry.hash());
        assert_eq!(stream.entries_published(), 1);
    }

    #[tokio::test]
    async fn subscribe_filtered_by_source() {
        let stream = AuditStream::new();
        let mut rx_daimon = stream.subscribe("libro/daimon/#");
        let mut rx_aegis = stream.subscribe("libro/aegis/#");

        let mut chain = AuditChain::new();
        let e1 = chain.append(EventSeverity::Info, "daimon", "start", serde_json::json!({}));
        stream.publish(e1);
        let e2 = chain.append(EventSeverity::Security, "aegis", "alert", serde_json::json!({}));
        stream.publish(e2);

        let msg = rx_daimon.recv().await.unwrap();
        assert_eq!(msg.payload.source(), "daimon");
        assert!(rx_daimon.try_recv().is_err()); // no more for daimon

        let msg = rx_aegis.recv().await.unwrap();
        assert_eq!(msg.payload.source(), "aegis");
        assert!(rx_aegis.try_recv().is_err());
    }

    #[tokio::test]
    async fn subscribe_by_action_wildcard() {
        let stream = AuditStream::new();
        let mut rx = stream.subscribe("libro/*/alert");

        let mut chain = AuditChain::new();
        let e1 = chain.append(EventSeverity::Info, "daimon", "start", serde_json::json!({}));
        stream.publish(e1);
        let e2 = chain.append(EventSeverity::Security, "aegis", "alert", serde_json::json!({}));
        stream.publish(e2);

        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.payload.action(), "alert");
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn no_delivery_on_mismatch() {
        let stream = AuditStream::new();
        let mut rx = stream.subscribe("libro/aegis/#");

        let mut chain = AuditChain::new();
        let entry = chain.append(EventSeverity::Info, "daimon", "start", serde_json::json!({}));
        stream.publish(entry);

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn custom_prefix() {
        let stream = AuditStream::with_prefix("audit/prod");
        let entry = AuditEntry::new(EventSeverity::Info, "daimon", "start", serde_json::json!({}), "");
        assert_eq!(stream.topic_for(&entry), "audit/prod/daimon/start");
    }

    #[test]
    fn debug_display() {
        let stream = AuditStream::new();
        let debug = format!("{stream:?}");
        assert!(debug.contains("AuditStream"));
        assert!(debug.contains("libro"));
    }

    #[tokio::test]
    async fn multiple_entries_stream() {
        let stream = AuditStream::new();
        let mut rx = stream.subscribe("libro/#");

        let mut chain = AuditChain::new();
        for i in 0..5 {
            let entry = chain.append(
                EventSeverity::Info,
                "src",
                format!("e{i}"),
                serde_json::json!({}),
            );
            stream.publish(entry);
        }

        for i in 0..5 {
            let msg = rx.recv().await.unwrap();
            assert_eq!(msg.payload.action(), format!("e{i}"));
        }
        assert_eq!(stream.entries_published(), 5);
    }
}
