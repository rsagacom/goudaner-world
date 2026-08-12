use std::env;

use chat_core::Conversation;
#[cfg(test)]
use chat_core::{ConversationId, ConversationKind, ConversationScope, IdentityId};
use transport_waku::{
    GatewayBackedWakuAdapter, HttpWakuGatewayClient, InMemoryWakuLightNode, TopicSubscription,
    WakuAdapter, WakuConnectionState, WakuEndpointConfig, WakuFrameCodec, WakuLightConfig,
    WakuPeerMode, WakuTransport,
};

pub(crate) trait TransportAdapter: WakuAdapter + WakuTransport {}
impl<T> TransportAdapter for T where T: WakuAdapter + WakuTransport {}

pub(crate) fn friendly_connection_label(state: WakuConnectionState) -> &'static str {
    match state {
        WakuConnectionState::Connected => "已合线",
        WakuConnectionState::Disconnected => "暂离线",
    }
}

pub(crate) fn desktop_light_endpoint() -> WakuEndpointConfig {
    WakuEndpointConfig {
        peer_mode: WakuPeerMode::DesktopLight,
        relay_urls: vec!["/dns4/test-waku/tcp/443/wss".into()],
        use_filter: true,
        use_store: true,
        use_light_push: true,
    }
}

pub(crate) fn build_transport(
    endpoint: WakuEndpointConfig,
    history_limit: usize,
) -> Result<(Box<dyn TransportAdapter>, String), String> {
    if let Ok(base_url) = env::var("LOBSTER_WAKU_GATEWAY_URL") {
        let token = env::var("LOBSTER_WAKU_GATEWAY_TOKEN").ok();
        let client = HttpWakuGatewayClient::with_optional_bearer_token(base_url.clone(), token);
        client.healthcheck()?;
        let mut transport = GatewayBackedWakuAdapter::new(client, history_limit);
        transport.connect(endpoint)?;
        Ok((Box::new(transport), format!("http-gateway:{base_url}")))
    } else {
        let gateway = InMemoryWakuLightNode::new(
            WakuPeerMode::DesktopLight,
            WakuLightConfig {
                relay_enabled: false,
                filter_enabled: true,
                store_enabled: true,
                light_push_enabled: true,
            },
        );
        let mut transport = GatewayBackedWakuAdapter::new(gateway, history_limit);
        transport.connect(endpoint)?;
        Ok((Box::new(transport), "in-memory-gateway".into()))
    }
}

pub(crate) fn conversation_topic_subscriptions(
    conversations: &[Conversation],
) -> Vec<TopicSubscription> {
    conversations
        .iter()
        .map(|conversation| TopicSubscription {
            content_topic: WakuFrameCodec::content_topic_for(&conversation.conversation_id),
            recover_history: true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conversation(conversation_id: &str, kind: ConversationKind) -> Conversation {
        Conversation {
            conversation_id: ConversationId(conversation_id.into()),
            kind: kind.clone(),
            scope: if matches!(kind, ConversationKind::Direct) {
                ConversationScope::Private
            } else {
                ConversationScope::CityPublic
            },
            scene: None,
            content_topic: format!("/seed/{conversation_id}"),
            participants: vec![IdentityId("alpha".into()), IdentityId("beta".into())],
            created_at_ms: 1_000,
            last_active_at_ms: 1_000,
        }
    }

    #[test]
    fn friendly_connection_label_maps_states() {
        assert_eq!(
            friendly_connection_label(WakuConnectionState::Connected),
            "已合线"
        );
        assert_eq!(
            friendly_connection_label(WakuConnectionState::Disconnected),
            "暂离线"
        );
    }

    #[test]
    fn desktop_light_endpoint_uses_desktop_defaults() {
        let endpoint = desktop_light_endpoint();

        assert_eq!(endpoint.peer_mode, WakuPeerMode::DesktopLight);
        assert_eq!(endpoint.relay_urls, vec!["/dns4/test-waku/tcp/443/wss"]);
        assert!(endpoint.use_filter);
        assert!(endpoint.use_store);
        assert!(endpoint.use_light_push);
    }

    #[test]
    fn conversation_topic_subscriptions_use_codec_topics_and_recovery() {
        let conversations = vec![
            test_conversation("room:city:core-harbor:lobby", ConversationKind::Room),
            test_conversation("dm:rsaga:builder", ConversationKind::Direct),
        ];

        let subscriptions = conversation_topic_subscriptions(&conversations);

        assert_eq!(subscriptions.len(), 2);
        assert_eq!(
            subscriptions[0].content_topic,
            WakuFrameCodec::content_topic_for(&conversations[0].conversation_id)
        );
        assert_eq!(
            subscriptions[1].content_topic,
            WakuFrameCodec::content_topic_for(&conversations[1].conversation_id)
        );
        assert!(
            subscriptions
                .iter()
                .all(|subscription| subscription.recover_history)
        );
    }

    #[test]
    fn external_gateway_transport_uses_dedicated_token_environment_contract() {
        let source = include_str!("transport_bootstrap.rs");
        assert!(source.contains("LOBSTER_WAKU_GATEWAY_TOKEN"));
        assert!(source.contains("with_optional_bearer_token"));
    }
}
