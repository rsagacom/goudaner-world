use super::*;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

impl GatewayRuntime {
    pub(crate) fn open(
        storage_root: impl Into<PathBuf>,
        history_limit: usize,
        upstream_base_url: Option<String>,
    ) -> Result<Self, String> {
        let storage_root = storage_root.into();
        let archive_policy = ArchivePolicy::default();
        let timeline_store = FileTimelineStore::open(&storage_root, archive_policy)?;
        let cli_provider_url = upstream_base_url;
        let dev_auth_bypass = Self::dev_auth_bypass_default();
        let secure_session_storage_key = Self::secure_session_storage_key_default(dev_auth_bypass)?;
        let mut runtime = Self {
            node: InMemoryWakuLightNode::new(
                WakuPeerMode::DesktopLight,
                WakuLightConfig {
                    relay_enabled: false,
                    filter_enabled: true,
                    store_enabled: true,
                    light_push_enabled: true,
                },
            ),
            upstream_gateway: None,
            upstream_base_url: None,
            mirror_sources: Vec::new(),
            connection_state: WakuConnectionState::Disconnected,
            endpoint: None,
            subscriptions: Vec::new(),
            cursors: HashMap::new(),
            history_limit,
            governance_path: storage_root.join("governance-state.json"),
            presence_path: storage_root.join("presence-state.json"),
            unread_path: storage_root.join("unread-state.json"),
            moderation_state_path: storage_root.join("moderation-state.json"),
            secure_sessions_path: storage_root.join("secure-sessions.json"),
            provider_config_path: storage_root.join("provider-config.json"),
            auth_state_path: storage_root.join("auth-state.json"),
            app_config_path: storage_root.join("app-config.json"),
            invites_path: storage_root.join("invites.json"),
            permission_groups_path: storage_root.join("permission-groups.json"),
            personal_room_access_policies_path: storage_root
                .join("personal-room-access-policies.json"),
            resident_relationships_path: storage_root.join("resident-relationships.json"),
            audit_log_path: storage_root.join("audit-log.json"),
            device_state_path: storage_root.join("device-state.json"),
            timeline_store,
            secure_sessions: SkeletonSecureSessionManager::new(),
            secure_session_storage_key,
            world: Self::default_world(),
            portability: ResidentPortability::protocol_safe_default(),
            cities: HashMap::new(),
            memberships: Vec::new(),
            public_rooms: Vec::new(),
            world_stewards: Vec::new(),
            city_trust: Vec::new(),
            world_square_notices: Vec::new(),
            safety_advisories: Vec::new(),
            safety_reports: Vec::new(),
            resident_sanctions: Vec::new(),
            registration_blacklist: Vec::new(),
            registrations: Vec::new(),
            email_otp_challenges: Vec::new(),
            auth_sessions: Vec::new(),
            message_counter: 0,
            presence: HashMap::new(),
            unread: HashMap::new(),
            rate_limits: HashMap::new(),
            message_moderation: HashMap::new(),
            allowed_devices: HashMap::new(),
            device_bindings: HashMap::new(),
            invites: HashMap::new(),
            permission_groups: HashMap::new(),
            resident_permission_groups: HashMap::new(),
            personal_room_access_policies: HashMap::new(),
            resident_relationships: HashMap::new(),
            audit_events: Vec::new(),
            audit_counter: 0,
            agent_token_hashes: Self::agent_token_hashes_default(),
            federation_token_hash: Self::federation_token_hash_default(),
            dev_auth_bypass,
            started_at_ms: Self::now_ms(),
            app_config: HashMap::new(),
        };
        runtime.load_governance_state()?;
        runtime.load_secure_sessions()?;
        runtime.load_provider_config()?;
        runtime.load_auth_state()?;
        runtime.load_app_config()?;
        runtime.load_invites()?;
        runtime.load_presence_state()?;
        runtime.load_unread_state()?;
        runtime.load_moderation_state()?;
        runtime.load_device_state()?;
        runtime.load_permission_groups()?;
        runtime.load_personal_room_access_policies()?;
        runtime.load_resident_relationships()?;
        runtime.load_audit_log()?;
        runtime.ensure_default_world_safety()?;
        if cli_provider_url.is_some() {
            runtime.set_upstream_provider_url(cli_provider_url)?;
        }
        if runtime.cities.is_empty() {
            runtime.seed_default_governance()?;
            runtime.ensure_default_world_safety()?;
        }
        let has_world_lobby = runtime
            .timeline_store
            .active_conversations()
            .into_iter()
            .any(|conversation| conversation.conversation_id.0 == "room:world:lobby");
        if !has_world_lobby && Self::should_seed_demo_messages(runtime.dev_auth_bypass, cfg!(test))
        {
            runtime.seed_demo_messages()?;
        }
        Ok(runtime)
    }

    /// Demo history is useful for tests and explicit local fixtures only. A
    /// production gateway must start with an empty timeline instead of
    /// presenting synthetic chat records as real user messages.
    pub(crate) fn should_seed_demo_messages(dev_auth_bypass: bool, test_build: bool) -> bool {
        test_build || dev_auth_bypass
    }

    pub(crate) fn federation_read_plan(&self) -> GatewayFederationReadPlan {
        GatewayFederationReadPlan {
            local_governance: self.governance_snapshot(),
            upstream_base_url: self.upstream_base_url.clone(),
            mirror_sources: self.mirror_sources.clone(),
        }
    }

    pub(crate) fn record_presence(&mut self, resident_id: &str) -> bool {
        let now_ms = Self::now_ms();
        let was_online = self.is_online(resident_id, 120_000);
        self.presence.insert(resident_id.to_string(), now_ms);
        let _ = self.persist_presence_state();
        !was_online
    }

    pub(crate) fn is_online(&self, resident_id: &str, threshold_ms: i64) -> bool {
        self.presence
            .get(resident_id)
            .map(|last_seen| Self::now_ms() - last_seen < threshold_ms)
            .unwrap_or(false)
    }

    pub(crate) fn increment_unread(
        &mut self,
        conversation_id: &ConversationId,
        exclude_sender: &IdentityId,
    ) {
        let conversation = self
            .timeline_store
            .active_conversations()
            .into_iter()
            .find(|item| item.conversation_id == *conversation_id);
        if let Some(conversation) = conversation {
            for participant in &conversation.participants {
                if participant == exclude_sender {
                    continue;
                }
                let key = format!("{}:{}", participant.0, conversation_id.0);
                let count = self.unread.get(&key).copied().unwrap_or(0);
                self.unread.insert(key, count.saturating_add(1));
            }
        }
        let _ = self.persist_unread_state();
    }

    pub(crate) fn mark_read(&mut self, resident_id: &IdentityId, conversation_id: &ConversationId) {
        let key = format!("{}:{}", resident_id.0, conversation_id.0);
        self.unread.insert(key, 0);
        let _ = self.persist_unread_state();
    }

    // --- Admin invite / member / log methods ---

    pub(crate) fn admin_create_invite(
        &mut self,
        actor_id: &str,
        max_uses: u32,
    ) -> Result<AdminCreateInviteResponse, String> {
        let code = format!("AJW-{:06}", (Self::now_ms() % 1_000_000) as u32);
        let now = Self::now_ms();
        self.invites.insert(
            code.clone(),
            InviteCode {
                code: code.clone(),
                created_at_ms: now,
                max_uses,
                used_count: 0,
                revoked: false,
                created_by: actor_id.to_string(),
            },
        );
        if let Err(error) = self.persist_invites() {
            self.invites.remove(&code);
            return Err(error);
        }
        Ok(AdminCreateInviteResponse {
            ok: true,
            code,
            created_at_ms: now,
            max_uses,
        })
    }

    pub(crate) fn admin_revoke_invite(&mut self, code: &str) -> Result<bool, String> {
        if let Some(invite) = self.invites.get_mut(code) {
            let previous = invite.revoked;
            invite.revoked = true;
            if let Err(error) = self.persist_invites() {
                if let Some(invite) = self.invites.get_mut(code) {
                    invite.revoked = previous;
                }
                return Err(error);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn persist_invites(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self.invites)
            .map_err(|e| format!("encode invites failed: {e}"))?;
        atomic_write_file(&self.invites_path, &bytes)
            .map_err(|e| format!("write invites failed: {e}"))
    }

    pub(crate) fn load_invites(&mut self) -> Result<(), String> {
        if !self.invites_path.exists() {
            return Ok(());
        }
        let bytes =
            std::fs::read(&self.invites_path).map_err(|e| format!("read invites failed: {e}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        self.invites =
            serde_json::from_slice(&bytes).map_err(|e| format!("decode invites failed: {e}"))?;
        Ok(())
    }

    pub(crate) fn admin_manage_room_member(
        &mut self,
        room_id: &str,
        resident_id: &str,
        action: &str,
    ) -> Result<bool, String> {
        let conversation_id = ConversationId(room_id.to_string());
        let mut conversations = self.timeline_store.active_conversations();
        if let Some(conv) = conversations
            .iter_mut()
            .find(|c| c.conversation_id == conversation_id)
        {
            match action {
                "add" => {
                    let rid = IdentityId(resident_id.to_string());
                    if !conv.participants.contains(&rid) {
                        conv.participants.push(rid);
                    }
                }
                "remove" => {
                    conv.participants.retain(|p| p.0 != resident_id);
                }
                _ => return Ok(false),
            }
            self.timeline_store
                .upsert_conversation(conv.clone())
                .map_err(|error| format!("persist room member change failed: {error}"))?;
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) fn admin_create_resident(
        &mut self,
        resident_id: &str,
        email: &str,
    ) -> Result<bool, String> {
        let rid = IdentityId(resident_id.to_string());
        if self.registrations.iter().any(|r| r.resident_id == rid) {
            return Ok(false);
        }
        self.registrations.push(ResidentRegistration {
            resident_id: rid,
            email: email.to_string(),
            email_hash_sha256: String::new(),
            mobile_hash_sha256: None,
            device_hashes_sha256: vec![],
            state: ResidentRegistrationState::Active,
            created_at_ms: Self::now_ms(),
            verified_at_ms: 0,
            last_login_at_ms: 0,
            nickname: None,
        });
        if let Err(error) = self.persist_auth_state() {
            self.registrations.pop();
            return Err(error);
        }
        Ok(true)
    }

    pub(crate) fn resident_has_authenticated_profile(&self, resident: &IdentityId) -> bool {
        self.registrations
            .iter()
            .any(|registration| registration.resident_id == *resident)
            || self
                .world_stewards
                .iter()
                .any(|steward| steward == resident)
    }

    pub(crate) fn personal_room_owner(conversation: &Conversation) -> Option<&IdentityId> {
        if conversation.kind != ConversationKind::Direct || conversation.participants.len() != 1 {
            return None;
        }
        let owner = conversation.participants.first()?;
        if conversation.conversation_id.0 == format!("home:{}", owner.0) {
            Some(owner)
        } else {
            None
        }
    }

    fn normalize_personal_room_policy_resident(raw: String) -> Result<IdentityId, String> {
        let resident_id = raw.trim().to_string();
        if resident_id.is_empty() || resident_id == "访客" {
            Err("personal room access policy requires an authenticated resident".into())
        } else {
            Ok(IdentityId(resident_id))
        }
    }

    pub(crate) fn personal_room_access_policy(
        &self,
        resident: &IdentityId,
    ) -> PersonalRoomAccessPolicy {
        self.personal_room_access_policies
            .get(&resident.0)
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn set_personal_room_access_policy(
        &mut self,
        request: PersonalRoomAccessPolicyRequest,
    ) -> Result<PersonalRoomAccessPolicyResponse, String> {
        let resident = Self::normalize_personal_room_policy_resident(request.resident_id)?;
        if !self.resident_has_authenticated_profile(&resident) {
            return Err(format!(
                "personal room owner {} must be a registered resident",
                resident.0
            ));
        }
        self.personal_room_access_policies
            .insert(resident.0.clone(), request.policy);
        self.persist_personal_room_access_policies()?;
        Ok(PersonalRoomAccessPolicyResponse {
            resident_id: resident.0,
            policy: request.policy,
        })
    }

    pub(crate) fn persist_personal_room_access_policies(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self.personal_room_access_policies)
            .map_err(|error| format!("encode personal room access policies failed: {error}"))?;
        atomic_write_file(&self.personal_room_access_policies_path, &bytes)
            .map_err(|error| format!("write personal room access policies failed: {error}"))
    }

    pub(crate) fn load_personal_room_access_policies(&mut self) -> Result<(), String> {
        if !self.personal_room_access_policies_path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&self.personal_room_access_policies_path)
            .map_err(|error| format!("read personal room access policies failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        self.personal_room_access_policies = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode personal room access policies failed: {error}"))?;
        Ok(())
    }

    fn normalize_relationship_resident(raw: &str, field: &str) -> Result<IdentityId, String> {
        let resident_id = raw.trim().to_string();
        if resident_id.is_empty() || resident_id == "访客" {
            Err(format!("{field} requires an authenticated resident"))
        } else {
            Ok(IdentityId(resident_id))
        }
    }

    fn resident_relationship_key(a: &IdentityId, b: &IdentityId) -> String {
        if a.0 <= b.0 {
            format!("{}:{}", a.0, b.0)
        } else {
            format!("{}:{}", b.0, a.0)
        }
    }

    fn sorted_relationship_pair(a: &IdentityId, b: &IdentityId) -> (String, String) {
        if a.0 <= b.0 {
            (a.0.clone(), b.0.clone())
        } else {
            (b.0.clone(), a.0.clone())
        }
    }

    fn validate_relationship_pair(
        &self,
        request: &ResidentRelationshipRequest,
    ) -> Result<(IdentityId, IdentityId, String), String> {
        let actor = Self::normalize_relationship_resident(&request.actor_id, "actor_id")?;
        let peer = Self::normalize_relationship_resident(&request.peer_id, "peer_id")?;
        if actor == peer {
            return Err("resident relationship requires two distinct residents".into());
        }
        if !self.resident_has_authenticated_profile(&actor) {
            return Err(format!("resident {} must be registered", actor.0));
        }
        if !self.resident_has_authenticated_profile(&peer) {
            return Err(format!("resident {} must be registered", peer.0));
        }
        let key = Self::resident_relationship_key(&actor, &peer);
        Ok((actor, peer, key))
    }

    pub(crate) fn residents_are_friends(&self, a: &IdentityId, b: &IdentityId) -> bool {
        if a == b {
            return true;
        }
        let key = Self::resident_relationship_key(a, b);
        self.resident_relationships
            .get(&key)
            .is_some_and(|record| record.state == ResidentRelationshipState::Friends)
    }

    pub(crate) fn request_resident_friendship(
        &mut self,
        request: ResidentRelationshipRequest,
    ) -> Result<ResidentRelationshipResponse, String> {
        let (actor, peer, key) = self.validate_relationship_pair(&request)?;
        if let Some(record) = self.resident_relationships.get(&key) {
            return Ok(ResidentRelationshipResponse {
                resident_id: actor.0,
                peer_id: peer.0,
                state: record.state,
            });
        }

        let now = Self::now_ms();
        let (resident_a, resident_b) = Self::sorted_relationship_pair(&actor, &peer);
        self.resident_relationships.insert(
            key,
            ResidentRelationshipRecord {
                resident_a,
                resident_b,
                state: ResidentRelationshipState::Pending,
                requested_by: actor.0.clone(),
                created_at_ms: now,
                updated_at_ms: now,
            },
        );
        self.persist_resident_relationships()?;
        Ok(ResidentRelationshipResponse {
            resident_id: actor.0,
            peer_id: peer.0,
            state: ResidentRelationshipState::Pending,
        })
    }

    pub(crate) fn accept_resident_friendship(
        &mut self,
        request: ResidentRelationshipRequest,
    ) -> Result<ResidentRelationshipResponse, String> {
        let (actor, peer, key) = self.validate_relationship_pair(&request)?;
        let Some(record) = self.resident_relationships.get_mut(&key) else {
            return Err("friendship request does not exist".into());
        };
        if record.state == ResidentRelationshipState::Friends {
            return Ok(ResidentRelationshipResponse {
                resident_id: actor.0,
                peer_id: peer.0,
                state: ResidentRelationshipState::Friends,
            });
        }
        if record.requested_by == actor.0 {
            return Err("requester cannot accept their own friendship request".into());
        }
        record.state = ResidentRelationshipState::Friends;
        record.updated_at_ms = Self::now_ms();
        self.persist_resident_relationships()?;
        Ok(ResidentRelationshipResponse {
            resident_id: actor.0,
            peer_id: peer.0,
            state: ResidentRelationshipState::Friends,
        })
    }

    pub(crate) fn persist_resident_relationships(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self.resident_relationships)
            .map_err(|error| format!("encode resident relationships failed: {error}"))?;
        atomic_write_file(&self.resident_relationships_path, &bytes)
            .map_err(|error| format!("write resident relationships failed: {error}"))
    }

    pub(crate) fn load_resident_relationships(&mut self) -> Result<(), String> {
        if !self.resident_relationships_path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&self.resident_relationships_path)
            .map_err(|error| format!("read resident relationships failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        self.resident_relationships = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode resident relationships failed: {error}"))?;
        Ok(())
    }

    pub(crate) fn admin_clear_processed_logs(&mut self) -> Result<usize, String> {
        let previous = self.message_moderation.clone();
        let before = self.message_moderation.len();
        self.message_moderation.retain(|_k, v| v != "handled");
        let cleared = before - self.message_moderation.len();
        if cleared > 0
            && let Err(error) = self.persist_moderation_state()
        {
            self.message_moderation = previous;
            return Err(error);
        }
        Ok(cleared)
    }

    pub(crate) fn admin_handle_log(&mut self, log_id: &str) -> Result<(), String> {
        let key = format!("log:{}", log_id);
        let previous = self.message_moderation.get(&key).cloned();
        self.message_moderation.insert(key, "handled".to_string());
        if let Err(error) = self.persist_moderation_state() {
            let key = format!("log:{}", log_id);
            match previous {
                Some(status) => {
                    self.message_moderation.insert(key, status);
                }
                None => {
                    self.message_moderation.remove(&key);
                }
            }
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn admin_logs(&self) -> Vec<AdminLogEntry> {
        self.message_moderation
            .iter()
            .filter_map(|(k, v)| {
                k.strip_prefix("log:").map(|log_id| AdminLogEntry {
                    log_id: log_id.to_string(),
                    status: v.clone(),
                })
            })
            .collect()
    }

    pub(crate) fn admin_list_invites(&self) -> Vec<InviteCode> {
        self.invites.values().cloned().collect()
    }

    // ── Permission Groups ──

    pub(crate) fn persist_permission_groups(&self) -> Result<(), String> {
        #[derive(Serialize)]
        struct PermissionGroupState {
            groups: Vec<PermissionGroup>,
            assignments: Vec<(String, String)>,
        }
        let state = PermissionGroupState {
            groups: self.permission_groups.values().cloned().collect(),
            assignments: self
                .resident_permission_groups
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        };
        let bytes = serde_json::to_vec_pretty(&state)
            .map_err(|e| format!("encode permission groups: {e}"))?;
        atomic_write_file(&self.permission_groups_path, &bytes)
            .map_err(|e| format!("write permission groups: {e}"))
    }

    pub(crate) fn load_permission_groups(&mut self) -> Result<(), String> {
        if !self.permission_groups_path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&self.permission_groups_path)
            .map_err(|e| format!("read permission groups: {e}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        #[derive(Deserialize)]
        struct PermissionGroupState {
            groups: Vec<PermissionGroup>,
            assignments: Vec<(String, String)>,
        }
        let state: PermissionGroupState =
            serde_json::from_slice(&bytes).map_err(|e| format!("decode permission groups: {e}"))?;
        self.permission_groups = state
            .groups
            .into_iter()
            .map(|g| (g.id.clone(), g))
            .collect();
        self.resident_permission_groups = state.assignments.into_iter().collect();
        Ok(())
    }

    pub(crate) fn admin_create_permission_group(
        &mut self,
        actor_id: &str,
        name: &str,
        description: &str,
        capabilities: Vec<String>,
    ) -> Result<CreatePermissionGroupResponse, String> {
        let id = format!("pg-{}", Self::now_ms());
        let group = PermissionGroup {
            id: id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            capabilities,
            created_at_ms: Self::now_ms(),
            created_by: actor_id.to_string(),
        };
        self.permission_groups.insert(id.clone(), group.clone());
        if let Err(error) = self.persist_permission_groups() {
            self.permission_groups.remove(&id);
            return Err(error);
        }
        Ok(CreatePermissionGroupResponse { ok: true, group })
    }

    pub(crate) fn admin_list_permission_groups(&self) -> Vec<PermissionGroup> {
        self.permission_groups.values().cloned().collect()
    }

    pub(crate) fn admin_assign_permission_group(
        &mut self,
        resident_id: &str,
        permission_group_id: &str,
    ) -> Result<AssignPermissionGroupResponse, String> {
        let resident_id = resident_id.to_string();
        let previous = self
            .resident_permission_groups
            .insert(resident_id.clone(), permission_group_id.to_string());
        if let Err(error) = self.persist_permission_groups() {
            match previous {
                Some(group_id) => {
                    self.resident_permission_groups
                        .insert(resident_id.clone(), group_id);
                }
                None => {
                    self.resident_permission_groups.remove(&resident_id);
                }
            }
            return Err(error);
        }
        Ok(AssignPermissionGroupResponse {
            ok: true,
            resident_id,
            permission_group_id: permission_group_id.to_string(),
        })
    }

    pub(crate) fn super_admins() -> Vec<String> {
        std::env::var("LOBSTER_SUPER_ADMINS")
            .unwrap_or_else(|_| "admin_rsaga".into())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    pub(crate) fn resident_has_capability(&self, resident_id: &str, capability: &str) -> bool {
        // Super admins (configured via LOBSTER_SUPER_ADMINS) have all capabilities
        if Self::super_admins()
            .iter()
            .any(|admin| admin == resident_id)
        {
            return true;
        }
        // Check resident's assigned permission group
        if let Some(group_id) = self.resident_permission_groups.get(resident_id)
            && let Some(group) = self.permission_groups.get(group_id)
        {
            return group.capabilities.iter().any(|c| c == capability);
        }
        false
    }

    fn dev_auth_bypass_default() -> bool {
        std::env::var("LOBSTER_DEV_AUTH_BYPASS")
            .map(|v| v == "1")
            .unwrap_or(cfg!(test))
    }

    fn secure_session_storage_key_default(
        dev_auth_bypass: bool,
    ) -> Result<Option<SecureSessionStorageKey>, String> {
        match std::env::var("LOBSTER_SECURE_SESSION_MASTER_KEY") {
            Ok(secret) => SecureSessionStorageKey::from_secret(&secret).map(Some),
            Err(std::env::VarError::NotPresent) if dev_auth_bypass || cfg!(test) => {
                SecureSessionStorageKey::from_secret(
                    "lobster-insecure-development-session-key-0001",
                )
                .map(Some)
            }
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => {
                Err("LOBSTER_SECURE_SESSION_MASTER_KEY must be valid UTF-8".into())
            }
        }
    }

    pub(crate) fn dev_auth_bypass_enabled(&self) -> bool {
        self.dev_auth_bypass
    }

    /// Resolve sidecar credentials from `LOBSTER_AGENT_TOKENS`.
    ///
    /// The value is a comma-separated list of `agent:<id>=<token>` pairs. Only
    /// hashes are retained in memory, so the configured secrets are not exposed
    /// by debug output or accidental state persistence.
    fn agent_token_hashes_default() -> HashMap<String, String> {
        std::env::var("LOBSTER_AGENT_TOKENS")
            .ok()
            .into_iter()
            .flat_map(|raw| raw.split(',').map(str::to_owned).collect::<Vec<_>>())
            .filter_map(|entry| {
                let (agent_id, token) = entry.split_once('=')?;
                let agent_id = agent_id.trim();
                let token = token.trim();
                if !agent_id.starts_with("agent:") || agent_id.len() <= "agent:".len() {
                    return None;
                }
                if token.is_empty() {
                    return None;
                }
                Some((
                    agent_id.to_owned(),
                    Self::hash_registration_handle("agent-token", token),
                ))
            })
            .collect()
    }

    pub(crate) fn validate_agent_token(&self, agent_id: &str, token: &str) -> bool {
        let agent_id = agent_id.trim();
        let token = token.trim();
        if agent_id.is_empty() || token.is_empty() {
            return false;
        }
        self.agent_token_hashes
            .get(agent_id)
            .is_some_and(|expected| {
                Self::hash_registration_handle("agent-token", token) == *expected
            })
    }

    fn federation_token_hash_default() -> Option<String> {
        std::env::var("LOBSTER_GATEWAY_FEDERATION_TOKEN")
            .ok()
            .and_then(|token| {
                let token = token.trim();
                (!token.is_empty())
                    .then(|| Self::hash_registration_handle("gateway-federation-token", token))
            })
    }

    pub(crate) fn validate_federation_token(&self, token: &str) -> bool {
        let token = token.trim();
        if token.is_empty() {
            return false;
        }
        self.federation_token_hash.as_ref().is_some_and(|expected| {
            Self::hash_registration_handle("gateway-federation-token", token) == *expected
        })
    }

    #[cfg(test)]
    pub(crate) fn set_dev_auth_bypass_for_tests(&mut self, enabled: bool) {
        self.dev_auth_bypass = enabled;
    }

    #[cfg(test)]
    pub(crate) fn set_agent_token_for_tests(&mut self, agent_id: &str, token: &str) {
        self.agent_token_hashes.insert(
            agent_id.trim().to_owned(),
            Self::hash_registration_handle("agent-token", token.trim()),
        );
    }

    #[cfg(test)]
    pub(crate) fn set_federation_token_for_tests(&mut self, token: &str) {
        self.federation_token_hash = Some(Self::hash_registration_handle(
            "gateway-federation-token",
            token.trim(),
        ));
    }

    // ── Audit Log ──

    pub(crate) fn log_audit_event(
        &mut self,
        actor_id: &str,
        action: &str,
        target: &str,
        reason: Option<&str>,
    ) {
        let sequence = self.audit_counter;
        self.audit_counter = self.audit_counter.saturating_add(1);
        let event = AuditEvent {
            event_id: format!("audit-{}-{sequence}", Self::now_ms()),
            actor_id: actor_id.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            reason: reason.map(|r| r.to_string()),
            timestamp_ms: Self::now_ms(),
        };
        self.audit_events.push(event);
        let _ = self.persist_audit_log();
    }

    pub(crate) fn admin_list_audit_events(&self, limit: usize) -> AuditLogResponse {
        let total = self.audit_events.len();
        let events = self
            .audit_events
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect();
        AuditLogResponse { events, total }
    }

    fn persist_audit_log(&self) -> Result<(), String> {
        let lines: Vec<String> = self
            .audit_events
            .iter()
            .map(|e| serde_json::to_string(e).unwrap_or_default())
            .collect();
        let bytes = lines.join("\n") + if lines.is_empty() { "" } else { "\n" };
        atomic_write_file(&self.audit_log_path, bytes.as_bytes())
            .map_err(|e| format!("write audit log: {e}"))
    }

    fn load_audit_log(&mut self) -> Result<(), String> {
        if !self.audit_log_path.exists() {
            return Ok(());
        }
        let raw = std::fs::read_to_string(&self.audit_log_path)
            .map_err(|e| format!("read audit log: {e}"))?;
        self.audit_events = raw
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<AuditEvent>(line).ok())
            .collect();
        self.audit_counter = self.audit_events.len() as u64;
        Ok(())
    }

    pub(crate) fn check_rate_limit(&mut self, sender_id: &str, max_per_minute: u32) -> Option<i64> {
        let now_ms = Self::now_ms();
        let window_ms = 60_000;
        let entry = self
            .rate_limits
            .entry(sender_id.to_string())
            .or_insert_with(|| RateLimitWindow {
                window_start_ms: now_ms,
                count: 0,
            });
        if now_ms - entry.window_start_ms > window_ms {
            entry.window_start_ms = now_ms;
            entry.count = 0;
        }
        if entry.count >= max_per_minute {
            let retry_ms = window_ms - (now_ms - entry.window_start_ms);
            return Some(retry_ms.max(1_000));
        }
        entry.count += 1;
        None
    }

    pub(crate) fn persist_presence_state(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self.presence)
            .map_err(|error| format!("encode presence state failed: {error}"))?;
        atomic_write_file(&self.presence_path, &bytes)
            .map_err(|error| format!("write presence state failed: {error}"))
    }

    pub(crate) fn load_presence_state(&mut self) -> Result<(), String> {
        if !self.presence_path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&self.presence_path)
            .map_err(|error| format!("read presence state failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        self.presence = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode presence state failed: {error}"))?;
        Ok(())
    }

    pub(crate) fn persist_unread_state(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self.unread)
            .map_err(|error| format!("encode unread state failed: {error}"))?;
        atomic_write_file(&self.unread_path, &bytes)
            .map_err(|error| format!("write unread state failed: {error}"))
    }

    pub(crate) fn load_unread_state(&mut self) -> Result<(), String> {
        if !self.unread_path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&self.unread_path)
            .map_err(|error| format!("read unread state failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        self.unread = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode unread state failed: {error}"))?;
        Ok(())
    }

    pub(crate) fn persist_moderation_state(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self.message_moderation)
            .map_err(|error| format!("encode moderation state failed: {error}"))?;
        atomic_write_file(&self.moderation_state_path, &bytes)
            .map_err(|error| format!("write moderation state failed: {error}"))
    }

    pub(crate) fn load_device_state(&mut self) -> Result<(), String> {
        if !self.device_state_path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&self.device_state_path)
            .map_err(|error| format!("read device state failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        #[derive(Deserialize)]
        struct DeviceStateFile {
            #[serde(default)]
            devices: Vec<DeviceRecord>,
            #[serde(default)]
            bindings: HashMap<String, String>,
        }
        let state: DeviceStateFile = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode device state failed: {error}"))?;
        for device in state.devices {
            self.allowed_devices.insert(device.address.clone(), device);
        }
        self.device_bindings = state.bindings;
        Ok(())
    }

    pub(crate) fn persist_device_state(&self) -> Result<(), String> {
        #[derive(Serialize)]
        struct DeviceStateFile {
            devices: Vec<DeviceRecord>,
            bindings: HashMap<String, String>,
        }
        let devices: Vec<DeviceRecord> = self.allowed_devices.values().cloned().collect();
        let state = DeviceStateFile {
            devices,
            bindings: self.device_bindings.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&state)
            .map_err(|error| format!("encode device state failed: {error}"))?;
        atomic_write_file(&self.device_state_path, &bytes)
            .map_err(|error| format!("write device state failed: {error}"))
    }

    pub(crate) fn admin_add_device(
        &mut self,
        address: String,
        label: String,
        actor_id: String,
    ) -> Result<DeviceRecord, String> {
        let normalized = Self::normalize_device_physical_address(&address)
            .ok_or_else(|| "invalid device address format".to_string())?;
        let now = Self::now_ms();
        let record = DeviceRecord {
            address: normalized.clone(),
            label: if label.is_empty() {
                "未命名设备".to_string()
            } else {
                label
            },
            added_at_ms: now,
            added_by: actor_id,
            blocked: false,
            bound_resident_id: self.device_bindings.get(&normalized).cloned(),
        };
        let previous = self
            .allowed_devices
            .insert(normalized.clone(), record.clone());
        if let Err(error) = self.persist_device_state() {
            match previous {
                Some(previous) => {
                    self.allowed_devices.insert(normalized, previous);
                }
                None => {
                    self.allowed_devices.remove(&normalized);
                }
            }
            return Err(format!("persist device state failed: {error}"));
        }
        Ok(record)
    }

    pub(crate) fn admin_remove_device(&mut self, address: &str) -> Result<(), String> {
        let normalized = Self::normalize_device_physical_address(address)
            .ok_or_else(|| "invalid device address format".to_string())?;
        let previous_device = self.allowed_devices.remove(&normalized);
        let previous_binding = self.device_bindings.remove(&normalized);
        if let Err(error) = self.persist_device_state() {
            if let Some(previous) = previous_device {
                self.allowed_devices.insert(normalized.clone(), previous);
            }
            if let Some(previous) = previous_binding {
                self.device_bindings.insert(normalized, previous);
            }
            return Err(format!("persist device state failed: {error}"));
        }
        Ok(())
    }

    pub(crate) fn admin_block_device(&mut self, address: &str) -> Result<(), String> {
        let normalized = Self::normalize_device_physical_address(address)
            .ok_or_else(|| "invalid device address format".to_string())?;
        let previous = self.allowed_devices.get(&normalized).cloned();
        if let Some(record) = self.allowed_devices.get_mut(&normalized) {
            record.blocked = true;
        }
        if let Err(error) = self.persist_device_state() {
            match previous {
                Some(previous) => {
                    self.allowed_devices.insert(normalized, previous);
                }
                None => {
                    self.allowed_devices.remove(&normalized);
                }
            }
            return Err(format!("persist device state failed: {error}"));
        }
        Ok(())
    }

    pub(crate) fn admin_unblock_device(&mut self, address: &str) -> Result<(), String> {
        let normalized = Self::normalize_device_physical_address(address)
            .ok_or_else(|| "invalid device address format".to_string())?;
        let previous = self.allowed_devices.get(&normalized).cloned();
        if let Some(record) = self.allowed_devices.get_mut(&normalized) {
            record.blocked = false;
        }
        if let Err(error) = self.persist_device_state() {
            match previous {
                Some(previous) => {
                    self.allowed_devices.insert(normalized, previous);
                }
                None => {
                    self.allowed_devices.remove(&normalized);
                }
            }
            return Err(format!("persist device state failed: {error}"));
        }
        Ok(())
    }

    pub(crate) fn admin_list_devices(&self) -> Vec<DeviceRecord> {
        self.allowed_devices.values().cloned().collect()
    }

    pub(crate) fn bind_device_to_resident(
        &mut self,
        device_address: &str,
        resident_id: &str,
    ) -> Result<(), String> {
        let previous_binding = self
            .device_bindings
            .insert(device_address.to_string(), resident_id.to_string());
        let previous_record = self.allowed_devices.get(device_address).cloned();
        if let Some(record) = self.allowed_devices.get_mut(device_address) {
            record.bound_resident_id = Some(resident_id.to_string());
        }
        if let Err(error) = self.persist_device_state() {
            match previous_binding {
                Some(previous) => {
                    self.device_bindings
                        .insert(device_address.to_string(), previous);
                }
                None => {
                    self.device_bindings.remove(device_address);
                }
            }
            match previous_record {
                Some(previous) => {
                    self.allowed_devices
                        .insert(device_address.to_string(), previous);
                }
                None => {
                    self.allowed_devices.remove(device_address);
                }
            }
            return Err(format!("persist device state failed: {error}"));
        }
        Ok(())
    }

    pub(crate) fn load_moderation_state(&mut self) -> Result<(), String> {
        if !self.moderation_state_path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&self.moderation_state_path)
            .map_err(|error| format!("read moderation state failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        self.message_moderation = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode moderation state failed: {error}"))?;
        Ok(())
    }

    pub(crate) fn unread_count(
        &self,
        resident_id: &IdentityId,
        conversation_id: &ConversationId,
    ) -> usize {
        let key = format!("{}:{}", resident_id.0, conversation_id.0);
        self.unread.get(&key).copied().unwrap_or(0)
    }

    pub(crate) fn enrich_resident_directory(&self) -> Vec<ResidentDirectoryEntry> {
        let snapshot = self.governance_snapshot();
        let mut residents = Self::resident_directory(&snapshot);
        let online_threshold_ms = 120_000;
        for entry in &mut residents {
            let last_seen = self.presence.get(&entry.resident_id).copied();
            entry.online = last_seen.map(|ts| Self::now_ms() - ts < online_threshold_ms);
            entry.last_seen_at_ms = last_seen;
            entry.avatar_id = Some(format!("avatar:{}", entry.resident_id));
            entry.nickname = self
                .registrations
                .iter()
                .find(|r| r.resident_id.0 == entry.resident_id)
                .and_then(|r| r.nickname.clone());
            let personal_room =
                self.timeline_store
                    .active_conversations()
                    .into_iter()
                    .find(|conversation| {
                        Self::personal_room_owner(conversation)
                            .is_some_and(|owner| owner.0 == entry.resident_id)
                    });
            entry.personal_room_id = personal_room.map(|c| c.conversation_id.0.clone());
        }
        residents
    }

    pub(crate) fn enrich_resident_directory_for_viewer(
        &self,
        viewer: Option<&IdentityId>,
    ) -> Vec<ResidentDirectoryEntry> {
        let mut residents = self.enrich_resident_directory();
        let Some(viewer) = viewer else {
            return residents;
        };

        for entry in &mut residents {
            if entry.resident_id == viewer.0 {
                continue;
            }
            let peer = IdentityId(entry.resident_id.clone());
            let key = Self::resident_relationship_key(viewer, &peer);
            if let Some(record) = self.resident_relationships.get(&key) {
                entry.relationship_state = Some(record.state);
                entry.relationship_requested_by = Some(record.requested_by.clone());
            }
        }
        residents
    }

    pub(crate) fn admin_summary(&self) -> AdminSummaryResponse {
        let residents = self.enrich_resident_directory();
        let conversations = self.timeline_store.active_conversations();
        let total_messages: usize = conversations
            .iter()
            .map(|conv| {
                self.timeline_store
                    .recent_messages(&conv.conversation_id, 1)
                    .len()
            })
            .sum();
        let online_count = residents
            .iter()
            .filter(|entry| entry.online == Some(true))
            .count();
        let shell_state = self.shell_state_for_viewer(None);
        AdminSummaryResponse {
            resident_count: residents.len(),
            room_count: conversations.len(),
            message_count: total_messages,
            online_count,
            gateway_uptime_ms: Self::now_ms() - self.started_at_ms,
            state_version: shell_state.state_version,
        }
    }

    pub(crate) fn admin_conversations(&self) -> Vec<AdminConversationSummary> {
        let conversations = self.timeline_store.active_conversations();
        let snapshot = self.governance_snapshot();
        conversations
            .into_iter()
            .map(|conv| {
                let message_count = self
                    .timeline_store
                    .recent_messages(&conv.conversation_id, 500)
                    .len();
                let title = Self::room_title(&conv.conversation_id);
                let is_frozen = snapshot
                    .public_rooms
                    .iter()
                    .any(|room| room.room_id.0 == conv.conversation_id.0 && room.frozen);
                AdminConversationSummary {
                    conversation_id: conv.conversation_id.0.clone(),
                    kind: format!("{:?}", conv.kind).to_lowercase(),
                    scope: format!("{:?}", conv.scope).to_lowercase(),
                    title,
                    participant_count: conv.participants.len(),
                    message_count,
                    is_frozen,
                    created_at_ms: conv.created_at_ms,
                    last_active_at_ms: conv.last_active_at_ms,
                }
            })
            .collect()
    }

    pub(crate) fn admin_message_audit(
        &self,
        conversation_id: &ConversationId,
        limit: usize,
    ) -> Option<AdminMessageAudit> {
        let now_ms = Self::now_ms();
        let messages = self.timeline_store.recent_messages(conversation_id, limit);
        let total_count = self.timeline_store.archived_count(conversation_id);
        let shell_messages: Vec<ShellRoomMessage> = messages
            .into_iter()
            .map(|entry| {
                let mod_key = format!(
                    "{}:{}",
                    entry.envelope.conversation_id.0, entry.envelope.message_id.0
                );
                let moderation_status = self.message_moderation.get(&mod_key).cloned();
                ShellRoomMessage {
                    message_id: entry.envelope.message_id.0,
                    reply_to_message_id: entry.envelope.reply_to_message_id.map(|m| m.0),
                    is_recalled: entry.recalled_at_ms.is_some(),
                    recalled_by: entry.recalled_by.map(|id| id.0),
                    recalled_at_ms: entry.recalled_at_ms,
                    is_edited: entry.edited_at_ms.is_some(),
                    edited_by: entry.edited_by.map(|id| id.0),
                    edited_at_ms: entry.edited_at_ms,
                    sender: entry.envelope.sender.0,
                    timestamp_ms: entry.envelope.timestamp_ms,
                    timestamp_label: Self::relative_label(now_ms, entry.envelope.timestamp_ms),
                    delivery_status: "delivered".to_string(),
                    text: if entry.recalled_at_ms.is_some() {
                        "消息已撤回".into()
                    } else {
                        entry.envelope.body.plain_text
                    },
                    moderation_status,
                }
            })
            .collect();
        let returned_count = shell_messages.len();
        Some(AdminMessageAudit {
            conversation_id: conversation_id.0.clone(),
            messages: shell_messages,
            total_count,
            returned_count,
        })
    }

    pub(crate) fn search_messages_for_viewer(
        &self,
        viewer: &IdentityId,
        query: &str,
        room_id: Option<&str>,
        limit: usize,
    ) -> Vec<ShellRoomMessage> {
        let now_ms = Self::now_ms();
        let query_lower = query.to_lowercase();
        let conversations = self.shell_visible_conversations_for_viewer(Some(viewer));

        let mut results: Vec<ShellRoomMessage> = conversations
            .into_iter()
            .filter(|conv| {
                room_id.is_none_or(|id| conv.conversation_id.0 == id)
                    && self.personal_room_messages_visible_to_viewer(conv, Some(viewer))
            })
            .flat_map(|conv| {
                let messages = self
                    .timeline_store
                    .recent_messages(&conv.conversation_id, limit);
                messages
                    .into_iter()
                    .filter(|entry| {
                        entry
                            .envelope
                            .body
                            .plain_text
                            .to_lowercase()
                            .contains(&query_lower)
                    })
                    .map(|entry| {
                        let mod_key = format!(
                            "{}:{}",
                            entry.envelope.conversation_id.0, entry.envelope.message_id.0
                        );
                        let moderation_status = self.message_moderation.get(&mod_key).cloned();
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
                            timestamp_label: Self::relative_label(
                                now_ms,
                                entry.envelope.timestamp_ms,
                            ),
                            delivery_status: "delivered".into(),
                            text: if entry.recalled_at_ms.is_some() {
                                "消息已撤回".into()
                            } else {
                                entry.envelope.body.plain_text
                            },
                            moderation_status,
                        }
                    })
            })
            .collect();

        results.sort_by_key(|message| std::cmp::Reverse(message.timestamp_ms));
        results.truncate(limit);
        results
    }

    pub(crate) fn admin_residents(&self) -> Vec<AdminResidentDetail> {
        let is_online = |rid: &str| -> bool {
            self.presence
                .get(rid)
                .map(|last| Self::now_ms() - last < 120_000)
                .unwrap_or(false)
        };
        let snapshot = self.governance_snapshot();
        let sanctions = &self.resident_sanctions;
        let mut by_resident: HashMap<String, AdminResidentDetail> = HashMap::new();

        let city_slugs: HashMap<String, String> = snapshot
            .cities
            .iter()
            .map(|c| (c.profile.city_id.0.clone(), c.profile.slug.clone()))
            .collect();

        // Registrations are the admin truth for account review. Seed them before
        // memberships so a verified account is visible even before joining a city.
        for registration in &self.registrations {
            let rid = registration.resident_id.0.clone();
            let resident_sanctions: Vec<AdminSanctionSummary> = sanctions
                .iter()
                .filter(|s| s.resident_id.0 == rid)
                .map(|s| AdminSanctionSummary {
                    sanction_id: s.sanction_id.clone(),
                    reason: s.reason.clone(),
                    status: format!("{:?}", s.status).to_lowercase(),
                    issued_at_ms: s.issued_at_ms,
                    lifted_at_ms: s.lifted_at_ms,
                })
                .collect();
            let is_banned = registration.state == ResidentRegistrationState::Suspended
                || resident_sanctions
                    .iter()
                    .any(|s| s.status == "active" && s.lifted_at_ms.is_none());
            by_resident.insert(
                rid.clone(),
                AdminResidentDetail {
                    resident_id: rid.clone(),
                    nickname: registration.nickname.clone(),
                    email_masked: Some(Self::mask_email(&registration.email)),
                    registration_state: Some(format!("{:?}", registration.state).to_lowercase()),
                    created_at_ms: Some(registration.created_at_ms),
                    verified_at_ms: Some(registration.verified_at_ms),
                    last_login_at_ms: Some(registration.last_login_at_ms),
                    roles: Vec::new(),
                    active_cities: Vec::new(),
                    pending_cities: Vec::new(),
                    sanctions: resident_sanctions,
                    is_banned,
                    online: is_online(&rid),
                    last_seen_at_ms: self.presence.get(&rid).copied(),
                    avatar_id: None,
                },
            );
        }

        for member in &self.memberships {
            let rid = &member.resident_id.0;
            let city_slug = city_slugs
                .get(&member.city_id.0)
                .cloned()
                .unwrap_or_else(|| member.city_id.0.clone());
            let entry = by_resident.entry(rid.clone()).or_insert_with(|| {
                let resident_sanctions: Vec<AdminSanctionSummary> = sanctions
                    .iter()
                    .filter(|s| s.resident_id.0 == *rid)
                    .map(|s| AdminSanctionSummary {
                        sanction_id: s.sanction_id.clone(),
                        reason: s.reason.clone(),
                        status: format!("{:?}", s.status).to_lowercase(),
                        issued_at_ms: s.issued_at_ms,
                        lifted_at_ms: s.lifted_at_ms,
                    })
                    .collect();
                let is_banned = resident_sanctions
                    .iter()
                    .any(|s| s.status == "active" && s.lifted_at_ms.is_none());
                let nickname = self
                    .registrations
                    .iter()
                    .find(|r| r.resident_id.0 == *rid)
                    .and_then(|r| r.nickname.clone());
                AdminResidentDetail {
                    resident_id: rid.clone(),
                    nickname,
                    email_masked: None,
                    registration_state: None,
                    created_at_ms: None,
                    verified_at_ms: None,
                    last_login_at_ms: None,
                    roles: Vec::new(),
                    active_cities: Vec::new(),
                    pending_cities: Vec::new(),
                    sanctions: resident_sanctions,
                    is_banned,
                    online: is_online(rid),
                    last_seen_at_ms: self.presence.get(rid).copied(),
                    avatar_id: None,
                }
            });
            match member.state {
                MembershipState::Active | MembershipState::Muted => {
                    if !entry.active_cities.contains(&city_slug) {
                        entry.active_cities.push(city_slug.clone());
                    }
                }
                MembershipState::PendingApproval if !entry.pending_cities.contains(&city_slug) => {
                    entry.pending_cities.push(city_slug.clone());
                }
                _ => {}
            }
            let role_str = format!("{:?}", member.role).to_lowercase();
            if !entry.roles.contains(&role_str) {
                entry.roles.push(role_str);
            }
        }

        let mut result: Vec<_> = by_resident.into_values().collect();
        result.sort_by(|a, b| a.resident_id.cmp(&b.resident_id));
        result
    }

    pub(crate) fn admin_ban_resident(
        &mut self,
        resident_id: &str,
        reason: &str,
    ) -> Result<(), String> {
        if reason.trim().is_empty() {
            return Err("ban reason required".into());
        }
        let rid = IdentityId(resident_id.to_string());
        let sanction = WorldResidentSanction {
            sanction_id: format!("resident-sanction:{}", self.next_message_id()),
            resident_id: rid,
            city_id: None,
            report_id: None,
            reason: reason.trim().into(),
            portability_revoked: true,
            status: WorldResidentSanctionStatus::Active,
            issued_by: IdentityId("admin".into()),
            issued_at_ms: Self::now_ms(),
            lifted_at_ms: None,
        };
        self.resident_sanctions.push(sanction);
        if let Err(error) = self.persist_governance_state() {
            self.resident_sanctions.pop();
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn revoke_sanction(&mut self, sanction_id: &str) -> Result<(), String> {
        let index = self
            .resident_sanctions
            .iter()
            .position(|sanction| sanction.sanction_id == sanction_id)
            .ok_or_else(|| format!("sanction not found: {sanction_id}"))?;
        if self.resident_sanctions[index].status == WorldResidentSanctionStatus::Lifted {
            return Err("sanction already lifted".into());
        }
        let previous = self.resident_sanctions[index].clone();
        self.resident_sanctions[index].status = WorldResidentSanctionStatus::Lifted;
        self.resident_sanctions[index].lifted_at_ms = Some(Self::now_ms());
        if let Err(error) = self.persist_governance_state() {
            self.resident_sanctions[index] = previous;
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn admin_set_nickname(
        &mut self,
        resident_id: &str,
        nickname: Option<&str>,
    ) -> Result<bool, String> {
        let index = self
            .registrations
            .iter()
            .position(|registration| registration.resident_id.0 == resident_id);
        match index {
            Some(index) => {
                let previous = self.registrations[index].nickname.clone();
                self.registrations[index].nickname = nickname.map(|n| n.to_string());
                if let Err(error) = self.persist_auth_state() {
                    self.registrations[index].nickname = previous;
                    return Err(error);
                }
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub(crate) fn shell_set_nickname(
        &mut self,
        resident_id: &str,
        nickname: Option<&str>,
    ) -> Result<(bool, Option<String>), String> {
        let idx = self
            .registrations
            .iter()
            .position(|r| r.resident_id.0 == resident_id);
        match idx {
            Some(i) => {
                let previous = self.registrations[i].nickname.clone();
                self.registrations[i].nickname = nickname.map(|n| n.to_string());
                let result = self.registrations[i].nickname.clone();
                if let Err(error) = self.persist_auth_state() {
                    self.registrations[i].nickname = previous;
                    return Err(error);
                }
                Ok((true, result))
            }
            None => Ok((false, None)),
        }
    }

    pub(crate) fn admin_unban_resident(&mut self, resident_id: &str) -> Result<usize, String> {
        let now_ms = Self::now_ms();
        let mut count = 0;
        let previous_sanctions = self.resident_sanctions.clone();
        for sanction in &mut self.resident_sanctions {
            if sanction.resident_id.0 == resident_id
                && sanction.status == WorldResidentSanctionStatus::Active
            {
                sanction.status = WorldResidentSanctionStatus::Lifted;
                sanction.lifted_at_ms = Some(now_ms);
                count += 1;
            }
        }
        if count > 0
            && let Err(error) = self.persist_governance_state()
        {
            self.resident_sanctions = previous_sanctions;
            return Err(error);
        }
        Ok(count)
    }

    pub(crate) fn admin_rooms_detail(&self) -> Vec<AdminRoomDetail> {
        let snapshot = self.governance_snapshot();
        self.timeline_store
            .active_conversations()
            .into_iter()
            .map(|conv| {
                let message_count = self
                    .timeline_store
                    .recent_messages(&conv.conversation_id, 500)
                    .len();
                let is_frozen = snapshot
                    .public_rooms
                    .iter()
                    .any(|room| room.room_id.0 == conv.conversation_id.0 && room.frozen);
                let title = Self::room_title(&conv.conversation_id);
                AdminRoomDetail {
                    id: conv.conversation_id.0,
                    kind: format!("{:?}", conv.kind).to_lowercase(),
                    title,
                    participant_count: conv.participants.len(),
                    message_count,
                    is_frozen,
                    has_scene: conv.scene.is_some(),
                    created_at_ms: conv.created_at_ms,
                    last_active_at_ms: conv.last_active_at_ms,
                }
            })
            .collect()
    }

    pub(crate) fn admin_freeze_room(&mut self, room_id: &str) -> Result<bool, String> {
        let Some(index) = self
            .public_rooms
            .iter()
            .position(|room| room.room_id.0 == room_id)
        else {
            return Err(format!("room not found: {room_id}"));
        };
        if self.public_rooms[index].frozen {
            return Err("room already frozen".into());
        }

        self.public_rooms[index].frozen = true;
        if let Err(error) = self.persist_governance_state() {
            self.public_rooms[index].frozen = false;
            return Err(error);
        }
        Ok(true)
    }

    pub(crate) fn admin_unfreeze_room(&mut self, room_id: &str) -> Result<bool, String> {
        let Some(index) = self
            .public_rooms
            .iter()
            .position(|room| room.room_id.0 == room_id)
        else {
            return Err(format!("room not found: {room_id}"));
        };
        if !self.public_rooms[index].frozen {
            return Err("room not frozen".into());
        }

        self.public_rooms[index].frozen = false;
        if let Err(error) = self.persist_governance_state() {
            self.public_rooms[index].frozen = true;
            return Err(error);
        }
        Ok(true)
    }

    pub(crate) fn admin_get_config(&self) -> HashMap<String, String> {
        self.app_config.clone()
    }

    pub(crate) fn persist_app_config(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self.app_config)
            .map_err(|error| format!("encode app config failed: {error}"))?;
        atomic_write_file(&self.app_config_path, &bytes)
            .map_err(|error| format!("write app config failed: {error}"))
    }

    pub(crate) fn load_app_config(&mut self) -> Result<(), String> {
        if !self.app_config_path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&self.app_config_path)
            .map_err(|error| format!("read app config failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        self.app_config = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode app config failed: {error}"))?;
        Ok(())
    }

    pub(crate) fn admin_set_config(
        &mut self,
        updates: HashMap<String, String>,
    ) -> Result<(), String> {
        let previous_config = self.app_config.clone();
        for (key, value) in updates {
            self.app_config.insert(key, value);
        }
        if let Err(error) = self.persist_app_config() {
            self.app_config = previous_config;
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn admin_moderate_message(
        &mut self,
        message_id: &str,
        conversation_id: &str,
        action: &str,
    ) -> Result<(), String> {
        if !matches!(action, "approved" | "blocked" | "handled") {
            return Err(format!("invalid action: {action}"));
        }
        let messages = self
            .timeline_store
            .recent_messages(&ConversationId(conversation_id.to_string()), 500);
        let found = messages
            .iter()
            .any(|m| m.envelope.message_id.0 == message_id);
        if !found {
            return Err("message not found".into());
        }
        let previous = self
            .message_moderation
            .insert(message_id.to_string(), action.to_string());
        if let Err(error) = self.persist_moderation_state() {
            match previous {
                Some(status) => {
                    self.message_moderation
                        .insert(message_id.to_string(), status);
                }
                None => {
                    self.message_moderation.remove(message_id);
                }
            }
            return Err(format!("persist moderation state failed: {error}"));
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn admin_message_moderation_status(&self, message_id: &str) -> Option<&str> {
        self.message_moderation.get(message_id).map(|s| s.as_str())
    }

    pub(crate) fn validate_scene_config(
        &self,
        _conversation_id: &ConversationId,
        image_layer: &Option<SceneImageLayer>,
        hotspot_layer: &Option<SceneHotspotLayer>,
    ) -> SceneValidateResponse {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        if let Some(layer) = image_layer
            && layer.asset_hint.trim().is_empty()
        {
            errors.push("image_layer.asset_hint must not be empty".into());
        }
        if let Some(layer) = hotspot_layer {
            if layer.hotspots.is_empty() {
                warnings.push("hotspot_layer has no hotspots".into());
            }
            for (i, hotspot) in layer.hotspots.iter().enumerate() {
                if hotspot.x_permyriad > 10000 || hotspot.y_permyriad > 10000 {
                    errors.push(format!(
                        "hotspot[{}] coords out of range: ({},{})",
                        i, hotspot.x_permyriad, hotspot.y_permyriad
                    ));
                }
                if hotspot.label.trim().is_empty() {
                    warnings.push(format!("hotspot[{}] has empty label", i));
                }
            }
        }
        SceneValidateResponse {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    pub(crate) fn resident_directory(snapshot: &GovernanceSnapshot) -> Vec<ResidentDirectoryEntry> {
        let mut by_resident = HashMap::<String, ResidentDirectoryEntry>::new();
        let city_labels = snapshot
            .cities
            .iter()
            .map(|city| (city.profile.city_id.0.clone(), city.profile.slug.clone()))
            .collect::<HashMap<_, _>>();

        for membership in &snapshot.memberships {
            let entry = by_resident
                .entry(membership.resident_id.0.clone())
                .or_insert_with(|| ResidentDirectoryEntry {
                    resident_id: membership.resident_id.0.clone(),
                    active_cities: Vec::new(),
                    pending_cities: Vec::new(),
                    roles: Vec::new(),
                    online: None,
                    last_seen_at_ms: None,
                    avatar_id: None,
                    personal_room_id: None,
                    relationship_state: None,
                    relationship_requested_by: None,
                    nickname: None,
                });
            let city_label = city_labels
                .get(&membership.city_id.0)
                .cloned()
                .unwrap_or_else(|| membership.city_id.0.clone());
            match membership.state {
                MembershipState::Active => {
                    if !entry.active_cities.contains(&city_label) {
                        entry.active_cities.push(city_label);
                    }
                }
                MembershipState::PendingApproval => {
                    if !entry.pending_cities.contains(&city_label) {
                        entry.pending_cities.push(city_label);
                    }
                }
                MembershipState::Muted | MembershipState::Suspended | MembershipState::Removed => {}
            }
            let role_label = format!("{:?}", membership.role);
            if !entry.roles.contains(&role_label) {
                entry.roles.push(role_label);
            }
        }

        let mut residents = by_resident.into_values().collect::<Vec<_>>();
        residents.sort_by_key(|entry| entry.resident_id.clone());
        for entry in &mut residents {
            entry.active_cities.sort();
            entry.pending_cities.sort();
            entry.roles.sort();
        }
        residents
    }

    pub(crate) fn default_world() -> WorldProfile {
        WorldProfile {
            world_id: WorldId("world:lobster".into()),
            title: "Lobster World".into(),
            portable_identity_required: true,
            allows_cross_city_private_messages: true,
        }
    }

    pub(crate) fn default_city_features() -> CityFeatureFlags {
        CityFeatureFlags {
            local_search: true,
            ai_sidecar: true,
            personal_bots: true,
            city_bots: true,
            room_scene_bots: true,
            commerce_bots: false,
            room_indexing: true,
            store_history: true,
        }
    }

    pub(crate) fn default_city_retention_policy() -> CityRetentionPolicy {
        CityRetentionPolicy {
            active_window_hours: 24,
            short_window_store_hours: 72,
            local_archive_days: Some(30),
        }
    }

    pub(crate) fn default_city_scene(slug: &str, title: &str) -> SceneMetadata {
        let landmarks = vec![
            SceneLandmark {
                slot_id: "lord-hall".into(),
                label: "城主府".into(),
                sprite_hint: "lord-hall".into(),
                interaction_hint: "查看治理与公告".into(),
            },
            SceneLandmark {
                slot_id: "resident-quarter".into(),
                label: "居民区".into(),
                sprite_hint: "resident-quarter".into(),
                interaction_hint: "浏览活跃居民与房间".into(),
            },
            SceneLandmark {
                slot_id: "portal".into(),
                label: "世界传送阵".into(),
                sprite_hint: "world-portal".into(),
                interaction_hint: "前往世界广场或其他城市".into(),
            },
        ];
        SceneMetadata {
            scope: SceneScope::City,
            render_style: SceneRenderStyle::SfcPixel,
            title_banner: Some(title.into()),
            background_preset: format!("city-{slug}"),
            ambiance: "像素城邦、公共广场与世界入口".into(),
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
            image_layer: Some(Self::scene_image_layer(format!("city-{slug}"), true)),
            hotspot_layer: Some(Self::scene_hotspot_layer("city-hotspots", true, &landmarks)),
            landmarks,
        }
    }

    pub(crate) fn default_public_room_scene(
        city_slug: &str,
        room_slug: &str,
        title: &str,
    ) -> SceneMetadata {
        let landmarks = vec![
            SceneLandmark {
                slot_id: "bulletin".into(),
                label: "公告牌".into(),
                sprite_hint: "bulletin-board".into(),
                interaction_hint: "查看固定消息与任务".into(),
            },
            SceneLandmark {
                slot_id: "chat-floor".into(),
                label: "会话区".into(),
                sprite_hint: "chat-floor".into(),
                interaction_hint: "阅读和发送频道消息".into(),
            },
        ];
        SceneMetadata {
            scope: SceneScope::PublicRoom,
            render_style: SceneRenderStyle::SfcPixel,
            title_banner: Some(title.into()),
            background_preset: format!("public-room-{city_slug}-{room_slug}"),
            ambiance: "公共频道、公告板与像素座位区".into(),
            owner_editable: true,
            avatar_editable: true,
            primary_avatar: None,
            assistant_slots: vec![AgentSceneSlot {
                slot_id: "public-room-host".into(),
                display_name: "频道主持".into(),
                scope: AgentScope::Room,
                use_cases: vec![AgentUseCase::Caretaking, AgentUseCase::Research],
                appearance_hint: "pixel-room-host".into(),
                can_leave_messages: true,
                can_edit_scene: false,
                can_trade_goods: false,
            }],
            image_layer: Some(Self::scene_image_layer(
                format!("public-room-{city_slug}-{room_slug}"),
                true,
            )),
            hotspot_layer: Some(Self::scene_hotspot_layer(
                "public-room-hotspots",
                true,
                &landmarks,
            )),
            landmarks,
        }
    }

    pub(crate) fn default_direct_scene(participants: &[IdentityId]) -> SceneMetadata {
        let display_name = participants
            .first()
            .map(|item| item.0.clone())
            .unwrap_or_else(|| "来访者".into());
        let landmarks = vec![
            SceneLandmark {
                slot_id: "desk".into(),
                label: "工作台".into(),
                sprite_hint: "desk-crt".into(),
                interaction_hint: "处理任务与草稿".into(),
            },
            SceneLandmark {
                slot_id: "sofa".into(),
                label: "会客沙发".into(),
                sprite_hint: "cozy-sofa".into(),
                interaction_hint: "进入私聊氛围区".into(),
            },
        ];
        SceneMetadata {
            scope: SceneScope::DirectRoom,
            render_style: SceneRenderStyle::SfcPixel,
            title_banner: Some("个人房间".into()),
            background_preset: "private-room-loft".into(),
            ambiance: "木地板、工作台、沙发与像素人物".into(),
            owner_editable: true,
            avatar_editable: true,
            primary_avatar: Some(PixelAvatarProfile {
                avatar_id: format!("avatar:{display_name}"),
                display_name,
                archetype: "pixel-resident".into(),
                palette_hint: "warm-amber".into(),
                accessory_hint: Some("徽章".into()),
            }),
            assistant_slots: vec![
                AgentSceneSlot {
                    slot_id: "room-caretaker".into(),
                    display_name: "看家助手".into(),
                    scope: AgentScope::Room,
                    use_cases: vec![AgentUseCase::Caretaking],
                    appearance_hint: "pixel-room-caretaker".into(),
                    can_leave_messages: true,
                    can_edit_scene: false,
                    can_trade_goods: false,
                },
                AgentSceneSlot {
                    slot_id: "room-decorator".into(),
                    display_name: "装修助手".into(),
                    scope: AgentScope::Room,
                    use_cases: vec![AgentUseCase::Decoration],
                    appearance_hint: "pixel-room-decorator".into(),
                    can_leave_messages: true,
                    can_edit_scene: true,
                    can_trade_goods: false,
                },
                AgentSceneSlot {
                    slot_id: "room-merchant".into(),
                    display_name: "摆摊助手".into(),
                    scope: AgentScope::Room,
                    use_cases: vec![AgentUseCase::Commerce],
                    appearance_hint: "pixel-room-merchant".into(),
                    can_leave_messages: true,
                    can_edit_scene: false,
                    can_trade_goods: true,
                },
            ],
            image_layer: Some(Self::scene_image_layer("private-room-loft", true)),
            hotspot_layer: Some(Self::scene_hotspot_layer(
                "direct-room-hotspots",
                true,
                &landmarks,
            )),
            landmarks,
        }
    }

    pub(crate) fn scene_image_layer(
        preset: impl Into<String>,
        owner_editable: bool,
    ) -> SceneImageLayer {
        let preset = preset.into();
        SceneImageLayer {
            layer_id: "image-layer".into(),
            asset_hint: preset.clone(),
            preset,
            // scene canvas uses height / width * 10000; 16:9 = 5625.
            aspect_ratio_permyriad: 5_625,
            owner_editable,
            day_image_url: None,
            night_image_url: None,
        }
    }

    pub(crate) fn scene_hotspot_layer(
        layer_id: impl Into<String>,
        owner_editable: bool,
        landmarks: &[SceneLandmark],
    ) -> SceneHotspotLayer {
        let count = landmarks.len().max(1);
        let hotspots = landmarks
            .iter()
            .enumerate()
            .map(|(index, landmark)| {
                let x = 1_500 + ((index as u16) * 6_800 / (count as u16));
                SceneHotspot {
                    hotspot_id: landmark.slot_id.clone(),
                    label: landmark.label.clone(),
                    sprite_hint: landmark.sprite_hint.clone(),
                    interaction_hint: landmark.interaction_hint.clone(),
                    x_permyriad: x.min(9_000),
                    y_permyriad: 2_400 + ((index as u16 % 2) * 2_600),
                    width_permyriad: 900,
                    height_permyriad: 700,
                }
            })
            .collect();
        SceneHotspotLayer {
            layer_id: layer_id.into(),
            coordinate_system: "scene-permyriad".into(),
            owner_editable,
            hotspots,
        }
    }

    pub(crate) fn summarize_scene(scene: Option<&SceneMetadata>) -> Option<String> {
        scene.map(|scene| {
            let scope = match scene.scope {
                SceneScope::City => "城市场景",
                SceneScope::PublicRoom => "公共房间",
                SceneScope::PersonalRoom => "个人房间",
                SceneScope::DirectRoom => "私聊房间",
            };
            let avatar = scene
                .primary_avatar
                .as_ref()
                .map(|item| format!(" · 人物 {}", item.display_name))
                .unwrap_or_default();
            format!("{scope} · {}{}", scene.ambiance, avatar)
        })
    }

    pub(crate) fn actor_is_world_steward(&self, actor_id: &IdentityId) -> bool {
        self.world_stewards.iter().any(|item| item == actor_id)
    }

    pub(crate) fn resident_portability_revoked(&self, resident_id: &IdentityId) -> bool {
        self.resident_sanctions.iter().any(|sanction| {
            sanction.resident_id == *resident_id
                && sanction.portability_revoked
                && sanction.status == WorldResidentSanctionStatus::Active
        })
    }

    pub(crate) fn trust_state_from_records(
        records: &[CityTrustRecord],
        city_id: &CityId,
    ) -> CityTrustState {
        records
            .iter()
            .find(|item| item.city_id == *city_id)
            .map(|item| item.state)
            .unwrap_or_default()
    }

    pub(crate) fn city_is_mirror_visible(city: &CityProfile, trust_state: CityTrustState) -> bool {
        city.public_room_discovery_enabled
            && city.federation_policy != FederationPolicy::Isolated
            && !matches!(
                trust_state,
                CityTrustState::Quarantined | CityTrustState::Isolated
            )
    }

    pub(crate) fn checksum_hex<T: Serialize>(value: &T) -> String {
        let bytes = serde_json::to_vec(value).unwrap_or_default();
        let digest = Sha256::digest(bytes);
        hex::encode(digest)
    }

    pub(crate) fn normalize_slug(raw: &str) -> String {
        let mut slug = raw
            .trim()
            .to_lowercase()
            .chars()
            .map(|char| {
                if char.is_ascii_alphanumeric() {
                    char
                } else {
                    '-'
                }
            })
            .collect::<String>();
        while slug.contains("--") {
            slug = slug.replace("--", "-");
        }
        slug.trim_matches('-').to_string()
    }

    pub(crate) fn resolve_city_id(&self, token: &str) -> Option<CityId> {
        let by_id = CityId(token.to_string());
        if self.cities.contains_key(&by_id) {
            return Some(by_id);
        }
        self.cities
            .values()
            .find(|city| city.profile.slug == token)
            .map(|city| city.profile.city_id.clone())
    }

    pub(crate) fn active_membership(
        &self,
        city_id: &CityId,
        resident_id: &IdentityId,
    ) -> Option<&CityMembership> {
        self.memberships.iter().find(|membership| {
            membership.city_id == *city_id
                && membership.resident_id == *resident_id
                && membership.state == MembershipState::Active
        })
    }

    pub(crate) fn active_membership_mut(
        &mut self,
        city_id: &CityId,
        resident_id: &IdentityId,
    ) -> Option<&mut CityMembership> {
        self.memberships.iter_mut().find(|membership| {
            membership.city_id == *city_id
                && membership.resident_id == *resident_id
                && membership.state != MembershipState::Removed
        })
    }

    pub(crate) fn now_ms() -> i64 {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        millis.min(i64::MAX as u128) as i64
    }

    pub(crate) fn next_message_id(&mut self) -> String {
        self.message_counter += 1;
        format!("gw-{}-{}", Self::now_ms(), self.message_counter)
    }
}
