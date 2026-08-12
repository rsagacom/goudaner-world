use super::*;

impl GatewayRuntime {
    pub(crate) fn fetch_world_snapshot_bundle_from_base_url(
        base_url: &str,
    ) -> Option<WorldSnapshotBundle> {
        let url = format!("{base_url}/v1/world-snapshot");
        ureq::get(&url)
            .call()
            .ok()?
            .into_json::<WorldSnapshotBundle>()
            .ok()
    }

    pub(crate) fn merge_governance_snapshots(
        primary: GovernanceSnapshot,
        secondary: GovernanceSnapshot,
    ) -> GovernanceSnapshot {
        let mut cities = primary.cities;
        for city in secondary.cities {
            if !cities
                .iter()
                .any(|existing| existing.profile.city_id == city.profile.city_id)
            {
                cities.push(city);
            }
        }
        cities.sort_by_key(|item| (item.profile.slug.clone(), item.profile.city_id.0.clone()));

        let mut memberships = primary.memberships;
        for membership in secondary.memberships {
            if !memberships.iter().any(|existing| {
                existing.city_id == membership.city_id
                    && existing.resident_id == membership.resident_id
            }) {
                memberships.push(membership);
            }
        }
        memberships.sort_by_key(|item| {
            (
                item.city_id.0.clone(),
                item.resident_id.0.clone(),
                item.joined_at_ms,
            )
        });

        let mut public_rooms = primary.public_rooms;
        for room in secondary.public_rooms {
            if !public_rooms
                .iter()
                .any(|existing| existing.room_id == room.room_id)
            {
                public_rooms.push(room);
            }
        }
        public_rooms.sort_by_key(|room| room.created_at_ms);

        let mut world_stewards = primary.world_stewards;
        for steward in secondary.world_stewards {
            if !world_stewards.iter().any(|existing| existing == &steward) {
                world_stewards.push(steward);
            }
        }
        world_stewards.sort_by_key(|item| item.0.clone());

        let mut city_trust = primary.city_trust;
        for trust in secondary.city_trust {
            if !city_trust
                .iter()
                .any(|existing| existing.city_id == trust.city_id)
            {
                city_trust.push(trust);
            }
        }
        city_trust.sort_by_key(|item| item.city_id.0.clone());

        let mut world_square_notices = primary.world_square_notices;
        for notice in secondary.world_square_notices {
            if !world_square_notices
                .iter()
                .any(|existing| existing.notice_id == notice.notice_id)
            {
                world_square_notices.push(notice);
            }
        }
        world_square_notices.sort_by_key(|item| item.posted_at_ms);

        let mut safety_advisories = primary.safety_advisories;
        for advisory in secondary.safety_advisories {
            if !safety_advisories
                .iter()
                .any(|existing| existing.advisory_id == advisory.advisory_id)
            {
                safety_advisories.push(advisory);
            }
        }
        safety_advisories.sort_by_key(|item| item.issued_at_ms);

        let mut safety_reports = primary.safety_reports;
        for report in secondary.safety_reports {
            if !safety_reports
                .iter()
                .any(|existing| existing.report_id == report.report_id)
            {
                safety_reports.push(report);
            }
        }
        safety_reports.sort_by_key(|item| item.submitted_at_ms);

        let mut resident_sanctions = primary.resident_sanctions;
        for sanction in secondary.resident_sanctions {
            if !resident_sanctions
                .iter()
                .any(|existing| existing.sanction_id == sanction.sanction_id)
            {
                resident_sanctions.push(sanction);
            }
        }
        resident_sanctions.sort_by_key(|item| item.issued_at_ms);

        let mut registration_blacklist = primary.registration_blacklist;
        for entry in secondary.registration_blacklist {
            if !registration_blacklist
                .iter()
                .any(|existing| existing.hash_sha256 == entry.hash_sha256)
            {
                registration_blacklist.push(entry);
            }
        }
        registration_blacklist.sort_by_key(|item| {
            (
                item.handle_kind.clone(),
                item.hash_sha256.clone(),
                item.added_at_ms,
            )
        });

        GovernanceSnapshot {
            world: primary.world,
            portability: primary.portability,
            cities,
            memberships,
            public_rooms,
            world_stewards,
            city_trust,
            world_square_notices,
            safety_advisories,
            safety_reports,
            resident_sanctions,
            registration_blacklist,
        }
    }

    pub(crate) fn governance_snapshot(&self) -> GovernanceSnapshot {
        let mut cities = self.cities.values().cloned().collect::<Vec<_>>();
        cities.sort_by_key(|city| city.profile.slug.clone());
        let mut memberships = self.memberships.clone();
        memberships.sort_by_key(|item| {
            (
                item.city_id.0.clone(),
                item.resident_id.0.clone(),
                item.joined_at_ms,
            )
        });
        let mut public_rooms = self.public_rooms.clone();
        public_rooms.sort_by_key(|room| room.created_at_ms);
        let mut world_stewards = self.world_stewards.clone();
        world_stewards.sort_by_key(|item| item.0.clone());
        let mut city_trust = self.city_trust.clone();
        city_trust.sort_by_key(|item| item.city_id.0.clone());
        let mut world_square_notices = self.world_square_notices.clone();
        world_square_notices.sort_by_key(|item| item.posted_at_ms);
        let mut safety_advisories = self.safety_advisories.clone();
        safety_advisories.sort_by_key(|item| item.issued_at_ms);
        let mut safety_reports = self.safety_reports.clone();
        safety_reports.sort_by_key(|item| item.submitted_at_ms);
        let mut resident_sanctions = self.resident_sanctions.clone();
        resident_sanctions.sort_by_key(|item| item.issued_at_ms);
        let mut registration_blacklist = self.registration_blacklist.clone();
        registration_blacklist.sort_by_key(|item| {
            (
                item.handle_kind.clone(),
                item.hash_sha256.clone(),
                item.added_at_ms,
            )
        });

        GovernanceSnapshot {
            world: self.world.clone(),
            portability: self.portability.clone(),
            cities,
            memberships,
            public_rooms,
            world_stewards,
            city_trust,
            world_square_notices,
            safety_advisories,
            safety_reports,
            resident_sanctions,
            registration_blacklist,
        }
    }

    pub(crate) fn persist_governance_state(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self.governance_snapshot())
            .map_err(|error| format!("encode governance state failed: {error}"))?;
        atomic_write_file(&self.governance_path, &bytes)
            .map_err(|error| format!("write governance state failed: {error}"))
    }

    pub(crate) fn persist_secure_sessions(&self) -> Result<(), String> {
        let storage_key = self.secure_session_storage_key.as_ref().ok_or_else(|| {
            "LOBSTER_SECURE_SESSION_MASTER_KEY is required before persisting secure sessions"
                .to_string()
        })?;
        let snapshot = self.secure_sessions.seal_snapshot(storage_key)?;
        let bytes = serde_json::to_vec_pretty(&snapshot)
            .map_err(|error| format!("encode secure session state failed: {error}"))?;
        atomic_write_file(&self.secure_sessions_path, &bytes)
            .map_err(|error| format!("write secure session state failed: {error}"))
    }

    pub(crate) fn load_governance_state(&mut self) -> Result<(), String> {
        if !self.governance_path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&self.governance_path)
            .map_err(|error| format!("read governance state failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        let snapshot: GovernanceSnapshot = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode governance state failed: {error}"))?;
        self.world = snapshot.world;
        self.portability = snapshot.portability;
        self.cities = snapshot
            .cities
            .into_iter()
            .map(|city| (city.profile.city_id.clone(), city))
            .collect();
        self.memberships = snapshot.memberships;
        self.public_rooms = snapshot.public_rooms;
        self.world_stewards = snapshot.world_stewards;
        self.city_trust = snapshot.city_trust;
        self.world_square_notices = snapshot.world_square_notices;
        self.safety_advisories = snapshot.safety_advisories;
        self.safety_reports = snapshot.safety_reports;
        self.resident_sanctions = snapshot.resident_sanctions;
        self.registration_blacklist = snapshot.registration_blacklist;
        let rooms = self.public_rooms.clone();
        for room in &rooms {
            self.ensure_room_conversation(room)?;
        }
        Ok(())
    }

    pub(crate) fn load_secure_sessions(&mut self) -> Result<(), String> {
        if !self.secure_sessions_path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&self.secure_sessions_path)
            .map_err(|error| format!("read secure session state failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        let storage_key = self.secure_session_storage_key.as_ref().ok_or_else(|| {
            "LOBSTER_SECURE_SESSION_MASTER_KEY is required to open secure session state".to_string()
        })?;
        if let Ok(snapshot) = serde_json::from_slice::<SealedSecureSessionSnapshot>(&bytes) {
            self.secure_sessions
                .restore_sealed_snapshot(&snapshot, storage_key)?;
            return Ok(());
        }

        // One-way compatibility migration for snapshots produced before at-rest
        // sealing. The plaintext file is replaced atomically before startup completes.
        let groups = serde_json::from_slice::<Vec<MlsGroupState>>(&bytes).map_err(|error| {
            format!("decode sealed or legacy secure session state failed: {error}")
        })?;
        self.secure_sessions.restore(groups);
        self.persist_secure_sessions()?;
        Ok(())
    }

    pub(crate) fn seed_default_governance(&mut self) -> Result<(), String> {
        let city_id = CityId("city:core-harbor".into());
        let city = CityState {
            profile: CityProfile {
                city_id: city_id.clone(),
                world_id: self.world.world_id.clone(),
                slug: "core-harbor".into(),
                title: "Core Harbor".into(),
                description: "Default city for local-first relay, shell, and governance testing."
                    .into(),
                scene: Some(Self::default_city_scene("core-harbor", "Core Harbor")),
                resident_portable: true,
                approval_required: false,
                public_room_discovery_enabled: true,
                federation_policy: FederationPolicy::Open,
                relay_budget_hint: RelayBudgetHint::Balanced,
                retention_policy: Self::default_city_retention_policy(),
            },
            features: Self::default_city_features(),
        };
        self.cities.insert(city_id.clone(), city);
        self.memberships.push(CityMembership {
            city_id: city_id.clone(),
            resident_id: IdentityId("rsaga".into()),
            role: CityRole::Lord,
            state: MembershipState::Active,
            joined_at_ms: Self::now_ms(),
            added_by: None,
        });
        self.memberships.push(CityMembership {
            city_id: city_id.clone(),
            resident_id: IdentityId("builder".into()),
            role: CityRole::Steward,
            state: MembershipState::Active,
            joined_at_ms: Self::now_ms(),
            added_by: Some(IdentityId("rsaga".into())),
        });
        self.memberships.push(CityMembership {
            city_id: city_id.clone(),
            resident_id: IdentityId("tiyan".into()),
            role: CityRole::Resident,
            state: MembershipState::Active,
            joined_at_ms: Self::now_ms(),
            added_by: Some(IdentityId("rsaga".into())),
        });
        let room = PublicRoomRecord {
            room_id: ConversationId("room:city:core-harbor:lobby".into()),
            city_id,
            slug: "lobby".into(),
            title: "City Lobby".into(),
            description: "Default public room for city residents.".into(),
            scene: Some(Self::default_public_room_scene(
                "core-harbor",
                "lobby",
                "City Lobby",
            )),
            created_by: IdentityId("rsaga".into()),
            created_at_ms: Self::now_ms(),
            frozen: false,
        };
        self.ensure_room_conversation(&room)?;
        self.public_rooms.push(room);

        let governance_city_id = CityId("city:aurora-hub".into());
        let governance_city = CityState {
            profile: CityProfile {
                city_id: governance_city_id.clone(),
                world_id: self.world.world_id.clone(),
                slug: "aurora-hub".into(),
                title: "Aurora Hub".into(),
                description: "Default governance city for lord-view notices and city operations."
                    .into(),
                scene: Some(Self::default_city_scene("aurora-hub", "Aurora Hub")),
                resident_portable: true,
                approval_required: false,
                public_room_discovery_enabled: true,
                federation_policy: FederationPolicy::Open,
                relay_budget_hint: RelayBudgetHint::Balanced,
                retention_policy: Self::default_city_retention_policy(),
            },
            features: Self::default_city_features(),
        };
        self.cities
            .insert(governance_city_id.clone(), governance_city);
        self.memberships.push(CityMembership {
            city_id: governance_city_id.clone(),
            resident_id: IdentityId("rsaga".into()),
            role: CityRole::Lord,
            state: MembershipState::Active,
            joined_at_ms: Self::now_ms(),
            added_by: None,
        });
        self.memberships.push(CityMembership {
            city_id: governance_city_id.clone(),
            resident_id: IdentityId("builder".into()),
            role: CityRole::Steward,
            state: MembershipState::Active,
            joined_at_ms: Self::now_ms(),
            added_by: Some(IdentityId("rsaga".into())),
        });
        let governance_room = PublicRoomRecord {
            room_id: ConversationId("room:city:aurora-hub:announcements".into()),
            city_id: governance_city_id,
            slug: "announcements".into(),
            title: "City Announcements".into(),
            description: "Default governance room for city notices and operator messages.".into(),
            scene: Some(Self::default_public_room_scene(
                "aurora-hub",
                "announcements",
                "City Announcements",
            )),
            created_by: IdentityId("rsaga".into()),
            created_at_ms: Self::now_ms(),
            frozen: false,
        };
        self.ensure_room_conversation(&governance_room)?;
        self.public_rooms.push(governance_room);
        self.persist_governance_state()
    }

    pub(crate) fn ensure_default_world_safety(&mut self) -> Result<(), String> {
        if self.world_stewards.is_empty() {
            self.world_stewards.push(IdentityId("rsaga".into()));
        }
        if self.world_square_notices.is_empty() {
            self.world_square_notices.push(WorldSquareNotice {
                notice_id: "notice:welcome".into(),
                title: "World Square online".into(),
                body:
                    "World Square mirrors city discovery, safety notices, and cross-city commons."
                        .into(),
                author_id: IdentityId("rsaga".into()),
                posted_at_ms: Self::now_ms(),
                severity: "info".into(),
                tags: vec!["world".into(), "square".into()],
            });
        }
        if self
            .city_trust
            .iter()
            .all(|trust| trust.city_id.0 != "city:core-harbor")
            && self.cities.contains_key(&CityId("city:core-harbor".into()))
        {
            self.city_trust.push(CityTrustRecord {
                city_id: CityId("city:core-harbor".into()),
                state: CityTrustState::Healthy,
                reason: Some("seed city".into()),
                updated_by: IdentityId("rsaga".into()),
                updated_at_ms: Self::now_ms(),
            });
        }
        if self
            .city_trust
            .iter()
            .all(|trust| trust.city_id.0 != "city:aurora-hub")
            && self.cities.contains_key(&CityId("city:aurora-hub".into()))
        {
            self.city_trust.push(CityTrustRecord {
                city_id: CityId("city:aurora-hub".into()),
                state: CityTrustState::Healthy,
                reason: Some("seed governance city".into()),
                updated_by: IdentityId("rsaga".into()),
                updated_at_ms: Self::now_ms(),
            });
        }
        self.persist_governance_state()
    }
}
