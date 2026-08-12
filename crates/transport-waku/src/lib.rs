use std::collections::{HashMap, HashSet};
use std::time::Duration;

use chat_core::{ConversationId, MessageEnvelope, MessageId};
use serde::{Deserialize, Serialize};

fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(10))
        .build()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakuLightConfig {
    pub relay_enabled: bool,
    pub filter_enabled: bool,
    pub store_enabled: bool,
    pub light_push_enabled: bool,
}

pub trait WakuTransport {
    fn publish(&mut self, message: &MessageEnvelope) -> Result<(), String>;
    fn poll(&mut self) -> Result<Vec<MessageEnvelope>, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WakuPeerMode {
    EmbeddedLight,
    MobileWebLight,
    DesktopLight,
    WearableLight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WakuConnectionState {
    Disconnected,
    Connected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakuEndpointConfig {
    pub peer_mode: WakuPeerMode,
    pub relay_urls: Vec<String>,
    pub use_filter: bool,
    pub use_store: bool,
    pub use_light_push: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WakuSyncCursor {
    pub last_timestamp_ms: Option<i64>,
    pub last_message_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakuSessionPlan {
    pub endpoint: WakuEndpointConfig,
    pub subscriptions: Vec<TopicSubscription>,
    pub history_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakuGatewayBootstrap {
    pub endpoint: WakuEndpointConfig,
    pub subscriptions: Vec<TopicSubscription>,
    pub history_limit: usize,
}

impl From<WakuSessionPlan> for WakuGatewayBootstrap {
    fn from(plan: WakuSessionPlan) -> Self {
        Self {
            endpoint: plan.endpoint,
            subscriptions: plan.subscriptions,
            history_limit: plan.history_limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WakuGatewayRequest {
    Connect {
        endpoint: WakuEndpointConfig,
    },
    Subscribe {
        subscriptions: Vec<TopicSubscription>,
    },
    Publish {
        frame: EncodedFrame,
    },
    Recover {
        content_topic: String,
        cursor: WakuSyncCursor,
        limit: usize,
    },
    Poll {
        subscriptions: Vec<TopicSubscription>,
        limit: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WakuGatewayResponse {
    Connected,
    Subscribed,
    Published,
    Frames { frames: Vec<EncodedFrame> },
    Error { message: String },
}

pub trait WakuAdapter {
    fn connection_state(&self) -> WakuConnectionState;
    fn connect(&mut self, endpoint: WakuEndpointConfig) -> Result<(), String>;
    fn subscribe_topics(&mut self, subscriptions: &[TopicSubscription]) -> Result<(), String>;
    fn recover_since(
        &self,
        content_topic: &str,
        cursor: &WakuSyncCursor,
        limit: usize,
    ) -> Result<Vec<EncodedFrame>, String>;
    fn poll_frames(&mut self) -> Result<Vec<EncodedFrame>, String>;
}

pub trait WakuGatewayClient {
    fn connect_gateway(&mut self, endpoint: &WakuEndpointConfig) -> Result<(), String>;
    fn publish_frame(&mut self, frame: EncodedFrame) -> Result<(), String>;
    fn recover_frames(
        &self,
        content_topic: &str,
        cursor: &WakuSyncCursor,
        limit: usize,
    ) -> Result<Vec<EncodedFrame>, String>;
}

#[derive(Clone)]
pub struct HttpWakuGatewayClient {
    base_url: String,
    bearer_token: Option<String>,
}

impl std::fmt::Debug for HttpWakuGatewayClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpWakuGatewayClient")
            .field("base_url", &self.base_url)
            .field(
                "bearer_token",
                &self.bearer_token.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

impl HttpWakuGatewayClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_optional_bearer_token(base_url, None)
    }

    pub fn with_bearer_token(base_url: impl Into<String>, bearer_token: impl Into<String>) -> Self {
        Self::with_optional_bearer_token(base_url, Some(bearer_token.into()))
    }

    pub fn with_optional_bearer_token(
        base_url: impl Into<String>,
        bearer_token: Option<String>,
    ) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            bearer_token: bearer_token.and_then(|token| {
                let token = token.trim().to_string();
                (!token.is_empty()).then_some(token)
            }),
        }
    }

    pub fn healthcheck(&self) -> Result<(), String> {
        let url = format!("{}/health", self.base_url);
        let response = http_agent()
            .get(&url)
            .call()
            .map_err(|error| format!("gateway health check failed: {error}"))?;
        if response.status() == 200 {
            Ok(())
        } else {
            Err(format!(
                "gateway health check returned unexpected status: {}",
                response.status()
            ))
        }
    }

    fn send_request(&self, request: &WakuGatewayRequest) -> Result<WakuGatewayResponse, String> {
        let url = format!("{}/v1/waku", self.base_url);
        let agent = http_agent();
        let mut request_builder = agent.post(&url);
        if let Some(token) = self.bearer_token.as_deref() {
            request_builder = request_builder.set("Authorization", &format!("Bearer {token}"));
        }
        let response = request_builder
            .send_json(
                serde_json::to_value(request)
                    .map_err(|error| format!("serialize gateway request failed: {error}"))?,
            )
            .map_err(|error| format!("gateway request failed: {error}"))?;
        response
            .into_json::<WakuGatewayResponse>()
            .map_err(|error| format!("decode gateway response failed: {error}"))
    }

    fn expect_frames(response: WakuGatewayResponse) -> Result<Vec<EncodedFrame>, String> {
        match response {
            WakuGatewayResponse::Frames { frames } => Ok(frames),
            WakuGatewayResponse::Error { message } => Err(message),
            other => Err(format!("unexpected gateway response: {other:?}")),
        }
    }

    fn expect_ack(response: WakuGatewayResponse, expected: &str) -> Result<(), String> {
        match response {
            WakuGatewayResponse::Connected if expected == "connected" => Ok(()),
            WakuGatewayResponse::Subscribed if expected == "subscribed" => Ok(()),
            WakuGatewayResponse::Published if expected == "published" => Ok(()),
            WakuGatewayResponse::Error { message } => Err(message),
            other => Err(format!("unexpected gateway response: {other:?}")),
        }
    }
}

impl WakuGatewayClient for HttpWakuGatewayClient {
    fn connect_gateway(&mut self, endpoint: &WakuEndpointConfig) -> Result<(), String> {
        let response = self.send_request(&WakuGatewayRequest::Connect {
            endpoint: endpoint.clone(),
        })?;
        Self::expect_ack(response, "connected")
    }

    fn publish_frame(&mut self, frame: EncodedFrame) -> Result<(), String> {
        let response = self.send_request(&WakuGatewayRequest::Publish { frame })?;
        Self::expect_ack(response, "published")
    }

    fn recover_frames(
        &self,
        content_topic: &str,
        cursor: &WakuSyncCursor,
        limit: usize,
    ) -> Result<Vec<EncodedFrame>, String> {
        let response = self.send_request(&WakuGatewayRequest::Recover {
            content_topic: content_topic.to_string(),
            cursor: cursor.clone(),
            limit,
        })?;
        Self::expect_frames(response)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncodedFrame {
    pub content_topic: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicSubscription {
    pub content_topic: String,
    pub recover_history: bool,
}

pub struct WakuFrameCodec;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WakuMessageFrameV2 {
    version: u8,
    message: MessageEnvelope,
    reply_to_message_id: Option<MessageId>,
}

impl WakuFrameCodec {
    pub fn content_topic_for(conversation_id: &ConversationId) -> String {
        format!(
            "/lobster-chat/1/conversation/{}/messages",
            conversation_id.0
        )
    }

    pub fn encode(message: &MessageEnvelope) -> Result<EncodedFrame, String> {
        let payload = postcard::to_allocvec(&WakuMessageFrameV2 {
            version: 2,
            message: message.clone(),
            reply_to_message_id: message.reply_to_message_id.clone(),
        })
        .map_err(|error| format!("encode message frame failed: {error}"))?;

        Ok(EncodedFrame {
            content_topic: Self::content_topic_for(&message.conversation_id),
            payload,
        })
    }

    pub fn decode(payload: &[u8]) -> Result<MessageEnvelope, String> {
        if let Ok(frame) = postcard::from_bytes::<WakuMessageFrameV2>(payload)
            && frame.version == 2
        {
            let mut message = frame.message;
            message.reply_to_message_id = frame.reply_to_message_id;
            return Ok(message);
        }
        postcard::from_bytes::<MessageEnvelope>(payload)
            .map_err(|error| format!("decode message frame failed: {error}"))
    }
}

#[derive(Debug, Clone)]
pub struct GatewayBackedWakuAdapter<C> {
    client: C,
    connection_state: WakuConnectionState,
    endpoint: Option<WakuEndpointConfig>,
    subscriptions: Vec<TopicSubscription>,
    cursors: HashMap<String, WakuSyncCursor>,
    history_limit: usize,
}

impl<C> GatewayBackedWakuAdapter<C> {
    pub fn new(client: C, history_limit: usize) -> Self {
        Self {
            client,
            connection_state: WakuConnectionState::Disconnected,
            endpoint: None,
            subscriptions: Vec::new(),
            cursors: HashMap::new(),
            history_limit,
        }
    }

    pub fn gateway_client(&self) -> &C {
        &self.client
    }

    pub fn gateway_client_mut(&mut self) -> &mut C {
        &mut self.client
    }

    fn update_cursor_from_frames(&mut self, content_topic: &str, frames: &[EncodedFrame]) {
        if let Some(last_frame) = frames.last()
            && let Ok(message) = WakuFrameCodec::decode(&last_frame.payload)
        {
            self.cursors.insert(
                content_topic.to_string(),
                WakuSyncCursor {
                    last_timestamp_ms: Some(message.timestamp_ms),
                    last_message_id: Some(message.message_id.0),
                },
            );
        }
    }

    pub fn session_bootstrap(&self) -> Option<WakuGatewayBootstrap> {
        self.endpoint.clone().map(|endpoint| WakuGatewayBootstrap {
            endpoint,
            subscriptions: self.subscriptions.clone(),
            history_limit: self.history_limit,
        })
    }
}

impl<C> WakuAdapter for GatewayBackedWakuAdapter<C>
where
    C: WakuGatewayClient,
{
    fn connection_state(&self) -> WakuConnectionState {
        self.connection_state
    }

    fn connect(&mut self, endpoint: WakuEndpointConfig) -> Result<(), String> {
        self.client.connect_gateway(&endpoint)?;
        self.endpoint = Some(endpoint);
        self.connection_state = WakuConnectionState::Connected;
        Ok(())
    }

    fn subscribe_topics(&mut self, subscriptions: &[TopicSubscription]) -> Result<(), String> {
        self.subscriptions = subscriptions.to_vec();
        for subscription in subscriptions {
            self.cursors
                .entry(subscription.content_topic.clone())
                .or_insert_with(|| {
                    if subscription.recover_history {
                        WakuSyncCursor::default()
                    } else {
                        WakuSyncCursor {
                            last_timestamp_ms: Some(i64::MAX),
                            last_message_id: None,
                        }
                    }
                });
        }
        Ok(())
    }

    fn recover_since(
        &self,
        content_topic: &str,
        cursor: &WakuSyncCursor,
        limit: usize,
    ) -> Result<Vec<EncodedFrame>, String> {
        self.client.recover_frames(content_topic, cursor, limit)
    }

    fn poll_frames(&mut self) -> Result<Vec<EncodedFrame>, String> {
        let topics = self.subscriptions.clone();
        let mut collected = Vec::new();

        for subscription in topics {
            let cursor = self
                .cursors
                .get(&subscription.content_topic)
                .cloned()
                .unwrap_or_default();
            let frames = self
                .client
                .recover_frames(&subscription.content_topic, &cursor, self.history_limit)
                .inspect_err(|_| {
                    self.connection_state = WakuConnectionState::Disconnected;
                })?;

            self.update_cursor_from_frames(&subscription.content_topic, &frames);
            collected.extend(frames);
        }

        self.connection_state = WakuConnectionState::Connected;
        Ok(collected)
    }
}

impl<C> WakuTransport for GatewayBackedWakuAdapter<C>
where
    C: WakuGatewayClient,
{
    fn publish(&mut self, message: &MessageEnvelope) -> Result<(), String> {
        let frame = WakuFrameCodec::encode(message)?;
        self.client.publish_frame(frame).inspect_err(|_| {
            self.connection_state = WakuConnectionState::Disconnected;
        })?;
        self.connection_state = WakuConnectionState::Connected;
        Ok(())
    }

    fn poll(&mut self) -> Result<Vec<MessageEnvelope>, String> {
        let mut messages = self
            .poll_frames()?
            .into_iter()
            .map(|frame| WakuFrameCodec::decode(&frame.payload))
            .collect::<Result<Vec<_>, _>>()?;
        messages.sort_by_key(|message| message.timestamp_ms);
        Ok(messages)
    }
}

#[derive(Debug, Clone)]
pub struct InMemoryWakuLightNode {
    pub mode: WakuPeerMode,
    pub config: WakuLightConfig,
    connection_state: WakuConnectionState,
    endpoint: Option<WakuEndpointConfig>,
    subscriptions: HashSet<String>,
    topic_frames: HashMap<String, Vec<EncodedFrame>>,
    topic_cursor: HashMap<String, usize>,
}

impl InMemoryWakuLightNode {
    pub fn new(mode: WakuPeerMode, config: WakuLightConfig) -> Self {
        Self {
            mode,
            config,
            connection_state: WakuConnectionState::Disconnected,
            endpoint: None,
            subscriptions: HashSet::new(),
            topic_frames: HashMap::new(),
            topic_cursor: HashMap::new(),
        }
    }

    pub fn subscribe(&mut self, subscription: TopicSubscription) {
        self.subscriptions
            .insert(subscription.content_topic.clone());

        let cursor = if subscription.recover_history {
            0
        } else {
            self.topic_frames
                .get(&subscription.content_topic)
                .map(|frames| frames.len())
                .unwrap_or(0)
        };
        self.topic_cursor.insert(subscription.content_topic, cursor);
    }

    pub fn recover_recent(
        &self,
        content_topic: &str,
        limit: usize,
    ) -> Result<Vec<MessageEnvelope>, String> {
        let Some(frames) = self.topic_frames.get(content_topic) else {
            return Ok(Vec::new());
        };

        frames
            .iter()
            .rev()
            .take(limit)
            .map(|frame| WakuFrameCodec::decode(&frame.payload))
            .collect::<Result<Vec<_>, _>>()
            .map(|mut items| {
                items.reverse();
                items
            })
    }
}

impl WakuAdapter for InMemoryWakuLightNode {
    fn connection_state(&self) -> WakuConnectionState {
        self.connection_state
    }

    fn connect(&mut self, endpoint: WakuEndpointConfig) -> Result<(), String> {
        self.mode = endpoint.peer_mode;
        self.config.filter_enabled = endpoint.use_filter;
        self.config.store_enabled = endpoint.use_store;
        self.config.light_push_enabled = endpoint.use_light_push;
        self.endpoint = Some(endpoint);
        self.connection_state = WakuConnectionState::Connected;
        Ok(())
    }

    fn subscribe_topics(&mut self, subscriptions: &[TopicSubscription]) -> Result<(), String> {
        for subscription in subscriptions {
            self.subscribe(subscription.clone());
        }
        Ok(())
    }

    fn recover_since(
        &self,
        content_topic: &str,
        cursor: &WakuSyncCursor,
        limit: usize,
    ) -> Result<Vec<EncodedFrame>, String> {
        let Some(frames) = self.topic_frames.get(content_topic) else {
            return Ok(Vec::new());
        };

        let filtered = frames
            .iter()
            .filter_map(|frame| {
                let message = WakuFrameCodec::decode(&frame.payload).ok()?;
                let keep = match cursor.last_timestamp_ms {
                    Some(last_ts) => message.timestamp_ms > last_ts,
                    None => true,
                };
                if keep { Some(frame.clone()) } else { None }
            })
            .take(limit)
            .collect::<Vec<_>>();

        Ok(filtered)
    }

    fn poll_frames(&mut self) -> Result<Vec<EncodedFrame>, String> {
        let messages = self.poll()?;
        messages
            .iter()
            .map(WakuFrameCodec::encode)
            .collect::<Result<Vec<_>, _>>()
    }
}

impl WakuGatewayClient for InMemoryWakuLightNode {
    fn connect_gateway(&mut self, endpoint: &WakuEndpointConfig) -> Result<(), String> {
        self.connect(endpoint.clone())
    }

    fn publish_frame(&mut self, frame: EncodedFrame) -> Result<(), String> {
        let message = WakuFrameCodec::decode(&frame.payload)?;
        self.publish(&message)
    }

    fn recover_frames(
        &self,
        content_topic: &str,
        cursor: &WakuSyncCursor,
        limit: usize,
    ) -> Result<Vec<EncodedFrame>, String> {
        self.recover_since(content_topic, cursor, limit)
    }
}

impl WakuTransport for InMemoryWakuLightNode {
    fn publish(&mut self, message: &MessageEnvelope) -> Result<(), String> {
        if !self.config.light_push_enabled && !self.config.relay_enabled {
            return Err(
                "waku light node cannot publish when relay and light-push are disabled".into(),
            );
        }

        let frame = WakuFrameCodec::encode(message)?;
        self.topic_frames
            .entry(frame.content_topic.clone())
            .or_default()
            .push(frame);
        Ok(())
    }

    fn poll(&mut self) -> Result<Vec<MessageEnvelope>, String> {
        if !self.config.filter_enabled && !self.config.store_enabled {
            return Ok(Vec::new());
        }

        let mut messages = Vec::new();
        let topics = self.subscriptions.clone();

        for topic in topics {
            let start = self.topic_cursor.get(&topic).copied().unwrap_or(0);
            let frames = self.topic_frames.get(&topic).cloned().unwrap_or_default();

            for frame in frames.iter().skip(start) {
                messages.push(WakuFrameCodec::decode(&frame.payload)?);
            }

            self.topic_cursor.insert(topic, frames.len());
        }

        messages.sort_by_key(|message| message.timestamp_ms);
        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use super::*;
    use chat_core::{
        ClientProfile, ConversationId, DeviceId, IdentityId, MessageBody, MessageEnvelope,
        MessageId, PayloadType,
    };

    fn sample_message() -> MessageEnvelope {
        MessageEnvelope {
            message_id: MessageId("msg-1".into()),
            conversation_id: ConversationId("dm:rsaga:builder".into()),
            sender: IdentityId("rsaga".into()),
            reply_to_message_id: None,
            sender_device: DeviceId("desktop-1".into()),
            sender_profile: ClientProfile::desktop_terminal(),
            payload_type: PayloadType::Text,
            body: MessageBody {
                preview: "hello".into(),
                plain_text: "hello from lobster".into(),
                language_tag: "zh-CN".into(),
            },
            ciphertext: vec![1, 2, 3, 4],
            timestamp_ms: 1_763_560_000_000,
            ephemeral: false,
        }
    }

    #[test]
    fn postcard_roundtrip_preserves_message() {
        let message = sample_message();
        let frame = WakuFrameCodec::encode(&message).expect("frame should encode");
        let decoded = WakuFrameCodec::decode(&frame.payload).expect("frame should decode");

        assert_eq!(
            frame.content_topic,
            "/lobster-chat/1/conversation/dm:rsaga:builder/messages"
        );
        assert_eq!(decoded, message);
    }

    fn light_config() -> WakuLightConfig {
        WakuLightConfig {
            relay_enabled: false,
            filter_enabled: true,
            store_enabled: true,
            light_push_enabled: true,
        }
    }

    fn endpoint_config() -> WakuEndpointConfig {
        WakuEndpointConfig {
            peer_mode: WakuPeerMode::DesktopLight,
            relay_urls: vec!["/dns4/test-waku/tcp/443/wss".into()],
            use_filter: true,
            use_store: true,
            use_light_push: true,
        }
    }

    #[test]
    fn subscribed_topics_receive_new_messages() {
        let mut node = InMemoryWakuLightNode::new(WakuPeerMode::EmbeddedLight, light_config());
        let message = sample_message();
        let topic = WakuFrameCodec::content_topic_for(&message.conversation_id);

        node.subscribe(TopicSubscription {
            content_topic: topic,
            recover_history: true,
        });
        node.publish(&message).expect("publish should succeed");

        let polled = node.poll().expect("poll should succeed");
        assert_eq!(polled, vec![message]);
    }

    #[test]
    fn recover_recent_reads_topic_history() {
        let mut node = InMemoryWakuLightNode::new(WakuPeerMode::MobileWebLight, light_config());
        let mut second = sample_message();
        second.message_id = MessageId("msg-2".into());
        second.timestamp_ms += 10;

        let topic = WakuFrameCodec::content_topic_for(&second.conversation_id);
        node.publish(&sample_message()).expect("publish one");
        node.publish(&second).expect("publish two");

        let recovered = node
            .recover_recent(&topic, 1)
            .expect("recover should succeed");
        assert_eq!(recovered, vec![second]);
    }

    #[test]
    fn adapter_connects_and_polls_encoded_frames() {
        let mut node = InMemoryWakuLightNode::new(WakuPeerMode::EmbeddedLight, light_config());
        node.connect(endpoint_config())
            .expect("connect should succeed");
        assert_eq!(node.connection_state(), WakuConnectionState::Connected);

        let message = sample_message();
        let topic = WakuFrameCodec::content_topic_for(&message.conversation_id);
        node.subscribe_topics(&[TopicSubscription {
            content_topic: topic,
            recover_history: true,
        }])
        .expect("subscribe should succeed");
        node.publish(&message).expect("publish should succeed");

        let frames = node.poll_frames().expect("poll frames should succeed");
        assert_eq!(frames.len(), 1);
    }

    #[test]
    fn adapter_recovers_since_cursor() {
        let mut node = InMemoryWakuLightNode::new(WakuPeerMode::DesktopLight, light_config());
        let first = sample_message();
        let mut second = sample_message();
        second.message_id = MessageId("msg-2".into());
        second.timestamp_ms += 10;
        let topic = WakuFrameCodec::content_topic_for(&first.conversation_id);

        node.publish(&first).expect("publish first");
        node.publish(&second).expect("publish second");

        let frames = node
            .recover_since(
                &topic,
                &WakuSyncCursor {
                    last_timestamp_ms: Some(first.timestamp_ms),
                    last_message_id: Some(first.message_id.0.clone()),
                },
                10,
            )
            .expect("recover since should succeed");

        assert_eq!(frames.len(), 1);
        let decoded = WakuFrameCodec::decode(&frames[0].payload).expect("decode");
        assert_eq!(decoded, second);
    }

    #[test]
    fn gateway_adapter_uses_gateway_client_for_publish_and_poll() {
        let gateway = InMemoryWakuLightNode::new(WakuPeerMode::DesktopLight, light_config());
        let mut adapter = GatewayBackedWakuAdapter::new(gateway, 16);
        adapter.connect(endpoint_config()).expect("connect gateway");

        let message = sample_message();
        let topic = WakuFrameCodec::content_topic_for(&message.conversation_id);
        adapter
            .subscribe_topics(&[TopicSubscription {
                content_topic: topic,
                recover_history: true,
            }])
            .expect("subscribe");
        adapter.publish(&message).expect("publish through gateway");

        let messages = adapter.poll().expect("poll through gateway");
        assert_eq!(messages, vec![message]);
    }

    #[test]
    fn gateway_adapter_reuses_cursor_between_polls() {
        let gateway = InMemoryWakuLightNode::new(WakuPeerMode::DesktopLight, light_config());
        let mut adapter = GatewayBackedWakuAdapter::new(gateway, 16);
        adapter.connect(endpoint_config()).expect("connect gateway");

        let first = sample_message();
        let mut second = sample_message();
        second.message_id = MessageId("msg-2".into());
        second.timestamp_ms += 50;
        let topic = WakuFrameCodec::content_topic_for(&first.conversation_id);

        adapter
            .subscribe_topics(&[TopicSubscription {
                content_topic: topic,
                recover_history: true,
            }])
            .expect("subscribe");
        adapter.publish(&first).expect("publish first");
        let first_poll = adapter.poll().expect("first poll");
        assert_eq!(first_poll, vec![first.clone()]);

        adapter.publish(&second).expect("publish second");
        let second_poll = adapter.poll().expect("second poll");
        assert_eq!(second_poll, vec![second]);
    }

    #[test]
    fn gateway_request_and_response_roundtrip_via_json() {
        let request = WakuGatewayRequest::Recover {
            content_topic: "/lobster-chat/1/conversation/dm:rsaga:builder/messages".into(),
            cursor: WakuSyncCursor {
                last_timestamp_ms: Some(1_763_560_000_000),
                last_message_id: Some("msg-1".into()),
            },
            limit: 32,
        };
        let encoded = serde_json::to_vec(&request).expect("request should serialize");
        let decoded: WakuGatewayRequest =
            serde_json::from_slice(&encoded).expect("request should deserialize");
        assert_eq!(decoded, request);

        let response = WakuGatewayResponse::Frames {
            frames: vec![WakuFrameCodec::encode(&sample_message()).expect("frame should encode")],
        };
        let encoded = serde_json::to_vec(&response).expect("response should serialize");
        let decoded: WakuGatewayResponse =
            serde_json::from_slice(&encoded).expect("response should deserialize");
        assert_eq!(decoded, response);
    }

    #[test]
    fn session_bootstrap_reflects_connected_endpoint_and_subscriptions() {
        let gateway = InMemoryWakuLightNode::new(WakuPeerMode::DesktopLight, light_config());
        let mut adapter = GatewayBackedWakuAdapter::new(gateway, 64);
        adapter
            .connect(endpoint_config())
            .expect("connect should succeed");
        adapter
            .subscribe_topics(&[TopicSubscription {
                content_topic: "/lobster-chat/demo/topic".into(),
                recover_history: true,
            }])
            .expect("subscribe should succeed");

        let bootstrap = adapter
            .session_bootstrap()
            .expect("connected adapter should provide bootstrap");
        assert_eq!(bootstrap.endpoint.peer_mode, WakuPeerMode::DesktopLight);
        assert_eq!(bootstrap.subscriptions.len(), 1);
        assert_eq!(bootstrap.history_limit, 64);
    }

    #[derive(Default)]
    struct ScriptedGatewayClient {
        recover_results: RefCell<VecDeque<Result<Vec<EncodedFrame>, String>>>,
    }

    impl ScriptedGatewayClient {
        fn with_recover_results(results: Vec<Result<Vec<EncodedFrame>, String>>) -> Self {
            Self {
                recover_results: RefCell::new(results.into()),
            }
        }
    }

    impl WakuGatewayClient for ScriptedGatewayClient {
        fn connect_gateway(&mut self, _endpoint: &WakuEndpointConfig) -> Result<(), String> {
            Ok(())
        }

        fn publish_frame(&mut self, _frame: EncodedFrame) -> Result<(), String> {
            Ok(())
        }

        fn recover_frames(
            &self,
            _content_topic: &str,
            _cursor: &WakuSyncCursor,
            _limit: usize,
        ) -> Result<Vec<EncodedFrame>, String> {
            self.recover_results
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Ok(Vec::new()))
        }
    }

    #[test]
    fn gateway_adapter_marks_disconnected_after_poll_error_and_recovers_on_retry() {
        let message = sample_message();
        let topic = WakuFrameCodec::content_topic_for(&message.conversation_id);
        let frame = WakuFrameCodec::encode(&message).expect("encode frame");
        let client = ScriptedGatewayClient::with_recover_results(vec![
            Err("gateway unavailable".into()),
            Ok(vec![frame]),
        ]);
        let mut adapter = GatewayBackedWakuAdapter::new(client, 16);
        adapter.connect(endpoint_config()).expect("connect gateway");
        adapter
            .subscribe_topics(&[TopicSubscription {
                content_topic: topic,
                recover_history: true,
            }])
            .expect("subscribe");

        let error = adapter.poll().expect_err("first poll should fail");
        assert!(error.contains("gateway unavailable"));
        assert_eq!(
            adapter.connection_state(),
            WakuConnectionState::Disconnected
        );

        let recovered = adapter.poll().expect("second poll should recover");
        assert_eq!(recovered, vec![message]);
        assert_eq!(adapter.connection_state(), WakuConnectionState::Connected);
    }
}
