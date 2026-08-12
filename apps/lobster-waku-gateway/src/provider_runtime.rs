use super::*;

impl GatewayRuntime {
    fn provider_url_authority(url: &str) -> Option<&str> {
        let (_, rest) = url.split_once("://")?;
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..end];
        (!authority.is_empty()).then_some(authority)
    }

    fn provider_authority_is_valid(authority: &str) -> bool {
        if authority.is_empty()
            || authority.contains('@')
            || authority.chars().any(char::is_whitespace)
        {
            return false;
        }

        if authority.starts_with('[') {
            let Some(end) = authority.find(']') else {
                return false;
            };
            let suffix = &authority[end + 1..];
            return suffix.is_empty()
                || suffix.strip_prefix(':').is_some_and(|port| {
                    !port.is_empty() && port.chars().all(|c| c.is_ascii_digit())
                });
        }

        if let Some((host, port)) = authority.split_once(':') {
            !host.is_empty() && !port.is_empty() && port.chars().all(|c| c.is_ascii_digit())
        } else {
            !authority.is_empty()
        }
    }

    fn provider_authority_is_loopback(authority: &str) -> bool {
        let host = if authority.starts_with('[') {
            let Some(end) = authority.find(']') else {
                return false;
            };
            &authority[..=end]
        } else {
            authority.split(':').next().unwrap_or_default()
        };

        host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "[::1]"
    }

    fn normalize_provider_url(&self, raw_url: &str) -> Result<Option<String>, String> {
        let trimmed = raw_url.trim().trim_end_matches('/').to_string();
        if trimmed.is_empty() {
            return Ok(None);
        }

        let Some((scheme, _)) = trimmed.split_once("://") else {
            return Err("provider URL must include an http or https scheme".into());
        };
        let scheme = scheme.to_ascii_lowercase();
        if scheme != "http" && scheme != "https" {
            return Err("provider URL must use http or https".into());
        }

        let authority = Self::provider_url_authority(&trimmed)
            .ok_or_else(|| "provider URL must include a host".to_string())?;
        if !Self::provider_authority_is_valid(authority) {
            return Err("provider URL has an invalid host or port".into());
        }

        if scheme == "http"
            && !(self.dev_auth_bypass && Self::provider_authority_is_loopback(authority))
        {
            return Err(
                "provider URL must use HTTPS; HTTP is only allowed for loopback dev/test providers"
                    .into(),
            );
        }

        Ok(Some(trimmed))
    }

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

    pub(crate) fn apply_upstream_provider_url(
        &mut self,
        upstream_base_url: Option<String>,
    ) -> Result<(), String> {
        let normalized = upstream_base_url
            .map(|url| self.normalize_provider_url(&url))
            .transpose()?
            .flatten();
        self.upstream_gateway = normalized.as_ref().map(|url| Self::upstream_client(url));
        self.upstream_base_url = normalized;
        Ok(())
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
        self.apply_upstream_provider_url(config.upstream_gateway_url)?;
        self.apply_mirror_sources(config.mirror_sources)?;
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
        self.apply_upstream_provider_url(upstream_base_url)?;
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

    pub(crate) fn apply_mirror_sources(
        &mut self,
        mirror_sources: Vec<MirrorSourceConfig>,
    ) -> Result<(), String> {
        let mut deduped = Vec::new();
        for source in mirror_sources {
            let base_url = if source.enabled {
                self.normalize_provider_url(&source.base_url)?
                    .ok_or_else(|| "enabled mirror base_url required".to_string())?
            } else {
                let Some(base_url) = Self::normalize_base_url(&source.base_url) else {
                    continue;
                };
                base_url
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
        Ok(())
    }

    pub(crate) fn add_world_mirror_source(
        &mut self,
        request: AddWorldMirrorSourceRequest,
    ) -> Result<Vec<MirrorSourceConfig>, String> {
        let Some(base_url) = Self::normalize_base_url(&request.base_url) else {
            return Err("mirror base_url required".into());
        };
        let enabled = request.enabled.unwrap_or(true);
        let base_url = if enabled {
            self.normalize_provider_url(&base_url)?
                .ok_or_else(|| "enabled mirror base_url required".to_string())?
        } else {
            base_url
        };
        let mut mirror_sources = self.mirror_sources.clone();
        if let Some(existing) = mirror_sources
            .iter_mut()
            .find(|existing| existing.base_url == base_url)
        {
            existing.enabled = enabled;
        } else {
            mirror_sources.push(MirrorSourceConfig { base_url, enabled });
        }
        self.apply_mirror_sources(mirror_sources)?;
        self.persist_provider_config()?;
        Ok(self.mirror_sources.clone())
    }

    pub(crate) fn connect_provider(
        &mut self,
        request: ConnectProviderRequest,
    ) -> Result<ProviderStatusResponse, String> {
        let provider_url = self
            .normalize_provider_url(&request.provider_url)?
            .ok_or_else(|| "provider url required".to_string())?;
        let client = Self::upstream_client(&provider_url);
        client.healthcheck()?;
        self.upstream_gateway = Some(client);
        self.upstream_base_url = Some(provider_url);
        self.persist_provider_config()?;
        Ok(self.provider_status())
    }

    pub(crate) fn disconnect_provider(&mut self) -> Result<ProviderStatusResponse, String> {
        self.apply_upstream_provider_url(None)?;
        self.persist_provider_config()?;
        Ok(self.provider_status())
    }
}
