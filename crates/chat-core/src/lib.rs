use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdentityId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub String);

pub fn canonical_direct_conversation_id(a: &IdentityId, b: &IdentityId) -> ConversationId {
    let mut ids = [a.0.clone(), b.0.clone()];
    ids.sort();
    ConversationId(format!("dm:{}:{}", ids[0], ids[1]))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CityId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientClass {
    Embedded,
    Desktop,
    MobileWeb,
    Wearable,
    Service,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientProfile {
    pub class: ClientClass,
    pub display_name: String,
    pub max_memory_kib: u32,
    pub supports_graphics: bool,
    pub supports_voice: bool,
    pub supports_camera: bool,
    pub supports_background_sync: bool,
}

impl ClientProfile {
    pub fn lobster_embedded() -> Self {
        Self {
            class: ClientClass::Embedded,
            display_name: "Lobster Embedded Host".into(),
            max_memory_kib: 4096,
            supports_graphics: false,
            supports_voice: false,
            supports_camera: false,
            supports_background_sync: true,
        }
    }

    pub fn desktop_terminal() -> Self {
        Self {
            class: ClientClass::Desktop,
            display_name: "Desktop Terminal".into(),
            max_memory_kib: 65536,
            supports_graphics: false,
            supports_voice: true,
            supports_camera: false,
            supports_background_sync: true,
        }
    }

    pub fn wearable_glasses() -> Self {
        Self {
            class: ClientClass::Wearable,
            display_name: "Wearable Glasses".into(),
            max_memory_kib: 2048,
            supports_graphics: false,
            supports_voice: true,
            supports_camera: true,
            supports_background_sync: false,
        }
    }

    pub fn mobile_web() -> Self {
        Self {
            class: ClientClass::MobileWeb,
            display_name: "Mobile Web".into(),
            max_memory_kib: 8192,
            supports_graphics: true,
            supports_voice: true,
            supports_camera: false,
            supports_background_sync: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayloadType {
    Text,
    System,
    AttachmentRef,
    Control,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageBody {
    pub preview: String,
    /// User-visible plaintext. In the current MLS skeleton phase this remains the privacy boundary;
    /// any transport placeholder that mirrors it must not be mistaken for end-to-end secrecy.
    pub plain_text: String,
    pub language_tag: String,
}

/// `MessageBody::plain_text` prefix that marks an image attachment reference.
/// The remainder is a 32-char hex id, optionally followed by `\n` + caption.
pub const ATTACHMENT_URL_PREFIX: &str = "attachment://";
const ATTACHMENT_ID_LEN: usize = 32;

/// Attachment ids are 128 bits of CSPRNG entropy rendered as lowercase hex.
pub fn validate_attachment_id(raw: &str) -> bool {
    raw.len() == ATTACHMENT_ID_LEN
        && raw
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// Split a stored `plain_text` value into `(attachment id, caption)`.
/// Values without a valid attachment reference come back as `(None, plain_text)`.
pub fn split_attachment_text(plain_text: &str) -> (Option<String>, String) {
    let Some(rest) = plain_text.strip_prefix(ATTACHMENT_URL_PREFIX) else {
        return (None, plain_text.to_string());
    };
    match rest.split_once('\n') {
        Some((id, caption)) if validate_attachment_id(id) => {
            (Some(id.to_string()), caption.to_string())
        }
        None if validate_attachment_id(rest) => (Some(rest.to_string()), String::new()),
        _ => (None, plain_text.to_string()),
    }
}

/// Encode an attachment reference (id + caption) for `MessageBody::plain_text`.
pub fn attachment_reference(id: &str, caption: &str) -> String {
    if caption.is_empty() {
        format!("{ATTACHMENT_URL_PREFIX}{id}")
    } else {
        format!("{ATTACHMENT_URL_PREFIX}{id}\n{caption}")
    }
}

/// Human-facing single-line fallback for clients that cannot render images
/// (TUI transcript, plain exports). `None` when the text is not a valid
/// attachment reference and must be shown as-is.
pub fn attachment_display_text(plain_text: &str) -> Option<String> {
    let (id, caption) = split_attachment_text(plain_text);
    id?;
    Some(if caption.is_empty() {
        "[图片]".to_string()
    } else {
        format!("[图片] {caption}")
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub message_id: MessageId,
    pub conversation_id: ConversationId,
    pub sender: IdentityId,
    pub sender_device: DeviceId,
    pub sender_profile: ClientProfile,
    pub payload_type: PayloadType,
    pub body: MessageBody,
    /// Transport bytes emitted by the active secure-session backend. The skeleton MLS backend only
    /// serializes the plaintext envelope into this field to preserve wire shape while crypto is stubbed.
    pub ciphertext: Vec<u8>,
    pub timestamp_ms: i64,
    pub ephemeral: bool,
    pub reply_to_message_id: Option<MessageId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryState {
    LocalOnly,
    PendingNetwork,
    Delivered,
    ArchivedLocal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationKind {
    Direct,
    Room,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationScope {
    Private,
    CityPublic,
    CityPrivate,
    CrossCityShared,
}

fn default_conversation_scope() -> ConversationScope {
    ConversationScope::Private
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneScope {
    City,
    PublicRoom,
    PersonalRoom,
    DirectRoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneRenderStyle {
    AsciiFallback,
    FcSymbolic,
    SfcPixel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentScope {
    Personal,
    Room,
    City,
    World,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentUseCase {
    Caretaking,
    Decoration,
    Commerce,
    Research,
    Moderation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSceneSlot {
    pub slot_id: String,
    pub display_name: String,
    pub scope: AgentScope,
    pub use_cases: Vec<AgentUseCase>,
    pub appearance_hint: String,
    pub can_leave_messages: bool,
    pub can_edit_scene: bool,
    pub can_trade_goods: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelAvatarProfile {
    pub avatar_id: String,
    pub display_name: String,
    pub archetype: String,
    pub palette_hint: String,
    pub accessory_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneLandmark {
    pub slot_id: String,
    pub label: String,
    pub sprite_hint: String,
    pub interaction_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneImageLayer {
    pub layer_id: String,
    pub preset: String,
    pub asset_hint: String,
    /// Scene canvas height / width * 10000 (16:9 is 5625).
    pub aspect_ratio_permyriad: u16,
    pub owner_editable: bool,
    /// 自定义白天背景图（与 night 必须同时提供或同时为空）
    #[serde(default)]
    pub day_image_url: Option<String>,
    /// 自定义夜晚背景图（与 day 必须同时提供或同时为空）
    #[serde(default)]
    pub night_image_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneHotspot {
    pub hotspot_id: String,
    pub label: String,
    pub sprite_hint: String,
    pub interaction_hint: String,
    pub x_permyriad: u16,
    pub y_permyriad: u16,
    pub width_permyriad: u16,
    pub height_permyriad: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneHotspotLayer {
    pub layer_id: String,
    pub coordinate_system: String,
    pub owner_editable: bool,
    #[serde(default)]
    pub hotspots: Vec<SceneHotspot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneMetadata {
    pub scope: SceneScope,
    pub render_style: SceneRenderStyle,
    pub title_banner: Option<String>,
    pub background_preset: String,
    pub ambiance: String,
    pub owner_editable: bool,
    pub avatar_editable: bool,
    pub primary_avatar: Option<PixelAvatarProfile>,
    #[serde(default)]
    pub assistant_slots: Vec<AgentSceneSlot>,
    #[serde(default)]
    pub image_layer: Option<SceneImageLayer>,
    #[serde(default)]
    pub hotspot_layer: Option<SceneHotspotLayer>,
    pub landmarks: Vec<SceneLandmark>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversation {
    pub conversation_id: ConversationId,
    pub kind: ConversationKind,
    #[serde(default = "default_conversation_scope")]
    pub scope: ConversationScope,
    #[serde(default)]
    pub scene: Option<SceneMetadata>,
    pub content_topic: String,
    pub participants: Vec<IdentityId>,
    pub created_at_ms: i64,
    pub last_active_at_ms: i64,
}

impl Conversation {
    pub fn touch(&mut self, timestamp_ms: i64) {
        if timestamp_ms > self.last_active_at_ms {
            self.last_active_at_ms = timestamp_ms;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivePolicy {
    pub active_window_hours: u32,
    pub local_retention_days: Option<u32>,
    pub allow_user_pinned_archive: bool,
    pub archive_when_idle_hours: u32,
}

impl Default for ArchivePolicy {
    fn default() -> Self {
        Self {
            active_window_hours: 24,
            local_retention_days: Some(30),
            allow_user_pinned_archive: true,
            archive_when_idle_hours: 24,
        }
    }
}

impl ArchivePolicy {
    pub fn active_window_ms(&self) -> i64 {
        i64::from(self.active_window_hours) * 60 * 60 * 1000
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub envelope: MessageEnvelope,
    pub delivery_state: DeliveryState,
    pub archived_at_ms: Option<i64>,
    pub pinned: bool,
    #[serde(default)]
    pub recalled_at_ms: Option<i64>,
    #[serde(default)]
    pub recalled_by: Option<IdentityId>,
    #[serde(default)]
    pub edited_at_ms: Option<i64>,
    #[serde(default)]
    pub edited_by: Option<IdentityId>,
}

impl TimelineEntry {
    pub fn is_active_at(&self, now_ms: i64, policy: &ArchivePolicy) -> bool {
        if self.archived_at_ms.is_some() {
            return false;
        }
        if self.pinned {
            return true;
        }
        now_ms - self.envelope.timestamp_ms <= policy.active_window_ms()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceMode {
    FullTerminal,
    CompactTerminal,
    WearableGlance,
    EmbeddedHeadless,
}

impl fmt::Display for SurfaceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            SurfaceMode::FullTerminal => "full-terminal",
            SurfaceMode::CompactTerminal => "compact-terminal",
            SurfaceMode::WearableGlance => "wearable-glance",
            SurfaceMode::EmbeddedHeadless => "embedded-headless",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldProfile {
    pub world_id: WorldId,
    pub title: String,
    pub portable_identity_required: bool,
    pub allows_cross_city_private_messages: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CityProfile {
    pub city_id: CityId,
    pub world_id: WorldId,
    pub slug: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub scene: Option<SceneMetadata>,
    pub resident_portable: bool,
    pub approval_required: bool,
    pub public_room_discovery_enabled: bool,
    #[serde(default = "default_federation_policy")]
    pub federation_policy: FederationPolicy,
    #[serde(default = "default_relay_budget_hint")]
    pub relay_budget_hint: RelayBudgetHint,
    #[serde(default = "default_city_retention_policy")]
    pub retention_policy: CityRetentionPolicy,
}

fn default_federation_policy() -> FederationPolicy {
    FederationPolicy::Open
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FederationPolicy {
    Open,
    Selective,
    Isolated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayBudgetHint {
    Minimal,
    Balanced,
    HighAvailability,
}

fn default_relay_budget_hint() -> RelayBudgetHint {
    RelayBudgetHint::Balanced
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CityRetentionPolicy {
    pub active_window_hours: u32,
    pub short_window_store_hours: u32,
    pub local_archive_days: Option<u32>,
}

fn default_city_retention_policy() -> CityRetentionPolicy {
    CityRetentionPolicy {
        active_window_hours: 24,
        short_window_store_hours: 72,
        local_archive_days: Some(30),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CityRole {
    Lord,
    Steward,
    Resident,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CityPermission {
    CreatePublicRoom,
    ClosePublicRoom,
    PublishAnnouncement,
    PinPublicMessage,
    ApproveResidentJoin,
    RemoveResident,
    MuteResident,
    AssignSteward,
    RevokeSteward,
    FreezeRoom,
    UpdateDiscoveryVisibility,
    EditCityProfile,
    UpdateFederationPolicy,
    UpdateRelayPolicy,
    UpdateRetentionPolicy,
    ToggleCityFeatures,
    PublishServiceStatus,
}

impl CityRole {
    pub fn permissions(self) -> &'static [CityPermission] {
        use CityPermission as Permission;

        match self {
            CityRole::Lord => &[
                Permission::CreatePublicRoom,
                Permission::ClosePublicRoom,
                Permission::PublishAnnouncement,
                Permission::PinPublicMessage,
                Permission::ApproveResidentJoin,
                Permission::RemoveResident,
                Permission::MuteResident,
                Permission::AssignSteward,
                Permission::RevokeSteward,
                Permission::FreezeRoom,
                Permission::UpdateDiscoveryVisibility,
                Permission::EditCityProfile,
                Permission::UpdateFederationPolicy,
                Permission::UpdateRelayPolicy,
                Permission::UpdateRetentionPolicy,
                Permission::ToggleCityFeatures,
                Permission::PublishServiceStatus,
            ],
            CityRole::Steward => &[
                Permission::PublishAnnouncement,
                Permission::PinPublicMessage,
                Permission::RemoveResident,
                Permission::MuteResident,
                Permission::FreezeRoom,
                Permission::UpdateDiscoveryVisibility,
                Permission::PublishServiceStatus,
            ],
            CityRole::Resident => &[],
        }
    }

    pub fn has(self, permission: CityPermission) -> bool {
        self.permissions().contains(&permission)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MembershipState {
    PendingApproval,
    Active,
    Muted,
    Suspended,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CityMembership {
    pub city_id: CityId,
    pub resident_id: IdentityId,
    pub role: CityRole,
    pub state: MembershipState,
    pub joined_at_ms: i64,
    pub added_by: Option<IdentityId>,
}

impl CityMembership {
    pub fn can_moderate_public_space(&self) -> bool {
        matches!(self.role, CityRole::Lord | CityRole::Steward)
            && matches!(self.state, MembershipState::Active | MembershipState::Muted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CityFeatureFlags {
    pub local_search: bool,
    pub ai_sidecar: bool,
    #[serde(default = "default_true")]
    pub personal_bots: bool,
    #[serde(default = "default_true")]
    pub city_bots: bool,
    #[serde(default = "default_true")]
    pub room_scene_bots: bool,
    #[serde(default)]
    pub commerce_bots: bool,
    pub room_indexing: bool,
    pub store_history: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidentPortability {
    pub may_leave_city: bool,
    pub may_join_other_cities: bool,
    pub may_keep_private_relationships: bool,
    /// Policy boundary for private-message plaintext. Skeleton envelopes do not relax this: cities
    /// still must not read resident private plaintext unless this flag is deliberately enabled.
    pub city_can_read_private_plaintext: bool,
}

impl ResidentPortability {
    pub fn protocol_safe_default() -> Self {
        Self {
            may_leave_city: true,
            may_join_other_cities: true,
            may_keep_private_relationships: true,
            city_can_read_private_plaintext: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidentExportRights {
    pub may_export_private_conversations: bool,
    pub may_export_group_conversations: bool,
    pub may_export_city_public_rooms: bool,
    pub export_requires_local_participation: bool,
}

impl ResidentExportRights {
    pub fn protocol_safe_default() -> Self {
        Self {
            may_export_private_conversations: true,
            may_export_group_conversations: true,
            may_export_city_public_rooms: true,
            export_requires_local_participation: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisitorUrgency {
    Info,
    NeedsAttention,
    Emergency,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolContract {
    pub tool_name: String,
    pub intent_summary: String,
    pub requires_confirmation: bool,
    pub estimated_duration_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaretakerPersona {
    pub caretaker_id: IdentityId,
    pub display_name: String,
    pub archetype: String,
    pub tone: String,
    pub memory_capacity: usize,
    pub max_notifications_per_day: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaretakerMemoryEntry {
    pub memory_id: String,
    pub caretaker_id: IdentityId,
    pub owner_id: IdentityId,
    pub note: String,
    pub created_at_ms: i64,
    pub reference_conversation: Option<ConversationId>,
    pub priority: NotificationPriority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisitorMessage {
    pub message_id: String,
    pub visitor_id: IdentityId,
    pub owner_id: IdentityId,
    pub summary: String,
    pub urgency: VisitorUrgency,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerNotification {
    pub notification_id: String,
    pub owner_id: IdentityId,
    pub caretaker_id: IdentityId,
    pub summary: String,
    pub priority: NotificationPriority,
    pub linked_contract: Option<ToolContract>,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModerationScope {
    Visitor,
    Room,
    City,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModerationDecision {
    Pending,
    Approved,
    Rejected,
    Escalated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModerationRequest {
    pub request_id: String,
    pub initiator: IdentityId,
    pub target: IdentityId,
    pub scope: ModerationScope,
    pub reason: String,
    pub requested_at_ms: i64,
    pub source_conversation: Option<ConversationId>,
    pub proposed_contract: Option<ToolContract>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModerationResult {
    pub request_id: String,
    pub decision: ModerationDecision,
    pub details: String,
    pub resolved_at_ms: i64,
    pub caretaker: Option<IdentityId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachment_reference_roundtrips_through_split() {
        let id = "0123456789abcdef0123456789abcdef";
        let encoded = attachment_reference(id, "看这只龙虾");
        assert_eq!(
            encoded,
            "attachment://0123456789abcdef0123456789abcdef\n看这只龙虾"
        );
        let (parsed_id, caption) = split_attachment_text(&encoded);
        assert_eq!(parsed_id.as_deref(), Some(id));
        assert_eq!(caption, "看这只龙虾");
    }

    #[test]
    fn attachment_reference_without_caption_has_no_newline() {
        let id = "0123456789abcdef0123456789abcdef";
        let encoded = attachment_reference(id, "");
        assert_eq!(encoded, "attachment://0123456789abcdef0123456789abcdef");
        let (parsed_id, caption) = split_attachment_text(&encoded);
        assert_eq!(parsed_id.as_deref(), Some(id));
        assert!(caption.is_empty());
    }

    #[test]
    fn attachment_display_text_falls_back_to_caption_label() {
        let id = "0123456789abcdef0123456789abcdef";
        assert_eq!(
            attachment_display_text(&attachment_reference(id, "看这只龙虾")),
            Some("[图片] 看这只龙虾".into())
        );
        assert_eq!(
            attachment_display_text(&attachment_reference(id, "")),
            Some("[图片]".into())
        );
    }

    #[test]
    fn attachment_display_text_rejects_plain_text_and_bad_ids() {
        assert_eq!(attachment_display_text("普通消息"), None);
        assert_eq!(attachment_display_text("attachment://nothex"), None);
        assert_eq!(
            attachment_display_text("attachment://0123456789ABCDEF0123456789ABCDEF"),
            None
        );
        assert!(!validate_attachment_id("0123"));
        assert!(validate_attachment_id("0123456789abcdef0123456789abcdef"));
    }

    #[test]
    fn wearable_profile_is_small_and_camera_capable() {
        let profile = ClientProfile::wearable_glasses();
        assert_eq!(profile.class, ClientClass::Wearable);
        assert!(profile.supports_camera);
        assert!(profile.supports_voice);
        assert!(profile.max_memory_kib < ClientProfile::desktop_terminal().max_memory_kib);
    }

    #[test]
    fn archive_policy_converts_hours_to_ms() {
        let policy = ArchivePolicy {
            active_window_hours: 48,
            ..ArchivePolicy::default()
        };
        assert_eq!(policy.active_window_ms(), 172_800_000);
    }

    #[test]
    fn city_lord_has_group_owner_plus_infra_permissions() {
        assert!(CityRole::Lord.has(CityPermission::CreatePublicRoom));
        assert!(CityRole::Lord.has(CityPermission::RemoveResident));
        assert!(CityRole::Lord.has(CityPermission::UpdateRelayPolicy));
        assert!(CityRole::Lord.has(CityPermission::UpdateRetentionPolicy));
    }

    #[test]
    fn steward_can_moderate_but_not_take_city_infra_control() {
        assert!(CityRole::Steward.has(CityPermission::MuteResident));
        assert!(CityRole::Steward.has(CityPermission::FreezeRoom));
        assert!(!CityRole::Steward.has(CityPermission::UpdateRelayPolicy));
        assert!(!CityRole::Steward.has(CityPermission::AssignSteward));
    }

    #[test]
    fn residents_keep_portability_and_private_plaintext_boundary() {
        let portability = ResidentPortability::protocol_safe_default();
        assert!(portability.may_leave_city);
        assert!(portability.may_join_other_cities);
        assert!(portability.may_keep_private_relationships);
        assert!(!portability.city_can_read_private_plaintext);
    }

    #[test]
    fn city_federation_policy_can_be_isolated_by_choice() {
        let profile = CityProfile {
            city_id: CityId("city:quiet-hill".into()),
            world_id: WorldId("world:lobster".into()),
            slug: "quiet-hill".into(),
            title: "Quiet Hill".into(),
            description: "A city that chooses not to federate.".into(),
            scene: None,
            resident_portable: true,
            approval_required: false,
            public_room_discovery_enabled: false,
            federation_policy: FederationPolicy::Isolated,
            relay_budget_hint: RelayBudgetHint::Minimal,
            retention_policy: CityRetentionPolicy {
                active_window_hours: 24,
                short_window_store_hours: 24,
                local_archive_days: Some(30),
            },
        };
        assert_eq!(profile.federation_policy, FederationPolicy::Isolated);
    }

    #[test]
    fn residents_keep_export_rights_for_private_and_group_history() {
        let export_rights = ResidentExportRights::protocol_safe_default();
        assert!(export_rights.may_export_private_conversations);
        assert!(export_rights.may_export_group_conversations);
        assert!(export_rights.may_export_city_public_rooms);
        assert!(export_rights.export_requires_local_participation);
    }

    #[test]
    fn scene_metadata_can_describe_city_and_personal_spaces() {
        let city_scene = SceneMetadata {
            scope: SceneScope::City,
            render_style: SceneRenderStyle::SfcPixel,
            title_banner: Some("凛冬城".into()),
            background_preset: "frozen-harbor".into(),
            ambiance: "夜色、壁炉与霓虹世界门".into(),
            owner_editable: true,
            avatar_editable: false,
            primary_avatar: None,
            assistant_slots: vec![AgentSceneSlot {
                slot_id: "city-concierge".into(),
                display_name: "城务执事".into(),
                scope: AgentScope::City,
                use_cases: vec![AgentUseCase::Caretaking, AgentUseCase::Moderation],
                appearance_hint: "pixel-npc-concierge".into(),
                can_leave_messages: true,
                can_edit_scene: false,
                can_trade_goods: false,
            }],
            image_layer: None,
            hotspot_layer: None,
            landmarks: vec![SceneLandmark {
                slot_id: "lord-hall".into(),
                label: "城主府".into(),
                sprite_hint: "lord-hall-01".into(),
                interaction_hint: "查看城市治理动态".into(),
            }],
        };
        let room_scene = SceneMetadata {
            scope: SceneScope::DirectRoom,
            render_style: SceneRenderStyle::SfcPixel,
            title_banner: Some("个人房间".into()),
            background_preset: "cozy-loft".into(),
            ambiance: "木地板、书桌与像素人物".into(),
            owner_editable: true,
            avatar_editable: true,
            primary_avatar: Some(PixelAvatarProfile {
                avatar_id: "avatar:reader".into(),
                display_name: "阅读者".into(),
                archetype: "pixel-resident".into(),
                palette_hint: "warm-amber".into(),
                accessory_hint: Some("围巾".into()),
            }),
            assistant_slots: vec![AgentSceneSlot {
                slot_id: "room-caretaker".into(),
                display_name: "房间管家".into(),
                scope: AgentScope::Room,
                use_cases: vec![AgentUseCase::Caretaking, AgentUseCase::Decoration],
                appearance_hint: "pixel-room-assistant".into(),
                can_leave_messages: true,
                can_edit_scene: true,
                can_trade_goods: false,
            }],
            image_layer: None,
            hotspot_layer: None,
            landmarks: vec![],
        };

        assert_eq!(city_scene.scope, SceneScope::City);
        assert_eq!(room_scene.scope, SceneScope::DirectRoom);
        assert!(room_scene.avatar_editable);
        assert!(city_scene.primary_avatar.is_none());
        assert_eq!(city_scene.assistant_slots.len(), 1);
        assert_eq!(room_scene.assistant_slots.len(), 1);
    }

    #[test]
    fn city_features_can_enable_bot_and_commerce_layers() {
        let features = CityFeatureFlags {
            local_search: true,
            ai_sidecar: true,
            personal_bots: true,
            city_bots: true,
            room_scene_bots: true,
            commerce_bots: true,
            room_indexing: true,
            store_history: true,
        };

        assert!(features.personal_bots);
        assert!(features.city_bots);
        assert!(features.room_scene_bots);
        assert!(features.commerce_bots);
    }

    #[test]
    fn canonical_direct_conversation_id_orders_identity_pair() {
        let rsaga = IdentityId("rsaga".into());
        let builder = IdentityId("builder".into());

        assert_eq!(
            canonical_direct_conversation_id(&rsaga, &builder),
            ConversationId("dm:builder:rsaga".into())
        );
        assert_eq!(
            canonical_direct_conversation_id(&builder, &rsaga),
            ConversationId("dm:builder:rsaga".into())
        );
    }

    #[test]
    fn identity_id_equality() {
        assert_eq!(IdentityId("a".into()), IdentityId("a".into()));
        assert_ne!(IdentityId("a".into()), IdentityId("b".into()));
    }
    #[test]
    fn conversation_id_format() {
        let c = ConversationId("dm:a:b".into());
        assert_eq!(c.0, "dm:a:b");
    }
    #[test]
    fn identity_id_clone_eq() {
        let a = IdentityId("test".into());
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(format!("{a:?}"), "IdentityId(\"test\")");
    }
    #[test]
    fn client_profiles_are_distinct() {
        assert_ne!(
            ClientProfile::desktop_terminal().class,
            ClientProfile::mobile_web().class
        );
        assert!(ClientProfile::lobster_embedded().supports_background_sync);
    }
    #[test]
    fn scene_image_layer_defaults() {
        let layer = SceneImageLayer {
            layer_id: "l1".into(),
            preset: "p1".into(),
            asset_hint: "a1".into(),
            aspect_ratio_permyriad: 5625,
            owner_editable: true,
            day_image_url: None,
            night_image_url: None,
        };
        assert_eq!(layer.aspect_ratio_permyriad, 5625);
        assert_eq!(layer.preset, "p1");
        assert!(layer.day_image_url.is_none());
    }
    #[test]
    fn conversation_scope_default_is_private() {
        assert_eq!(default_conversation_scope(), ConversationScope::Private);
    }
    #[test]
    fn scene_hotspot_defaults() {
        let h = SceneHotspot {
            hotspot_id: "h1".into(),
            label: "test".into(),
            sprite_hint: "def".into(),
            interaction_hint: "click".into(),
            x_permyriad: 100,
            y_permyriad: 200,
            width_permyriad: 300,
            height_permyriad: 400,
        };
        assert_eq!(h.label, "test");
        assert_eq!(h.x_permyriad, 100);
        assert_eq!(h.height_permyriad, 400);
    }
    #[test]
    fn scene_hotspot_layer_defaults() {
        let layer = SceneHotspotLayer {
            layer_id: "l1".into(),
            coordinate_system: "scene-permyriad".into(),
            owner_editable: false,
            hotspots: vec![],
        };
        assert!(!layer.owner_editable);
        assert!(layer.hotspots.is_empty());
    }
    #[test]
    fn all_client_classes_distinct() {
        let all = [
            ClientClass::Embedded,
            ClientClass::Desktop,
            ClientClass::MobileWeb,
            ClientClass::Wearable,
            ClientClass::Service,
        ];
        let mut unique = std::collections::BTreeSet::new();
        for c in all {
            unique.insert(format!("{c:?}"));
        }
        assert_eq!(unique.len(), 5, "all 5 client classes should be distinct");
    }
    #[test]
    fn conversation_id_default_invalid() {
        assert_eq!(ConversationId(String::new()).0, "");
    }
}
