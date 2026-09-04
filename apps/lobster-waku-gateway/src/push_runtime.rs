//! WebPush delivery runtime (蓝图序 2：推送通知).
//!
//! Storage model: `push-subscriptions.json` (0600) keyed by endpoint URL, and
//! `vapid-signing-key.json` (0600) holding the ES256 private key generated on
//! first use. Both ride the same atomic-write discipline as the other state
//! files.
//!
//! Delivery model: called while the runtime lock is held, so all network work
//! is handed to detached threads. Endpoints that answer 404/410 (gone/expired
//! subscriptions) are recorded in a shared dead-letter buffer and pruned at
//! the next delivery call — the delivery path itself never blocks or retries.

use std::{sync::Arc, time::Duration};

use chat_core::{ConversationId, IdentityId};
use crypto_mls::webpush::{self, VapidKeyPair};

use super::*;

const PUSH_TTL_SECS: &str = "86400";
const VAPID_TOKEN_LIFETIME_SECS: u64 = 12 * 3600;
const DEFAULT_PUSH_SUBJECT: &str = "mailto:no-reply@chat.ajw.cn";
const DEFAULT_PUSH_TITLE: &str = "我和狗蛋儿的家";

impl GatewayRuntime {
    pub(crate) fn load_push_state(&mut self) -> Result<(), String> {
        self.load_push_subscriptions()?;
        self.load_vapid_key()?;
        Ok(())
    }

    fn load_push_subscriptions(&mut self) -> Result<(), String> {
        let path = self.push_subscriptions_path.clone();
        if !path.exists() {
            return Ok(());
        }
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("read push subscriptions failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(());
        }
        let subscriptions: Vec<PushSubscriptionRecord> = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode push subscriptions failed: {error}"))?;
        self.push_subscriptions = subscriptions
            .into_iter()
            .map(|record| (record.endpoint.clone(), record))
            .collect();
        Ok(())
    }

    fn persist_push_subscriptions(&self) -> Result<(), String> {
        let subscriptions: Vec<&PushSubscriptionRecord> =
            self.push_subscriptions.values().collect();
        let bytes = serde_json::to_vec(&subscriptions)
            .map_err(|error| format!("encode push subscriptions failed: {error}"))?;
        atomic_write_file(&self.push_subscriptions_path, &bytes)
    }

    fn load_vapid_key(&mut self) -> Result<(), String> {
        if self.vapid_key.is_some() {
            return Ok(());
        }
        let path = self.vapid_key_path.clone();
        if path.exists() {
            let bytes =
                std::fs::read(&path).map_err(|error| format!("read vapid key failed: {error}"))?;
            let stored: VapidKeyFile = serde_json::from_slice(&bytes)
                .map_err(|error| format!("decode vapid key failed: {error}"))?;
            let key_pair = VapidKeyPair::from_pkcs8(&stored.pkcs8)
                .map_err(|error| format!("vapid private key unavailable: {error}"))?;
            self.vapid_key = Some(VapidSigningKey {
                inner: Arc::new(key_pair),
            });
            return Ok(());
        }
        let (key_pair, pkcs8) = VapidKeyPair::generate()
            .map_err(|error| format!("vapid key generation failed: {error}"))?;
        let stored = VapidKeyFile {
            pkcs8: pkcs8.clone(),
        };
        let bytes = serde_json::to_vec(&stored)
            .map_err(|error| format!("encode vapid key failed: {error}"))?;
        atomic_write_file(&path, &bytes)
            .map_err(|error| format!("persist vapid key failed: {error}"))?;
        self.vapid_key = Some(VapidSigningKey {
            inner: Arc::new(key_pair),
        });
        Ok(())
    }

    pub(crate) fn vapid_public_key_base64url(&self) -> Option<String> {
        self.vapid_key
            .as_ref()
            .map(|key| key.inner.public_key_base64url())
    }

    /// Register (or refresh) a push subscription for the authenticated resident.
    pub(crate) fn push_subscribe(
        &mut self,
        resident: &IdentityId,
        endpoint: &str,
        p256dh: &str,
        auth: &str,
    ) -> Result<(), String> {
        self.validate_push_endpoint(endpoint)?;
        let p256dh_decoded = webpush::base64url_decode(p256dh)
            .map_err(|error| format!("p256dh key invalid: {error}"))?;
        let auth_decoded = webpush::base64url_decode(auth)
            .map_err(|error| format!("auth secret invalid: {error}"))?;
        if p256dh_decoded.len() != 65 || p256dh_decoded.first() != Some(&0x04) {
            return Err("p256dh key must be an uncompressed P-256 point".into());
        }
        if auth_decoded.len() != 16 {
            return Err("auth secret must be 16 bytes".into());
        }
        // fail-closed：订阅即试算一次加密，拒绝非法曲线点（ECDH 不成立）
        if let Err(error) =
            webpush::encrypt_message(b"subscription probe", &p256dh_decoded, &auth_decoded)
        {
            return Err(format!("subscription keys unusable: {error}"));
        }
        self.push_subscriptions.insert(
            endpoint.to_string(),
            PushSubscriptionRecord {
                resident_id: resident.clone(),
                endpoint: endpoint.to_string(),
                p256dh: p256dh.to_string(),
                auth: auth.to_string(),
                created_at_ms: Self::now_ms(),
            },
        );
        self.persist_push_subscriptions()
    }

    /// Subscription owner for an endpoint, if registered.
    pub(crate) fn push_subscription_resident(&self, endpoint: &str) -> Option<&IdentityId> {
        self.push_subscriptions
            .get(endpoint)
            .map(|record| &record.resident_id)
    }

    /// Remove a subscription by endpoint; returns whether one existed.
    pub(crate) fn push_unsubscribe(&mut self, endpoint: &str) -> Result<bool, String> {
        let existed = self.push_subscriptions.remove(endpoint).is_some();
        if existed {
            self.persist_push_subscriptions()?;
        }
        Ok(existed)
    }

    /// Push endpoints must be HTTPS; loopback HTTP stays allowed in dev/test
    /// so the delivery path is exercisable without public infrastructure.
    fn validate_push_endpoint(&self, endpoint: &str) -> Result<(), String> {
        let Some((scheme, rest)) = endpoint.split_once("://") else {
            return Err("push endpoint must include an http or https scheme".into());
        };
        if !scheme.eq_ignore_ascii_case("https") {
            let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
            let loopback = authority.starts_with("127.0.0.1")
                || authority.starts_with("localhost")
                || authority.starts_with("[::1]");
            if !(scheme.eq_ignore_ascii_case("http") && loopback && self.dev_auth_bypass) {
                return Err("push endpoint must use HTTPS in production".into());
            }
        }
        Ok(())
    }

    /// Fan a freshly published message out to every other participant's push
    /// subscriptions. Fire-and-forget: never blocks the send path and never
    /// fails the message on push errors.
    pub(crate) fn deliver_message_push(
        &mut self,
        conversation_id: &ConversationId,
        sender: &IdentityId,
        preview: &str,
    ) {
        self.prune_dead_push_endpoints();
        if self.push_subscriptions.is_empty() || self.vapid_key.is_none() {
            return;
        }
        let Some(conversation) = self
            .timeline_store
            .active_conversations()
            .into_iter()
            .find(|conversation| &conversation.conversation_id == conversation_id)
        else {
            return;
        };
        let recipients: Vec<&IdentityId> = conversation
            .participants
            .iter()
            .filter(|participant| *participant != sender)
            .collect();
        if recipients.is_empty() {
            return;
        }
        let title = self
            .app_config
            .get("push_notification_title")
            .cloned()
            .unwrap_or_else(|| DEFAULT_PUSH_TITLE.to_string());
        let subject = self
            .app_config
            .get("push_subject_mailto")
            .cloned()
            .unwrap_or_else(|| DEFAULT_PUSH_SUBJECT.to_string());
        let body = format!("{}: {}", sender.0, preview);
        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "tag": conversation_id.0,
        })
        .to_string();
        let payload = payload.as_bytes().to_vec();
        let targets: Vec<PushTarget> = self
            .push_subscriptions
            .values()
            .filter(|record| {
                recipients
                    .iter()
                    .any(|recipient| **recipient == record.resident_id)
            })
            .map(|record| PushTarget {
                endpoint: record.endpoint.clone(),
                p256dh: record.p256dh.clone(),
                auth: record.auth.clone(),
            })
            .collect();
        let vapid_key = match self.vapid_key.clone() {
            Some(key) => key,
            None => return,
        };
        let dead_endpoints = self.dead_push_endpoints.clone();
        for target in targets {
            let vapid_key = vapid_key.clone();
            let dead_endpoints = dead_endpoints.clone();
            let payload = payload.clone();
            let subject = subject.clone();
            std::thread::spawn(move || {
                let Some(origin) = endpoint_origin(&target.endpoint) else {
                    return;
                };
                let expires_at_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|value| value.as_secs())
                    .unwrap_or(0)
                    + VAPID_TOKEN_LIFETIME_SECS;
                let jwt = match vapid_key
                    .inner
                    .sign_token(&origin, &subject, expires_at_secs)
                {
                    Ok(jwt) => jwt,
                    Err(_) => return,
                };
                let (Ok(ua_public), Ok(auth_secret)) = (
                    webpush::base64url_decode(&target.p256dh),
                    webpush::base64url_decode(&target.auth),
                ) else {
                    return;
                };
                let ciphertext = match webpush::encrypt_message(&payload, &ua_public, &auth_secret)
                {
                    Ok(ciphertext) => ciphertext,
                    Err(error) => {
                        eprintln!("push: encrypt notification failed: {error}");
                        return;
                    }
                };
                let agent = ureq::AgentBuilder::new()
                    .timeout(Duration::from_secs(10))
                    .build();
                let outcome = agent
                    .post(&target.endpoint)
                    .set("Content-Encoding", "aes128gcm")
                    .set("TTL", PUSH_TTL_SECS)
                    .set("Urgency", "normal")
                    .set(
                        "Authorization",
                        &format!(
                            "vapid t={jwt}, k={}",
                            vapid_key.inner.public_key_base64url()
                        ),
                    )
                    .send_bytes(&ciphertext);
                let subscription_gone = matches!(outcome, Err(ureq::Error::Status(status, _)) if status == 404 || status == 410);
                if subscription_gone && let Ok(mut dead) = dead_endpoints.lock() {
                    dead.push(target.endpoint);
                }
            });
        }
    }

    /// Drop subscriptions whose push service reported them gone; persist only
    /// when something actually changed.
    fn prune_dead_push_endpoints(&mut self) {
        let dead: Vec<String> = {
            let mut dead = self
                .dead_push_endpoints
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            dead.drain(..).collect()
        };
        if dead.is_empty() {
            return;
        }
        let mut changed = false;
        for endpoint in dead {
            if self.push_subscriptions.remove(&endpoint).is_some() {
                changed = true;
            }
        }
        if changed {
            let _ = self.persist_push_subscriptions();
        }
    }
}

fn endpoint_origin(endpoint: &str) -> Option<String> {
    let (scheme, rest) = endpoint.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.is_empty() {
        return None;
    }
    Some(format!("{scheme}://{authority}"))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct VapidKeyFile {
    pkcs8: Vec<u8>,
}
