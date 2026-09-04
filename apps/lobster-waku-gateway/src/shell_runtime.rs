use super::*;
use std::{
    collections::{BTreeSet, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

impl GatewayRuntime {
    fn direct_pair_label_from_ids<'a, I>(participants: I) -> Option<String>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let labels = participants
            .into_iter()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        match labels.len() {
            0 => None,
            1 => Some(labels[0].to_string()),
            _ => Some(labels.join(" 与 ")),
        }
    }

    fn direct_pair_label_from_conversation(conversation: &Conversation) -> Option<String> {
        if conversation.kind != ConversationKind::Direct {
            return None;
        }

        Self::direct_pair_label_from_ids(
            conversation.participants.iter().map(|item| item.0.as_str()),
        )
    }

    fn direct_pair_label_from_conversation_id(conversation_id: &ConversationId) -> Option<String> {
        if !conversation_id.0.starts_with("dm:") {
            return None;
        }

        Self::direct_pair_label_from_ids(conversation_id.0.split(':').skip(1))
    }

    pub(crate) fn shell_identity_anchor(&self, conversation: &Conversation) -> Option<String> {
        if conversation.kind != ConversationKind::Direct {
            return None;
        }

        let participants = &conversation.participants;
        let registered_matches = self
            .registrations
            .iter()
            .map(|item| item.resident_id.0.as_str())
            .filter(|resident_id| participants.iter().any(|item| item.0 == *resident_id))
            .collect::<Vec<_>>();
        if registered_matches.len() == 1 {
            return Some(registered_matches[0].to_string());
        }

        let steward_matches = self
            .world_stewards
            .iter()
            .map(|item| item.0.as_str())
            .filter(|resident_id| participants.iter().any(|item| item.0 == *resident_id))
            .collect::<Vec<_>>();
        if steward_matches.len() == 1 {
            return Some(steward_matches[0].to_string());
        }

        if participants.iter().any(|item| item.0 == "rsaga") {
            return Some("rsaga".into());
        }

        None
    }

    pub(crate) fn shell_peer_label(&self, conversation: &Conversation) -> Option<String> {
        if conversation.kind != ConversationKind::Direct {
            return None;
        }

        let anchor = self.shell_identity_anchor(conversation)?;
        conversation
            .participants
            .iter()
            .find(|item| item.0 != anchor)
            .map(|item| item.0.clone())
    }

    pub(crate) fn viewer_peer_label(
        &self,
        conversation: &Conversation,
        viewer: &IdentityId,
    ) -> Option<String> {
        if conversation.kind != ConversationKind::Direct {
            return None;
        }

        if !conversation.participants.iter().any(|item| item == viewer) {
            return None;
        }

        conversation
            .participants
            .iter()
            .find(|item| *item != viewer)
            .map(|item| item.0.clone())
    }

    pub(crate) fn viewer_identity_anchor(
        &self,
        conversation: &Conversation,
        viewer: &IdentityId,
    ) -> Option<String> {
        if conversation.kind != ConversationKind::Direct {
            return None;
        }

        if conversation.participants.iter().any(|item| item == viewer) {
            return Some(viewer.0.clone());
        }

        self.shell_identity_anchor(conversation)
    }

    fn personal_room_visible_to_viewer(
        &self,
        conversation: &Conversation,
        viewer: Option<&IdentityId>,
    ) -> bool {
        let Some(owner) = Self::personal_room_owner(conversation) else {
            return false;
        };
        let Some(viewer) = viewer else {
            return false;
        };

        if owner == viewer {
            return self.resident_has_authenticated_profile(viewer);
        }
        if !self.resident_has_authenticated_profile(viewer) {
            return false;
        }

        match self.personal_room_access_policy(owner) {
            PersonalRoomAccessPolicy::RegisteredAll => true,
            PersonalRoomAccessPolicy::FriendsOnly => self.residents_are_friends(owner, viewer),
        }
    }

    pub(crate) fn personal_room_messages_visible_to_viewer(
        &self,
        conversation: &Conversation,
        viewer: Option<&IdentityId>,
    ) -> bool {
        let Some(owner) = Self::personal_room_owner(conversation) else {
            return true;
        };
        viewer.is_some_and(|viewer| {
            owner == viewer && self.resident_has_authenticated_profile(viewer)
        })
    }

    pub(crate) fn conversation_title_for_viewer(
        &self,
        conversation: &Conversation,
        viewer: Option<&IdentityId>,
    ) -> String {
        match conversation.conversation_id.0.as_str() {
            "room:world:lobby"
            | "room:city:core-harbor:lobby"
            | "room:city:aurora-hub:announcements" => {
                return Self::room_title(&conversation.conversation_id);
            }
            _ => {}
        }
        if let Some(room) = self.public_room_by_conversation_id(&conversation.conversation_id) {
            return room.title.clone();
        }
        match conversation.kind {
            ConversationKind::Direct => {
                let counterpart = viewer
                    .and_then(|viewer| self.viewer_peer_label(conversation, viewer))
                    .or_else(|| self.shell_peer_label(conversation));
                if let Some(counterpart) = counterpart.filter(|value| !value.is_empty()) {
                    format!("正在与 {counterpart} 聊天")
                } else if let Some(pair_label) =
                    Self::direct_pair_label_from_conversation(conversation)
                {
                    format!("{pair_label} 的私聊")
                } else {
                    "私聊会话".into()
                }
            }
            ConversationKind::Room => Self::room_title(&conversation.conversation_id),
        }
    }

    fn shell_action_template(
        action: &str,
        draft_template: &str,
        send_label: &str,
        state_templates: &[(&str, &str)],
    ) -> ShellActionTemplateProjection {
        ShellActionTemplateProjection {
            action: action.into(),
            draft_template: draft_template.into(),
            send_label: send_label.into(),
            state_templates: state_templates
                .iter()
                .map(
                    |(state, draft_template)| ShellActionStateTemplateProjection {
                        state: (*state).into(),
                        draft_template: (*draft_template).into(),
                    },
                )
                .collect(),
        }
    }

    fn shell_action_templates() -> Vec<ShellActionTemplateProjection> {
        vec![
            Self::shell_action_template("续聊", "续聊：", "继续发送", &[]),
            Self::shell_action_template("私聊", "私聊：", "发起私聊", &[]),
            Self::shell_action_template(
                "整理",
                "整理：\n- 目标：\n- 待办：\n- 风险：",
                "提交整理",
                &[("已归档", "整理：\n- 回看：\n- 新补充：\n- 风险：")],
            ),
            Self::shell_action_template(
                "留条",
                "留条：\n- 留给：\n- 内容：\n- 提醒：",
                "留下便条",
                &[("已补充", "留条：\n- 留给：\n- 补充：\n- 下一步：")],
            ),
            Self::shell_action_template(
                "委托",
                "委托：\n- 需求：\n- 截止：\n- 交付：",
                "发出委托",
                &[
                    ("已回执", "委托：\n- 回执：\n- 待确认：\n- 下一步："),
                    ("已完成", "委托：\n- 新需求：\n- 截止：\n- 交付："),
                ],
            ),
            Self::shell_action_template(
                "交易",
                "交易：\n- 标的：\n- 数量：\n- 备注：",
                "记录交易",
                &[
                    ("已确认", "交易：\n- 结果：\n- 待结清：\n- 备注："),
                    ("已结清", "交易：\n- 新标的：\n- 数量：\n- 备注："),
                ],
            ),
        ]
    }

    pub(crate) fn room_scene_banner(conversation: &Conversation) -> Option<String> {
        match conversation.conversation_id.0.as_str() {
            "room:world:lobby" => Some("世界广场".into()),
            "room:city:core-harbor:lobby" => Some("城邦大厅".into()),
            "room:city:aurora-hub:announcements" => Some("城主告示".into()),
            _ => conversation
                .scene
                .as_ref()
                .and_then(|scene| scene.title_banner.clone()),
        }
    }

    pub(crate) fn shell_scene_fields(
        conversation: &Conversation,
    ) -> (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        let scene_banner = Self::room_scene_banner(conversation);
        let scene_summary = Self::summarize_scene(conversation.scene.as_ref());
        let room_variant = conversation
            .scene
            .as_ref()
            .map(|scene| scene.background_preset.clone());
        let room_motif = conversation
            .scene
            .as_ref()
            .map(|scene| scene.ambiance.clone());
        (scene_banner, scene_summary, room_variant, room_motif)
    }

    pub(crate) fn room_participant_label(
        conversation_id: &ConversationId,
        peer_label: Option<&str>,
    ) -> String {
        match conversation_id.0.as_str() {
            "room:world:lobby" => "跨城共响回廊".into(),
            "room:city:core-harbor:lobby" => "核心港回声大厅".into(),
            "room:city:aurora-hub:announcements" => "城务与告示".into(),
            value if value.starts_with("dm:") => match peer_label.filter(|item| !item.is_empty()) {
                Some(peer_label) => format!("你与 {peer_label}"),
                None => Self::direct_pair_label_from_conversation_id(conversation_id)
                    .unwrap_or_else(|| "私聊会话".into()),
            },
            _ => Self::room_title(conversation_id),
        }
    }

    pub(crate) fn room_route_label(conversation_id: &ConversationId) -> String {
        if conversation_id.0.starts_with("dm:") {
            "居所直达".into()
        } else if conversation_id.0.starts_with("room:world:") {
            "跨城共响线".into()
        } else if conversation_id.0.starts_with("room:city:") {
            "城内回响线".into()
        } else {
            "系统状态同步".into()
        }
    }

    pub(crate) fn shell_list_summary(room: &ShellRoomState) -> String {
        let mut parts = vec![room.title.clone()];
        if let Some(member_count) = room.member_count {
            parts.push(format!("{member_count} 人"));
        }
        if !room.messages.is_empty() {
            parts.push(format!("{} 条消息", room.messages.len()));
        }
        parts.join(" · ")
    }

    pub(crate) fn shell_status_line(room: &ShellRoomState) -> String {
        let mut parts = Vec::new();
        if let Some(route_label) = room.route_label.as_ref().filter(|value| !value.is_empty()) {
            parts.push(route_label.clone());
        }
        if !room.meta.is_empty() {
            parts.push(room.meta.clone());
        }
        if parts.is_empty() {
            "等待新消息".into()
        } else {
            parts.join(" · ")
        }
    }

    pub(crate) fn shell_subtitle(room: &ShellRoomState, last_sender: &str) -> String {
        if room.id.starts_with("dm:") {
            let mut parts = Vec::new();
            if let Some(route_label) = room.route_label.as_ref().filter(|value| !value.is_empty()) {
                parts.push(route_label.clone());
            }
            if let Some(participant_label) = room
                .participant_label
                .as_ref()
                .filter(|value| !value.is_empty())
            {
                parts.push(participant_label.clone());
            }
            if !parts.is_empty() {
                return parts.join(" · ");
            }
        }
        format!("最近发言：{last_sender}")
    }

    pub(crate) fn shell_overview_summary(room: &ShellRoomState) -> String {
        if room.id.starts_with("dm:") {
            if let Some(counterpart) = room
                .peer_label
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
            {
                return format!("正在与 {counterpart} 聊天");
            }
            return room.title.clone();
        }
        if room.id.starts_with("room:") {
            return format!(
                "{} · 群聊",
                room.participant_label
                    .clone()
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| room.title.clone())
            );
        }
        room.title.clone()
    }

    pub(crate) fn shell_thread_headline(room: &ShellRoomState) -> String {
        if room.id.starts_with("dm:") {
            if let Some(counterpart) = room
                .peer_label
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
            {
                return format!("正在与 {counterpart} 聊天");
            }
            return room.title.clone();
        }
        if room.id.starts_with("room:") {
            return format!(
                "{} · 群聊",
                room.participant_label
                    .clone()
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| room.title.clone())
            );
        }
        room.title.clone()
    }

    pub(crate) fn shell_chat_status_summary(room: &ShellRoomState) -> String {
        if room.is_frozen {
            return "房间已冻结，仅管理员可发言".into();
        }
        if room.id.starts_with("dm:") {
            return "可直接继续回复".into();
        }
        if room.id.starts_with("room:") {
            return "群聊当前比较安静".into();
        }
        "等待新消息".into()
    }

    pub(crate) fn shell_queue_summary(caretaker: Option<&ShellCaretakerProjection>) -> String {
        let mut items = Vec::new();
        if let Some(caretaker) = caretaker {
            if caretaker.pending_visitors > 0 {
                items.push(format!("{} 条访客提醒待处理", caretaker.pending_visitors));
            }
            if !caretaker.notifications.is_empty() {
                items.push(format!("{} 条巡视提醒待看", caretaker.notifications.len()));
            }
        }
        if items.is_empty() {
            "窗口清爽，可继续巡视或记录".into()
        } else {
            items.join(" · ")
        }
    }

    pub(crate) fn shell_preview_text(room: &ShellRoomState) -> String {
        room.messages
            .last()
            .map(|message| message.text.clone())
            .unwrap_or_else(|| "还没有消息，先发第一句吧。".into())
    }

    pub(crate) fn shell_last_activity_label(room: &ShellRoomState) -> String {
        room.messages
            .last()
            .map(|message| format!("{} · {}", message.sender, message.timestamp_label))
            .unwrap_or_else(|| "暂无消息".into())
    }

    pub(crate) fn shell_activity_time_label(room: &ShellRoomState) -> String {
        room.messages
            .last()
            .map(|message| message.timestamp_label.clone())
            .unwrap_or_else(|| "暂无消息".into())
    }

    pub(crate) fn shell_context_summary(
        room: &ShellRoomState,
        detail_card: Option<&ShellDetailCardProjection>,
    ) -> String {
        room.scene_summary
            .clone()
            .filter(|value| !value.is_empty())
            .or_else(|| {
                detail_card
                    .map(|card| card.summary_copy.clone())
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or_else(|| room.subtitle.clone())
    }

    pub(crate) fn shell_search_terms(
        room: &ShellRoomState,
        caretaker: Option<&ShellCaretakerProjection>,
        detail_card: Option<&ShellDetailCardProjection>,
        workflow: Option<&ShellWorkflowProjection>,
        inline_actions: &[ShellInlineActionProjection],
    ) -> Vec<String> {
        let mut terms = Vec::new();
        let mut seen = BTreeSet::new();
        let mut push = |value: &str| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return;
            }
            if seen.insert(trimmed.to_string()) {
                terms.push(trimmed.to_string());
            }
        };

        for value in [
            room.id.as_str(),
            room.title.as_str(),
            room.subtitle.as_str(),
            room.meta.as_str(),
            room.kind_hint.as_deref().unwrap_or_default(),
            room.self_label.as_deref().unwrap_or_default(),
            room.peer_label.as_deref().unwrap_or_default(),
            room.participant_label.as_deref().unwrap_or_default(),
            room.route_label.as_deref().unwrap_or_default(),
            room.list_summary.as_deref().unwrap_or_default(),
            room.status_line.as_deref().unwrap_or_default(),
            room.thread_headline.as_deref().unwrap_or_default(),
            room.chat_status_summary.as_deref().unwrap_or_default(),
            room.queue_summary.as_deref().unwrap_or_default(),
            room.preview_text.as_deref().unwrap_or_default(),
            room.last_activity_label.as_deref().unwrap_or_default(),
            room.activity_time_label.as_deref().unwrap_or_default(),
            room.overview_summary.as_deref().unwrap_or_default(),
            room.context_summary.as_deref().unwrap_or_default(),
            room.scene_banner.as_deref().unwrap_or_default(),
            room.scene_summary.as_deref().unwrap_or_default(),
            room.room_variant.as_deref().unwrap_or_default(),
            room.room_motif.as_deref().unwrap_or_default(),
        ] {
            push(value);
        }

        if let Some(caretaker) = caretaker {
            for value in [
                caretaker.name.as_str(),
                caretaker.role_label.as_str(),
                caretaker.persona.as_str(),
                caretaker.status.as_str(),
                caretaker.memory.as_str(),
                caretaker.auto_reply.as_str(),
            ] {
                push(value);
            }
            for note in &caretaker.notifications {
                push(note);
            }
            for message in &caretaker.messages {
                push(message.visitor.as_str());
                push(message.note.as_str());
                push(message.urgency.as_str());
            }
            if let Some(patrol) = caretaker.patrol.as_ref() {
                for value in [
                    patrol.mode.as_str(),
                    patrol.last_check.as_str(),
                    patrol.outcome.as_str(),
                ] {
                    push(value);
                }
            }
        }

        if let Some(detail_card) = detail_card {
            for value in [
                detail_card.summary_title.as_str(),
                detail_card.summary_copy.as_str(),
                detail_card.kicker.as_str(),
                detail_card.title.as_str(),
                detail_card.monogram.as_str(),
            ] {
                push(value);
            }
            for item in &detail_card.meta {
                push(item.label.as_str());
                push(item.value.as_str());
            }
            for action in &detail_card.actions {
                push(action);
            }
        }

        if let Some(workflow) = workflow {
            for value in [
                workflow.action.as_str(),
                workflow.state.as_str(),
                workflow.title.as_str(),
                workflow.summary.as_str(),
            ] {
                push(value);
            }
            for step in &workflow.steps {
                push(step.label.as_str());
                push(step.copy.as_str());
                if let Some(advance_label) = step.advance_label.as_deref() {
                    push(advance_label);
                }
            }
        }

        for action in inline_actions {
            for value in [
                action.role.as_str(),
                action.label.as_str(),
                action.action.as_str(),
                action.next_state.as_deref().unwrap_or_default(),
            ] {
                push(value);
            }
        }

        terms
    }

    fn shell_monogram(value: &str, fallback: &str) -> String {
        value
            .chars()
            .next()
            .map(|ch| ch.to_string())
            .unwrap_or_else(|| fallback.into())
    }

    pub(crate) fn shell_caretaker(
        conversation_id: &ConversationId,
        participant_label: &str,
    ) -> Option<ShellCaretakerProjection> {
        if conversation_id.0.starts_with("dm:") {
            return Some(ShellCaretakerProjection {
                name: "旺财".into(),
                role_label: "房间管家".into(),
                persona: "高冷但可靠，会替主人记住来访者和留言。".into(),
                status: "在线值守".into(),
                memory: "今天已经替主人记住 1 条续聊提醒和 1 条房内留言。".into(),
                auto_reply: "主人在房内继续聊天，急事我会先记下来。".into(),
                pending_visitors: 1,
                notifications: vec!["有一条新的续聊提醒待主人确认。".into()],
                messages: vec![ShellCaretakerMessage {
                    visitor: participant_label.to_string(),
                    note: "想确认这轮对话是否继续沿着当前话题推进。".into(),
                    urgency: "普通".into(),
                }],
                patrol: Some(ShellCaretakerPatrol {
                    mode: "房内巡看".into(),
                    last_check: "刚刚".into(),
                    outcome: "房内状态稳定，没有新的异常提醒。".into(),
                }),
            });
        }

        if conversation_id.0.starts_with("room:") {
            return Some(ShellCaretakerProjection {
                name: "巡逻犬".into(),
                role_label: "频道巡视".into(),
                persona: "只在需要时出面，把公共频道的留言和提醒收成可执行摘要。".into(),
                status: "在线巡视".into(),
                memory: "刚整理完 2 条公共提醒和 1 条跨城入口说明。".into(),
                auto_reply: "公共问题先收口成待办，再决定是否继续下沉到房间。".into(),
                pending_visitors: 1,
                notifications: vec!["公共频道入口说明仍待补齐。".into()],
                messages: vec![ShellCaretakerMessage {
                    visitor: participant_label.to_string(),
                    note: "有人提到需要补一张更直白的入口提示卡。".into(),
                    urgency: "高".into(),
                }],
                patrol: Some(ShellCaretakerPatrol {
                    mode: "公共巡逻".into(),
                    last_check: "刚刚".into(),
                    outcome: "当前没有异常消息，已把入口缺口整理成提醒。".into(),
                }),
            });
        }

        None
    }

    pub(crate) fn shell_detail_card(
        conversation_id: &ConversationId,
        room_title: &str,
        self_label: Option<&str>,
        peer_label: Option<&str>,
        participant_label: &str,
        member_count: usize,
        caretaker: Option<&ShellCaretakerProjection>,
    ) -> ShellDetailCardProjection {
        let caretaker_name = caretaker
            .map(|item| item.name.as_str())
            .unwrap_or(room_title);
        let status = caretaker.map(|item| item.status.as_str()).unwrap_or("在线");

        if conversation_id.0.starts_with("dm:") {
            let anchored_direct = self_label.is_some_and(|value| !value.is_empty())
                && peer_label.is_some_and(|value| !value.is_empty());
            let counterpart = peer_label
                .filter(|value| !value.is_empty())
                .unwrap_or(participant_label);
            let mut meta = if anchored_direct {
                vec![
                    ShellLabelValue {
                        label: "住户".into(),
                        value: self_label.unwrap_or("当前住户").into(),
                    },
                    ShellLabelValue {
                        label: "对端".into(),
                        value: counterpart.into(),
                    },
                ]
            } else {
                vec![ShellLabelValue {
                    label: "会话".into(),
                    value: participant_label.into(),
                }]
            };
            meta.extend([
                ShellLabelValue {
                    label: "同住AI".into(),
                    value: caretaker_name.into(),
                },
                ShellLabelValue {
                    label: "当前".into(),
                    value: participant_label.into(),
                },
                ShellLabelValue {
                    label: "状态".into(),
                    value: status.into(),
                },
            ]);
            return ShellDetailCardProjection {
                summary_title: "住宅私聊 / 房内状态".into(),
                summary_copy: if anchored_direct {
                    format!(
                        "{} 会帮你记住与 {} 的留言和提醒，适合续聊、记任务和直接追问。",
                        caretaker_name, counterpart
                    )
                } else {
                    format!(
                        "{} 会帮你记住 {} 的留言和提醒，适合续聊、记任务和直接追问。",
                        caretaker_name, counterpart
                    )
                },
                kicker: "住宅私聊 / 角色卡".into(),
                title: if anchored_direct {
                    format!("{caretaker_name} / 与 {counterpart} 的房内状态")
                } else {
                    format!("{caretaker_name} / {participant_label} 的私聊")
                },
                monogram: "旺".into(),
                meta,
                actions: vec!["续聊".into(), "整理".into(), "留条".into()],
            };
        }

        ShellDetailCardProjection {
            summary_title: "公共频道 / 当前状态".into(),
            summary_copy: format!(
                "{} 会盯住公共提醒和巡视结果，适合看公告、围观和跨城讨论。",
                caretaker_name
            ),
            kicker: "公共频道 / 角色卡".into(),
            title: format!("{caretaker_name} / 频道状态"),
            monogram: "巡".into(),
            meta: vec![
                ShellLabelValue {
                    label: "角色".into(),
                    value: caretaker
                        .map(|item| item.role_label.clone())
                        .unwrap_or_else(|| "公共频道向导".into()),
                },
                ShellLabelValue {
                    label: "称号".into(),
                    value: caretaker_name.into(),
                },
                ShellLabelValue {
                    label: "当前".into(),
                    value: participant_label.into(),
                },
                ShellLabelValue {
                    label: "状态".into(),
                    value: format!("{status} · {member_count} 人"),
                },
            ],
            actions: vec!["私聊".into(), "委托".into(), "交易".into()],
        }
    }

    pub(crate) fn shell_workflow(
        conversation_id: &ConversationId,
    ) -> Option<ShellWorkflowProjection> {
        if conversation_id.0 != "room:world:lobby" {
            return None;
        }

        Some(ShellWorkflowProjection {
            action: "委托".into(),
            state: "待回执".into(),
            title: "委托阶段".into(),
            summary: "当前委托正在等待第一轮回执。".into(),
            steps: vec![
                ShellWorkflowStage {
                    label: "待回执".into(),
                    copy: "先确认需求是否被接住。".into(),
                    advance_label: Some("标记已回执".into()),
                },
                ShellWorkflowStage {
                    label: "已回执".into(),
                    copy: "已经收到第一轮回执，继续补材料。".into(),
                    advance_label: Some("标记已完成".into()),
                },
                ShellWorkflowStage {
                    label: "已完成".into(),
                    copy: "这一轮委托已经收口。".into(),
                    advance_label: None,
                },
            ],
        })
    }

    pub(crate) fn shell_inline_actions(
        workflow: Option<&ShellWorkflowProjection>,
    ) -> Vec<ShellInlineActionProjection> {
        match workflow {
            Some(item) if item.action == "委托" && item.state == "待回执" => vec![
                ShellInlineActionProjection {
                    role: "primary".into(),
                    label: "跟进委托".into(),
                    action: "委托".into(),
                    next_state: None,
                },
                ShellInlineActionProjection {
                    role: "secondary".into(),
                    label: "标记已回执".into(),
                    action: "委托".into(),
                    next_state: Some("已回执".into()),
                },
            ],
            _ => Vec::new(),
        }
    }

    fn shell_scene_stage_projection(
        room: &ShellRoomState,
        detail_card: Option<&ShellDetailCardProjection>,
    ) -> SceneStageProjection {
        let summary = if room.id.starts_with("dm:") {
            detail_card
                .map(|card| card.summary_copy.clone())
                .filter(|value| !value.is_empty())
                .or_else(|| room.scene_summary.clone())
                .unwrap_or_else(|| room.subtitle.clone())
        } else {
            room.scene_summary
                .clone()
                .or_else(|| detail_card.map(|card| card.summary_copy.clone()))
                .unwrap_or_else(|| room.subtitle.clone())
        };
        SceneStageProjection {
            title: room.title.clone(),
            summary,
            badge: room.scene_banner.clone(),
        }
    }

    fn shell_scene_portrait_projection(
        room: &ShellRoomState,
        caretaker: Option<&ShellCaretakerProjection>,
        detail_card: Option<&ShellDetailCardProjection>,
    ) -> ScenePortraitProjection {
        let fallback_name = if room
            .room_variant
            .as_deref()
            .is_some_and(|variant| variant == "home")
        {
            "房"
        } else {
            "巡"
        };
        let title = caretaker
            .map(|item| item.name.clone())
            .or_else(|| detail_card.map(|card| card.title.clone()))
            .unwrap_or_else(|| room.title.clone());
        let summary = caretaker
            .map(|item| {
                [
                    item.role_label.as_str(),
                    item.persona.as_str(),
                    item.memory.as_str(),
                    item.auto_reply.as_str(),
                ]
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(" · ")
            })
            .filter(|value| !value.is_empty())
            .or_else(|| room.scene_summary.clone())
            .or_else(|| detail_card.map(|card| card.summary_copy.clone()))
            .unwrap_or_else(|| room.subtitle.clone());
        ScenePortraitProjection {
            title: title.clone(),
            summary,
            badge: caretaker
                .map(|item| item.role_label.clone())
                .or_else(|| room.scene_banner.clone()),
            status: caretaker
                .map(|item| item.status.clone())
                .or_else(|| {
                    detail_card.and_then(|card| {
                        card.meta
                            .iter()
                            .find(|entry| entry.label == "状态")
                            .map(|entry| entry.value.clone())
                    })
                })
                .or_else(|| Some(room.meta.clone())),
            monogram: Some(
                detail_card
                    .map(|card| card.monogram.clone())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| Self::shell_monogram(&title, fallback_name)),
            ),
        }
    }

    pub(crate) fn shell_image_layer(conversation: &Conversation) -> Option<SceneImageLayer> {
        let scene = conversation.scene.as_ref()?;
        scene.image_layer.clone().or_else(|| {
            Some(Self::scene_image_layer(
                scene.background_preset.clone(),
                scene.owner_editable,
            ))
        })
    }

    pub(crate) fn shell_hotspot_layer(conversation: &Conversation) -> Option<SceneHotspotLayer> {
        let scene = conversation.scene.as_ref()?;
        scene.hotspot_layer.clone().or_else(|| {
            Some(Self::scene_hotspot_layer(
                "gateway-hotspots",
                scene.owner_editable,
                &scene.landmarks,
            ))
        })
    }

    pub(crate) fn relative_label(now_ms: i64, timestamp_ms: i64) -> String {
        let delta_ms = now_ms.saturating_sub(timestamp_ms);
        let delta_minutes = delta_ms / (60 * 1000);
        if delta_minutes < 1 {
            "now".into()
        } else if delta_minutes < 60 {
            format!("{delta_minutes}m ago")
        } else if delta_minutes < 24 * 60 {
            format!("{}h ago", delta_minutes / 60)
        } else {
            format!("{}d ago", delta_minutes / (24 * 60))
        }
    }

    pub(crate) fn shell_recent_messages(
        &self,
        conversation_id: &ConversationId,
        limit: usize,
    ) -> Vec<ShellRoomMessage> {
        self.shell_messages_before(conversation_id, limit, None)
    }

    pub(crate) fn shell_messages_before(
        &self,
        conversation_id: &ConversationId,
        limit: usize,
        before_message_id: Option<&str>,
    ) -> Vec<ShellRoomMessage> {
        let now_ms = Self::now_ms();
        let all = self
            .timeline_store
            .recent_messages(conversation_id, limit * 4);
        let before_idx = before_message_id.and_then(|before_id| {
            all.iter()
                .position(|e| e.envelope.message_id.0 == before_id)
        });
        let eligible: Vec<_> = match before_idx {
            Some(idx) => all.into_iter().skip(idx + 1).take(limit).collect(),
            None => all.into_iter().take(limit).collect(),
        };
        eligible
            .into_iter()
            .map(|entry| {
                let mod_key = format!(
                    "{}:{}",
                    entry.envelope.conversation_id.0, entry.envelope.message_id.0
                );
                let moderation_status = self.message_moderation.get(&mod_key).cloned();
                let (text, attachment) = self.shell_projection_fields(
                    &entry.envelope.body.plain_text,
                    entry.recalled_at_ms.is_some(),
                );
                ShellRoomMessage {
                    message_id: entry.envelope.message_id.0,
                    reply_to_message_id: entry
                        .envelope
                        .reply_to_message_id
                        .map(|message_id| message_id.0),
                    is_recalled: entry.recalled_at_ms.is_some(),
                    recalled_by: entry.recalled_by.map(|identity| identity.0),
                    recalled_at_ms: entry.recalled_at_ms,
                    is_edited: entry.edited_at_ms.is_some(),
                    edited_by: entry.edited_by.map(|identity| identity.0),
                    edited_at_ms: entry.edited_at_ms,
                    sender: entry.envelope.sender.0,
                    timestamp_ms: entry.envelope.timestamp_ms,
                    timestamp_label: Self::relative_label(now_ms, entry.envelope.timestamp_ms),
                    delivery_status: "delivered".into(),
                    text,
                    attachment,
                    moderation_status,
                }
            })
            .collect()
    }

    pub(crate) fn room_title(conversation_id: &ConversationId) -> String {
        match conversation_id.0.as_str() {
            "room:world:lobby" => "世界广场".into(),
            "room:city:core-harbor:lobby" => "第一城大厅".into(),
            "room:city:aurora-hub:announcements" => "城主告示".into(),
            value if value.starts_with("dm:") => {
                Self::direct_pair_label_from_conversation_id(conversation_id)
                    .map(|pair_label| format!("{pair_label} 的私聊"))
                    .unwrap_or_else(|| "私聊会话".into())
            }
            value if value.starts_with("room:city:") => {
                format!("城邦门牌 · {}", value.trim_start_matches("room:"))
            }
            value if value.starts_with("room:world:") => {
                format!("世界门牌 · {}", value.trim_start_matches("room:"))
            }
            value => value.into(),
        }
    }

    pub(crate) fn room_kind_hint(conversation_id: &ConversationId) -> String {
        if conversation_id.0.starts_with("dm:") {
            "居所".into()
        } else if conversation_id.0.starts_with("room:world:") {
            "世界广场".into()
        } else if conversation_id.0.starts_with("room:city:") {
            "城邦大厅".into()
        } else {
            "系统会话".into()
        }
    }

    pub(crate) fn shell_kind(kind: &ConversationKind) -> String {
        match kind {
            ConversationKind::Direct => "direct".into(),
            ConversationKind::Room => "public".into(),
        }
    }

    pub(crate) fn shell_scope(scope: &ConversationScope) -> String {
        match scope {
            ConversationScope::Private => "private".into(),
            ConversationScope::CityPublic => "city_public".into(),
            ConversationScope::CityPrivate => "city_private".into(),
            ConversationScope::CrossCityShared => "cross_city_shared".into(),
        }
    }

    pub(crate) fn shell_visible_conversations_for_viewer(
        &self,
        viewer: Option<&IdentityId>,
    ) -> Vec<Conversation> {
        self.shell_visible_conversations_for_viewer_with_mode(viewer, false)
    }

    fn shell_visible_conversations_for_viewer_with_mode(
        &self,
        viewer: Option<&IdentityId>,
        include_unscoped_private: bool,
    ) -> Vec<Conversation> {
        let mut conversations = self.timeline_store.active_conversations();

        conversations.retain(|conversation| match conversation.kind {
            ConversationKind::Direct => {
                if Self::personal_room_owner(conversation).is_some() {
                    return self.personal_room_visible_to_viewer(conversation, viewer);
                }
                let Some(viewer) = viewer else {
                    // Anonymous shell state may expose public rooms, but a
                    // direct conversation is always resident-scoped. Do not
                    // let the absence of a resident_id turn private threads
                    // into a public projection.
                    return include_unscoped_private;
                };
                conversation
                    .participants
                    .iter()
                    .any(|participant| participant == viewer)
            }
            ConversationKind::Room => {
                let Some(viewer) = viewer else {
                    return true;
                };
                conversation
                    .participants
                    .iter()
                    .any(|participant| participant == viewer)
                    || self.cli_room_visible_to(&conversation.conversation_id, viewer)
            }
        });
        conversations
    }

    #[cfg(test)]
    pub(crate) fn shell_state(&self) -> ShellState {
        self.shell_state_for_viewer_with_mode(None, true)
    }

    pub(crate) fn shell_state_for_viewer(&self, viewer: Option<&IdentityId>) -> ShellState {
        self.shell_state_for_viewer_with_mode(viewer, false)
    }

    fn shell_state_for_viewer_with_mode(
        &self,
        viewer: Option<&IdentityId>,
        include_unscoped_private: bool,
    ) -> ShellState {
        let mut rooms = self
            .shell_visible_conversations_for_viewer_with_mode(viewer, include_unscoped_private)
            .into_iter()
            .map(|conversation| {
                let messages_visible =
                    self.personal_room_messages_visible_to_viewer(&conversation, viewer);
                let last_sender = if messages_visible {
                    self.timeline_store
                        .recent_messages(&conversation.conversation_id, 1)
                        .last()
                        .map(|entry| entry.envelope.sender.0.clone())
                        .unwrap_or_else(|| "system".into())
                } else {
                    "system".into()
                };
                let room_messages = if messages_visible {
                    self.shell_recent_messages(&conversation.conversation_id, 32)
                } else {
                    Vec::new()
                };
                let last_count = room_messages.len();
                let scene_summary = Self::summarize_scene(conversation.scene.as_ref());
                let scene_banner = Self::room_scene_banner(&conversation);
                let room_variant = conversation
                    .scene
                    .as_ref()
                    .map(|scene| scene.background_preset.clone());
                let room_motif = conversation
                    .scene
                    .as_ref()
                    .map(|scene| scene.ambiance.clone());
                let kind_hint = Self::room_kind_hint(&conversation.conversation_id);
                let self_label = viewer
                    .and_then(|viewer| self.viewer_identity_anchor(&conversation, viewer))
                    .or_else(|| self.shell_identity_anchor(&conversation));
                let peer_label = viewer
                    .and_then(|viewer| self.viewer_peer_label(&conversation, viewer))
                    .or_else(|| self.shell_peer_label(&conversation));
                let participant_label = Self::room_participant_label(
                    &conversation.conversation_id,
                    peer_label.as_deref(),
                );
                let route_label = Self::room_route_label(&conversation.conversation_id);
                let personal_room_owner = Self::personal_room_owner(&conversation);
                let caretaker =
                    Self::shell_caretaker(&conversation.conversation_id, &participant_label);
                let is_frozen = self
                    .public_room_by_conversation_id(&conversation.conversation_id)
                    .is_some_and(|room| room.frozen);
                let mut room = ShellRoomState {
                    id: conversation.conversation_id.0.clone(),
                    kind: Self::shell_kind(&conversation.kind),
                    scope: Self::shell_scope(&conversation.scope),
                    title: self.conversation_title_for_viewer(&conversation, viewer),
                    subtitle: String::new(),
                    meta: format!("消息数：{last_count}"),
                    kind_hint: Some(kind_hint),
                    self_label,
                    peer_label,
                    participant_label: Some(participant_label),
                    route_label: Some(route_label),
                    list_summary: None,
                    status_line: None,
                    thread_headline: None,
                    chat_status_summary: None,
                    queue_summary: None,
                    preview_text: None,
                    last_activity_label: None,
                    activity_time_label: None,
                    overview_summary: None,
                    context_summary: None,
                    search_terms: Vec::new(),
                    member_count: Some(conversation.participants.len()),
                    owner_resident_id: personal_room_owner.map(|owner| owner.0.clone()),
                    personal_room_access_policy: personal_room_owner
                        .map(|owner| self.personal_room_access_policy(owner)),
                    scene_banner,
                    scene_summary,
                    room_variant,
                    room_motif,
                    image_layer: Self::shell_image_layer(&conversation),
                    hotspot_layer: Self::shell_hotspot_layer(&conversation),
                    is_frozen,
                    unread_count: viewer
                        .map(|v| self.unread_count(v, &conversation.conversation_id))
                        .unwrap_or(0),
                    messages: room_messages,
                };
                room.subtitle = Self::shell_subtitle(&room, &last_sender);
                room.list_summary = Some(Self::shell_list_summary(&room));
                room.status_line = Some(Self::shell_status_line(&room));
                room.thread_headline = Some(Self::shell_thread_headline(&room));
                room.chat_status_summary = Some(Self::shell_chat_status_summary(&room));
                room.queue_summary = Some(Self::shell_queue_summary(caretaker.as_ref()));
                room.preview_text = Some(Self::shell_preview_text(&room));
                room.last_activity_label = Some(Self::shell_last_activity_label(&room));
                room.activity_time_label = Some(Self::shell_activity_time_label(&room));
                let detail_card = Some(Self::shell_detail_card(
                    &conversation.conversation_id,
                    &room.title,
                    room.self_label.as_deref(),
                    room.peer_label.as_deref(),
                    room.participant_label.as_deref().unwrap_or("当前会话"),
                    room.member_count.unwrap_or_default(),
                    caretaker.as_ref(),
                ));
                let workflow = Self::shell_workflow(&conversation.conversation_id);
                let inline_actions = Self::shell_inline_actions(workflow.as_ref());
                room.overview_summary = Some(Self::shell_overview_summary(&room));
                room.context_summary =
                    Some(Self::shell_context_summary(&room, detail_card.as_ref()));
                room.search_terms = Self::shell_search_terms(
                    &room,
                    caretaker.as_ref(),
                    detail_card.as_ref(),
                    workflow.as_ref(),
                    &inline_actions,
                );
                room
            })
            .collect::<Vec<_>>();

        rooms.sort_by_key(|room| {
            room.messages
                .last()
                .map(|message| message.timestamp_ms)
                .unwrap_or_default()
        });
        rooms.reverse();
        let active_conversation_id = rooms.first().map(|room| room.id.clone());
        let conversation_shell = ConversationShellState {
            active_conversation_id: active_conversation_id.clone(),
            action_templates: Self::shell_action_templates(),
            conversations: rooms
                .iter()
                .map(|room| {
                    let caretaker = Self::shell_caretaker(
                        &ConversationId(room.id.clone()),
                        room.participant_label.as_deref().unwrap_or("当前会话"),
                    );
                    let detail_card = Some(Self::shell_detail_card(
                        &ConversationId(room.id.clone()),
                        &room.title,
                        room.self_label.as_deref(),
                        room.peer_label.as_deref(),
                        room.participant_label.as_deref().unwrap_or("当前会话"),
                        room.member_count.unwrap_or_default(),
                        caretaker.as_ref(),
                    ));
                    let workflow = Self::shell_workflow(&ConversationId(room.id.clone()));
                    let inline_actions = Self::shell_inline_actions(workflow.as_ref());
                    let queue_summary = Self::shell_queue_summary(caretaker.as_ref());
                    ConversationShellConversation {
                        conversation_id: room.id.clone(),
                        kind: room.kind.clone(),
                        scope: room.scope.clone(),
                        title: room.title.clone(),
                        subtitle: room.subtitle.clone(),
                        meta: room.meta.clone(),
                        kind_hint: room.kind_hint.clone(),
                        self_label: room.self_label.clone(),
                        peer_label: room.peer_label.clone(),
                        participant_label: room.participant_label.clone(),
                        route_label: room.route_label.clone(),
                        list_summary: room.list_summary.clone(),
                        status_line: room.status_line.clone(),
                        thread_headline: room.thread_headline.clone(),
                        chat_status_summary: room.chat_status_summary.clone(),
                        queue_summary: Some(queue_summary),
                        preview_text: room.preview_text.clone(),
                        last_activity_label: room.last_activity_label.clone(),
                        activity_time_label: room.activity_time_label.clone(),
                        unread_count: room.unread_count,
                        overview_summary: Some(Self::shell_overview_summary(room)),
                        context_summary: Some(Self::shell_context_summary(
                            room,
                            detail_card.as_ref(),
                        )),
                        search_terms: room.search_terms.clone(),
                        member_count: room.member_count,
                        personal_room_access_policy: room.personal_room_access_policy,
                        caretaker,
                        detail_card,
                        workflow,
                        inline_actions,
                        messages: room.messages.clone(),
                    }
                })
                .collect(),
        };
        let scene_render = SceneRenderState {
            active_conversation_id: active_conversation_id.clone(),
            scenes: rooms
                .iter()
                .map(|room| {
                    let caretaker = Self::shell_caretaker(
                        &ConversationId(room.id.clone()),
                        room.participant_label.as_deref().unwrap_or("当前会话"),
                    );
                    let detail_card = Some(Self::shell_detail_card(
                        &ConversationId(room.id.clone()),
                        &room.title,
                        room.self_label.as_deref(),
                        room.peer_label.as_deref(),
                        room.participant_label.as_deref().unwrap_or("当前会话"),
                        room.member_count.unwrap_or_default(),
                        caretaker.as_ref(),
                    ));
                    SceneRenderConversation {
                        conversation_id: room.id.clone(),
                        scene_banner: room.scene_banner.clone(),
                        scene_summary: room.scene_summary.clone(),
                        room_variant: room.room_variant.clone(),
                        room_motif: room.room_motif.clone(),
                        image_layer: room.image_layer.clone(),
                        hotspot_layer: room.hotspot_layer.clone(),
                        stage: Some(Self::shell_scene_stage_projection(
                            room,
                            detail_card.as_ref(),
                        )),
                        portrait: Some(Self::shell_scene_portrait_projection(
                            room,
                            caretaker.as_ref(),
                            detail_card.as_ref(),
                        )),
                    }
                })
                .collect(),
        };
        let state_version = Self::shell_state_version(&rooms);
        ShellState {
            state_version,
            rooms,
            conversation_shell,
            scene_render,
        }
    }

    fn shell_state_version(rooms: &[ShellRoomState]) -> String {
        let mut message_count = 0usize;
        let mut recall_count = 0usize;
        let mut edit_count = 0usize;
        let mut metadata_hasher = DefaultHasher::new();
        let mut latest_timestamp_ms = 0i64;
        let mut latest_recalled_at_ms = 0i64;
        let mut latest_edited_at_ms = 0i64;
        let mut latest_message_id = "";
        for room in rooms {
            Self::hash_shell_room_metadata(room, &mut metadata_hasher);
            for message in &room.messages {
                message_count += 1;
                if message.is_recalled {
                    recall_count += 1;
                    if let Some(recalled_at_ms) = message.recalled_at_ms {
                        latest_recalled_at_ms = latest_recalled_at_ms.max(recalled_at_ms);
                    }
                }
                if message.is_edited {
                    edit_count += 1;
                    if let Some(edited_at_ms) = message.edited_at_ms {
                        latest_edited_at_ms = latest_edited_at_ms.max(edited_at_ms);
                    }
                }
                if message.timestamp_ms > latest_timestamp_ms
                    || (message.timestamp_ms == latest_timestamp_ms
                        && message.message_id.as_str() > latest_message_id)
                {
                    latest_timestamp_ms = message.timestamp_ms;
                    latest_message_id = &message.message_id;
                }
            }
        }
        let metadata_fingerprint = metadata_hasher.finish();
        format!(
            "shell:v1:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            rooms.len(),
            message_count,
            latest_timestamp_ms,
            latest_message_id,
            recall_count,
            latest_recalled_at_ms,
            edit_count,
            latest_edited_at_ms,
            metadata_fingerprint
        )
    }

    fn hash_shell_room_metadata(room: &ShellRoomState, hasher: &mut DefaultHasher) {
        room.id.hash(hasher);
        room.kind.hash(hasher);
        room.scope.hash(hasher);
        room.title.hash(hasher);
        room.subtitle.hash(hasher);
        room.meta.hash(hasher);
        room.kind_hint.hash(hasher);
        room.self_label.hash(hasher);
        room.peer_label.hash(hasher);
        room.participant_label.hash(hasher);
        room.route_label.hash(hasher);
        room.list_summary.hash(hasher);
        room.status_line.hash(hasher);
        room.thread_headline.hash(hasher);
        room.chat_status_summary.hash(hasher);
        room.queue_summary.hash(hasher);
        room.overview_summary.hash(hasher);
        room.context_summary.hash(hasher);
        room.preview_text.hash(hasher);
        room.last_activity_label.hash(hasher);
        room.activity_time_label.hash(hasher);
        room.search_terms.hash(hasher);
        room.member_count.hash(hasher);
        room.owner_resident_id.hash(hasher);
        room.personal_room_access_policy.hash(hasher);
        room.scene_banner.hash(hasher);
        room.scene_summary.hash(hasher);
        room.room_variant.hash(hasher);
        room.room_motif.hash(hasher);
        room.is_frozen.hash(hasher);
    }
}
