use super::*;

impl GatewayRuntime {
    pub(crate) fn conversation_blueprint(
        conversation_id: &ConversationId,
        timestamp_ms: i64,
        sender: &IdentityId,
    ) -> Conversation {
        let kind = if conversation_id.0.starts_with("dm:") {
            ConversationKind::Direct
        } else {
            ConversationKind::Room
        };
        let scope = if conversation_id.0.starts_with("room:world:") {
            ConversationScope::CrossCityShared
        } else if conversation_id.0.starts_with("room:") {
            ConversationScope::CityPublic
        } else {
            ConversationScope::Private
        };
        let participants = if matches!(kind, ConversationKind::Direct) {
            conversation_id
                .0
                .split(':')
                .skip(1)
                .map(|part| IdentityId(part.to_string()))
                .collect::<Vec<_>>()
        } else {
            let mut parts = vec![sender.clone()];
            let op = IdentityId("rsaga".into());
            if sender != &op {
                parts.push(op);
            }
            parts
        };

        Conversation {
            conversation_id: conversation_id.clone(),
            kind: kind.clone(),
            scope,
            scene: if matches!(kind, ConversationKind::Direct) {
                Some(Self::default_direct_scene(&participants))
            } else {
                Some(Self::default_public_room_scene(
                    "shared",
                    "channel",
                    &Self::room_title(conversation_id),
                ))
            },
            content_topic: transport_waku::WakuFrameCodec::content_topic_for(conversation_id),
            participants,
            created_at_ms: timestamp_ms,
            last_active_at_ms: timestamp_ms,
        }
    }

    pub(crate) fn ensure_conversation_for(
        &mut self,
        message: &MessageEnvelope,
    ) -> Result<(), String> {
        let exists = self
            .timeline_store
            .active_conversations()
            .into_iter()
            .any(|item| item.conversation_id == message.conversation_id);
        if exists {
            return Ok(());
        }
        self.timeline_store
            .upsert_conversation(Self::conversation_blueprint(
                &message.conversation_id,
                message.timestamp_ms,
                &message.sender,
            ))
    }

    pub(crate) fn ensure_room_conversation(
        &mut self,
        room: &PublicRoomRecord,
    ) -> Result<(), String> {
        let exists = self
            .timeline_store
            .active_conversations()
            .into_iter()
            .any(|item| item.conversation_id == room.room_id);
        if exists {
            return Ok(());
        }
        self.timeline_store.upsert_conversation(Conversation {
            conversation_id: room.room_id.clone(),
            kind: ConversationKind::Room,
            scope: ConversationScope::CityPublic,
            scene: room.scene.clone(),
            content_topic: transport_waku::WakuFrameCodec::content_topic_for(&room.room_id),
            participants: {
                let mut parts = vec![room.created_by.clone()];
                let op = IdentityId("rsaga".into());
                if room.created_by != op {
                    parts.push(op);
                }
                parts
            },
            created_at_ms: room.created_at_ms,
            last_active_at_ms: room.created_at_ms,
        })
    }

    pub(crate) fn direct_conversation_id(a: &IdentityId, b: &IdentityId) -> ConversationId {
        canonical_direct_conversation_id(a, b)
    }

    pub(crate) fn legacy_direct_conversation_id(a: &IdentityId, b: &IdentityId) -> ConversationId {
        ConversationId(format!("dm:{}:{}", a.0, b.0))
    }

    pub(crate) fn resolve_direct_conversation_id(
        &self,
        a: &IdentityId,
        b: &IdentityId,
    ) -> ConversationId {
        let canonical = Self::direct_conversation_id(a, b);
        let legacy = Self::legacy_direct_conversation_id(a, b);
        let reverse_legacy = Self::legacy_direct_conversation_id(b, a);

        let known = self
            .timeline_store
            .active_conversations()
            .into_iter()
            .map(|conversation| conversation.conversation_id)
            .collect::<Vec<_>>();
        if known
            .iter()
            .any(|conversation_id| conversation_id == &canonical)
        {
            return canonical;
        }
        if known
            .iter()
            .any(|conversation_id| conversation_id == &legacy)
        {
            return legacy;
        }
        if known
            .iter()
            .any(|conversation_id| conversation_id == &reverse_legacy)
        {
            return reverse_legacy;
        }
        canonical
    }

    pub(crate) fn ensure_verified_resident_guide_conversation(
        &mut self,
        resident_id: &IdentityId,
    ) -> Result<(), String> {
        let guide_id = IdentityId("guide".into());
        if *resident_id == guide_id {
            return Ok(());
        }

        let conversation_id = Self::direct_conversation_id(resident_id, &guide_id);
        self.ensure_direct_conversation(&conversation_id, &[resident_id.clone(), guide_id])
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn resolve_cli_direct_conversation_id(
        &self,
        a: &IdentityId,
        b: &IdentityId,
    ) -> ConversationId {
        self.resolve_direct_conversation_id(a, b)
    }

    pub(crate) fn ensure_direct_conversation(
        &mut self,
        conversation_id: &ConversationId,
        participants: &[IdentityId],
    ) -> Result<(), String> {
        let exists = self
            .timeline_store
            .active_conversations()
            .into_iter()
            .any(|item| item.conversation_id == *conversation_id);
        if exists {
            return Ok(());
        }

        self.timeline_store.upsert_conversation(Conversation {
            conversation_id: conversation_id.clone(),
            kind: ConversationKind::Direct,
            scope: ConversationScope::Private,
            scene: Some(Self::default_direct_scene(participants)),
            content_topic: transport_waku::WakuFrameCodec::content_topic_for(conversation_id),
            participants: participants.to_vec(),
            created_at_ms: Self::now_ms(),
            last_active_at_ms: Self::now_ms(),
        })
    }

    pub(crate) fn open_direct_session(
        &mut self,
        request: OpenDirectSessionRequest,
    ) -> Result<MlsGroupView, String> {
        let requester = IdentityId(Self::normalize_direct_resident_id(request.requester_id)?);
        let peer = IdentityId(Self::normalize_direct_resident_id(request.peer_id)?);
        if requester == peer {
            return Err("direct session requires two distinct residents".into());
        }

        let conversation_id = self.resolve_direct_conversation_id(&requester, &peer);
        self.ensure_direct_conversation(&conversation_id, &[requester.clone(), peer.clone()])?;

        if let Some(existing) = self.secure_sessions.group_state(&conversation_id) {
            return Ok(MlsGroupView::from(existing));
        }

        let members = vec![
            request
                .requester_device_id
                .map(|device| MlsMember::device(requester.0.clone(), device))
                .unwrap_or_else(|| MlsMember::identity(requester.0.clone())),
            request
                .peer_device_id
                .map(|device| MlsMember::device(peer.0.clone(), device))
                .unwrap_or_else(|| MlsMember::identity(peer.0.clone())),
        ];
        let group = self
            .secure_sessions
            .bootstrap_direct(&conversation_id, members)?;
        self.persist_secure_sessions()?;
        Ok(MlsGroupView::from(&group))
    }

    /// Ensure a personal room (1-participant Direct conversation) exists for `resident`.
    /// Reuses ensure_direct_conversation with a single participant; open_direct_session
    /// forbids 1-participant DMs, so personal rooms need this dedicated path.
    pub(crate) fn ensure_personal_room(
        &mut self,
        resident: &IdentityId,
    ) -> Result<ConversationId, String> {
        let conversation_id = ConversationId(format!("home:{}", resident.0));
        self.ensure_direct_conversation(&conversation_id, std::slice::from_ref(resident))?;
        Ok(conversation_id)
    }

    pub(crate) fn open_personal_room(
        &mut self,
        request: PersonalRoomRequest,
    ) -> Result<PersonalRoomResponse, String> {
        let resident = IdentityId(Self::normalize_direct_resident_id(request.resident_id)?);
        if !self.resident_has_authenticated_profile(&resident) {
            return Err(format!(
                "personal room owner {} must be a registered resident",
                resident.0
            ));
        }
        let conversation_id = self.ensure_personal_room(&resident)?;
        Ok(PersonalRoomResponse {
            room_id: conversation_id.0,
        })
    }

    fn normalize_direct_resident_id(raw: String) -> Result<String, String> {
        let resident_id = raw.trim().to_string();
        if resident_id.is_empty() || resident_id == "访客" {
            Err("direct session requires authenticated residents".into())
        } else {
            Ok(resident_id)
        }
    }

    pub(crate) fn update_shell_scene(
        &mut self,
        request: UpdateShellSceneRequest,
    ) -> Result<UpdateShellSceneResponse, String> {
        let actor = IdentityId(Self::normalize_direct_resident_id(request.actor)?);
        let conversation_id = ConversationId(request.room_id.trim().to_string());
        if conversation_id.0.is_empty() {
            return Err("scene update requires a room id".into());
        }

        let mut conversation = self
            .timeline_store
            .active_conversations()
            .into_iter()
            .find(|item| item.conversation_id == conversation_id)
            .ok_or_else(|| "scene update target room not found".to_string())?;

        if !conversation.participants.contains(&actor) {
            return Err("scene update actor is not a room participant".into());
        }

        let scene = conversation
            .scene
            .as_mut()
            .ok_or_else(|| "scene update target has no scene metadata".to_string())?;
        if !scene.owner_editable {
            return Err("scene is not owner editable".into());
        }

        if let Some(image_layer) = request.image_layer {
            if let Some(image_layer) = image_layer {
                Self::validate_scene_image_layer(&image_layer)?;
                if !image_layer.owner_editable {
                    return Err("image layer must remain owner editable".into());
                }
                scene.image_layer = Some(image_layer);
            } else {
                scene.image_layer = None;
            }
        }

        if let Some(hotspot_layer) = request.hotspot_layer {
            if let Some(hotspot_layer) = hotspot_layer {
                Self::validate_scene_hotspot_layer(&hotspot_layer)?;
                if !hotspot_layer.owner_editable {
                    return Err("hotspot layer must remain owner editable".into());
                }
                scene.hotspot_layer = Some(hotspot_layer);
            } else {
                scene.hotspot_layer = None;
            }
        }

        let updated_at_ms = Self::now_ms();
        conversation.touch(updated_at_ms);
        let image_layer = Self::shell_image_layer(&conversation);
        let hotspot_layer = Self::shell_hotspot_layer(&conversation);
        self.timeline_store.upsert_conversation(conversation)?;

        Ok(UpdateShellSceneResponse {
            ok: true,
            conversation_id: conversation_id.0,
            image_layer,
            hotspot_layer,
            updated_at_ms,
        })
    }

    fn check_scene_edit_permission(
        &self,
        actor_id: &str,
        conversation_id: &ConversationId,
    ) -> Result<(), String> {
        let actor = IdentityId(actor_id.to_string());
        // 世界管家可以编辑任何场景
        if self.actor_is_world_steward(&actor) {
            return Ok(());
        }
        // 世界入口 / 世界广场 → 只有世界管家
        if conversation_id.0 == "room:world:entry" || conversation_id.0 == "room:world:square" {
            return Err("only world stewards can edit world entry/square scenes".into());
        }
        // 私宅 (dm:*) → 房主可以编辑
        if conversation_id.0.starts_with("dm:") {
            let conv = self
                .timeline_store
                .active_conversations()
                .into_iter()
                .find(|c| c.conversation_id == *conversation_id);
            if let Some(conv) = conv
                && !conv.participants.contains(&actor)
            {
                return Err("only the room owner can edit private room scenes".into());
            }
            return Ok(());
        }
        // 公共房间 → 世界管家（已在上面通过）
        Err("only world stewards can edit public room scenes".into())
    }

    pub(crate) fn admin_update_scene(
        &mut self,
        request: AdminUpdateSceneRequest,
    ) -> Result<UpdateShellSceneResponse, String> {
        let conversation_id = ConversationId(request.room_id.trim().to_string());
        if conversation_id.0.is_empty() {
            return Err("scene update requires a room id".into());
        }

        // 权限校验
        if let Some(ref actor) = request.actor_id {
            self.check_scene_edit_permission(actor, &conversation_id)?;
        }

        let mut conversation = self
            .timeline_store
            .active_conversations()
            .into_iter()
            .find(|item| item.conversation_id == conversation_id)
            .ok_or_else(|| "scene update target room not found".to_string())?;

        if conversation.scene.is_none() {
            conversation.scene = Some(Self::default_public_room_scene(
                "admin",
                "custom",
                &Self::room_title(&conversation_id),
            ));
        }

        let scene = conversation
            .scene
            .as_mut()
            .ok_or_else(|| "scene update target has no scene metadata".to_string())?;
        if !scene.owner_editable {
            return Err("scene is not owner editable".into());
        }

        if let Some(image_layer) = request.image_layer {
            if let Some(image_layer) = image_layer {
                Self::validate_scene_image_layer(&image_layer)?;
                scene.image_layer = Some(image_layer);
            } else {
                scene.image_layer = None;
            }
        }

        if let Some(hotspot_layer) = request.hotspot_layer {
            if let Some(hotspot_layer) = hotspot_layer {
                Self::validate_scene_hotspot_layer(&hotspot_layer)?;
                scene.hotspot_layer = Some(hotspot_layer);
            } else {
                scene.hotspot_layer = None;
            }
        }

        let updated_at_ms = Self::now_ms();
        conversation.touch(updated_at_ms);
        let image_layer = Self::shell_image_layer(&conversation);
        let hotspot_layer = Self::shell_hotspot_layer(&conversation);
        self.timeline_store.upsert_conversation(conversation)?;

        Ok(UpdateShellSceneResponse {
            ok: true,
            conversation_id: conversation_id.0,
            image_layer,
            hotspot_layer,
            updated_at_ms,
        })
    }

    fn validate_scene_image_layer(layer: &SceneImageLayer) -> Result<(), String> {
        if layer.layer_id.trim().is_empty() {
            return Err("image layer id is required".into());
        }
        if layer.preset.trim().is_empty() {
            return Err("image layer preset is required".into());
        }
        if layer.asset_hint.trim().is_empty() {
            return Err("image layer asset hint is required".into());
        }
        if !(5_000..=30_000).contains(&layer.aspect_ratio_permyriad) {
            return Err("image layer aspect ratio is out of range".into());
        }
        // 自定义背景：白天/夜晚必须同时提供
        let has_day = layer
            .day_image_url
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty());
        let has_night = layer
            .night_image_url
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty());
        if has_day != has_night {
            return Err("custom images must include both day and night urls (or neither)".into());
        }
        Ok(())
    }

    fn validate_scene_hotspot_layer(layer: &SceneHotspotLayer) -> Result<(), String> {
        if layer.layer_id.trim().is_empty() {
            return Err("hotspot layer id is required".into());
        }
        if layer.coordinate_system.trim() != "scene-permyriad" {
            return Err("hotspot layer coordinate system must be scene-permyriad".into());
        }
        if layer.hotspots.len() > 32 {
            return Err("hotspot layer supports at most 32 hotspots".into());
        }
        for hotspot in &layer.hotspots {
            if hotspot.hotspot_id.trim().is_empty() || hotspot.label.trim().is_empty() {
                return Err("hotspot id and label are required".into());
            }
            if hotspot.width_permyriad == 0 || hotspot.height_permyriad == 0 {
                return Err("hotspot size must be positive".into());
            }
            if hotspot.x_permyriad > 10_000
                || hotspot.y_permyriad > 10_000
                || hotspot.x_permyriad.saturating_add(hotspot.width_permyriad) > 10_000
                || hotspot.y_permyriad.saturating_add(hotspot.height_permyriad) > 10_000
            {
                return Err("hotspot bounds must stay inside the scene".into());
            }
        }
        Ok(())
    }
}
