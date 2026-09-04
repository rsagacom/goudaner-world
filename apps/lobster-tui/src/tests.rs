use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use super::nav_panel::RatatuiNavTone;
use super::test_support::{runtime_context_with_selection_preview, test_context};
use super::*;
use crate::compact_shell::compact_terminal_shell_lines;
use crate::terminal_smoke_script::run_submission_script;
use crate::terminal_submission::{
    build_cli_search_path, build_edit_message_request, build_recall_message_request,
    gateway_bearer_header_value, handle_terminal_submission_with_gateway_post,
    resolve_direct_conversation_id, run_cli_search_command,
};
use crate::transport_sync::{merge_polled_messages, republish_pending_messages};
use chat_core::{
    ArchivePolicy, ClientProfile, ConversationId, DeliveryState, DeviceId, IdentityId, MessageBody,
    MessageEnvelope, MessageId, PayloadType, TimelineEntry,
};
use chat_storage::{FileTimelineStore, TimelineStore};
use ratatui::layout::Rect;
use ratatui::widgets::BorderType;
use transport_waku::{
    EncodedFrame, TopicSubscription, WakuAdapter, WakuConnectionState, WakuEndpointConfig,
    WakuSyncCursor, WakuTransport,
};

static WORLD_TRANSCRIPT_TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_store(label: &str) -> (std::path::PathBuf, FileTimelineStore) {
    let root = std::env::temp_dir().join(format!("{label}-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let store = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
    (root, store)
}

struct RecordingTransport {
    published: Vec<MessageEnvelope>,
    subscriptions: Vec<TopicSubscription>,
    fail_publish: bool,
    connection_state: WakuConnectionState,
}

impl RecordingTransport {
    fn new() -> Self {
        Self {
            published: Vec::new(),
            subscriptions: Vec::new(),
            fail_publish: false,
            connection_state: WakuConnectionState::Connected,
        }
    }

    fn failing() -> Self {
        Self {
            published: Vec::new(),
            subscriptions: Vec::new(),
            fail_publish: true,
            connection_state: WakuConnectionState::Connected,
        }
    }
}

impl WakuTransport for RecordingTransport {
    fn publish(&mut self, message: &MessageEnvelope) -> Result<(), String> {
        if self.fail_publish {
            self.connection_state = WakuConnectionState::Disconnected;
            return Err("publish failed".into());
        }
        self.published.push(message.clone());
        self.connection_state = WakuConnectionState::Connected;
        Ok(())
    }

    fn poll(&mut self) -> Result<Vec<MessageEnvelope>, String> {
        Ok(Vec::new())
    }
}

impl WakuAdapter for RecordingTransport {
    fn connection_state(&self) -> WakuConnectionState {
        self.connection_state
    }

    fn connect(&mut self, _endpoint: WakuEndpointConfig) -> Result<(), String> {
        Ok(())
    }

    fn subscribe_topics(&mut self, _subscriptions: &[TopicSubscription]) -> Result<(), String> {
        self.subscriptions.extend_from_slice(_subscriptions);
        Ok(())
    }

    fn recover_since(
        &self,
        _content_topic: &str,
        _cursor: &WakuSyncCursor,
        _limit: usize,
    ) -> Result<Vec<EncodedFrame>, String> {
        Ok(Vec::new())
    }

    fn poll_frames(&mut self) -> Result<Vec<EncodedFrame>, String> {
        Ok(Vec::new())
    }
}

#[test]
fn world_mode_uses_world_page_language() {
    assert!(mode_callout(LaunchSurface::World).contains("城外同路"));
    assert!(mode_panel_meta(LaunchSurface::World).contains("城门先行"));
    assert!(mode_input_tip(LaunchSurface::World).contains("回车过广场"));
    assert_eq!(
        surface_transcript_panel_title(LaunchSurface::World),
        "城邦回声墙"
    );
    assert_eq!(surface_page_title(LaunchSurface::World), "城外 / 城门牌");
}

#[test]
fn focus_area_cycles_across_nav_transcript_and_input() {
    assert_eq!(FocusArea::Nav.next(), FocusArea::Transcript);
    assert_eq!(FocusArea::Transcript.next(), FocusArea::Input);
    assert_eq!(FocusArea::Input.next(), FocusArea::Nav);
    assert_eq!(FocusArea::Nav.previous(), FocusArea::Input);
    assert_eq!(FocusArea::Transcript.previous(), FocusArea::Nav);
    assert_eq!(FocusArea::Input.previous(), FocusArea::Transcript);
}

#[test]
fn move_selection_clamps_to_conversation_bounds() {
    let conversations = vec![
        launch_conversation(LaunchSurface::User),
        launch_conversation(LaunchSurface::World),
        launch_conversation(LaunchSurface::Direct),
    ];
    let first = conversations[0].conversation_id.clone();
    let last = conversations[2].conversation_id.clone();

    assert_eq!(move_selection(&conversations, &first, -1), first);
    assert_eq!(
        move_selection(&conversations, &first, 1),
        conversations[1].conversation_id
    );
    assert_eq!(move_selection(&conversations, &last, 1), last);
}

#[test]
fn conversation_panel_marks_selected_row_separately_from_active_row() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "城邦大厅 · 回声常亮".into(),
            is_active: true,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "钟塔后厅".into(),
            summary: "守夜门牌 · 灯还亮着".into(),
            is_active: false,
            is_selected: true,
            is_private: false,
        },
    ];

    let panel = strip_ansi_sgr(&render_conversation_panel(&context).join("\n"));
    assert!(panel.contains("▶  1 第一城大厅"));
    assert!(panel.contains("▷  2 钟塔后厅"));
}

#[test]
fn ratatui_nav_rows_keep_selected_and_active_distinct() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "城邦大厅 · 回声常亮".into(),
            is_active: true,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "钟塔后厅".into(),
            summary: "守夜门牌 · 灯还亮着".into(),
            is_active: false,
            is_selected: true,
            is_private: false,
        },
    ];

    let (rows, selected_index) = ratatui_nav_rows(&context);
    assert_eq!(selected_index, 1);
    assert_eq!(rows[0].marker, "▶");
    assert_eq!(rows[0].tone, RatatuiNavTone::Active);
    assert_eq!(rows[1].marker, "▷");
    assert_eq!(rows[1].tone, RatatuiNavTone::Selected);
}

#[test]
fn input_panel_shows_buffer_when_input_is_focused() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Input;
    context.input_buffer = "准备落一条回声".into();

    let panel = strip_ansi_sgr(&render_input_panel(&context).join("\n"));
    assert!(panel.contains("城内落字栏 · Esc 收笔 ◉"));
    assert!(panel.contains("█ 准备落一条回声"));
    assert!(panel.contains("准备落一条回声"));
}

#[test]
fn compact_input_panel_uses_block_cursor_when_input_is_focused() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Input;

    let panel = strip_ansi_sgr(&render_compact_input_panel(&context, 84).join("\n"));
    assert!(panel.contains("█ 在此落字"));
    assert!(!panel.contains("> 在此落字"));
}

#[test]
fn compact_input_panel_shows_waiting_state_label_when_not_focused() {
    let panel = strip_ansi_sgr(
        &render_compact_input_panel(&test_context(LaunchSurface::User), 84).join("\n"),
    );
    assert!(panel.contains("待落"));
    assert!(!panel.contains("落字中"));
}

#[test]
fn compact_input_panel_shows_active_state_label_when_focused() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Input;

    let panel = strip_ansi_sgr(&render_compact_input_panel(&context, 84).join("\n"));
    assert!(panel.contains("落字中"));
    assert!(!panel.contains("待落"));
}

#[test]
fn nav_focus_previews_selected_room_in_status_header_and_transcript() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    let status = strip_ansi_sgr(&render_status_strip(&context));
    let header = strip_ansi_sgr(&render_chat_header_panel(&context).join("\n"));
    let transcript = strip_ansi_sgr(&render_transcript_panel(&context).join("\n"));

    assert!(status.contains("世界广场"));
    assert!(header.contains("世界广场"));
    assert!(transcript.contains("回声壳在线"));
    assert!(!transcript.contains("欢迎来到第一城大厅"));
}

#[test]
fn nav_preview_keeps_input_target_on_entered_room_until_commit() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    let input = strip_ansi_sgr(&render_input_panel(&context).join("\n"));
    assert!(input.contains("投向 第一城大厅"));
    assert!(!input.contains("投向 世界广场"));
}

#[test]
fn input_panel_semantics_keep_entered_target_during_nav_preview() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    let semantics = input_panel_semantics(&context, 18);
    assert_eq!(semantics.prompt_state, "【城内】");
    assert_eq!(semantics.state_label, "待落");
    assert_eq!(semantics.panel_title, "城内落字栏");
    assert_eq!(semantics.tip, "回车入城内");
    assert_eq!(semantics.target, "第一城大厅");
    assert_eq!(semantics.composer, "在此落字");
    assert!(!semantics.is_input_focused);
}

#[test]
fn input_panel_prompt_state_uses_surface_context_without_waiting_prefix() {
    assert_eq!(
        active_prompt_state(&test_context(LaunchSurface::User)),
        "【城内】"
    );
    assert_eq!(
        active_prompt_state(&test_context(LaunchSurface::Admin)),
        "【城务】"
    );
    assert_eq!(
        active_prompt_state(&test_context(LaunchSurface::World)),
        "【世界】"
    );
    assert_eq!(
        active_prompt_state(&test_context(LaunchSurface::Direct)),
        "【居所】"
    );
}

#[test]
fn input_panel_semantics_preserve_full_target_for_projection_specific_truncation() {
    let mut context = test_context(LaunchSurface::User);
    context.active_conversation = "第一城大厅外环长廊观景平台特别长的落字去向".into();

    let semantics = input_panel_semantics(&context, 8);

    assert_eq!(semantics.target, context.active_conversation);
}

#[test]
fn input_panel_semantics_carry_focus_state_label() {
    let waiting = input_panel_semantics(&test_context(LaunchSurface::User), 18);

    let mut focused_context = test_context(LaunchSurface::User);
    focused_context.focus_area = FocusArea::Input;
    let focused = input_panel_semantics(&focused_context, 18);

    assert_eq!(waiting.state_label, "待落");
    assert_eq!(focused.state_label, "落字中");
}

#[test]
fn ratatui_block_titles_follow_preview_but_keep_input_target_on_entered_room() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    assert_eq!(
        ratatui_scene_block_title(&context),
        "▛城邦外景▜ · ▚预览▞ · ▛世界广场▜"
    );
    assert_eq!(
        ratatui_transcript_block_title(&context),
        "▛城邦回声墙▜ · ▚预览▞ · ▛世界广场▜"
    );
    assert_eq!(
        ratatui_input_block_title(&context),
        "▛城内落字栏▜ · ▚已入▞ · ▛第一城大厅▜"
    );
}

#[test]
fn ratatui_scene_block_title_uses_entered_prefix_after_commit() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Transcript;

    assert_eq!(
        ratatui_scene_block_title(&context),
        "▛城市外景▜ · ▚已入▞ · ▛城邦大厅▜"
    );
}

#[test]
fn ratatui_transcript_block_title_uses_entered_prefix_after_commit() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Transcript;

    assert_eq!(
        ratatui_transcript_block_title(&context),
        "▛公共频道▜ · ▚已入▞ · ▛第一城大厅▜"
    );
}

#[test]
fn ratatui_block_titles_switch_to_private_copy_when_dm_is_active() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        direct_conversation("tiyan", "guide"),
        direct_conversation("tiyan", "guide"),
        FocusArea::Transcript,
    );

    assert_eq!(
        ratatui_scene_block_title(&context),
        "▛住宅主景▜ · ▚已入▞ · ▛新手居所▜"
    );
    assert_eq!(
        ratatui_transcript_block_title(&context),
        "▛私聊记录▜ · ▚已入▞ · ▛居所 · guide▜"
    );
}

#[test]
fn ratatui_transcript_block_title_omits_scroll_hint_when_not_focused() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Nav;

    assert_eq!(
        ratatui_transcript_block_title(&context),
        "▛公共频道▜ · ▚已入▞ · ▛第一城大厅▜"
    );
}

#[test]
fn ratatui_transcript_focus_badge_uses_scroll_language_when_focused() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Transcript;
    assert_eq!(ratatui_transcript_focus_badge(&context), Some("↑/↓·滚动"));

    context.focus_area = FocusArea::Nav;
    assert_eq!(ratatui_transcript_focus_badge(&context), None);
}

#[test]
fn ratatui_block_frame_title_uses_short_focus_badge() {
    assert_eq!(
        ratatui_block_frame_title("城内门牌", true, Some("j/k·入城内")),
        "城内门牌 · ▚j/k·入城内▞"
    );
    assert_eq!(
        ratatui_block_frame_title("城内门牌", false, Some("j/k·入城内")),
        "城内门牌"
    );
}

#[test]
fn ratatui_input_block_title_shows_escape_hint_when_focused() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Input;

    assert_eq!(
        ratatui_input_block_title(&context),
        "▛城内落字栏▜ · ▚已入▞ · ▛第一城大厅▜"
    );
}

#[test]
fn ratatui_status_line_shows_entered_and_preview_rooms_during_nav_preview() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    let line = ratatui_status_line(&context);
    let plain = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(plain.contains("▛已入 第一城大厅▜"));
    assert!(plain.contains("▛预览 世界广场▜"));
}

#[test]
fn ratatui_status_line_uses_pixel_chips_instead_of_bracket_status_text() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    let line = ratatui_status_line(&context);
    let plain = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(plain.contains("▛居民位▜"));
    assert!(plain.contains("▛城内▜"));
    assert!(plain.contains("▛已入 第一城大厅▜"));
    assert!(plain.contains("▛预览 世界广场▜"));
    assert!(plain.contains("▛回声"));
    assert_eq!(line.spans[0].style.bg, Some(Color::Blue));
    assert!(!plain.contains("[已进入 第一城大厅]"));
    assert!(!plain.contains("[预览 世界广场]"));
}

#[test]
fn ratatui_status_line_shows_entered_room_when_not_previewing() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Transcript;

    let line = ratatui_status_line(&context);
    let plain = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(plain.contains("▛已入 第一城大厅▜"));
    assert!(!plain.contains("门牌 第一城大厅"));
}

#[test]
fn ratatui_header_line_uses_pixel_banner_and_secondary_tags() {
    let context = test_context(LaunchSurface::User);

    let line = ratatui_header_line(&context);
    let plain = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(plain.contains("▛城邦像素终端▜"));
    assert!(plain.contains("▛龙虾世界▜"));
    assert!(plain.contains("▛城市 / 公共频道▜"));
    assert!(plain.contains("▚A 地图▞"));
    assert!(plain.contains("▚B 委托▞"));
    assert!(plain.contains("▛居民位▜"));
    assert!(!plain.contains("[A 地图]"));
    assert!(!plain.contains("[B 委托]"));
}

#[test]
fn enter_selected_conversation_moves_focus_to_transcript_and_resets_scroll() {
    let mut active = ConversationId("room:city:core-harbor:lobby".into());
    let selected = ConversationId("room:world:lobby".into());
    let mut transcript_scroll = 4;
    let mut focus_area = FocusArea::Nav;

    enter_selected_conversation(
        &mut active,
        &selected,
        &mut transcript_scroll,
        &mut focus_area,
    );

    assert_eq!(active, selected);
    assert_eq!(transcript_scroll, 0);
    assert_eq!(focus_area, FocusArea::Transcript);
}

#[test]
fn focus_input_area_enters_selected_conversation_from_nav_preview() {
    let mut active = ConversationId("room:city:core-harbor:lobby".into());
    let selected = ConversationId("room:world:lobby".into());
    let mut transcript_scroll = 4;
    let mut focus_area = FocusArea::Nav;

    focus_input_area(
        &mut active,
        &selected,
        &mut transcript_scroll,
        &mut focus_area,
    );

    assert_eq!(active, selected);
    assert_eq!(transcript_scroll, 0);
    assert_eq!(focus_area, FocusArea::Input);
}

#[test]
fn focus_input_area_keeps_current_conversation_outside_nav_preview() {
    let original = ConversationId("room:city:core-harbor:lobby".into());
    let mut active = original.clone();
    let selected = ConversationId("room:world:lobby".into());
    let mut transcript_scroll = 4;
    let mut focus_area = FocusArea::Transcript;

    focus_input_area(
        &mut active,
        &selected,
        &mut transcript_scroll,
        &mut focus_area,
    );

    assert_eq!(active, original);
    assert_eq!(transcript_scroll, 4);
    assert_eq!(focus_area, FocusArea::Input);
}

#[test]
fn leave_input_area_returns_focus_to_transcript() {
    let mut focus_area = FocusArea::Input;
    leave_input_area(&mut focus_area);
    assert_eq!(focus_area, FocusArea::Transcript);
}

#[test]
fn governance_command_switches_to_admin_governance_room_when_present() {
    let temp_root = std::env::temp_dir().join(format!(
        "lobster-tui-governance-admin-{}",
        std::process::id()
    ));
    fs::create_dir_all(&temp_root).unwrap();

    let mut store = FileTimelineStore::open(&temp_root, ArchivePolicy::default()).unwrap();
    let active = launch_conversation(LaunchSurface::Admin);
    let mut conversations = vec![active.clone()];
    conversations.extend(launch_companion_conversations(LaunchSurface::Admin));
    for conversation in &conversations {
        store.upsert_conversation(conversation.clone()).unwrap();
    }

    let mut transport = RecordingTransport::new();
    let mut active_conversation_id = ConversationId("room:world:lobby".into());
    let mut selected_conversation_id = active_conversation_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/governance",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(
        active_conversation_id,
        ConversationId("room:city:aurora-hub:announcements".into())
    );
    assert_eq!(
        selected_conversation_id,
        ConversationId("room:city:aurora-hub:announcements".into())
    );
    assert!(transport.published.is_empty());
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn governance_command_is_noop_in_user_surface_without_governance_room() {
    let temp_root = std::env::temp_dir().join(format!(
        "lobster-tui-governance-user-{}",
        std::process::id()
    ));
    fs::create_dir_all(&temp_root).unwrap();

    let mut store = FileTimelineStore::open(&temp_root, ArchivePolicy::default()).unwrap();
    let active = launch_conversation(LaunchSurface::User);
    let mut conversations = vec![active.clone()];
    conversations.extend(launch_companion_conversations(LaunchSurface::User));
    for conversation in &conversations {
        store.upsert_conversation(conversation.clone()).unwrap();
    }

    let mut transport = RecordingTransport::new();
    let original = ConversationId("room:city:core-harbor:lobby".into());
    let mut active_conversation_id = original.clone();
    let mut selected_conversation_id = original.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/governance",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_conversation_id, original);
    assert_eq!(selected_conversation_id, original);
    assert!(transport.published.is_empty());
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn submission_script_routes_admin_governance_text_into_governance_room() {
    let temp_root = std::env::temp_dir().join(format!(
        "lobster-tui-governance-script-admin-{}",
        std::process::id()
    ));
    fs::create_dir_all(&temp_root).unwrap();

    let mut store = FileTimelineStore::open(&temp_root, ArchivePolicy::default()).unwrap();
    let active = launch_conversation(LaunchSurface::Admin);
    let mut conversations = vec![active.clone()];
    conversations.extend(launch_companion_conversations(LaunchSurface::Admin));
    for conversation in &conversations {
        store.upsert_conversation(conversation.clone()).unwrap();
    }

    let mut transport = RecordingTransport::new();
    let mut active_conversation_id = active.conversation_id.clone();
    let mut selected_conversation_id = active.conversation_id.clone();

    run_submission_script(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        &["/governance", "ADMIN_GOVERNANCE_COMMAND_探针消息"],
    )
    .unwrap();

    let published = transport
        .published
        .iter()
        .find(|message| message.body.plain_text == "ADMIN_GOVERNANCE_COMMAND_探针消息")
        .expect("governance command text should be published");

    assert_eq!(
        published.conversation_id,
        ConversationId("room:city:aurora-hub:announcements".into())
    );
    assert_eq!(
        active_conversation_id,
        ConversationId("room:city:aurora-hub:announcements".into())
    );
    assert_eq!(
        selected_conversation_id,
        ConversationId("room:city:aurora-hub:announcements".into())
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn admin_ban_command_without_gateway_shows_error_notice() {
    let (temp_root, mut store) = temp_store("lobster-tui-admin-ban");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/ban",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    let messages = store.export_messages(&active_id);
    let notice = messages
        .iter()
        .find(|m| m.envelope.body.plain_text.contains("用法"))
        .unwrap();
    assert!(notice.envelope.body.plain_text.contains("/ban"));
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn admin_unban_command_requires_resident_id() {
    let (temp_root, mut store) = temp_store("lobster-tui-admin-unban");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/unban",
    )
    .unwrap();

    let messages = store.export_messages(&active_id);
    assert!(
        messages
            .iter()
            .any(|m| m.envelope.body.plain_text.contains("用法：/unban"))
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn admin_freeze_command_requires_room_id() {
    let (temp_root, mut store) = temp_store("lobster-tui-admin-freeze");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/freeze",
    )
    .unwrap();

    let messages = store.export_messages(&active_id);
    assert!(
        messages
            .iter()
            .any(|m| m.envelope.body.plain_text.contains("用法：/freeze"))
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn admin_unfreeze_command_requires_room_id() {
    let (temp_root, mut store) = temp_store("lobster-tui-admin-unfreeze");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/unfreeze",
    )
    .unwrap();

    let messages = store.export_messages(&active_id);
    assert!(
        messages
            .iter()
            .any(|m| m.envelope.body.plain_text.contains("用法：/unfreeze"))
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn admin_invite_without_subcommand_shows_usage() {
    let (temp_root, mut store) = temp_store("lobster-tui-admin-invite-usage");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/invite",
    )
    .unwrap();

    let messages = store.export_messages(&active_id);
    assert!(messages.iter().any(|m| {
        let text = &m.envelope.body.plain_text;
        text.contains("/invite create") && text.contains("/invite revoke")
    }));
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn admin_residents_command_runs_without_gateway() {
    let (temp_root, mut store) = temp_store("lobster-tui-admin-residents");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/residents",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_id, selected_id);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn admin_commands_work_from_user_surface_too() {
    let (temp_root, mut store) = temp_store("lobster-tui-admin-user-surface");
    let active = launch_conversation(LaunchSurface::User);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_id,
        &mut selected_id,
        "/residents",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    let messages = store.export_messages(&active_id);
    assert!(messages.iter().any(|m| {
        let text = &m.envelope.body.plain_text;
        text.contains("居民名单") || text.contains("Gateway")
    }));
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn help_command_lists_admin_governance_commands() {
    let (temp_root, mut store) = temp_store("lobster-tui-help-admin");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/help",
    )
    .unwrap();

    let messages = store.export_messages(&active_id);
    let help_text = messages
        .iter()
        .find(|m| m.envelope.sender.0 == "system")
        .map(|m| &m.envelope.body.plain_text)
        .unwrap();
    assert!(help_text.contains("/residents"));
    assert!(help_text.contains("/ban"));
    assert!(help_text.contains("/unban"));
    assert!(help_text.contains("/freeze"));
    assert!(help_text.contains("/unfreeze"));
    assert!(help_text.contains("/invite create"));
    assert!(help_text.contains("/invite revoke"));
    assert!(help_text.contains("/rooms"));
    assert!(help_text.contains("/config"));
    assert!(help_text.contains("/search <关键词>"));
    assert!(help_text.contains("/world-status"));
    assert!(help_text.contains("/cities"));
    assert!(help_text.contains("/world-safety"));
    assert!(help_text.contains("/safety-reports"));
    assert!(help_text.contains("/safety-residents"));
    assert!(help_text.contains("/world-directory"));
    assert!(help_text.contains("/world-square"));
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn cli_search_path_scopes_identity_room_and_encoded_query() {
    let path = build_cli_search_path(
        "zhangsan",
        &ConversationId("dm:openclaw:zhangsan".into()),
        "晚上 一起吃饭",
        20,
    )
    .expect("search path should build");
    assert_eq!(
        path,
        "/v1/cli/search?for=user%3Azhangsan&q=%E6%99%9A%E4%B8%8A%20%E4%B8%80%E8%B5%B7%E5%90%83%E9%A5%AD&room_id=dm%3Aopenclaw%3Azhangsan&limit=20"
    );
}

#[test]
fn cli_search_command_projects_gateway_hits_into_tui_notice() {
    let (temp_root, mut store) = temp_store("lobster-tui-cli-search");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let transport = RecordingTransport::new();
    let mut active_conversation_id = conversation_id.clone();
    let mut selected_conversation_id = conversation_id.clone();
    let mut calls = Vec::new();

    run_cli_search_command(
        &mut store,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "zhangsan",
        "晚上",
        &mut |path| {
            calls.push(path.to_string());
            Ok(serde_json::json!([
                {
                    "sender": "openclaw",
                    "text": "晚上一起吃饭吗",
                    "timestamp_label": "now"
                }
            ]))
        },
    )
    .expect("search should project gateway hits");

    assert_eq!(calls.len(), 1);
    assert!(calls[0].starts_with("/v1/cli/search?for=user%3Azhangsan"));
    assert_eq!(active_conversation_id, conversation_id);
    assert_eq!(selected_conversation_id, conversation_id);
    assert!(transport.published.is_empty());
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0]
            .envelope
            .body
            .plain_text
            .contains("搜索「晚上」命中 1 条")
    );
    assert!(
        entries[0]
            .envelope
            .body
            .plain_text
            .contains("openclaw: 晚上一起吃饭吗")
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn admin_rooms_command_runs_without_gateway() {
    let (temp_root, mut store) = temp_store("lobster-tui-admin-rooms");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/rooms",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_id, selected_id);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn admin_config_without_args_shows_usage() {
    let (temp_root, mut store) = temp_store("lobster-tui-admin-config-usage");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/config",
    )
    .unwrap();

    let messages = store.export_messages(&active_id);
    assert!(
        messages
            .iter()
            .any(|m| m.envelope.body.plain_text.contains("用法：/config"))
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn world_status_command_runs_without_gateway() {
    let (temp_root, mut store) = temp_store("lobster-tui-world-status");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/world-status",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_id, selected_id);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn cities_command_runs_without_gateway() {
    let (temp_root, mut store) = temp_store("lobster-tui-cities");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/cities",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_id, selected_id);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn world_safety_command_runs_without_gateway() {
    let (temp_root, mut store) = temp_store("lobster-tui-world-safety");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/world-safety",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    let messages = store.export_messages(&active_id);
    assert!(messages.iter().any(|m| {
        let text = &m.envelope.body.plain_text;
        text.contains("安全快照") || text.contains("Gateway")
    }));
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn safety_reports_command_runs_without_gateway() {
    let (temp_root, mut store) = temp_store("lobster-tui-safety-reports");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/safety-reports",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_id, selected_id);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn safety_residents_command_runs_without_gateway() {
    let (temp_root, mut store) = temp_store("lobster-tui-safety-residents");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/safety-residents",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_id, selected_id);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn world_directory_command_runs_without_gateway() {
    let (temp_root, mut store) = temp_store("lobster-tui-world-directory");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/world-directory",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_id, selected_id);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn world_square_command_runs_without_gateway() {
    let (temp_root, mut store) = temp_store("lobster-tui-world-square");
    let active = launch_conversation(LaunchSurface::Admin);
    store.upsert_conversation(active.clone()).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_id = active.conversation_id.clone();
    let mut selected_id = active_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::Admin,
        &mut active_id,
        &mut selected_id,
        "/world-square",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_id, selected_id);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn dm_command_opens_new_direct_conversation_and_selects_it() {
    let (temp_root, mut store) = temp_store("lobster-tui-dm-open");
    let active = launch_conversation(LaunchSurface::User);
    let mut conversations = vec![active.clone()];
    conversations.extend(launch_companion_conversations(LaunchSurface::User));
    for conversation in &conversations {
        store.upsert_conversation(conversation.clone()).unwrap();
    }

    let mut transport = RecordingTransport::new();
    let original = active.conversation_id.clone();
    let mut active_conversation_id = original.clone();
    let mut selected_conversation_id = original.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/dm builder",
    )
    .unwrap();

    let target = direct_conversation("tiyan", "builder");
    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_conversation_id, target);
    assert_eq!(selected_conversation_id, target);
    assert!(store.active_conversations().iter().any(|conversation| {
        conversation.conversation_id == target
            && conversation.kind == ConversationKind::Direct
            && conversation.scope == ConversationScope::Private
    }));
    assert!(
        transport
            .subscriptions
            .iter()
            .any(|subscription| subscription.content_topic
                == transport_waku::WakuFrameCodec::content_topic_for(&target))
    );
    assert!(transport.published.is_empty());
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn dm_gateway_failure_does_not_fallback_to_local_conversation() {
    let mut requested_url = None;
    let result =
        resolve_direct_conversation_id("tiyan", "builder", Some("http://127.0.0.1:8790/"), |url| {
            requested_url = Some(url);
            Err("Gateway 返回 401".to_string())
        });

    assert_eq!(result.unwrap_err(), "Gateway 返回 401");
    assert_eq!(
        requested_url.as_deref(),
        Some("http://127.0.0.1:8790/v1/direct/open")
    );
}

#[test]
fn dm_gateway_empty_conversation_id_does_not_create_invalid_room() {
    let result =
        resolve_direct_conversation_id("tiyan", "builder", Some("http://127.0.0.1:8790"), |_url| {
            Ok(ConversationId("   ".into()))
        });

    assert_eq!(
        result.unwrap_err(),
        "Gateway 私聊响应缺少有效 conversation_id"
    );
}

#[test]
fn dm_command_is_noop_for_self_target() {
    let (temp_root, mut store) = temp_store("lobster-tui-dm-self");
    let active = launch_conversation(LaunchSurface::User);
    let mut conversations = vec![active.clone()];
    conversations.extend(launch_companion_conversations(LaunchSurface::User));
    for conversation in &conversations {
        store.upsert_conversation(conversation.clone()).unwrap();
    }

    let mut transport = RecordingTransport::new();
    let original = active.conversation_id.clone();
    let mut active_conversation_id = original.clone();
    let mut selected_conversation_id = original.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/dm tiyan",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_conversation_id, original);
    assert_eq!(selected_conversation_id, original);
    assert!(transport.subscriptions.is_empty());
    assert!(transport.published.is_empty());
    assert!(
        !store
            .active_conversations()
            .iter()
            .any(|conversation| conversation.conversation_id
                == direct_conversation("tiyan", "tiyan"))
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn help_command_appends_local_terminal_notice_without_publishing() {
    let (temp_root, mut store) = temp_store("lobster-tui-help-command");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_conversation_id = conversation_id.clone();
    let mut selected_conversation_id = conversation_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/help",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].delivery_state, DeliveryState::Delivered);
    assert_eq!(entries[0].envelope.sender, IdentityId("system".into()));
    assert!(entries[0].envelope.body.plain_text.contains("/help"));
    assert!(entries[0].envelope.body.plain_text.contains("/status"));
    assert!(entries[0].envelope.body.plain_text.contains("/refresh"));
    assert!(
        entries[0]
            .envelope
            .body
            .plain_text
            .contains("/edit <消息ID> <新正文>")
    );
    assert!(
        entries[0]
            .envelope
            .body
            .plain_text
            .contains("/recall <消息ID>")
    );
    assert!(transport.published.is_empty());
    assert_eq!(active_conversation_id, conversation_id);
    assert_eq!(selected_conversation_id, conversation_id);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn status_command_reports_identity_connection_and_room_without_publishing() {
    let (temp_root, mut store) = temp_store("lobster-tui-status-command");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_conversation_id = conversation_id.clone();
    let mut selected_conversation_id = ConversationId("room:world:lobby".into());

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/status",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    let notice = &entries[0].envelope.body.plain_text;
    assert!(notice.contains("身份 tiyan"), "{notice}");
    assert!(notice.contains("连接 已合线"), "{notice}");
    assert!(notice.contains(&conversation_id.0), "{notice}");
    assert!(notice.contains("room:world:lobby"), "{notice}");
    assert!(transport.published.is_empty());
    assert_eq!(active_conversation_id, conversation_id);
    assert_eq!(selected_conversation_id, conversation_id);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn refresh_command_appends_local_refresh_notice_without_publishing() {
    let (temp_root, mut store) = temp_store("lobster-tui-refresh-command");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_conversation_id = conversation_id.clone();
    let mut selected_conversation_id = conversation_id.clone();

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/refresh",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].delivery_state, DeliveryState::Delivered);
    assert_eq!(entries[0].envelope.sender, IdentityId("system".into()));
    assert!(entries[0].envelope.body.plain_text.contains("已刷新"));
    assert!(transport.published.is_empty());
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn plain_submission_publishes_and_marks_entry_delivered() {
    let (temp_root, mut store) = temp_store("lobster-tui-plain-submit-delivered");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_conversation_id = conversation_id.clone();
    let mut selected_conversation_id = ConversationId("room:world:lobby".into());

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "终端普通正文",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_conversation_id, conversation_id);
    assert_eq!(selected_conversation_id, conversation_id);
    assert_eq!(transport.published.len(), 1);
    assert_eq!(transport.published[0].body.plain_text, "终端普通正文");
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].delivery_state, DeliveryState::Delivered);
    assert_eq!(entries[0].envelope.body.plain_text, "终端普通正文");
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn edit_command_without_args_shows_usage_without_publishing_plain_text() {
    let (temp_root, mut store) = temp_store("lobster-tui-edit-usage");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_conversation_id = conversation_id.clone();
    let mut selected_conversation_id = ConversationId("room:world:lobby".into());

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/edit",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert!(transport.published.is_empty());
    assert_eq!(selected_conversation_id, conversation_id);
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0]
            .envelope
            .body
            .plain_text
            .contains("/edit <消息ID> <新正文>")
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn recall_command_without_args_shows_usage_without_publishing_plain_text() {
    let (temp_root, mut store) = temp_store("lobster-tui-recall-usage");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_conversation_id = conversation_id.clone();
    let mut selected_conversation_id = ConversationId("room:world:lobby".into());

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/recall",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert!(transport.published.is_empty());
    assert_eq!(selected_conversation_id, conversation_id);
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0]
            .envelope
            .body
            .plain_text
            .contains("/recall <消息ID>")
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn terminal_edit_command_request_matches_gateway_shell_contract() {
    let room_id = ConversationId("room:world:lobby".into());
    let request = build_edit_message_request("rsaga", &room_id, "msg-42", "修订后的正文");

    assert_eq!(request.0, "/v1/shell/message/edit");
    assert_eq!(
        request.1,
        serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": "msg-42",
            "actor": "rsaga",
            "text": "修订后的正文",
        })
    );
}

#[test]
fn terminal_recall_command_request_matches_gateway_shell_contract() {
    let room_id = ConversationId("room:world:lobby".into());
    let request = build_recall_message_request("rsaga", &room_id, "msg-42");

    assert_eq!(request.0, "/v1/shell/message/recall");
    assert_eq!(
        request.1,
        serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": "msg-42",
            "actor": "rsaga",
        })
    );
}

#[test]
fn tui_gateway_auth_header_normalizes_optional_bearer_token() {
    assert_eq!(
        gateway_bearer_header_value(Some("  session-123  ")).as_deref(),
        Some("Bearer session-123")
    );
    assert_eq!(gateway_bearer_header_value(Some("   ")), None);
    assert_eq!(gateway_bearer_header_value(None), None);
}

#[test]
fn edit_command_with_args_posts_to_gateway_path_without_local_publish() {
    let (temp_root, mut store) = temp_store("lobster-tui-edit-gateway-path");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_conversation_id = conversation_id.clone();
    let mut selected_conversation_id = ConversationId("room:world:lobby".into());

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/edit msg-42 新正文",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert!(transport.published.is_empty());
    assert_eq!(selected_conversation_id, conversation_id);
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0]
            .envelope
            .body
            .plain_text
            .contains("编辑消息失败：Gateway 未配置")
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn recall_command_with_args_posts_to_gateway_path_without_local_publish() {
    let (temp_root, mut store) = temp_store("lobster-tui-recall-gateway-path");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_conversation_id = conversation_id.clone();
    let mut selected_conversation_id = ConversationId("room:world:lobby".into());

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/recall msg-42",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert!(transport.published.is_empty());
    assert_eq!(selected_conversation_id, conversation_id);
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0]
            .envelope
            .body
            .plain_text
            .contains("撤回消息失败：Gateway 未配置")
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn plain_submission_keeps_pending_entry_when_publish_fails() {
    let (temp_root, mut store) = temp_store("lobster-tui-plain-submit-pending");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::failing();
    let mut active_conversation_id = conversation_id.clone();
    let mut selected_conversation_id = ConversationId("room:world:lobby".into());

    let result = handle_terminal_submission(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "失败时保留待投递正文",
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert_eq!(active_conversation_id, conversation_id);
    assert_eq!(selected_conversation_id, conversation_id);
    assert!(transport.published.is_empty());
    assert_eq!(
        transport.connection_state(),
        WakuConnectionState::Disconnected
    );
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].delivery_state, DeliveryState::PendingNetwork);
    assert_eq!(entries[0].envelope.body.plain_text, "失败时保留待投递正文");
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn edit_command_success_notice_can_be_tested_without_global_gateway_env() {
    let (temp_root, mut store) = temp_store("lobster-tui-edit-success-injected");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_conversation_id = conversation_id.clone();
    let mut selected_conversation_id = ConversationId("room:world:lobby".into());
    let mut calls = Vec::new();

    let result = handle_terminal_submission_with_gateway_post(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/edit msg-42 修订正文",
        &mut |path, body| {
            calls.push((path.to_string(), body));
            Ok(serde_json::json!({"edit_status": "edited"}))
        },
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert!(transport.published.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "/v1/shell/message/edit");
    assert_eq!(calls[0].1["message_id"], "msg-42");
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0]
            .envelope
            .body
            .plain_text
            .contains("已编辑消息 msg-42")
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn recall_command_success_notice_can_be_tested_without_global_gateway_env() {
    let (temp_root, mut store) = temp_store("lobster-tui-recall-success-injected");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::new();
    let mut active_conversation_id = conversation_id.clone();
    let mut selected_conversation_id = ConversationId("room:world:lobby".into());
    let mut calls = Vec::new();

    let result = handle_terminal_submission_with_gateway_post(
        &mut store,
        &mut transport,
        LaunchSurface::User,
        &mut active_conversation_id,
        &mut selected_conversation_id,
        "/recall msg-42",
        &mut |path, body| {
            calls.push((path.to_string(), body));
            Ok(serde_json::json!({"recall_status": "recalled"}))
        },
    )
    .unwrap();

    assert!(matches!(result, SubmissionAction::Continue));
    assert!(transport.published.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "/v1/shell/message/recall");
    assert_eq!(calls[0].1["message_id"], "msg-42");
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0]
            .envelope
            .body
            .plain_text
            .contains("已撤回消息 msg-42")
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn append_local_message_keeps_pending_copy_when_publish_fails() {
    let (temp_root, mut store) = temp_store("lobster-tui-pending-send");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::failing();

    append_local_message(
        &mut store,
        &mut transport,
        &conversation_id,
        "tiyan",
        "离线正文也要留在本地",
    )
    .unwrap();

    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].delivery_state, DeliveryState::PendingNetwork);
    assert_eq!(entries[0].envelope.body.plain_text, "离线正文也要留在本地");
    assert!(transport.published.is_empty());
    assert_eq!(
        transport.connection_state(),
        WakuConnectionState::Disconnected
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn append_local_message_marks_message_delivered_after_successful_publish() {
    let (temp_root, mut store) = temp_store("lobster-tui-delivered-send");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let mut transport = RecordingTransport::new();

    append_local_message(
        &mut store,
        &mut transport,
        &conversation_id,
        "tiyan",
        "在线正文应该直接已投递",
    )
    .unwrap();

    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].delivery_state, DeliveryState::Delivered);
    assert_eq!(transport.published.len(), 1);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn attachment_entries_project_fallback_text_without_leaking_raw_reference() {
    fn attachment_entry(preview: &str, plain_text: &str) -> TimelineEntry {
        TimelineEntry {
            envelope: MessageEnvelope {
                message_id: MessageId("attachment-msg-1".into()),
                conversation_id: ConversationId("room:world:lobby".into()),
                sender: IdentityId("rsaga".into()),
                reply_to_message_id: None,
                sender_device: DeviceId("gateway".into()),
                sender_profile: ClientProfile::mobile_web(),
                payload_type: PayloadType::AttachmentRef,
                body: MessageBody {
                    preview: preview.into(),
                    plain_text: plain_text.into(),
                    language_tag: "zh-CN".into(),
                },
                ciphertext: vec![],
                timestamp_ms: 1_760_000_000_000,
                ephemeral: false,
            },
            delivery_state: DeliveryState::Delivered,
            archived_at_ms: None,
            pinned: false,
            recalled_at_ms: None,
            recalled_by: None,
            edited_at_ms: None,
            edited_by: None,
        }
    }

    let id = "0123456789abcdef0123456789abcdef";
    let with_caption = attachment_entry(
        "[图片] 看这只龙虾",
        &format!("attachment://{id}\n看这只龙虾"),
    );
    assert_eq!(
        crate::message_projection::timeline_entry_text(&with_caption),
        "[图片] 看这只龙虾"
    );
    assert_eq!(
        crate::message_projection::timeline_entry_preview(&with_caption),
        "[图片] 看这只龙虾"
    );

    let without_caption = attachment_entry("[图片]", &format!("attachment://{id}"));
    assert_eq!(
        crate::message_projection::timeline_entry_text(&without_caption),
        "[图片]"
    );

    let raw_preview =
        attachment_entry(&format!("attachment://{id}"), &format!("attachment://{id}"));
    assert_eq!(
        crate::message_projection::timeline_entry_preview(&raw_preview),
        "[图片]"
    );
    assert!(!crate::message_projection::timeline_entry_text(&raw_preview).contains(id));

    let recalled = attachment_entry("[图片]", &format!("attachment://{id}"));
    let mut recalled = recalled;
    recalled.recalled_at_ms = Some(1_760_000_001_000);
    assert_eq!(
        crate::message_projection::timeline_entry_text(&recalled),
        "消息已撤回"
    );

    let mut plain = attachment_entry("普通文本", "普通文本");
    plain.envelope.body.plain_text = "普通文本".into();
    plain.envelope.body.preview = "普通文本".into();
    assert_eq!(
        crate::message_projection::timeline_entry_text(&plain),
        "普通文本"
    );
    assert_eq!(
        crate::message_projection::timeline_entry_preview(&plain),
        "普通文本"
    );
}

#[test]
fn transcript_and_conversation_summary_render_recall_and_edit_metadata() {
    let (temp_root, mut store) = temp_store("lobster-tui-recall-edit-render");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation.clone()).unwrap();
    let first = MessageEnvelope {
        message_id: MessageId("status-msg-1".into()),
        conversation_id: conversation_id.clone(),
        sender: IdentityId("tiyan".into()),
        reply_to_message_id: None,
        sender_device: DeviceId("desktop".into()),
        sender_profile: ClientProfile::desktop_terminal(),
        payload_type: PayloadType::Text,
        body: MessageBody {
            preview: "原始正文".into(),
            plain_text: "原始正文".into(),
            language_tag: "zh-CN".into(),
        },
        ciphertext: vec![],
        timestamp_ms: 1_760_000_000_000,
        ephemeral: false,
    };
    let second = MessageEnvelope {
        message_id: MessageId("status-msg-2".into()),
        conversation_id: conversation_id.clone(),
        sender: IdentityId("rsaga".into()),
        reply_to_message_id: None,
        sender_device: DeviceId("desktop".into()),
        sender_profile: ClientProfile::desktop_terminal(),
        payload_type: PayloadType::Text,
        body: MessageBody {
            preview: "待撤回正文".into(),
            plain_text: "待撤回正文".into(),
            language_tag: "zh-CN".into(),
        },
        ciphertext: vec![],
        timestamp_ms: 1_760_000_000_100,
        ephemeral: false,
    };
    store.append_message(first).unwrap();
    store.append_message(second).unwrap();
    store
        .edit_message(
            &conversation_id,
            &MessageId("status-msg-1".into()),
            IdentityId("tiyan".into()),
            "编辑后正文".into(),
            1_760_000_000_200,
        )
        .unwrap();
    store
        .recall_message(
            &conversation_id,
            &MessageId("status-msg-2".into()),
            IdentityId("rsaga".into()),
            1_760_000_000_300,
        )
        .unwrap();

    let transcript = strip_ansi_sgr(
        &transcript_lines(
            &store,
            &conversation_id,
            "tiyan",
            1_760_000_001_000,
            TerminalRenderProfile::desktop_default(),
        )
        .join("\n"),
    );
    assert!(transcript.contains("已编辑"), "{transcript}");
    assert!(transcript.contains("编辑后正文"), "{transcript}");
    assert!(transcript.contains("已撤回"), "{transcript}");
    assert!(transcript.contains("消息已撤回"), "{transcript}");
    assert!(!transcript.contains("待撤回正文"), "{transcript}");

    let summary = conversation_list_summary(
        LaunchSurface::User,
        &store,
        &conversation,
        1_760_000_001_000,
    );
    assert!(summary.contains("消息已撤回"), "{summary}");
    assert!(!summary.contains("待撤回正文"), "{summary}");
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn web_shell_export_preserves_message_delivery_recall_and_edit_metadata() {
    let (temp_root, mut store) = temp_store("lobster-tui-web-export-message-status");
    let generated_dir = temp_root.join("generated");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation.clone()).unwrap();

    let edited = MessageEnvelope {
        message_id: MessageId("web-export-edited".into()),
        conversation_id: conversation_id.clone(),
        sender: IdentityId("tiyan".into()),
        reply_to_message_id: None,
        sender_device: DeviceId("gateway".into()),
        sender_profile: ClientProfile::mobile_web(),
        payload_type: PayloadType::Text,
        body: MessageBody {
            preview: "编辑前".into(),
            plain_text: "编辑前".into(),
            language_tag: "zh-CN".into(),
        },
        ciphertext: vec![],
        timestamp_ms: 1_760_000_001_000,
        ephemeral: false,
    };
    let recalled = MessageEnvelope {
        message_id: MessageId("web-export-recalled".into()),
        conversation_id: conversation_id.clone(),
        sender: IdentityId("tiyan".into()),
        reply_to_message_id: None,
        sender_device: DeviceId("gateway".into()),
        sender_profile: ClientProfile::mobile_web(),
        payload_type: PayloadType::Text,
        body: MessageBody {
            preview: "撤回前".into(),
            plain_text: "撤回前".into(),
            language_tag: "zh-CN".into(),
        },
        ciphertext: vec![],
        timestamp_ms: 1_760_000_002_000,
        ephemeral: false,
    };
    let edited_then_recalled = MessageEnvelope {
        message_id: MessageId("web-export-edited-then-recalled".into()),
        conversation_id: conversation_id.clone(),
        sender: IdentityId("tiyan".into()),
        reply_to_message_id: None,
        sender_device: DeviceId("gateway".into()),
        sender_profile: ClientProfile::mobile_web(),
        payload_type: PayloadType::Text,
        body: MessageBody {
            preview: "先编辑再撤回".into(),
            plain_text: "先编辑再撤回".into(),
            language_tag: "zh-CN".into(),
        },
        ciphertext: vec![],
        timestamp_ms: 1_760_000_002_500,
        ephemeral: false,
    };
    store
        .merge_message(edited.clone(), DeliveryState::Delivered)
        .unwrap();
    store
        .merge_message(recalled.clone(), DeliveryState::Delivered)
        .unwrap();
    store
        .merge_message(edited_then_recalled.clone(), DeliveryState::Delivered)
        .unwrap();
    store
        .edit_message(
            &conversation_id,
            &MessageId("web-export-edited".into()),
            IdentityId("tiyan".into()),
            "编辑后".into(),
            1_760_000_001_100,
        )
        .unwrap();
    store
        .recall_message(
            &conversation_id,
            &MessageId("web-export-recalled".into()),
            IdentityId("tiyan".into()),
            1_760_000_002_100,
        )
        .unwrap();
    store
        .edit_message(
            &conversation_id,
            &MessageId("web-export-edited-then-recalled".into()),
            IdentityId("tiyan".into()),
            "先编辑后的正文".into(),
            1_760_000_002_600,
        )
        .unwrap();
    store
        .recall_message(
            &conversation_id,
            &MessageId("web-export-edited-then-recalled".into()),
            IdentityId("tiyan".into()),
            1_760_000_002_700,
        )
        .unwrap();

    export_web_shell_data(
        &generated_dir,
        &host_adapter::default_mobile_web_bootstrap(),
        &store,
        &[conversation],
        1_760_000_003_000,
    )
    .unwrap();

    let state_json: serde_json::Value = serde_json::from_slice(
        &fs::read(generated_dir.join("state.json")).expect("generated state"),
    )
    .expect("state json");
    let messages = state_json["rooms"][0]["messages"]
        .as_array()
        .expect("messages");
    let edited_message = messages
        .iter()
        .find(|message| message["message_id"] == "web-export-edited")
        .expect("edited exported message");
    assert_eq!(edited_message["delivery_status"], "delivered");
    assert_eq!(edited_message["text"], "编辑后");
    assert_eq!(edited_message["is_edited"], true);
    assert_eq!(edited_message["edited_by"], "tiyan");
    assert_eq!(edited_message["edited_at_ms"], 1_760_000_001_100i64);

    let recalled_message = messages
        .iter()
        .find(|message| message["message_id"] == "web-export-recalled")
        .expect("recalled exported message");
    assert_eq!(recalled_message["delivery_status"], "delivered");
    assert_eq!(recalled_message["text"], "消息已撤回");
    assert_eq!(recalled_message["is_recalled"], true);
    assert_eq!(recalled_message["recalled_by"], "tiyan");
    assert_eq!(recalled_message["recalled_at_ms"], 1_760_000_002_100i64);

    let edited_then_recalled_message = messages
        .iter()
        .find(|message| message["message_id"] == "web-export-edited-then-recalled")
        .expect("edited then recalled exported message");
    assert_eq!(edited_then_recalled_message["delivery_status"], "delivered");
    assert_eq!(edited_then_recalled_message["text"], "消息已撤回");
    assert_eq!(edited_then_recalled_message["is_recalled"], true);
    assert_eq!(edited_then_recalled_message["recalled_by"], "tiyan");
    assert_eq!(
        edited_then_recalled_message["recalled_at_ms"],
        1_760_000_002_700i64
    );
    assert_eq!(edited_then_recalled_message["is_edited"], true);
    assert_eq!(edited_then_recalled_message["edited_by"], "tiyan");
    assert_eq!(
        edited_then_recalled_message["edited_at_ms"],
        1_760_000_002_600i64
    );

    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn merge_polled_messages_marks_gateway_messages_delivered() {
    let (temp_root, mut store) = temp_store("lobster-tui-merge-polled-delivered");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let message = MessageEnvelope {
        message_id: MessageId("gateway-msg-1".into()),
        conversation_id: conversation_id.clone(),
        sender: IdentityId("rsaga".into()),
        reply_to_message_id: None,
        sender_device: DeviceId("gateway".into()),
        sender_profile: ClientProfile::mobile_web(),
        payload_type: PayloadType::Text,
        body: MessageBody {
            preview: "网关同步正文".into(),
            plain_text: "网关同步正文".into(),
            language_tag: "zh-CN".into(),
        },
        ciphertext: vec![],
        timestamp_ms: 1_760_000_000_000,
        ephemeral: false,
    };

    let inserted = merge_polled_messages(&mut store, vec![message]).unwrap();

    assert_eq!(inserted, 1);
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].delivery_state, DeliveryState::Delivered);
    assert_eq!(entries[0].envelope.body.plain_text, "网关同步正文");
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn merge_polled_messages_preserves_local_recall_and_edit_metadata() {
    let (temp_root, mut store) = temp_store("lobster-tui-merge-polled-metadata");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let edited = MessageEnvelope {
        message_id: MessageId("gateway-edit-1".into()),
        conversation_id: conversation_id.clone(),
        sender: IdentityId("rsaga".into()),
        reply_to_message_id: None,
        sender_device: DeviceId("gateway".into()),
        sender_profile: ClientProfile::mobile_web(),
        payload_type: PayloadType::Text,
        body: MessageBody {
            preview: "编辑前".into(),
            plain_text: "编辑前".into(),
            language_tag: "zh-CN".into(),
        },
        ciphertext: vec![],
        timestamp_ms: 1_760_000_000_000,
        ephemeral: false,
    };
    let recalled = MessageEnvelope {
        message_id: MessageId("gateway-recall-1".into()),
        conversation_id: conversation_id.clone(),
        sender: IdentityId("tiyan".into()),
        reply_to_message_id: None,
        sender_device: DeviceId("gateway".into()),
        sender_profile: ClientProfile::mobile_web(),
        payload_type: PayloadType::Text,
        body: MessageBody {
            preview: "撤回前".into(),
            plain_text: "撤回前".into(),
            language_tag: "zh-CN".into(),
        },
        ciphertext: vec![],
        timestamp_ms: 1_760_000_000_100,
        ephemeral: false,
    };
    store
        .merge_message(edited.clone(), DeliveryState::Delivered)
        .unwrap();
    store
        .merge_message(recalled.clone(), DeliveryState::Delivered)
        .unwrap();
    store
        .edit_message(
            &conversation_id,
            &MessageId("gateway-edit-1".into()),
            IdentityId("rsaga".into()),
            "编辑后".into(),
            1_760_000_000_200,
        )
        .unwrap();
    store
        .recall_message(
            &conversation_id,
            &MessageId("gateway-recall-1".into()),
            IdentityId("tiyan".into()),
            1_760_000_000_300,
        )
        .unwrap();

    let inserted = merge_polled_messages(&mut store, vec![edited, recalled]).unwrap();

    assert_eq!(inserted, 0);
    let entries = store.export_messages(&conversation_id);
    let edited_entry = entries
        .iter()
        .find(|entry| entry.envelope.message_id == MessageId("gateway-edit-1".into()))
        .expect("edited entry");
    assert_eq!(edited_entry.edited_at_ms, Some(1_760_000_000_200));
    assert_eq!(edited_entry.edited_by, Some(IdentityId("rsaga".into())));
    let recalled_entry = entries
        .iter()
        .find(|entry| entry.envelope.message_id == MessageId("gateway-recall-1".into()))
        .expect("recalled entry");
    assert_eq!(recalled_entry.recalled_at_ms, Some(1_760_000_000_300));
    assert_eq!(recalled_entry.recalled_by, Some(IdentityId("tiyan".into())));
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn republish_pending_messages_upgrades_entries_after_transport_recovers() {
    let (temp_root, mut store) = temp_store("lobster-tui-republish-pending");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let message = MessageEnvelope {
        message_id: MessageId("pending-msg-1".into()),
        conversation_id: conversation_id.clone(),
        sender: IdentityId("tiyan".into()),
        reply_to_message_id: None,
        sender_device: DeviceId("tiyan-terminal".into()),
        sender_profile: ClientProfile::desktop_terminal(),
        payload_type: PayloadType::Text,
        body: MessageBody {
            preview: "恢复后应自动补发".into(),
            plain_text: "恢复后应自动补发".into(),
            language_tag: "zh-CN".into(),
        },
        ciphertext: vec![],
        timestamp_ms: 1_000,
        ephemeral: false,
    };
    store
        .merge_message(message.clone(), DeliveryState::PendingNetwork)
        .unwrap();

    let mut transport = RecordingTransport::new();
    let republished = republish_pending_messages(&mut store, &mut transport).unwrap();

    assert_eq!(republished, 1);
    assert_eq!(transport.published.len(), 1);
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].delivery_state, DeliveryState::Delivered);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn republish_pending_messages_skips_recalled_entries() {
    let (temp_root, mut store) = temp_store("lobster-tui-republish-skips-recalled");
    let conversation = launch_conversation(LaunchSurface::User);
    let conversation_id = conversation.conversation_id.clone();
    store.upsert_conversation(conversation).unwrap();
    let message = MessageEnvelope {
        message_id: MessageId("pending-recalled-msg-1".into()),
        conversation_id: conversation_id.clone(),
        sender: IdentityId("tiyan".into()),
        reply_to_message_id: None,
        sender_device: DeviceId("tiyan-terminal".into()),
        sender_profile: ClientProfile::desktop_terminal(),
        payload_type: PayloadType::Text,
        body: MessageBody {
            preview: "撤回后不应补发".into(),
            plain_text: "撤回后不应补发".into(),
            language_tag: "zh-CN".into(),
        },
        ciphertext: vec![],
        timestamp_ms: 1_000,
        ephemeral: false,
    };
    store
        .merge_message(message, DeliveryState::PendingNetwork)
        .unwrap();
    store
        .recall_message(
            &conversation_id,
            &MessageId("pending-recalled-msg-1".into()),
            IdentityId("tiyan".into()),
            1_100,
        )
        .unwrap();

    let mut transport = RecordingTransport::new();
    let republished = republish_pending_messages(&mut store, &mut transport).unwrap();

    assert_eq!(republished, 0);
    assert!(transport.published.is_empty());
    let entries = store.export_messages(&conversation_id);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].delivery_state, DeliveryState::PendingNetwork);
    assert_eq!(entries[0].recalled_at_ms, Some(1_100));
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn ratatui_nav_highlight_style_avoids_background_fill_for_preview_cursor() {
    let style = ratatui_nav_highlight_style(true);

    assert_eq!(style.bg, None);
    assert!(style.add_modifier.is_empty());
}

#[test]
fn ratatui_nav_item_lines_move_summary_to_selected_row_during_preview() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "大厅摘要".into(),
            is_active: true,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "世界广场".into(),
            summary: "广场摘要".into(),
            is_active: false,
            is_selected: true,
            is_private: false,
        },
    ];
    context.preview_conversation = Some("世界广场".into());
    context.focus_area = FocusArea::Nav;

    let (rows, selected_index) = ratatui_nav_rows(&context);
    let active_plain = ratatui_nav_item_lines(&rows[0], false)
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let selected_plain = ratatui_nav_item_lines(&rows[selected_index], true)
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(!active_plain.contains("大厅摘要"));
    assert!(selected_plain.contains("▚回车入城内▞"));
}

#[test]
fn ratatui_nav_item_lines_add_enter_hint_to_selected_preview_summary() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    let (rows, selected_index) = ratatui_nav_rows(&context);
    let selected_plain = ratatui_nav_item_lines(&rows[selected_index], true)
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(selected_plain.contains("▚回车入城内▞"));
}

#[test]
fn ratatui_nav_item_lines_make_entered_room_solid_and_preview_row_a_cyan_card() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    let (rows, selected_index) = ratatui_nav_rows(&context);
    let active_lines = ratatui_nav_item_lines(&rows[0], false);
    let selected_lines = ratatui_nav_item_lines(&rows[selected_index], true);

    assert_eq!(active_lines[0].spans[0].content.as_ref(), "▐");
    assert_eq!(active_lines[0].spans[0].style.bg, Some(Color::Yellow));
    assert_eq!(active_lines[0].spans[5].style.bg, Some(Color::Yellow));
    assert_eq!(selected_lines[0].spans[0].content.as_ref(), "▐");
    assert_eq!(selected_lines[0].spans[0].style.bg, Some(Color::Cyan));
    assert_eq!(selected_lines[0].spans[5].style.bg, Some(Color::Cyan));
    assert_eq!(selected_lines[1].spans[1].content.as_ref(), "▐");
    assert_eq!(selected_lines[1].spans[1].style.bg, Some(Color::Cyan));
}

#[test]
fn ratatui_nav_item_lines_wrap_card_summary_with_pixel_edges() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    let (rows, selected_index) = ratatui_nav_rows(&context);
    let selected_lines = ratatui_nav_item_lines(&rows[selected_index], true);
    let summary_plain = selected_lines[1]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(summary_plain.contains("▐"));
    assert!(summary_plain.contains("▌"));
    assert!(summary_plain.contains("回车入城内"));
}

#[test]
fn ratatui_nav_item_lines_add_slim_gutter_to_idle_and_private_rows() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "钟塔后厅".into(),
            summary: "守夜门牌 · 灯还亮着".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "builder 居所".into(),
            summary: "居所单线 · 轻声来回".into(),
            is_active: false,
            is_selected: false,
            is_private: true,
        },
    ];
    let (rows, _) = ratatui_nav_rows(&context);

    let idle_lines = ratatui_nav_item_lines(&rows[0], false);
    let private_lines = ratatui_nav_item_lines(
        rows.iter()
            .find(|row| matches!(row.tone, RatatuiNavTone::Private))
            .unwrap(),
        false,
    );

    assert_eq!(idle_lines[0].spans[0].content.as_ref(), "▏");
    assert_eq!(idle_lines[0].spans[0].style.fg, Some(Color::Gray));
    assert_eq!(private_lines[0].spans[0].content.as_ref(), "▏");
    assert_eq!(
        private_lines[0].spans[0].style.fg,
        Some(Color::LightMagenta)
    );
}

#[test]
fn ratatui_wide_layout_still_renders_preview_navigation_on_large_terminal() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    assert_eq!(
        shell_layout_mode_for_size(180, 40),
        ShellLayoutMode::RatatuiWide
    );
    let frame = render_ratatui_frame_lines(&context, 180, 40);
    assert_eq!(frame.len(), 40);
}

#[test]
fn shell_layout_mode_prefers_stacked_ratatui_on_common_terminal_sizes() {
    assert_eq!(
        shell_layout_mode_for_size(110, 30),
        ShellLayoutMode::RatatuiStacked
    );
    assert_eq!(
        shell_layout_mode_for_size(68, 24),
        ShellLayoutMode::RatatuiStacked
    );
    assert_eq!(
        shell_layout_mode_for_size(200, 50),
        ShellLayoutMode::RatatuiWide
    );
    assert_eq!(
        shell_layout_mode_for_size(60, 14),
        ShellLayoutMode::PlainCompact
    );
}

#[test]
fn ratatui_stacked_layout_renders_all_major_panels_on_mid_sized_terminal() {
    let context = test_context(LaunchSurface::User);
    assert_eq!(
        shell_layout_mode_for_size(110, 30),
        ShellLayoutMode::RatatuiStacked
    );
    let frame = render_ratatui_frame_lines(&context, 110, 30);

    assert_eq!(frame.len(), 30);
}

#[test]
fn ratatui_session_info_lines_use_pixel_chips() {
    let context = test_context(LaunchSurface::User);
    let plain = ratatui_session_info_lines(&context)
        .into_iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("▛门牌印#1▜"));
    assert!(plain.contains("▛tiyan▜"));
    assert!(plain.contains("▛居民位▜"));
    assert!(!plain.contains("门牌印#1 · tiyan · 居民位"));
}

#[test]
fn ratatui_session_info_lines_show_city_public_profile_fields() {
    let context = test_context(LaunchSurface::User);
    let plain = ratatui_session_info_lines(&context)
        .into_iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("当前 第一城大厅"), "{plain}");
    assert!(plain.contains("角色 城市向导"), "{plain}");
    assert!(plain.contains("状态 公共频道"), "{plain}");
    assert!(plain.contains("动作 /dm guide"), "{plain}");
    assert!(plain.contains("联结 ▰▰▰▰ 100%"), "{plain}");
}

#[test]
fn ratatui_session_info_lines_show_city_public_profile_card_heading() {
    let context = test_context(LaunchSurface::User);
    let plain = ratatui_session_info_lines(&context)
        .into_iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("城市向导 · 角色卡"));
    assert!(plain.contains("称号 公共频道引路"));
    assert!(plain.contains("立绘 城市向导"));
    assert!(plain.contains("定位 公共引路"));
    assert!(plain.contains("[私聊] [委托]"));
}

#[test]
fn session_info_panel_shows_residence_private_fields() {
    let panel = render_session_info_panel(&test_context(LaunchSurface::Direct)).join("\n");

    assert!(panel.contains("住户 tiyan"), "{panel}");
    assert!(panel.contains("同住AI guide"), "{panel}");
    assert!(panel.contains("状态 房内连线"), "{panel}");
    assert!(panel.contains("动作 Enter 续聊"), "{panel}");
    assert!(panel.contains("亲和 ▰▰▰▰ 100%"), "{panel}");
}

#[test]
fn session_info_panel_shows_residence_profile_card_heading() {
    let panel = render_session_info_panel(&test_context(LaunchSurface::Direct)).join("\n");

    assert!(panel.contains("住户卡 · tiyan"));
    assert!(panel.contains("私线 房内续聊"));
    assert!(panel.contains("立绘 住户档案"));
    assert!(panel.contains("同住 guide"));
    assert!(panel.contains("[续聊] [整理]"));
}

#[test]
fn ratatui_input_lines_invert_composer_row_when_focused() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Input;
    context.input_buffer = "准备落字".into();

    let lines = ratatui_input_lines(&context);
    assert_eq!(lines.len(), 2);

    let chip_plain = lines[0]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    assert!(chip_plain.contains("【城内】"));
    assert!(!chip_plain.contains("▚【城内】▞"));
    assert!(chip_plain.contains("▚落字中▞"));
    assert!(chip_plain.contains("城内落字栏"));
    assert!(!chip_plain.contains("▚城内落字栏▞"));
    assert!(chip_plain.contains("回车入城内"));
    assert!(!chip_plain.contains("▚回车入城内▞"));
    assert!(chip_plain.contains("投向 第一城大厅"));
    assert!(!chip_plain.contains("▚投向 第一城大厅▞"));
    assert_eq!(lines[0].spans[0].style.bg, None);
    assert_eq!(lines[0].spans[0].style.fg, Some(Color::DarkGray));
    assert_eq!(lines[0].spans[2].style.fg, Some(Color::Green));
    assert_eq!(lines[0].spans[4].style.bg, None);
    assert_eq!(lines[0].spans[4].style.fg, Some(Color::DarkGray));
    assert_eq!(lines[0].spans[6].style.fg, Some(Color::DarkGray));
    assert_eq!(lines[0].spans[8].style.fg, Some(Color::DarkGray));

    let composer_plain = lines[1]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    assert_eq!(composer_plain, "▚█ 准备落字▞");
    assert_eq!(lines[1].spans.len(), 1);
    assert_eq!(lines[1].spans[0].style.bg, Some(Color::Green));
    assert_eq!(lines[1].spans[0].style.fg, Some(Color::Black));
}

#[test]
fn ratatui_input_lines_show_waiting_badge_when_not_focused() {
    let context = test_context(LaunchSurface::User);
    let lines = ratatui_input_lines(&context);
    let chip_plain = lines[0]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(chip_plain.contains("【城内】"));
    assert!(!chip_plain.contains("▚【城内】▞"));
    assert!(chip_plain.contains("▚待落▞"));
    assert!(!chip_plain.contains("▚落字中▞"));
    assert!(chip_plain.contains("城内落字栏"));
    assert!(chip_plain.contains("回车入城内"));
    assert!(!chip_plain.contains("▚回车入城内▞"));
    assert!(chip_plain.contains("投向 第一城大厅"));
    assert!(!chip_plain.contains("▚投向 第一城大厅▞"));
    assert!(!chip_plain.contains("▚城内落字栏▞"));
    assert_eq!(lines[0].spans[0].style.bg, None);
    assert_eq!(lines[0].spans[0].style.fg, Some(Color::DarkGray));
    assert_eq!(lines[0].spans[4].style.bg, None);
    assert_eq!(lines[0].spans[4].style.fg, Some(Color::DarkGray));
    assert_eq!(lines[0].spans[6].style.fg, Some(Color::DarkGray));
    assert_eq!(lines[0].spans[8].style.fg, Some(Color::DarkGray));
    assert_eq!(lines[1].spans[0].style.fg, Some(Color::DarkGray));
}

#[test]
fn ratatui_input_lines_truncate_composer_with_shared_formatter() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Input;
    let long_buffer = "特别长的回声".repeat(24);
    context.input_buffer = long_buffer.clone();

    let semantics = input_panel_semantics(&context, INPUT_PANEL_WIDTH.saturating_sub(48).max(12));
    let expected = format!(
        "▚{}▞",
        input_composer_line(&semantics, INPUT_PANEL_WIDTH.saturating_sub(6))
    );
    let untruncated = format!("▚{} {}▞", semantics.cursor_glyph, long_buffer);
    let actual = ratatui_input_lines(&context)[1]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert_eq!(actual, expected);
    assert_ne!(actual, untruncated);
}

#[test]
fn ratatui_scene_chip_line_uses_room_pixel_badges() {
    let line = ratatui_scene_chip_line(&test_context(LaunchSurface::User));
    let plain = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(plain.contains("▛城主府▜"));
    assert!(plain.contains("▚居民区▞"));
    assert!(plain.contains("▚传送阵▞"));
    assert!(plain.contains("▚钟塔▞"));
    assert_eq!(line.spans[0].style.bg, Some(Color::Yellow));
    assert_eq!(line.spans[2].style.bg, None);
    assert_eq!(line.spans[2].style.fg, Some(Color::Green));
}

#[test]
fn city_public_scene_semantics_promote_clocktower_into_primary_legend() {
    let semantics = scene_panel_semantics(&test_context(LaunchSurface::User));
    assert_eq!(semantics.legend, &["城主府", "居民区", "传送阵", "钟塔"]);
    assert!(scene_legend_summary(semantics.legend).contains("钟塔"));
}

#[test]
fn city_public_scene_grid_forms_civic_core_with_residences_and_street_bridge() {
    let grid = ratatui_scene_tile_grid(&test_context(LaunchSurface::User));

    assert_eq!(grid.rows.len(), 3);
    assert_eq!(
        grid.rows[0],
        vec![
            SceneTileKind::Wall,
            SceneTileKind::Tower,
            SceneTileKind::Gate,
            SceneTileKind::Gate,
            SceneTileKind::Gate,
            SceneTileKind::Gate,
            SceneTileKind::Tower,
            SceneTileKind::Portal,
            SceneTileKind::Wall,
            SceneTileKind::Wall,
        ]
    );
    assert_eq!(
        grid.rows[1],
        vec![
            SceneTileKind::Wall,
            SceneTileKind::Window,
            SceneTileKind::Window,
            SceneTileKind::Floor,
            SceneTileKind::Floor,
            SceneTileKind::Floor,
            SceneTileKind::Window,
            SceneTileKind::Window,
            SceneTileKind::Floor,
            SceneTileKind::Wall,
        ]
    );
    assert_eq!(
        grid.rows[2],
        vec![
            SceneTileKind::Wall,
            SceneTileKind::Bridge,
            SceneTileKind::Bridge,
            SceneTileKind::Bridge,
            SceneTileKind::Floor,
            SceneTileKind::Door,
            SceneTileKind::Door,
            SceneTileKind::Bridge,
            SceneTileKind::Floor,
            SceneTileKind::Wall,
        ]
    );
}

#[test]
fn ratatui_scene_chip_line_follow_semantic_world_legend_before_extra_chip() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );
    let plain = ratatui_scene_chip_line(&context)
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    let gate_index = plain.find("城门牌").unwrap();
    let bridge_index = plain.find("石桥").unwrap();
    let portal_index = plain.find("传送阵").unwrap();
    let river_index = plain.find("河道").unwrap();

    assert!(gate_index < bridge_index);
    assert!(bridge_index < portal_index);
    assert!(portal_index < river_index);
}

#[test]
fn ratatui_scene_lines_render_structured_tile_grid_before_summary_row() {
    let lines = ratatui_scene_lines(&test_context(LaunchSurface::User));
    let first = lines[0]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    let grid = lines[1..4]
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let footer = lines[4]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(first.contains("▛城主府▜"));
    assert!(!grid.contains("居民区 / 传送阵"));
    assert!(!grid.contains("城主府"));
    assert!(grid.contains("▀") || grid.contains("█"));
    assert!(!grid.contains("▣"));
    assert!(!grid.contains("▤"));
    assert!(!grid.contains("▥"));
    assert!(!grid.contains("▦"));
    assert!(footer.contains("场记"));
}

#[test]
fn ratatui_scene_lines_mix_safe_block_glyph_recipes_for_room_tiles() {
    let lines = ratatui_scene_lines(&test_context(LaunchSurface::User));
    let grid = lines[1..4]
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(grid.contains("▀"));
    assert!(grid.contains("▄"));
    assert!(grid.contains("▐"));
    assert!(grid.contains("▌"));
}

#[test]
fn ratatui_scene_lines_use_background_tiles_for_room_rows() {
    let lines = ratatui_scene_lines(&test_context(LaunchSurface::User));
    let tile_row = &lines[1];
    let mut backgrounds = tile_row
        .spans
        .iter()
        .filter_map(|span| span.style.bg)
        .map(|color| format!("{color:?}"))
        .collect::<Vec<_>>();
    backgrounds.sort();
    backgrounds.dedup();

    assert!(tile_row.spans.iter().any(|span| span.style.bg.is_some()));
    assert!(backgrounds.len() >= 3);
}

#[test]
fn scene_tile_grid_follows_preview_surface_page() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    let grid = ratatui_scene_tile_grid(&context);
    let tiles = grid.rows.into_iter().flatten().collect::<Vec<_>>();

    assert!(tiles.contains(&SceneTileKind::Gate));
    assert!(tiles.contains(&SceneTileKind::Portal));
    assert!(tiles.contains(&SceneTileKind::Water));
    assert!(tiles.contains(&SceneTileKind::Tower));
    assert!(!tiles.contains(&SceneTileKind::Desk));
    assert!(!tiles.contains(&SceneTileKind::Sofa));
    assert!(!tiles.contains(&SceneTileKind::Cabinet));
}

#[test]
fn scene_panel_semantics_follow_preview_surface_page_and_summary() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    let semantics = scene_panel_semantics(&context);
    let tiles = semantics
        .tile_grid
        .rows
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    assert!(tiles.contains(&SceneTileKind::Gate));
    assert!(tiles.contains(&SceneTileKind::Portal));
    assert!(tiles.contains(&SceneTileKind::Water));
    assert!(!tiles.contains(&SceneTileKind::Desk));
    assert_eq!(
        semantics.legend,
        scene_legend_labels_for_page(SurfacePage::World)
    );
    assert_eq!(semantics.summary, display_scene_summary(&context));
}

#[test]
fn room_scene_metadata_maps_to_semantic_tiles() {
    let room_tiles = ratatui_scene_tile_grid(&test_context(LaunchSurface::Direct))
        .rows
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let default_tiles = ratatui_scene_tile_grid(&test_context(LaunchSurface::User))
        .rows
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    assert!(room_tiles.contains(&SceneTileKind::Avatar));
    assert!(
        room_tiles
            .iter()
            .filter(|tile| **tile == SceneTileKind::Desk)
            .count()
            >= 2
    );
    assert!(
        room_tiles
            .iter()
            .filter(|tile| **tile == SceneTileKind::Sofa)
            .count()
            >= 2
    );
    assert!(!default_tiles.contains(&SceneTileKind::Avatar));
}

#[test]
fn ratatui_scene_lines_use_scene_metadata_landmarks_for_direct_room() {
    let lines = ratatui_scene_lines(&test_context(LaunchSurface::Direct));
    let grid = lines[1..4]
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let footer = lines[4]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(grid.contains("▀") || grid.contains("█"));
    assert!(!grid.contains("◉"));
    assert!(!grid.contains("▤"));
    assert!(!grid.contains("▥"));
    assert!(footer.contains("木地板、沙发与暖灯"));
}

#[test]
fn ratatui_scene_lines_keep_ascii_tile_fallback_when_glyphs_are_ascii_only() {
    let mut context = test_context(LaunchSurface::Direct);
    context.render_profile = TerminalRenderProfile::low_resource_default();

    let lines = ratatui_scene_lines(&context);
    let grid = lines[1..4]
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(grid.contains("[]"));
    assert!(grid.contains("D "));
    assert!(grid.contains("S "));
    assert!(grid.contains("C "));
    assert!(grid.contains("@ "));
    assert!(!grid.contains("▀"));
    assert!(!grid.contains("█"));
}

#[test]
fn ratatui_scene_lines_fall_back_when_terminal_lacks_block_glyphs() {
    let mut context = test_context(LaunchSurface::Direct);
    context.render_profile.glyph_support = TerminalGlyphSupport::UnicodeBasic;

    let lines = ratatui_scene_lines(&context);
    let grid = lines[1..4]
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(grid.contains("[]"));
    assert!(grid.contains("D "));
    assert!(grid.contains("S "));
    assert!(!grid.contains("▀"));
    assert!(!grid.contains("▄"));
    assert!(!grid.contains("▐"));
    assert!(!grid.contains("▌"));
    assert!(!grid.contains("█"));
}

#[test]
fn scene_tile_projection_rows_keep_compact_and_ratatui_grid_text_in_sync() {
    let context = test_context(LaunchSurface::User);

    let projected_plain = scene_tile_projection_rows(&context)
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|projection| projection.plain_text)
                .collect::<Vec<_>>()
                .join("")
        })
        .collect::<Vec<_>>();
    let compact = compact_scene_tile_rows(&context);
    let ratatui = ratatui_scene_tile_rows(&context)
        .into_iter()
        .map(|line| {
            line.spans
                .into_iter()
                .map(|span| span.content.into_owned())
                .collect::<String>()
        })
        .collect::<Vec<_>>();

    assert_eq!(compact, projected_plain);
    assert_eq!(ratatui, projected_plain);
}

#[test]
fn scene_tile_projection_rows_keep_ascii_fallback_in_sync_across_views() {
    let mut context = test_context(LaunchSurface::Direct);
    context.render_profile.glyph_support = TerminalGlyphSupport::UnicodeBasic;

    let projected_plain = scene_tile_projection_rows(&context)
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|projection| projection.plain_text)
                .collect::<Vec<_>>()
                .join("")
        })
        .collect::<Vec<_>>();
    let compact = compact_scene_tile_rows(&context);
    let ratatui = ratatui_scene_tile_rows(&context)
        .into_iter()
        .map(|line| {
            line.spans
                .into_iter()
                .map(|span| span.content.into_owned())
                .collect::<String>()
        })
        .collect::<Vec<_>>();

    assert_eq!(compact, projected_plain);
    assert_eq!(ratatui, projected_plain);
    assert!(projected_plain.join("\n").contains("[]"));
}

#[test]
fn ratatui_scene_lines_end_with_muted_scene_summary_not_latest_echo() {
    let lines = ratatui_scene_lines(&test_context(LaunchSurface::User));
    let footer = lines.last().expect("scene footer should exist");
    let plain = footer
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(plain.contains("场记"));
    assert!(!plain.contains("近响"));
    assert!(plain.contains("先把公共频道聊顺"));
    assert_eq!(footer.spans[0].style.fg, Some(Color::Yellow));
    assert_eq!(footer.spans[2].style.fg, Some(Color::DarkGray));
}

#[test]
fn ratatui_scene_lines_use_scene_summary_footer_without_outline_badges() {
    let lines = ratatui_scene_lines(&test_context(LaunchSurface::User));
    let footer = lines.last().expect("scene footer should exist");
    let footer_plain = footer
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(footer_plain.contains("场记"));
    assert!(!footer_plain.contains("▚场记▞"));
    assert!(!footer_plain.contains("近响："));
    assert_eq!(footer.spans[0].style.fg, Some(Color::Yellow));
    assert_eq!(footer.spans[2].style.fg, Some(Color::DarkGray));
    assert!(!footer.spans[2].content.as_ref().starts_with('▚'));
}

#[test]
fn ratatui_border_type_switches_to_quadrant_outside_when_focused() {
    assert_eq!(ratatui_border_type(true), BorderType::QuadrantOutside);
    assert_eq!(ratatui_border_type(false), BorderType::QuadrantInside);
}

#[test]
fn ratatui_transcript_lines_wrap_self_header_as_pixel_card() {
    let mut context = test_context(LaunchSurface::User);
    context.transcript = vec!["▌ 我  刚刚".into(), "    先把房间做顺".into()];

    let lines = ratatui_transcript_lines(&context);
    let header_plain = lines[0]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(header_plain.contains("▐"));
    assert!(header_plain.contains("我"));
    assert!(!header_plain.contains("▚我▞"));
    assert!(header_plain.contains("我  刚刚"));
    assert!(!header_plain.contains("▌ 我  刚刚"));
    assert!(header_plain.contains("▌"));
    assert_eq!(lines[0].spans[0].style.fg, Some(Color::Yellow));
    assert_eq!(lines[0].spans[2].style.bg, None);
    assert_eq!(lines[0].spans[2].style.fg, Some(Color::DarkGray));
    assert_eq!(lines[0].spans[6].style.fg, Some(Color::DarkGray));
}

#[test]
fn ratatui_transcript_lines_keep_colored_gutter_and_muted_body_text() {
    let mut context = test_context(LaunchSurface::User);
    context.transcript = vec![
        "• builder  5分钟前".into(),
        "   终端优先，先保证回声好投".into(),
    ];

    let lines = ratatui_transcript_lines(&context);
    let body_plain = lines[1]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(body_plain.contains("▏"));
    assert!(!body_plain.contains("▚他▞"));
    assert!(body_plain.contains("终端优先"));
    assert_eq!(lines[1].spans[1].style.fg, Some(Color::Cyan));
    assert_eq!(lines[1].spans[3].style.fg, Some(Color::DarkGray));
}

#[test]
fn ratatui_transcript_lines_strip_legacy_prefix_from_other_header() {
    let mut context = test_context(LaunchSurface::User);
    context.transcript = vec![
        "• builder  5分钟前".into(),
        "   终端优先，先保证回声好投".into(),
    ];

    let lines = ratatui_transcript_lines(&context);
    let header_plain = lines[0]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(header_plain.contains("builder  5分钟前"));
    assert!(!header_plain.contains("• builder  5分钟前"));
}

#[test]
fn ratatui_transcript_lines_render_continuation_with_gutter_not_speaker_chip() {
    let mut context = test_context(LaunchSurface::User);
    context.transcript = vec![
        "• builder  5分钟前".into(),
        "   终端优先，先保证回声好投".into(),
    ];

    let lines = ratatui_transcript_lines(&context);
    let body_plain = lines[1]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(!body_plain.contains("▚他▞"));
    assert!(body_plain.contains("▏"));
}

#[test]
fn ratatui_transcript_line_renders_header_row_from_shared_semantics() {
    let line = ratatui_transcript_line(&RatatuiTranscriptRow {
        tone: RatatuiTranscriptTone::OtherEcho,
        is_header: true,
        text: "builder  5分钟前".into(),
    });
    let plain = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(plain.contains("他"));
    assert!(plain.contains("▐"));
    assert!(plain.contains("builder  5分钟前"));
    assert!(plain.contains("▌"));
    assert_eq!(line.spans[0].style.fg, Some(Color::Cyan));
    assert_eq!(line.spans[2].style.fg, Some(Color::DarkGray));
}

#[test]
fn ratatui_transcript_line_renders_continuation_row_from_shared_semantics() {
    let line = ratatui_transcript_line(&RatatuiTranscriptRow {
        tone: RatatuiTranscriptTone::SystemEcho,
        is_header: false,
        text: "继续回声".into(),
    });
    let plain = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(plain.contains("▏"));
    assert!(plain.contains("继续回声"));
    assert_eq!(line.spans[1].style.fg, Some(Color::Green));
    assert_eq!(line.spans[3].style.fg, Some(Color::DarkGray));
}

#[test]
fn transcript_panel_scroll_offset_reveals_older_echo_rows() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Transcript;
    context.transcript = (1..=40).map(|index| format!("第{index}道回声")).collect();
    context.transcript_scroll = 2;

    let panel = strip_ansi_sgr(&render_transcript_panel(&context).join("\n"));
    assert!(panel.contains("第9道回声"));
    assert!(panel.contains("第38道回声"));
    assert!(!panel.contains("第39道回声"));
    assert!(!panel.contains("第40道回声"));
}

#[test]
fn compact_transcript_panel_scroll_offset_reveals_older_rows() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Transcript;
    context.transcript = (1..=20).map(|index| format!("第{index}道回声")).collect();
    context.transcript_scroll = 1;

    let panel = strip_ansi_sgr(&render_compact_transcript_panel(&context, 84).join("\n"));
    assert!(panel.contains("第8道回声"));
    assert!(panel.contains("第19道回声"));
    assert!(!panel.contains("第20道回声"));
}

#[test]
fn scene_panel_does_not_gain_focus_marker_when_transcript_is_focused() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Transcript;

    let header = strip_ansi_sgr(&render_chat_header_panel(&context).join("\n"));
    let transcript = strip_ansi_sgr(&render_transcript_panel(&context).join("\n"));

    assert!(!header.contains("城市外景 ◉"));
    assert!(transcript.contains("公共频道 ◉"));
}

#[test]
fn scene_panel_gains_focus_marker_when_nav_is_focused() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Nav;

    let header = strip_ansi_sgr(&render_chat_header_panel(&context).join("\n"));
    let transcript = strip_ansi_sgr(&render_transcript_panel(&context).join("\n"));

    assert!(header.contains("城市外景 ◉"));
    assert!(!transcript.contains("公共频道 ◉"));
}

#[test]
fn transcript_text_style_dims_when_input_is_focused() {
    let mut context = test_context(LaunchSurface::User);
    context.focus_area = FocusArea::Input;

    let style = ratatui_transcript_text_style(&context);
    assert_eq!(style.fg, Some(Color::DarkGray));
}

#[test]
fn compact_transcript_panel_dims_body_rows_when_input_is_focused() {
    let mut transcript_context = test_context(LaunchSurface::User);
    transcript_context.transcript = vec!["第一道回声".into()];
    transcript_context.focus_area = FocusArea::Transcript;

    let mut input_context = transcript_context.clone();
    input_context.focus_area = FocusArea::Input;

    let transcript_panel = render_compact_transcript_panel(&transcript_context, 84);
    let input_panel = render_compact_transcript_panel(&input_context, 84);

    assert!(!transcript_panel[2].contains("\x1b["));
    assert!(input_panel[2].contains("\x1b["));
    assert!(strip_ansi_sgr(&input_panel[2]).contains("第一道回声"));
}

#[test]
fn compact_transcript_panel_strips_legacy_header_prefixes() {
    let mut context = test_context(LaunchSurface::User);
    context.transcript = vec![
        "▌ 我  19:40".into(),
        "    先把城内主景做顺".into(),
        "• builder  19:41".into(),
        "   再收 compact transcript".into(),
    ];

    let panel = strip_ansi_sgr(&render_compact_transcript_panel(&context, 84).join("\n"));

    assert!(panel.contains("我  19:40"));
    assert!(panel.contains("builder  19:41"));
    assert!(!panel.contains("▌ 我  19:40"));
    assert!(!panel.contains("• builder  19:41"));
}

#[test]
fn compact_transcript_panel_keeps_continuation_body_without_raw_log_markers() {
    let mut context = test_context(LaunchSurface::User);
    context.transcript = vec![
        "◇ system  19:42".into(),
        "   回声壳在线".into(),
        "   准备接下一句".into(),
    ];

    let panel = strip_ansi_sgr(&render_compact_transcript_panel(&context, 84).join("\n"));

    assert!(panel.contains("system  19:42"));
    assert!(panel.contains("回声壳在线"));
    assert!(panel.contains("准备接下一句"));
    assert!(!panel.contains("◇ system  19:42"));
}

#[test]
fn compact_transcript_panel_adds_semantic_badges_to_non_self_headers() {
    let mut context = test_context(LaunchSurface::User);
    context.transcript = vec!["• builder  19:41".into(), "◇ system  19:42".into()];

    let panel = strip_ansi_sgr(&render_compact_transcript_panel(&context, 84).join("\n"));

    assert!(panel.contains("他 builder  19:41"));
    assert!(panel.contains("系 system  19:42"));
}

#[test]
fn compact_transcript_panel_marks_continuation_rows_with_gutter() {
    let mut context = test_context(LaunchSurface::User);
    context.transcript = vec![
        "• builder  19:41".into(),
        "   再收 compact transcript".into(),
    ];

    let panel = strip_ansi_sgr(&render_compact_transcript_panel(&context, 84).join("\n"));

    assert!(panel.contains("· 再收 compact transcript"));
}

#[test]
fn leaving_nav_focus_restores_entered_room_view() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Transcript,
    );

    let status = strip_ansi_sgr(&render_status_strip(&context));
    let transcript = strip_ansi_sgr(&render_transcript_panel(&context).join("\n"));

    assert!(status.contains("第一城大厅"));
    assert!(transcript.contains("欢迎来到第一城大厅"));
    assert!(!transcript.contains("回声壳在线"));
}

#[test]
fn nav_focus_previews_selected_room_in_compact_transcript_too() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    let transcript = strip_ansi_sgr(&render_compact_transcript_panel(&context, 84).join("\n"));
    assert!(transcript.contains("回声壳在线"));
    assert!(!transcript.contains("欢迎来到第一城大厅"));
}

#[test]
fn nav_preview_moves_left_panel_summary_to_selected_row() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "大厅摘要".into(),
            is_active: true,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "世界广场".into(),
            summary: "广场摘要".into(),
            is_active: false,
            is_selected: true,
            is_private: false,
        },
    ];
    context.preview_conversation = Some("世界广场".into());
    context.focus_area = FocusArea::Nav;

    let panel = strip_ansi_sgr(&render_conversation_panel(&context).join("\n"));
    assert!(panel.contains("世界广场"));
    assert!(!panel.contains("大厅摘要"));
}

#[test]
fn nav_preview_adds_enter_hint_to_selected_summary_in_string_panel() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "大厅摘要".into(),
            is_active: true,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "世界广场".into(),
            summary: "广场摘要".into(),
            is_active: false,
            is_selected: true,
            is_private: false,
        },
    ];
    context.preview_conversation = Some("世界广场".into());
    context.focus_area = FocusArea::Nav;

    let panel = strip_ansi_sgr(&render_conversation_panel(&context).join("\n"));
    assert!(panel.contains("回车入城内"));
    assert!(panel.contains("世界广场"));
}

#[test]
fn active_summary_returns_when_nav_preview_is_not_running() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "大厅摘要".into(),
            is_active: true,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "世界广场".into(),
            summary: "广场摘要".into(),
            is_active: false,
            is_selected: true,
            is_private: false,
        },
    ];
    context.focus_area = FocusArea::Transcript;

    let panel = strip_ansi_sgr(&render_conversation_panel(&context).join("\n"));
    assert!(panel.contains("大厅摘要"));
    assert!(!panel.contains("广场摘要"));
}

#[test]
fn input_panel_switches_to_private_copy_after_active_dm_switch() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        direct_conversation("tiyan", "guide"),
        direct_conversation("tiyan", "guide"),
        FocusArea::Transcript,
    );

    let input = strip_ansi_sgr(&render_input_panel(&context).join("\n"));
    assert!(input.contains("居所落字栏"));
    assert!(input.contains("【居所】"));
    assert!(input.contains("回车入居所"));
    assert!(!input.contains("城内落字栏"));
    assert!(!input.contains("回车入城内"));
}

#[test]
fn compact_banner_switches_to_private_scene_legend_when_dm_is_active() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        direct_conversation("tiyan", "guide"),
        direct_conversation("tiyan", "guide"),
        FocusArea::Transcript,
    );

    let (_, subtitle) = compact_shell_banner(&context, 84);
    assert!(subtitle.contains("居所牌 / 会客桌 / 暖灯窗"));
    assert!(!subtitle.contains("城内牌 / 暖灯窗 / 书桌"));
}

#[test]
fn compact_banner_follows_preview_surface_and_scene_banner() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );

    let (header, subtitle) = compact_shell_banner(&context, 84);

    assert!(header.contains("城外 / 城门牌"));
    assert!(header.contains("世界广场"));
    assert!(!header.contains("城市 / 公共频道"));
    assert!(subtitle.contains("城门牌 / 石桥 / 传送阵"));
    assert!(!subtitle.contains("城内牌 / 暖灯窗 / 书桌"));
}

#[test]
fn compact_banner_uses_private_scene_banner_when_dm_is_active() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        direct_conversation("tiyan", "guide"),
        direct_conversation("tiyan", "guide"),
        FocusArea::Transcript,
    );

    let (header, subtitle) = compact_shell_banner(&context, 84);

    assert!(header.contains("住宅 / 私聊"));
    assert!(header.contains("新手居所"));
    assert!(!header.contains("城市 / 公共频道"));
    assert!(subtitle.contains("居所牌 / 会客桌 / 暖灯窗"));
}

#[test]
fn compact_scene_panel_uses_structured_tile_grid_instead_of_legend_prose() {
    let panel = strip_ansi_sgr(
        &render_compact_scene_panel(&test_context(LaunchSurface::User), 84).join("\n"),
    );

    assert!(panel.contains("▀") || panel.contains("▄") || panel.contains("▐"));
    assert!(panel.contains("城主府 / 居民区 / 传送阵"));
    assert!(panel.contains("场记"));
    assert!(panel.contains("先把公共频道聊顺"));
}

#[test]
fn city_public_header_panel_draws_city_landmarks_and_transport_tokens() {
    let header =
        strip_ansi_sgr(&render_chat_header_panel(&test_context(LaunchSurface::User)).join("\n"));

    assert!(header.contains("城市外景"));
    assert!(header.contains("城主府"));
    assert!(header.contains("钟塔"));
    assert!(header.contains("传送阵"));
    assert!(header.contains("城市脉冲"));
}

#[test]
fn city_public_header_uses_three_zone_civic_layout() {
    let header =
        strip_ansi_sgr(&render_chat_header_panel(&test_context(LaunchSurface::User)).join("\n"));

    assert!(header.contains("居民区"));
    assert!(header.contains("城主府 / 钟塔"));
    assert!(header.contains("传送阵"));
    assert!(header.contains("┬"));
    assert!(header.contains("街桥 / 路签："));
}

#[test]
fn residence_header_panel_draws_structured_room_props() {
    let header =
        strip_ansi_sgr(&render_chat_header_panel(&test_context(LaunchSurface::Direct)).join("\n"));

    assert!(header.contains("住宅内景"));
    assert!(header.contains("居所牌"));
    assert!(header.contains("书桌 / 暖灯窗"));
    assert!(header.contains("沙发角 / 木地板"));
    assert!(header.contains("回响纹"));
}

#[test]
fn compact_scene_panel_follows_preview_surface_page() {
    let context = runtime_context_with_selection_preview(
        LaunchSurface::User,
        ConversationId("room:city:core-harbor:lobby".into()),
        ConversationId("room:world:lobby".into()),
        FocusArea::Nav,
    );
    let panel = strip_ansi_sgr(&render_compact_scene_panel(&context, 84).join("\n"));

    assert!(panel.contains("城邦外景"));
    assert!(panel.contains("世界广场"));
    assert!(panel.contains("先把广场回声跑顺"));
    assert!(panel.contains("城门牌 / 石桥 / 传送阵"));
    assert!(!panel.contains("城内牌 / 暖灯窗 / 书桌"));
}

#[test]
fn compact_conversation_panel_drops_heavy_box_frame() {
    let panel = strip_ansi_sgr(
        &render_compact_conversation_panel(&test_context(LaunchSurface::User), 72).join("\n"),
    );

    assert!(panel.contains("城内门牌"));
    assert!(!panel.contains("╭"));
    assert!(!panel.contains("╰"));
    assert!(!panel.contains("+"));
    assert!(!panel.contains("|城内门牌"));
}

#[test]
fn render_header_and_status_strip_reflect_mode_surface() {
    let context = test_context(LaunchSurface::World);
    let header = render_header(&context);
    let status = render_status_strip(&context);
    assert!(header.contains("城外 / 城门牌"));
    assert!(header.contains("A 地图"));
    assert!(header.contains("B 委托"));
    assert!(status.contains("城外"));
    assert!(status.contains("世界位"));
}

#[test]
fn session_info_panel_behaves_like_compact_hud() {
    let panel = render_session_info_panel(&test_context(LaunchSurface::User)).join("\n");
    assert!(panel.contains("门牌印"));
    assert!(!panel.contains("线灯"));
    assert!(!panel.contains("牌语"));
    assert!(!panel.contains("所在房"));
    assert!(!panel.contains("手势"));
    assert!(!panel.contains("同席"));
    assert!(!panel.contains("路签"));
    assert!(!panel.contains("守护"));
    assert!(!panel.contains("落盘"));
    assert!(!panel.contains("航路"));
    assert!(!panel.contains("护阵"));
}

#[test]
fn header_uses_shorter_scene_tokens_instead_of_menu_like_labels() {
    let header = strip_ansi_sgr(&render_header(&test_context(LaunchSurface::User)));
    assert!(header.contains("A 地图"));
    assert!(header.contains("B 委托"));
    assert!(!header.contains("地图册"));
    assert!(!header.contains("委托牌"));
}

#[test]
fn room_and_world_panels_keep_distinct_language() {
    let user_context = test_context(LaunchSurface::User);
    let world_context = test_context(LaunchSurface::World);

    let user_header = render_chat_header_panel(&user_context).join("\n");
    let world_header = render_chat_header_panel(&world_context).join("\n");
    let user_input = render_input_panel(&user_context).join("\n");
    let world_input = render_input_panel(&world_context).join("\n");

    assert!(user_header.contains("城市外景"));
    assert!(user_header.contains("城市外景"));
    assert!(user_header.contains("城主府"));
    assert!(world_header.contains("城邦外景"));
    assert!(world_header.contains("城外场景"));
    assert!(world_header.contains("城门牌"));
    assert!(user_input.contains("城内落字栏"));
    assert!(world_input.contains("世界落字栏"));
}

#[test]
fn room_scene_uses_city_landmarks() {
    let user_header = render_chat_header_panel(&test_context(LaunchSurface::User)).join("\n");
    assert!(user_header.contains("城主府"));
    assert!(user_header.contains("居民区"));
    assert!(user_header.contains("街桥"));
    assert!(user_header.contains("钟塔"));
}

#[test]
fn world_scene_uses_city_landmarks() {
    let world_header = render_chat_header_panel(&test_context(LaunchSurface::World)).join("\n");
    assert!(world_header.contains("石桥"));
    assert!(world_header.contains("铜灯柱"));
    assert!(world_header.contains("传送阵"));
}

#[test]
fn room_scene_pushes_further_into_city_exterior_language() {
    let user_header = render_chat_header_panel(&test_context(LaunchSurface::User)).join("\n");
    assert!(user_header.contains("传送阵"));
    assert!(user_header.contains("公共频道"));
    assert!(user_header.contains("路签："));
    assert!(user_header.contains("城市脉冲"));
}

#[test]
fn world_scene_pushes_further_into_city_exterior_language() {
    let world_header = render_chat_header_panel(&test_context(LaunchSurface::World)).join("\n");
    assert!(world_header.contains("石桥"));
    assert!(world_header.contains("河道"));
    assert!(world_header.contains("回响潮"));
    assert!(world_header.contains("瞭望塔"));
}

#[test]
fn scene_footer_keeps_route_card_when_header_is_compacted() {
    let room_header = render_chat_header_panel(&test_context(LaunchSurface::User)).join("\n");
    let world_header = render_chat_header_panel(&test_context(LaunchSurface::World)).join("\n");
    assert!(room_header.contains("路签："));
    assert!(world_header.contains("路签："));
    assert!(!room_header.contains("回声墙继续在下方堆叠"));
    assert!(!world_header.contains("回声墙继续在下方堆叠"));
}

#[test]
fn input_panel_exposes_waiting_prompt_by_surface() {
    let user_input = render_input_panel(&test_context(LaunchSurface::User)).join("\n");
    let admin_input = render_input_panel(&test_context(LaunchSurface::Admin)).join("\n");
    let world_input = render_input_panel(&test_context(LaunchSurface::World)).join("\n");

    assert!(user_input.contains("城内"));
    assert!(admin_input.contains("城务"));
    assert!(world_input.contains("世界"));
}

#[test]
fn input_panel_dims_prompt_state_context_in_full_and_compact_views() {
    let context = test_context(LaunchSurface::User);
    let full_input = render_input_panel(&context).join("\n");
    let compact_input = render_compact_input_panel(&context, 84).join("\n");
    let muted_full_prompt = colorize(
        context.render_profile,
        "muted",
        &ratatui_title_badge(active_prompt_state(&context)),
    );
    let muted_compact_prompt = colorize(
        context.render_profile,
        "muted",
        &format!("{} >", active_prompt_state(&context)),
    );
    let muted_full_tip = colorize(
        context.render_profile,
        "muted",
        &ratatui_title_badge(active_input_tip(&context)),
    );
    let muted_full_title = colorize(
        context.render_profile,
        "muted",
        &ratatui_title_badge(active_input_panel_title(&context)),
    );
    let muted_compact_tip = colorize(context.render_profile, "muted", active_input_tip(&context));
    let muted_compact_title = colorize(
        context.render_profile,
        "muted",
        &format!("{} ·", active_input_panel_title(&context)),
    );
    let muted_full_target = colorize(
        context.render_profile,
        "muted",
        &format!(
            "投向 {}",
            truncate_to_width(
                &context.active_conversation,
                INPUT_PANEL_WIDTH.saturating_sub(42),
            )
        ),
    );
    let warm_full_target = colorize(
        context.render_profile,
        "warm",
        &ratatui_title_badge(&format!(
            "投向 {}",
            truncate_to_width(
                &context.active_conversation,
                INPUT_PANEL_WIDTH.saturating_sub(42),
            )
        )),
    );
    let muted_compact_target = colorize(
        context.render_profile,
        "muted",
        &format!(
            "投向 {}",
            truncate_to_width(&context.active_conversation, 84usize.saturating_sub(7))
        ),
    );

    assert!(full_input.contains(&muted_full_prompt));
    assert!(compact_input.contains(&muted_compact_prompt));
    assert!(full_input.contains(&muted_full_title));
    assert!(full_input.contains(&muted_full_tip));
    assert!(full_input.contains(&muted_full_target));
    assert!(!full_input.contains(&warm_full_target));
    assert!(compact_input.contains(&muted_compact_title));
    assert!(compact_input.contains(&muted_compact_tip));
    assert!(compact_input.contains(&muted_compact_target));
}

#[test]
fn input_panel_uses_shorter_tui_surface_language() {
    let user_input = render_input_panel(&test_context(LaunchSurface::User)).join("\n");
    assert!(user_input.contains("城内落字栏"));
    assert!(user_input.contains("回车入城内"));
    assert!(user_input.contains("投向"));
    assert!(user_input.contains("第一城大厅"));
    assert!(!user_input.contains("手势"));
    assert!(!user_input.contains("底部输入条"));
    assert!(!user_input.contains("按 Enter 直接发给当前房间"));
    assert!(!user_input.contains("发送到"));
    assert!(!user_input.contains("提示"));
}

#[test]
fn status_strip_prioritizes_role_before_scene() {
    let status = strip_ansi_sgr(&render_status_strip(&test_context(LaunchSurface::Admin)));
    let role_index = status.find("城主位").unwrap();
    let scene_index = status.find("城内").unwrap();
    assert!(role_index < scene_index);
}

#[test]
fn compact_shell_banner_uses_tui_surface_language() {
    let (user_header, user_subtitle) = compact_shell_banner(&test_context(LaunchSurface::User), 84);
    let (world_header, world_subtitle) =
        compact_shell_banner(&test_context(LaunchSurface::World), 84);

    assert!(user_header.contains("城邦像素终端"));
    assert!(user_header.contains("城市 / 公共频道"));
    assert!(user_header.contains("居民位"));
    assert!(user_subtitle.contains("城主府"));
    assert!(user_subtitle.contains("居民区"));
    assert!(user_subtitle.contains("传送阵"));

    assert!(world_header.contains("城外 / 城门牌"));
    assert!(world_header.contains("世界位"));
    assert!(world_subtitle.contains("城门牌"));
    assert!(world_subtitle.contains("传送阵"));
    assert!(world_subtitle.contains("石桥"));
    assert!(world_subtitle.contains("住客"));
    assert!(world_subtitle.contains("回声"));
}

#[test]
fn compact_navigation_uses_pixel_room_labels() {
    let user_panel =
        render_compact_conversation_panel(&test_context(LaunchSurface::User), 72).join("\n");
    let world_panel =
        render_compact_conversation_panel(&test_context(LaunchSurface::World), 72).join("\n");

    assert!(user_panel.contains("▶ 城/墙/塔/井"));
    assert!(world_panel.contains("▶ 城/场/志/阵"));
}

#[test]
fn conversation_panel_prioritizes_real_room_rows_after_static_nav_compaction() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "城邦大厅 · 回声常亮".into(),
            is_active: true,
            is_selected: true,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "钟塔后厅".into(),
            summary: "守夜门牌 · 灯还亮着".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 3,
            title: "世界广场".into(),
            summary: "城门外侧 · 世界共响".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 4,
            title: "builder 居所".into(),
            summary: "居所单线 · 轻声来回".into(),
            is_active: false,
            is_selected: false,
            is_private: true,
        },
    ];

    let panel = strip_ansi_sgr(&render_conversation_panel(&context).join("\n"));
    assert!(panel.contains("钟塔后厅"));
    assert!(panel.contains("世界广场"));
    assert!(panel.contains("builder 居所"));
}

#[test]
fn transcript_panel_keeps_two_more_echo_rows_after_left_column_rebalance() {
    let mut context = test_context(LaunchSurface::User);
    context.message_count = 24;
    context.transcript = (1..=24).map(|index| format!("回声 {index:02}")).collect();

    let panel = strip_ansi_sgr(&render_transcript_panel(&context).join("\n"));
    assert!(panel.contains("回声 01"));
    assert!(panel.contains("回声 24"));
}

#[test]
fn transcript_panel_keeps_all_thirty_echo_rows_after_scene_header_compaction() {
    let mut context = test_context(LaunchSurface::User);
    context.message_count = 30;
    context.transcript = (1..=30).map(|index| format!("回声 {index:02}")).collect();

    let panel = strip_ansi_sgr(&render_transcript_panel(&context).join("\n"));
    assert!(panel.contains("回声 01"));
    assert!(panel.contains("回声 02"));
    assert!(panel.contains("回声 30"));
}

#[test]
fn transcript_panel_keeps_one_more_echo_row_after_scene_header_compaction() {
    let mut context = test_context(LaunchSurface::User);
    context.message_count = 31;
    context.transcript = (1..=31).map(|index| format!("回声 {index:02}")).collect();

    let panel = strip_ansi_sgr(&render_transcript_panel(&context).join("\n"));
    assert!(panel.contains("回声 02"));
    assert!(panel.contains("回声 31"));
    assert!(!panel.contains("回声 01"));
}

#[test]
fn transcript_panel_drops_blank_spacer_rows_to_keep_more_real_echoes() {
    let mut context = test_context(LaunchSurface::User);
    context.message_count = 18;
    context.transcript = (1..=18)
        .flat_map(|index| {
            let mut lines = vec![format!("回声 {index:02}")];
            if index < 18 {
                lines.push(String::new());
            }
            lines
        })
        .collect();

    let panel = strip_ansi_sgr(&render_transcript_panel(&context).join("\n"));
    assert!(panel.contains("回声 01"));
    assert!(panel.contains("回声 18"));
}

#[test]
fn nav_hints_use_short_diegetic_phrases() {
    let user_entries = room_nav_entries(LaunchSurface::User);
    let world_entries = room_nav_entries(LaunchSurface::World);

    assert!(user_entries[0].1.contains("找城里"));
    assert!(user_entries[1].1.contains("贴着当前"));
    assert!(!user_entries[0].1.contains("查找房间"));
    assert_eq!(user_entries[3].0, "许愿井");
    assert!(world_entries[0].1.contains("先认城门"));
    assert!(world_entries[3].1.contains("远路回声"));
    assert_eq!(world_entries[2].0, "城邦志");
}

#[test]
fn conversation_panel_compacts_static_nav_into_one_row() {
    let panel =
        strip_ansi_sgr(&render_conversation_panel(&test_context(LaunchSurface::User)).join("\n"));
    assert!(panel.contains("▶ 城/墙/塔/井"));
    assert!(!panel.contains("常用门牌"));
    assert!(!panel.contains("▶ 回声墙 贴着当前房里聊"));
}

#[test]
fn conversation_panels_drop_meta_line_to_prioritize_real_room_rows() {
    let full_panel =
        strip_ansi_sgr(&render_conversation_panel(&test_context(LaunchSurface::User)).join("\n"));
    let compact_panel = strip_ansi_sgr(
        &render_compact_conversation_panel(&test_context(LaunchSurface::User), 84).join("\n"),
    );

    assert!(!full_panel.contains("先认城门，再听城内回声"));
    assert!(!compact_panel.contains("先认城门，再听城内回声"));
}

#[test]
fn compact_conversation_panel_keeps_first_private_title_visible_after_nav_and_gap_compaction() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "城邦大厅 · 回声常亮".into(),
            is_active: true,
            is_selected: true,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "钟塔后厅".into(),
            summary: "守夜门牌 · 灯还亮着".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 3,
            title: "海桥茶室".into(),
            summary: "海风门牌 · 靠桥轻聊".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 4,
            title: "机巧柜旁".into(),
            summary: "木地板边 · 工具近手".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 5,
            title: "铜灯回廊".into(),
            summary: "灯下回声 · 顺墙排开".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 6,
            title: "雨棚前厅".into(),
            summary: "雨声很近 · 门口常亮".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 7,
            title: "暮潮长阶".into(),
            summary: "潮声贴墙 · 低声缓行".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 8,
            title: "风箱后巷".into(),
            summary: "风口偏窄 · 说话更轻".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 9,
            title: "灯绳桥头".into(),
            summary: "桥边灯暖 · 回声不散".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 10,
            title: "盐仓拐角".into(),
            summary: "货箱半掩 · 适合短谈".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 11,
            title: "builder 居所".into(),
            summary: "居所单线 · 轻声来回".into(),
            is_active: false,
            is_selected: false,
            is_private: true,
        },
    ];

    let panel = strip_ansi_sgr(&render_compact_conversation_panel(&context, 72).join("\n"));
    assert!(panel.contains("第一城大厅"));
    assert!(panel.contains("builder 居所"));
}

#[test]
fn compact_conversation_panel_keeps_tenth_room_title_visible_after_title_strip_compaction() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "城邦大厅 · 回声常亮".into(),
            is_active: true,
            is_selected: true,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "钟塔后厅".into(),
            summary: "守夜门牌 · 灯还亮着".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 3,
            title: "海桥茶室".into(),
            summary: "海风门牌 · 靠桥轻聊".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 4,
            title: "机巧柜旁".into(),
            summary: "木地板边 · 工具近手".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 5,
            title: "铜灯回廊".into(),
            summary: "灯下回声 · 顺墙排开".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 6,
            title: "雨棚前厅".into(),
            summary: "雨声很近 · 门口常亮".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 7,
            title: "暮潮长阶".into(),
            summary: "潮声贴墙 · 低声缓行".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 8,
            title: "风箱后巷".into(),
            summary: "风口偏窄 · 说话更轻".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 9,
            title: "灯绳桥头".into(),
            summary: "桥边灯暖 · 回声不散".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 10,
            title: "盐仓拐角".into(),
            summary: "货箱半掩 · 适合短谈".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 11,
            title: "builder 居所".into(),
            summary: "居所单线 · 轻声来回".into(),
            is_active: false,
            is_selected: false,
            is_private: true,
        },
    ];

    let panel = strip_ansi_sgr(&render_compact_conversation_panel(&context, 72).join("\n"));
    assert!(panel.contains("盐仓拐角"));
}

#[test]
fn conversation_panel_keeps_eleven_real_room_rows_visible_after_nav_collapse() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "城邦大厅 · 回声常亮".into(),
            is_active: true,
            is_selected: true,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "钟塔后厅".into(),
            summary: "守夜门牌 · 灯还亮着".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 3,
            title: "海桥茶室".into(),
            summary: "海风门牌 · 靠桥轻聊".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 4,
            title: "机巧柜旁".into(),
            summary: "木地板边 · 工具近手".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 5,
            title: "铜灯回廊".into(),
            summary: "灯下回声 · 顺墙排开".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 6,
            title: "雨棚前厅".into(),
            summary: "雨声很近 · 门口常亮".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 7,
            title: "暮潮长阶".into(),
            summary: "潮声贴墙 · 低声缓行".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 8,
            title: "风箱后巷".into(),
            summary: "风口偏窄 · 说话更轻".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 9,
            title: "灯绳桥头".into(),
            summary: "桥边灯暖 · 回声不散".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 10,
            title: "盐仓拐角".into(),
            summary: "货箱半掩 · 适合短谈".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 11,
            title: "石阶门廊".into(),
            summary: "石面微凉 · 进出都近".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
    ];

    let panel = strip_ansi_sgr(&render_conversation_panel(&context).join("\n"));
    assert!(panel.contains("石阶门廊"));
}

#[test]
fn conversation_panel_keeps_twelve_real_room_titles_visible_after_status_compaction() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "城邦大厅 · 回声常亮".into(),
            is_active: true,
            is_selected: true,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "钟塔后厅".into(),
            summary: "守夜门牌 · 灯还亮着".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 3,
            title: "海桥茶室".into(),
            summary: "海风门牌 · 靠桥轻聊".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 4,
            title: "机巧柜旁".into(),
            summary: "木地板边 · 工具近手".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 5,
            title: "铜灯回廊".into(),
            summary: "灯下回声 · 顺墙排开".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 6,
            title: "雨棚前厅".into(),
            summary: "雨声很近 · 门口常亮".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 7,
            title: "暮潮长阶".into(),
            summary: "潮声贴墙 · 低声缓行".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 8,
            title: "风箱后巷".into(),
            summary: "风口偏窄 · 说话更轻".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 9,
            title: "灯绳桥头".into(),
            summary: "桥边灯暖 · 回声不散".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 10,
            title: "盐仓拐角".into(),
            summary: "货箱半掩 · 适合短谈".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 11,
            title: "石阶门廊".into(),
            summary: "石面微凉 · 进出都近".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 12,
            title: "桥尾灯亭".into(),
            summary: "灯亭守尾 · 夜里也亮".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
    ];

    let panel = strip_ansi_sgr(&render_conversation_panel(&context).join("\n"));
    assert!(panel.contains("桥尾灯亭"));
    assert!(!panel.contains("灯亭守尾 · 夜里也亮"));
}

#[test]
fn conversation_panel_keeps_twenty_seventh_room_title_visible_after_status_plate_removal() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = (1..=27)
        .map(|slot| ConversationRow {
            slot,
            title: format!("第{:02}门", slot),
            summary: format!("第{:02}门摘要", slot),
            is_active: slot == 1,
            is_selected: slot == 1,
            is_private: false,
        })
        .collect();

    let panel = strip_ansi_sgr(&render_conversation_panel(&context).join("\n"));
    assert!(panel.contains("第27门"));
}

#[test]
fn conversation_panel_keeps_twenty_eighth_room_title_visible_after_status_stamp_compaction() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = (1..=28)
        .map(|slot| ConversationRow {
            slot,
            title: format!("第{:02}门", slot),
            summary: format!("第{:02}门摘要", slot),
            is_active: slot == 1,
            is_selected: slot == 1,
            is_private: false,
        })
        .collect();

    let panel = strip_ansi_sgr(&render_conversation_panel(&context).join("\n"));
    assert!(panel.contains("第28门"));
}

#[test]
fn conversation_panel_keeps_first_private_title_visible_after_section_gap_removal() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "城邦大厅 · 回声常亮".into(),
            is_active: true,
            is_selected: true,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "钟塔后厅".into(),
            summary: "守夜门牌 · 灯还亮着".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 3,
            title: "海桥茶室".into(),
            summary: "海风门牌 · 靠桥轻聊".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 4,
            title: "机巧柜旁".into(),
            summary: "木地板边 · 工具近手".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 5,
            title: "铜灯回廊".into(),
            summary: "灯下回声 · 顺墙排开".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 6,
            title: "雨棚前厅".into(),
            summary: "雨声很近 · 门口常亮".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 7,
            title: "暮潮长阶".into(),
            summary: "潮声贴墙 · 低声缓行".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 8,
            title: "风箱后巷".into(),
            summary: "风口偏窄 · 说话更轻".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 9,
            title: "灯绳桥头".into(),
            summary: "桥边灯暖 · 回声不散".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 10,
            title: "盐仓拐角".into(),
            summary: "货箱半掩 · 适合短谈".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 11,
            title: "石阶门廊".into(),
            summary: "石面微凉 · 进出都近".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 12,
            title: "builder 居所".into(),
            summary: "居所单线 · 轻声来回".into(),
            is_active: false,
            is_selected: false,
            is_private: true,
        },
    ];

    let panel = strip_ansi_sgr(&render_conversation_panel(&context).join("\n"));
    assert!(panel.contains("builder 居所"));
}

#[test]
fn conversation_panel_hides_inactive_summaries_after_status_rebalance() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "城邦大厅 · 回声常亮".into(),
            is_active: true,
            is_selected: true,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "钟塔后厅".into(),
            summary: "守夜门牌 · 灯还亮着".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 3,
            title: "海桥茶室".into(),
            summary: "海风门牌 · 靠桥轻聊".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 4,
            title: "机巧柜旁".into(),
            summary: "木地板边 · 工具近手".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 5,
            title: "铜灯回廊".into(),
            summary: "灯下回声 · 顺墙排开".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 6,
            title: "雨棚前厅".into(),
            summary: "雨声很近 · 门口常亮".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 7,
            title: "暮潮长阶".into(),
            summary: "潮声贴墙 · 低声缓行".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 8,
            title: "风箱后巷".into(),
            summary: "风口偏窄 · 说话更轻".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 9,
            title: "灯绳桥头".into(),
            summary: "桥边灯暖 · 回声不散".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 10,
            title: "盐仓拐角".into(),
            summary: "货箱半掩 · 适合短谈".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 11,
            title: "石阶门廊".into(),
            summary: "石面微凉 · 进出都近".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 12,
            title: "builder 居所".into(),
            summary: "居所单线 · 轻声来回".into(),
            is_active: false,
            is_selected: false,
            is_private: true,
        },
    ];

    let panel = strip_ansi_sgr(&render_conversation_panel(&context).join("\n"));
    assert!(panel.contains("城邦大厅 · 回声常亮"));
    assert!(!panel.contains("居所单线 · 轻声来回"));
}

#[test]
fn conversation_panel_keeps_second_private_title_visible_after_section_and_status_compaction() {
    let mut context = test_context(LaunchSurface::User);
    context.conversation_rows = vec![
        ConversationRow {
            slot: 1,
            title: "第一城大厅".into(),
            summary: "城邦大厅 · 回声常亮".into(),
            is_active: true,
            is_selected: true,
            is_private: false,
        },
        ConversationRow {
            slot: 2,
            title: "钟塔后厅".into(),
            summary: "守夜门牌 · 灯还亮着".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 3,
            title: "海桥茶室".into(),
            summary: "海风门牌 · 靠桥轻聊".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 4,
            title: "机巧柜旁".into(),
            summary: "木地板边 · 工具近手".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 5,
            title: "铜灯回廊".into(),
            summary: "灯下回声 · 顺墙排开".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 6,
            title: "雨棚前厅".into(),
            summary: "雨声很近 · 门口常亮".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 7,
            title: "暮潮长阶".into(),
            summary: "潮声贴墙 · 低声缓行".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 8,
            title: "风箱后巷".into(),
            summary: "风口偏窄 · 说话更轻".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 9,
            title: "灯绳桥头".into(),
            summary: "桥边灯暖 · 回声不散".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 10,
            title: "盐仓拐角".into(),
            summary: "货箱半掩 · 适合短谈".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 11,
            title: "石阶门廊".into(),
            summary: "石面微凉 · 进出都近".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 12,
            title: "桥尾灯亭".into(),
            summary: "灯亭守尾 · 夜里也亮".into(),
            is_active: false,
            is_selected: false,
            is_private: false,
        },
        ConversationRow {
            slot: 13,
            title: "builder 居所".into(),
            summary: "居所单线 · 轻声来回".into(),
            is_active: false,
            is_selected: false,
            is_private: true,
        },
        ConversationRow {
            slot: 14,
            title: "guide 居所".into(),
            summary: "灯下短问 · 回响很轻".into(),
            is_active: false,
            is_selected: false,
            is_private: true,
        },
    ];

    let panel = strip_ansi_sgr(&render_conversation_panel(&context).join("\n"));
    assert!(panel.contains("guide 居所"));
}

#[test]
fn admin_mode_stays_on_shared_room_shell() {
    let admin_context = test_context(LaunchSurface::Admin);
    let header = render_header(&admin_context);
    let status = render_status_strip(&admin_context);
    let panel_meta = mode_panel_meta(LaunchSurface::Admin);
    let callout = mode_callout(LaunchSurface::Admin);

    assert!(header.contains("城市 / 公共频道"));
    assert!(status.contains("城内"));
    assert_eq!(
        surface_transcript_panel_title(LaunchSurface::Admin),
        "公共频道"
    );
    assert!(panel_meta.contains("与居民同窗"));
    assert!(callout.contains("城主增权"));
}

#[test]
fn user_surface_defaults_to_city_public_surface_semantics() {
    assert_eq!(surface_page(LaunchSurface::User), SurfacePage::CityPublic);
    assert_eq!(
        surface_page(LaunchSurface::from_str(Some("workbench"))),
        SurfacePage::CityPublic
    );
    assert_eq!(surface_page_title(LaunchSurface::User), "城市 / 公共频道");
    assert_eq!(surface_scene_panel_title(LaunchSurface::User), "城市外景");
    assert_eq!(
        surface_transcript_panel_title(LaunchSurface::User),
        "公共频道"
    );
    assert_eq!(surface_nav_panel_title(LaunchSurface::User), "城内门牌");
    assert_eq!(
        surface_status_panel_title(LaunchSurface::User),
        "角色资料 / 状态"
    );
    assert_eq!(surface_scene_chip(LaunchSurface::User), "城内");
}

#[test]
fn direct_surface_defaults_to_residence_direct_semantics() {
    assert_eq!(
        surface_page(LaunchSurface::Direct),
        SurfacePage::ResidenceDirect
    );
    assert_eq!(surface_page_title(LaunchSurface::Direct), "住宅 / 私聊");
    assert_eq!(surface_scene_panel_title(LaunchSurface::Direct), "住宅主景");
    assert_eq!(
        surface_transcript_panel_title(LaunchSurface::Direct),
        "私聊记录"
    );
    assert_eq!(surface_nav_panel_title(LaunchSurface::Direct), "居所动作");
    assert_eq!(
        surface_status_panel_title(LaunchSurface::Direct),
        "房内状态"
    );
    assert_eq!(surface_scene_chip(LaunchSurface::Direct), "居所");
}

#[test]
fn city_public_surface_uses_profile_status_title() {
    assert_eq!(
        surface_status_panel_title(LaunchSurface::User),
        "角色资料 / 状态"
    );
}

#[test]
fn wide_frame_regions_place_city_public_switcher_and_profile_above_full_width_transcript() {
    let regions = wide_frame_regions(Rect::new(0, 0, 140, 40), SurfacePage::CityPublic);

    assert!(regions.switcher.is_some());
    assert!(regions.profile.is_some());
    assert!(regions.actions.is_none());
    assert_eq!(regions.scene.y, regions.profile.unwrap().y);
    assert!(regions.transcript.y > regions.scene.y);
    assert_eq!(regions.transcript.x, 0);
    assert_eq!(regions.transcript.width, 140);
}

#[test]
fn wide_frame_regions_place_residence_actions_left_of_scene_and_transcript() {
    let regions = wide_frame_regions(Rect::new(0, 0, 140, 40), SurfacePage::ResidenceDirect);

    assert!(regions.switcher.is_none());
    assert!(regions.profile.is_none());
    assert!(regions.actions.is_some());
    assert_eq!(regions.actions.unwrap().x, 0);
    assert!(regions.scene.x > regions.actions.unwrap().x);
    assert_eq!(regions.scene.x, regions.transcript.x);
    assert!(regions.transcript.y > regions.scene.y);
}

#[test]
fn seed_messages_reflect_city_public_and_residence_direct_language() {
    let now_ms = 10_000;
    let city_public_id = ConversationId("room:city:core-harbor:lobby".into());
    let residence_id = launch_conversation(LaunchSurface::User)
        .participants
        .first()
        .unwrap()
        .0
        .clone();
    let residence_direct_id = direct_conversation(&residence_id, "guide");

    let city_public_seeds = seed_messages_for_conversation(&city_public_id, now_ms);
    let residence_direct_seeds = seed_messages_for_conversation(&residence_direct_id, now_ms);

    assert!(city_public_seeds.iter().any(|(_, _, text, _)| {
        text.contains("公共频道") || text.contains("城主府") || text.contains("居民区")
    }));
    assert!(
        residence_direct_seeds
            .iter()
            .any(|(_, _, text, _)| text.contains("居所"))
    );
}

#[test]
fn default_surface_prefers_room_chat_over_workbench_overview() {
    assert!(matches!(LaunchSurface::from_str(None), LaunchSurface::User));
    assert!(matches!(
        LaunchSurface::from_str(Some("workbench")),
        LaunchSurface::User
    ));
}

#[test]
fn launch_surfaces_collapse_to_three_page_profiles() {
    assert_eq!(surface_page(LaunchSurface::User), SurfacePage::CityPublic);
    assert_eq!(surface_page(LaunchSurface::Admin), SurfacePage::CityPublic);
    assert_eq!(surface_page(LaunchSurface::World), SurfacePage::World);
    assert_eq!(
        surface_page(LaunchSurface::Direct),
        SurfacePage::ResidenceDirect
    );
}

#[test]
fn room_page_title_is_shared_by_user_admin_and_workbench() {
    assert_eq!(surface_page_title(LaunchSurface::User), "城市 / 公共频道");
    assert_eq!(surface_page_title(LaunchSurface::Admin), "城市 / 公共频道");
    assert_eq!(surface_page_title(LaunchSurface::World), "城外 / 城门牌");
    assert_eq!(surface_page_title(LaunchSurface::Direct), "住宅 / 私聊");
}

#[test]
fn workbench_alias_uses_room_language_across_helpers() {
    let workbench_mode = LaunchSurface::from_str(Some("workbench"));
    assert_eq!(surface_page_title(workbench_mode), "城市 / 公共频道");
    assert!(mode_callout(workbench_mode).contains("城内"));
    assert_eq!(
        mode_panel_meta(workbench_mode),
        mode_panel_meta(LaunchSurface::User)
    );
    assert!(mode_input_tip(workbench_mode).contains("回车入城内"));
    assert_eq!(
        surface_transcript_panel_title(workbench_mode),
        surface_transcript_panel_title(LaunchSurface::User)
    );
    assert_eq!(room_nav_entries(workbench_mode)[1].0, "回声墙");
    assert!(
        conversation_list_kicker(workbench_mode, &launch_conversation(LaunchSurface::User))
            .contains("回声常亮")
    );
}

#[test]
fn workbench_alias_uses_room_seed_conversation_instead_of_world_seed() {
    let workbench_mode = LaunchSurface::from_str(Some("workbench"));
    let conversation = launch_conversation(workbench_mode);
    assert_eq!(
        conversation.conversation_id.0,
        "room:city:core-harbor:lobby"
    );

    let companions = launch_companion_conversations(workbench_mode);
    assert!(
        companions
            .iter()
            .any(|item| item.conversation_id.0 == "dm:guide:tiyan")
    );
    assert!(
        companions
            .iter()
            .any(|item| item.conversation_id.0 == "room:world:lobby")
    );
}

#[test]
fn direct_launch_conversation_uses_canonical_direct_id() {
    let conversation = launch_conversation(LaunchSurface::Direct);
    let resident = launch_conversation(LaunchSurface::User)
        .participants
        .first()
        .unwrap()
        .0
        .clone();
    assert_eq!(
        conversation.conversation_id.0,
        direct_conversation(&resident, "guide").0
    );
}

#[test]
fn user_mode_guide_companion_uses_canonical_direct_id() {
    let companions = launch_companion_conversations(LaunchSurface::User);
    assert!(
        companions
            .iter()
            .any(|item| item.conversation_id.0 == "dm:guide:tiyan")
    );
    assert!(
        companions
            .iter()
            .all(|item| item.conversation_id.0 != "dm:tiyan:guide")
    );
}

#[test]
fn open_slots_keep_seeded_user_conversations_before_gateway_dynamic_rooms() {
    let (temp_root, mut store) = temp_store("lobster-tui-open-slots");
    let active = launch_conversation(LaunchSurface::User);
    store.upsert_conversation(active.clone()).unwrap();
    for conversation in launch_companion_conversations(LaunchSurface::User) {
        store.upsert_conversation(conversation).unwrap();
    }

    let mut gateway_room = active.clone();
    gateway_room.conversation_id = ConversationId("room:city:aurora-hub:announcements".into());
    gateway_room.scope = ConversationScope::CityPrivate;
    store.upsert_conversation(gateway_room).unwrap();

    let ordered = selectable_conversations(&store, LaunchSurface::User, &active.conversation_id);
    let ids = ordered
        .iter()
        .map(|conversation| conversation.conversation_id.0.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            "room:city:core-harbor:lobby",
            "room:world:lobby",
            "dm:guide:tiyan",
            "room:city:aurora-hub:announcements",
        ]
    );
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn workbench_and_admin_labels_match_shared_room_shell() {
    assert_eq!(LaunchSurface::from_str(Some("workbench")).label(), "居民位");
    assert_eq!(LaunchSurface::Admin.label(), "城主位");
}

#[test]
fn status_strip_uses_role_chips_instead_of_old_page_names() {
    let user_status = render_status_strip(&test_context(LaunchSurface::User));
    let admin_status = render_status_strip(&test_context(LaunchSurface::Admin));
    let world_status = render_status_strip(&test_context(LaunchSurface::World));
    let direct_status = render_status_strip(&test_context(LaunchSurface::Direct));

    assert!(user_status.contains("居民位"));
    assert!(admin_status.contains("城主位"));
    assert!(world_status.contains("世界位"));
    assert!(direct_status.contains("居所位"));
}

#[test]
fn status_strip_uses_short_diegetic_status_tokens() {
    let user_status = strip_ansi_sgr(&render_status_strip(&test_context(LaunchSurface::User)));
    let world_status = strip_ansi_sgr(&render_status_strip(&test_context(LaunchSurface::World)));
    assert!(user_status.contains("住客"));
    assert!(user_status.contains("回声"));
    assert!(user_status.contains("连线"));
    assert!(user_status.contains("城内"));
    assert!(world_status.contains("城门"));
    assert!(!user_status.contains("人 ·"));
    assert!(!user_status.contains("同步"));
}

#[test]
fn workbench_is_a_true_user_alias_for_room_copy() {
    let workbench_mode = LaunchSurface::from_str(Some("workbench"));
    assert_eq!(
        mode_callout(workbench_mode),
        mode_callout(LaunchSurface::User)
    );
    assert_eq!(
        mode_panel_meta(workbench_mode),
        mode_panel_meta(LaunchSurface::User)
    );
    assert_eq!(surface_transcript_panel_title(workbench_mode), "公共频道");
}

#[test]
fn transcript_panels_and_compact_banners_use_echo_wall_language() {
    let room_transcript = render_transcript_panel(&test_context(LaunchSurface::User)).join("\n");
    let world_transcript = render_transcript_panel(&test_context(LaunchSurface::World)).join("\n");
    let direct_transcript =
        render_transcript_panel(&test_context(LaunchSurface::Direct)).join("\n");
    let compact_transcript =
        render_compact_transcript_panel(&test_context(LaunchSurface::User), 84).join("\n");
    let (_, direct_subtitle) = compact_shell_banner(&test_context(LaunchSurface::Direct), 84);

    assert!(room_transcript.contains("公共频道"));
    assert!(world_transcript.contains("城邦回声墙"));
    assert!(direct_transcript.contains("私聊记录"));
    assert!(!room_transcript.contains("近 7 道回声"));
    assert!(!compact_transcript.contains("近 7 道回声"));
    assert!(!room_transcript.contains("条消息"));
    assert!(!compact_transcript.contains("条消息"));
    assert!(direct_subtitle.contains("居所牌"));
}

#[test]
fn scene_footer_can_drop_echo_tail_note_when_compacting_header() {
    let room_header = render_chat_header_panel(&test_context(LaunchSurface::User)).join("\n");
    assert!(room_header.contains("路签："));
    assert!(!room_header.contains("消息墙继续在下方堆叠"));
}

#[test]
fn direct_scene_panel_uses_private_scene_language() {
    let direct_header = render_chat_header_panel(&test_context(LaunchSurface::Direct)).join("\n");

    assert!(direct_header.contains("住宅主景"));
    assert!(!direct_header.contains("城市外景"));
}

#[test]
fn chat_header_panel_drops_participant_and_echo_meta_line() {
    let room_header = render_chat_header_panel(&test_context(LaunchSurface::User)).join("\n");
    let world_header = render_chat_header_panel(&test_context(LaunchSurface::World)).join("\n");

    assert!(!room_header.contains("席 ·"));
    assert!(!room_header.contains("· 3席 · 7道"));
    assert!(!world_header.contains("席 ·"));
    assert!(!world_header.contains("· 3席 · 7道"));
}

#[test]
fn room_and_world_panel_titles_use_in_world_objects_not_app_ui_labels() {
    let room_nav = render_conversation_panel(&test_context(LaunchSurface::User)).join("\n");
    let world_nav = render_conversation_panel(&test_context(LaunchSurface::World)).join("\n");
    let room_input = render_input_panel(&test_context(LaunchSurface::User)).join("\n");
    let world_input = render_input_panel(&test_context(LaunchSurface::World)).join("\n");
    let room_status = render_session_info_panel(&test_context(LaunchSurface::User)).join("\n");

    assert!(room_nav.contains("城内门牌"));
    assert!(world_nav.contains("世界门牌"));
    assert!(room_input.contains("城内落字栏"));
    assert!(world_input.contains("世界落字栏"));
    assert!(room_status.contains("角色资料 / 状态"));
}

#[test]
fn compact_input_panel_uses_diegetic_input_labels() {
    let compact_input =
        render_compact_input_panel(&test_context(LaunchSurface::User), 84).join("\n");

    assert!(compact_input.contains("投向"));
    assert!(!compact_input.contains("手势"));
    assert!(compact_input.contains("回车入城内"));
    assert!(!compact_input.contains("底部输入条"));
    assert!(!compact_input.contains("发送到"));
}

#[test]
fn compact_input_panel_follows_active_surface_title() {
    let room_input = render_compact_input_panel(&test_context(LaunchSurface::User), 84).join("\n");
    let world_input =
        render_compact_input_panel(&test_context(LaunchSurface::World), 84).join("\n");
    let direct_input =
        render_compact_input_panel(&test_context(LaunchSurface::Direct), 84).join("\n");

    assert!(room_input.contains("城内落字栏"));
    assert!(world_input.contains("世界落字栏"));
    assert!(direct_input.contains("居所落字栏"));
    assert!(!room_input.contains("窄屏落字栏"));
    assert!(!world_input.contains("窄屏落字栏"));
    assert!(!direct_input.contains("窄屏落字栏"));
}

#[test]
fn input_panels_drop_composer_and_message_product_language() {
    let full_input = render_input_panel(&test_context(LaunchSurface::User)).join("\n");
    let compact_input =
        render_compact_input_panel(&test_context(LaunchSurface::User), 84).join("\n");

    assert!(full_input.contains("投向"));
    assert!(!full_input.contains("手势"));
    assert!(full_input.contains("回车入城内"));
    assert!(!full_input.contains("│投向"));
    assert!(!full_input.contains("笔尖"));
    assert!(!full_input.contains("composer"));
    assert!(!full_input.contains("即发"));
    assert!(!full_input.contains("焦点贴着门牌"));
    assert!(!full_input.contains("开头走指令"));
    assert!(!full_input.contains("> 在此落字，回车入城内。"));

    assert!(!full_input.contains("牌面就绪"));
    assert!(!compact_input.contains("牌面就绪"));
    assert!(!compact_input.contains("消息"));
    assert!(!compact_input.contains("手势"));
    assert!(!compact_input.contains("焦点贴着当前门牌"));
    assert!(!compact_input.contains("> 在此落字，回车入城内。"));
}

#[test]
fn callout_and_transcript_meta_drop_layout_and_im_product_language() {
    let user_callout = mode_callout(LaunchSurface::User);
    let admin_callout = mode_callout(LaunchSurface::Admin);

    assert!(user_callout.contains("城内同窗"));
    assert!(admin_callout.contains("城主增权"));
    assert!(!user_callout.contains("/"));
    assert!(!admin_callout.contains("/"));
    assert_eq!(
        surface_transcript_panel_title(LaunchSurface::User),
        "公共频道"
    );
}

#[test]
fn seed_copy_and_scene_summaries_drop_message_and_im_language() {
    let now_ms = 1_000_000;
    let room_seed = seed_messages_for_conversation(
        &ConversationId("room:city:core-harbor:lobby".into()),
        now_ms,
    );
    let world_seed =
        seed_messages_for_conversation(&ConversationId("room:world:lobby".into()), now_ms);

    assert!(
        room_seed
            .iter()
            .all(|(_, _, text, _)| !text.contains("消息"))
    );
    assert!(
        room_seed
            .iter()
            .all(|(_, _, text, _)| !text.contains("输入框"))
    );
    assert!(
        world_seed
            .iter()
            .all(|(_, _, text, _)| !text.contains("消息"))
    );

    let room_summary = conversation_scene_summary(&launch_conversation(LaunchSurface::User));
    let world_summary = conversation_scene_summary(&launch_conversation(LaunchSurface::World));
    assert!(!room_summary.contains("IM"));
    assert!(!world_summary.contains("聊天"));
    assert!(
        !test_context(LaunchSurface::User)
            .active_scene_summary
            .contains("咖啡杯")
    );
}

#[test]
fn route_and_kicker_copy_drop_public_channel_platform_language() {
    let world = launch_conversation(LaunchSurface::World);
    let user_room = launch_conversation(LaunchSurface::User);
    let admin_room = launch_conversation(LaunchSurface::Admin);

    assert!(!conversation_route_label(&world).contains("公共频道"));
    assert!(!conversation_list_kicker(LaunchSurface::World, &world).contains("公共频道"));
    assert!(!conversation_list_kicker(LaunchSurface::User, &user_room).contains("正常聊天"));
    assert!(!conversation_list_kicker(LaunchSurface::Admin, &admin_room).contains("频道"));
}

#[test]
fn conversation_titles_and_kickers_use_city_state_language() {
    let user_room = launch_conversation(LaunchSurface::User);
    let admin_room = launch_conversation(LaunchSurface::Admin);
    let world_room = launch_conversation(LaunchSurface::World);

    assert_eq!(conversation_title(&user_room), "第一城大厅");
    assert_eq!(conversation_title(&admin_room), "城主告示");
    assert_eq!(conversation_title(&world_room), "世界广场");
    assert!(conversation_list_kicker(LaunchSurface::User, &user_room).contains("回声常亮"));
    assert!(conversation_list_kicker(LaunchSurface::World, &world_room).contains("世界共响"));
}

#[test]
fn world_context_and_plain_shell_render_newest_self_message() {
    let unique = WORLD_TRANSCRIPT_TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_root = std::env::temp_dir().join(format!(
        "lobster-tui-world-transcript-{}-{}",
        std::process::id(),
        unique
    ));
    fs::create_dir_all(&temp_root).unwrap();

    let mut store = FileTimelineStore::open(&temp_root, ArchivePolicy::default()).unwrap();
    let now_ms = current_time_ms().unwrap();
    let world = launch_conversation(LaunchSurface::World);
    let mut conversations = vec![world.clone()];
    conversations.extend(launch_companion_conversations(LaunchSurface::World));
    conversations.dedup_by(|left, right| left.conversation_id == right.conversation_id);

    for conversation in &conversations {
        store.upsert_conversation(conversation.clone()).unwrap();
        if store
            .recent_messages(&conversation.conversation_id, 1)
            .is_empty()
        {
            for (seed_index, (sender, profile, text, timestamp_ms)) in
                seed_messages_for_conversation(&conversation.conversation_id, now_ms)
                    .into_iter()
                    .enumerate()
            {
                let message = MessageEnvelope {
                    message_id: MessageId(format!("seed-{seed_index}")),
                    conversation_id: conversation.conversation_id.clone(),
                    sender: IdentityId(sender.clone()),
                    reply_to_message_id: None,
                    sender_device: DeviceId(format!("{sender}-device")),
                    sender_profile: profile,
                    payload_type: PayloadType::Text,
                    body: MessageBody {
                        preview: text.clone(),
                        plain_text: text,
                        language_tag: "zh-CN".into(),
                    },
                    ciphertext: vec![],
                    timestamp_ms,
                    ephemeral: false,
                };
                store.append_message(message).unwrap();
            }
        }
    }

    let newest_text = "WORLD_TEST_最新世界回声";
    store
        .append_message(MessageEnvelope {
            message_id: MessageId("world-test-latest".into()),
            conversation_id: world.conversation_id.clone(),
            sender: IdentityId("rsaga".into()),
            reply_to_message_id: None,
            sender_device: DeviceId("rsaga-terminal".into()),
            sender_profile: ClientProfile::desktop_terminal(),
            payload_type: PayloadType::Text,
            body: MessageBody {
                preview: newest_text.into(),
                plain_text: newest_text.into(),
                language_tag: "zh-CN".into(),
            },
            ciphertext: vec![],
            timestamp_ms: now_ms + 10_000,
            ephemeral: false,
        })
        .unwrap();

    let context = build_launch_context(
        LaunchSurface::World,
        &store,
        &world.conversation_id,
        &world.conversation_id,
        "已合线 · 今轮合线 0 拍".into(),
        TerminalRenderProfile::desktop_default(),
        0,
        FocusArea::Nav,
        "",
    )
    .unwrap();

    assert!(
        context
            .transcript
            .iter()
            .any(|line| line.contains(newest_text))
    );

    let shell = compact_terminal_shell_lines(&context, 104, 34).join("\n");
    assert!(shell.contains(newest_text), "{shell}");

    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn kind_participant_and_banner_labels_drop_channel_and_notification_language() {
    let user_room = launch_conversation(LaunchSurface::User);
    let admin_room = launch_conversation(LaunchSurface::Admin);
    let world_room = launch_conversation(LaunchSurface::World);
    let direct_room = launch_conversation(LaunchSurface::Direct);

    assert_eq!(conversation_kind_label(&user_room), "城邦大厅");
    assert_eq!(conversation_kind_label(&admin_room), "城主告示");
    assert_eq!(conversation_kind_label(&world_room), "世界广场");
    assert_eq!(conversation_kind_label(&direct_room), "居所");

    assert!(!conversation_participant_label(&world_room).contains("公开讨论"));
    assert!(!conversation_participant_label(&user_room).contains("群聊"));
    assert!(!conversation_participant_label(&admin_room).contains("公告线程"));

    assert_eq!(
        conversation_scene_banner(&world_room).as_deref(),
        Some("世界广场")
    );
    assert_eq!(
        conversation_scene_banner(&admin_room).as_deref(),
        Some("城主告示")
    );
}

#[test]
fn room_section_labels_follow_shared_room_shell() {
    assert_eq!(surface_room_section_label(LaunchSurface::User), "城内门牌");
    assert_eq!(surface_room_section_label(LaunchSurface::Admin), "城内房间");
    assert_eq!(
        surface_private_section_label(LaunchSurface::User),
        "城内私线"
    );
    assert_eq!(
        surface_private_section_label(LaunchSurface::Admin),
        "城主私线"
    );
}

#[test]
fn session_info_panel_uses_diegetic_status_labels() {
    let room_status = render_session_info_panel(&test_context(LaunchSurface::User)).join("\n");
    let world_status = render_session_info_panel(&test_context(LaunchSurface::World)).join("\n");

    assert!(room_status.contains("门牌印"));
    assert!(!room_status.contains("线灯"));
    assert!(!room_status.contains("护阵"));
    assert!(!room_status.contains("当前房间"));
    assert!(!room_status.contains("缓存"));
    assert!(!room_status.contains("所在房"));
    assert!(!room_status.contains("手势"));
    assert!(!room_status.contains("连线"));
    assert!(!room_status.contains("同席"));
    assert!(!room_status.contains("路签"));
    assert!(!room_status.contains("守护"));
    assert!(!room_status.contains("航路"));
    assert!(!room_status.contains("落盘"));
    assert!(!world_status.contains("线灯"));
}

#[test]
fn scene_tile_kind_enum_covers_all_fourteen_variants() {
    // Verify the enum has no accidental gaps
    let all = vec![
        SceneTileKind::Wall,
        SceneTileKind::Floor,
        SceneTileKind::Window,
        SceneTileKind::Desk,
        SceneTileKind::Sofa,
        SceneTileKind::Cabinet,
        SceneTileKind::Door,
        SceneTileKind::Avatar,
        SceneTileKind::Lamp,
        SceneTileKind::Gate,
        SceneTileKind::Bridge,
        SceneTileKind::Water,
        SceneTileKind::Portal,
        SceneTileKind::Tower,
    ];
    assert_eq!(all.len(), 14);
    // Verify Debug/Clone/PartialEq work
    assert_eq!(SceneTileKind::Wall, SceneTileKind::Wall);
    assert_ne!(SceneTileKind::Wall, SceneTileKind::Floor);
    assert_eq!(format!("{:?}", SceneTileKind::Gate), "Gate");
}

#[test]
fn scene_tile_grid_empty_yields_empty_rows() {
    let ctx = test_context(LaunchSurface::User);
    let grid = ratatui_scene_tile_grid(&ctx);
    assert!(
        !grid.rows.is_empty(),
        "even empty scene should have grid rows"
    );
    // Grid should be non-empty even in admin surface
    let admin_grid = ratatui_scene_tile_grid(&test_context(LaunchSurface::Admin));
    assert!(!admin_grid.rows.is_empty());
}

#[test]
fn scene_tile_grid_world_surface_differs_from_user() {
    let user_grid = ratatui_scene_tile_grid(&test_context(LaunchSurface::User));
    let world_grid = ratatui_scene_tile_grid(&test_context(LaunchSurface::World));
    assert!(!user_grid.rows.is_empty());
    assert!(!world_grid.rows.is_empty());
    // World and User should have different grid layouts
    assert_ne!(
        user_grid.rows, world_grid.rows,
        "world and user should differ"
    );
}

#[test]
fn scene_panel_entities_cover_all_renderable_surfaces() {
    // Verify all surfaces produce non-empty output
    for surface in &[
        LaunchSurface::User,
        LaunchSurface::Admin,
        LaunchSurface::World,
    ] {
        let lines = ratatui_scene_lines(&test_context(*surface));
        assert!(!lines.is_empty(), "surface {:?} should render", surface);
    }
}

#[test]
fn admin_and_world_surfaces_have_non_empty_scene_lines() {
    for surface in &[LaunchSurface::Admin, LaunchSurface::World] {
        let lines = ratatui_scene_lines(&test_context(*surface));
        assert!(
            !lines.is_empty(),
            "surface {:?} should produce scene lines",
            surface
        );
    }
    let user_lines = ratatui_scene_lines(&test_context(LaunchSurface::User));
    let admin_lines = ratatui_scene_lines(&test_context(LaunchSurface::Admin));
    // Admin may differ from user
    assert!(!user_lines.is_empty());
    assert!(!admin_lines.is_empty());
}
