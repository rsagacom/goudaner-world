use super::*;

impl GatewayRuntime {
    fn upstream_federation_token() -> Option<String> {
        std::env::var("LOBSTER_WAKU_UPSTREAM_TOKEN")
            .ok()
            .and_then(|token| {
                let token = token.trim().to_string();
                (!token.is_empty()).then_some(token)
            })
    }

    fn upstream_client(url: &str) -> HttpWakuGatewayClient {
        HttpWakuGatewayClient::with_optional_bearer_token(
            url.to_string(),
            Self::upstream_federation_token(),
        )
    }

    pub(crate) fn upstream_status(&self) -> Option<String> {
        self.upstream_base_url
            .as_ref()
            .map(|url| format!("gateway-federation:{url}"))
    }

    pub(crate) fn provider_status(&self) -> ProviderStatusResponse {
        if let (Some(url), Some(client)) = (
            self.upstream_base_url.as_ref(),
            self.upstream_gateway.as_ref(),
        ) {
            let reachable = client.healthcheck().is_ok();
            return ProviderStatusResponse {
                mode: "remote-gateway".into(),
                base_url: Some(url.clone()),
                connection_state: self.connection_state,
                reachable,
            };
        }

        ProviderStatusResponse {
            mode: "local-memory".into(),
            base_url: None,
            connection_state: self.connection_state,
            reachable: true,
        }
    }

    pub(crate) fn apply_upstream_provider_url(&mut self, upstream_base_url: Option<String>) {
        let normalized = upstream_base_url.and_then(|url| {
            let trimmed = url.trim().trim_end_matches('/').to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });
        self.upstream_gateway = normalized.as_ref().map(|url| Self::upstream_client(url));
        self.upstream_base_url = normalized;
    }

    pub(crate) fn provider_config_snapshot(&self) -> PersistedProviderConfig {
        PersistedProviderConfig {
            upstream_gateway_url: self.upstream_base_url.clone(),
            mirror_sources: self.mirror_sources.clone(),
        }
    }

    pub(crate) fn persist_provider_config(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self.provider_config_snapshot())
            .map_err(|error| format!("encode provider config failed: {error}"))?;
        atomic_write_file(&self.provider_config_path, &bytes)
            .map_err(|error| format!("write provider config failed: {error}"))
    }

    pub(crate) fn load_provider_config(&mut self) -> Result<(), String> {
        if !self.provider_config_path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&self.provider_config_path)
            .map_err(|error| format!("read provider config failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        let config: PersistedProviderConfig = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode provider config failed: {error}"))?;
        self.apply_upstream_provider_url(config.upstream_gateway_url);
        self.apply_mirror_sources(config.mirror_sources);
        Ok(())
    }

    pub(crate) fn auth_state_snapshot(&self) -> PersistedAuthState {
        PersistedAuthState {
            registrations: self.registrations.clone(),
            email_otp_challenges: self.email_otp_challenges.clone(),
            auth_sessions: self.auth_sessions.clone(),
        }
    }

    pub(crate) fn persist_auth_state(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self.auth_state_snapshot())
            .map_err(|error| format!("encode auth state failed: {error}"))?;
        atomic_write_file(&self.auth_state_path, &bytes)
            .map_err(|error| format!("write auth state failed: {error}"))
    }

    pub(crate) fn load_auth_state(&mut self) -> Result<(), String> {
        if !self.auth_state_path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&self.auth_state_path)
            .map_err(|error| format!("read auth state failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        let snapshot: PersistedAuthState = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode auth state failed: {error}"))?;
        self.registrations = snapshot.registrations;
        self.email_otp_challenges = snapshot.email_otp_challenges;
        self.auth_sessions = snapshot.auth_sessions;
        Ok(())
    }

    pub(crate) fn set_upstream_provider_url(
        &mut self,
        upstream_base_url: Option<String>,
    ) -> Result<(), String> {
        self.apply_upstream_provider_url(upstream_base_url);
        self.persist_provider_config()
    }

    pub(crate) fn normalize_base_url(url: &str) -> Option<String> {
        let trimmed = url.trim().trim_end_matches('/').to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    pub(crate) fn apply_mirror_sources(&mut self, mirror_sources: Vec<MirrorSourceConfig>) {
        let mut deduped = Vec::new();
        for source in mirror_sources {
            let Some(base_url) = Self::normalize_base_url(&source.base_url) else {
                continue;
            };
            if deduped
                .iter()
                .any(|existing: &MirrorSourceConfig| existing.base_url == base_url)
            {
                continue;
            }
            deduped.push(MirrorSourceConfig {
                base_url,
                enabled: source.enabled,
            });
        }
        deduped.sort_by_key(|item| item.base_url.clone());
        self.mirror_sources = deduped;
    }

    pub(crate) fn add_world_mirror_source(
        &mut self,
        request: AddWorldMirrorSourceRequest,
    ) -> Result<Vec<MirrorSourceConfig>, String> {
        let Some(base_url) = Self::normalize_base_url(&request.base_url) else {
            return Err("mirror base_url required".into());
        };
        let enabled = request.enabled.unwrap_or(true);
        let mut mirror_sources = self.mirror_sources.clone();
        if let Some(existing) = mirror_sources
            .iter_mut()
            .find(|existing| existing.base_url == base_url)
        {
            existing.enabled = enabled;
        } else {
            mirror_sources.push(MirrorSourceConfig { base_url, enabled });
        }
        self.apply_mirror_sources(mirror_sources);
        self.persist_provider_config()?;
        Ok(self.mirror_sources.clone())
    }

    pub(crate) fn connect_provider(
        &mut self,
        request: ConnectProviderRequest,
    ) -> Result<ProviderStatusResponse, String> {
        let provider_url = request.provider_url.trim();
        if provider_url.is_empty() {
            return Err("provider url required".into());
        }
        let client = Self::upstream_client(provider_url);
        client.healthcheck()?;
        self.upstream_gateway = Some(client);
        self.upstream_base_url = Some(provider_url.trim_end_matches('/').to_string());
        self.persist_provider_config()?;
        Ok(self.provider_status())
    }

    pub(crate) fn disconnect_provider(&mut self) -> Result<ProviderStatusResponse, String> {
        self.apply_upstream_provider_url(None);
        self.persist_provider_config()?;
        Ok(self.provider_status())
    }
}
