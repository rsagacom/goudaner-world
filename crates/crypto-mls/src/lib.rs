use std::collections::HashMap;
use std::sync::LazyLock;

use chat_core::{
    ConversationId, ConversationScope, DeviceId, IdentityId, MessageEnvelope, PayloadType,
};
use ring::hkdf;
use ring::rand::{SecureRandom, SystemRandom};
use ring::{aead, digest};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const STORAGE_SNAPSHOT_VERSION: u8 = 1;
const STORAGE_SNAPSHOT_ALGORITHM: &str = "AES-256-GCM-HKDF-SHA256";
const STORAGE_SNAPSHOT_AAD: &[u8] = b"lobster-secure-session-snapshot-v1";
type Key = [u8; KEY_LEN];

static RNG: LazyLock<SystemRandom> = LazyLock::new(SystemRandom::new);

fn generate_key() -> Result<Key, String> {
    let mut k = [0u8; KEY_LEN];
    RNG.fill(&mut k)
        .map_err(|e| format!("rng key generation failed: {e}"))?;
    Ok(k)
}

fn derive_nonce(gid: &str, epoch: u64, ctr: u64) -> [u8; NONCE_LEN] {
    let mut ctx = digest::Context::new(&digest::SHA256);
    ctx.update(gid.as_bytes());
    ctx.update(&epoch.to_le_bytes());
    ctx.update(&ctr.to_le_bytes());
    let h = ctx.finish();
    let mut n = [0u8; NONCE_LEN];
    n.copy_from_slice(&h.as_ref()[..NONCE_LEN]);
    n
}

fn derive_epoch_key(prev: &[u8], epoch: u64) -> Result<Key, String> {
    let s = hkdf::Salt::new(hkdf::HKDF_SHA256, &[]);
    let o = s.extract(&[prev, &epoch.to_le_bytes()].concat());
    let mut k = [0u8; KEY_LEN];
    o.expand(&[b"lobster-epoch"], hkdf::HKDF_SHA256)
        .map_err(|e| format!("hkdf epoch expand failed: {e}"))?
        .fill(&mut k)
        .map_err(|e| format!("hkdf epoch fill failed: {e}"))?;
    Ok(k)
}

pub fn derive_group_key(members: &[MlsMember]) -> Result<Key, String> {
    if members.is_empty() {
        return Err("empty".into());
    }
    let mut keys: Vec<&str> = members
        .iter()
        .filter_map(|m| m.public_key.as_deref())
        .collect();
    if keys.is_empty() {
        return generate_key();
    }
    keys.sort();
    let s = hkdf::Salt::new(hkdf::HKDF_SHA256, b"lobster-mls-group-key-v1");
    let o = s.extract(keys.join(":").as_bytes());
    let mut k = [0u8; KEY_LEN];
    o.expand(&[b"group-key"], hkdf::HKDF_SHA256)
        .map_err(|e| format!("{e}"))?
        .fill(&mut k)
        .map_err(|e| format!("{e}"))?;
    Ok(k)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MlsGroupKind {
    Direct,
    Room,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MlsMember {
    pub identity_id: IdentityId,
    pub device_id: Option<DeviceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
}
impl MlsMember {
    pub fn device(i: impl Into<String>, d: impl Into<String>) -> Self {
        Self {
            identity_id: IdentityId(i.into()),
            device_id: Some(DeviceId(d.into())),
            public_key: None,
        }
    }
    pub fn identity(i: impl Into<String>) -> Self {
        Self {
            identity_id: IdentityId(i.into()),
            device_id: None,
            public_key: None,
        }
    }
    pub fn with_public_key(mut self, k: impl Into<String>) -> Self {
        self.public_key = Some(k.into());
        self
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MlsGroupState {
    pub group_id: String,
    pub conversation_id: ConversationId,
    pub kind: MlsGroupKind,
    pub scope: ConversationScope,
    pub epoch: u64,
    pub members: Vec<MlsMember>,
    pub pending_rekey: bool,
    pub group_key: Vec<u8>,
}

/// Public projection of a secure-session group. Key material must never cross
/// an API or logging boundary with the lifecycle metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MlsGroupView {
    pub group_id: String,
    pub conversation_id: ConversationId,
    pub kind: MlsGroupKind,
    pub scope: ConversationScope,
    pub epoch: u64,
    pub members: Vec<MlsMember>,
    pub pending_rekey: bool,
}

impl From<&MlsGroupState> for MlsGroupView {
    fn from(group: &MlsGroupState) -> Self {
        Self {
            group_id: group.group_id.clone(),
            conversation_id: group.conversation_id.clone(),
            kind: group.kind,
            scope: group.scope,
            epoch: group.epoch,
            members: group.members.clone(),
            pending_rekey: group.pending_rekey,
        }
    }
}

/// Derived storage key for sealing secure-session snapshots. The original
/// deployment secret is not retained and Debug output is always redacted.
#[derive(Clone, PartialEq, Eq)]
pub struct SecureSessionStorageKey(Vec<u8>);

impl SecureSessionStorageKey {
    pub fn from_secret(secret: &str) -> Result<Self, String> {
        let secret = secret.trim();
        if secret.len() < KEY_LEN {
            return Err("secure session storage secret must be at least 32 characters".into());
        }
        let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"lobster-secure-session-storage-v1");
        let prk = salt.extract(secret.as_bytes());
        let mut key = [0u8; KEY_LEN];
        prk.expand(&[b"snapshot-key"], hkdf::HKDF_SHA256)
            .map_err(|error| format!("secure session storage key expansion failed: {error}"))?
            .fill(&mut key)
            .map_err(|error| format!("secure session storage key fill failed: {error}"))?;
        Ok(Self(key.to_vec()))
    }

    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for SecureSessionStorageKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl std::fmt::Debug for SecureSessionStorageKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecureSessionStorageKey([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedSecureSessionSnapshot {
    pub schema_version: u8,
    pub algorithm: String,
    pub nonce_hex: String,
    pub ciphertext_hex: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SecureSessionSnapshotV1 {
    groups: Vec<MlsGroupState>,
}

impl Drop for MlsGroupState {
    fn drop(&mut self) {
        self.group_key.zeroize();
    }
}

// Suppress Debug for security — key material must not be logged
impl std::fmt::Debug for MlsGroupState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MlsGroupState")
            .field("group_id", &self.group_id)
            .field("conversation_id", &self.conversation_id)
            .field("kind", &self.kind)
            .field("scope", &self.scope)
            .field("epoch", &self.epoch)
            .field("members", &self.members)
            .field("pending_rekey", &self.pending_rekey)
            .field("group_key", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MlsWireFormat {
    SkeletonPostcard,
    Aes256Gcm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MlsCiphertextEnvelope {
    pub group_id: String,
    pub conversation_id: ConversationId,
    pub epoch: u64,
    pub sender: IdentityId,
    pub sender_device: DeviceId,
    pub payload_type: PayloadType,
    pub wire_format: MlsWireFormat,
    pub ciphertext: Vec<u8>,
}

pub trait SecureSessionManager {
    fn bootstrap_direct(
        &mut self,
        c: &ConversationId,
        m: Vec<MlsMember>,
    ) -> Result<MlsGroupState, String>;
    fn bootstrap_room(
        &mut self,
        c: &ConversationId,
        s: ConversationScope,
        m: Vec<MlsMember>,
    ) -> Result<MlsGroupState, String>;
    fn group_state(&self, c: &ConversationId) -> Option<&MlsGroupState>;
    fn current_epoch(&self, c: &ConversationId) -> Result<u64, String>;
    fn rotate_epoch(&mut self, c: &ConversationId) -> Result<u64, String>;
    fn add_member(&mut self, c: &ConversationId, m: MlsMember) -> Result<(), String>;
    fn remove_member(&mut self, c: &ConversationId, i: &IdentityId) -> Result<(), String>;
    fn seal(&self, msg: &MessageEnvelope) -> Result<MlsCiphertextEnvelope, String>;
    fn open(&self, env: &MlsCiphertextEnvelope) -> Result<MessageEnvelope, String>;
}

#[derive(Debug, Default, Clone)]
pub struct SkeletonSecureSessionManager {
    groups: HashMap<ConversationId, MlsGroupState>,
}

impl SkeletonSecureSessionManager {
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
        }
    }
    pub fn snapshot(&self) -> Vec<MlsGroupState> {
        let mut g: Vec<_> = self.groups.values().cloned().collect();
        g.sort_by_key(|x| x.conversation_id.0.clone());
        g
    }
    pub fn restore(&mut self, gs: Vec<MlsGroupState>) {
        self.groups = gs
            .into_iter()
            .map(|g| (g.conversation_id.clone(), g))
            .collect()
    }

    pub fn seal_snapshot(
        &self,
        storage_key: &SecureSessionStorageKey,
    ) -> Result<SealedSecureSessionSnapshot, String> {
        let snapshot = SecureSessionSnapshotV1 {
            groups: self.snapshot(),
        };
        let mut protected = serde_json::to_vec(&snapshot)
            .map_err(|error| format!("encode secure session snapshot failed: {error}"))?;
        let mut nonce = [0u8; NONCE_LEN];
        RNG.fill(&mut nonce)
            .map_err(|error| format!("secure session snapshot nonce generation failed: {error}"))?;
        let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, storage_key.as_bytes())
            .map_err(|error| format!("secure session snapshot key rejected: {error}"))?;
        aead::LessSafeKey::new(unbound)
            .seal_in_place_append_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(STORAGE_SNAPSHOT_AAD),
                &mut protected,
            )
            .map_err(|error| format!("seal secure session snapshot failed: {error}"))?;
        Ok(SealedSecureSessionSnapshot {
            schema_version: STORAGE_SNAPSHOT_VERSION,
            algorithm: STORAGE_SNAPSHOT_ALGORITHM.into(),
            nonce_hex: hex::encode(nonce),
            ciphertext_hex: hex::encode(protected),
        })
    }

    pub fn restore_sealed_snapshot(
        &mut self,
        snapshot: &SealedSecureSessionSnapshot,
        storage_key: &SecureSessionStorageKey,
    ) -> Result<(), String> {
        if snapshot.schema_version != STORAGE_SNAPSHOT_VERSION {
            return Err(format!(
                "unsupported secure session snapshot version: {}",
                snapshot.schema_version
            ));
        }
        if snapshot.algorithm != STORAGE_SNAPSHOT_ALGORITHM {
            return Err(format!(
                "unsupported secure session snapshot algorithm: {}",
                snapshot.algorithm
            ));
        }
        let nonce: [u8; NONCE_LEN] = hex::decode(&snapshot.nonce_hex)
            .map_err(|error| format!("decode secure session snapshot nonce failed: {error}"))?
            .try_into()
            .map_err(|_| "secure session snapshot nonce has invalid length".to_string())?;
        let mut protected = hex::decode(&snapshot.ciphertext_hex).map_err(|error| {
            format!("decode secure session snapshot ciphertext failed: {error}")
        })?;
        let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, storage_key.as_bytes())
            .map_err(|error| format!("secure session snapshot key rejected: {error}"))?;
        let plaintext_len = match aead::LessSafeKey::new(unbound).open_in_place(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::from(STORAGE_SNAPSHOT_AAD),
            &mut protected,
        ) {
            Ok(plaintext) => plaintext.len(),
            Err(_) => {
                protected.zeroize();
                return Err("open secure session snapshot failed".into());
            }
        };
        let decoded =
            serde_json::from_slice::<SecureSessionSnapshotV1>(&protected[..plaintext_len])
                .map_err(|error| format!("decode secure session snapshot failed: {error}"));
        protected.zeroize();
        self.restore(decoded?.groups);
        Ok(())
    }
    #[allow(clippy::possible_missing_else)]
    fn build(
        &self,
        c: &ConversationId,
        k: MlsGroupKind,
        s: ConversationScope,
        ms: Vec<MlsMember>,
    ) -> Result<MlsGroupState, String> {
        if ms.is_empty() {
            return Err("empty".into());
        }
        if k == MlsGroupKind::Direct && ms.len() != 2 {
            return Err("direct needs 2".into());
        }
        let key = derive_group_key(&ms)?.to_vec();
        Ok(MlsGroupState {
            group_id: format!("mls:{}", c.0),
            conversation_id: c.clone(),
            kind: k,
            scope: s,
            epoch: 1,
            members: ms,
            pending_rekey: false,
            group_key: key,
        })
    }
    fn gm(&mut self, c: &ConversationId) -> Result<&mut MlsGroupState, String> {
        self.groups
            .get_mut(c)
            .ok_or_else(|| format!("not found: {}", c.0))
    }
    fn g(&self, c: &ConversationId) -> Result<&MlsGroupState, String> {
        self.groups
            .get(c)
            .ok_or_else(|| format!("not found: {}", c.0))
    }
}

#[allow(clippy::possible_missing_else)]
impl SecureSessionManager for SkeletonSecureSessionManager {
    fn bootstrap_direct(
        &mut self,
        c: &ConversationId,
        ms: Vec<MlsMember>,
    ) -> Result<MlsGroupState, String> {
        let g = self.build(c, MlsGroupKind::Direct, ConversationScope::Private, ms)?;
        self.groups.insert(c.clone(), g.clone());
        Ok(g)
    }
    fn bootstrap_room(
        &mut self,
        c: &ConversationId,
        s: ConversationScope,
        ms: Vec<MlsMember>,
    ) -> Result<MlsGroupState, String> {
        if s == ConversationScope::Private {
            return Err("room must be non-private".into());
        }
        let g = self.build(c, MlsGroupKind::Room, s, ms)?;
        self.groups.insert(c.clone(), g.clone());
        Ok(g)
    }
    fn group_state(&self, c: &ConversationId) -> Option<&MlsGroupState> {
        self.groups.get(c)
    }
    fn current_epoch(&self, c: &ConversationId) -> Result<u64, String> {
        Ok(self.g(c)?.epoch)
    }
    fn rotate_epoch(&mut self, c: &ConversationId) -> Result<u64, String> {
        let g = self.gm(c)?;
        let ne = g.epoch + 1;
        let dk = derive_epoch_key(&g.group_key, ne)?;
        g.epoch = ne;
        g.pending_rekey = false;
        g.group_key = dk.to_vec();
        Ok(ne)
    }
    fn add_member(&mut self, c: &ConversationId, m: MlsMember) -> Result<(), String> {
        let g = self.gm(c)?;
        if g.members.iter().any(|x| x.identity_id == m.identity_id) {
            return Err("duplicate".into());
        }
        g.members.push(m);
        g.pending_rekey = true;
        Ok(())
    }
    fn remove_member(&mut self, c: &ConversationId, i: &IdentityId) -> Result<(), String> {
        let g = self.gm(c)?;
        let b = g.members.len();
        g.members.retain(|x| &x.identity_id != i);
        if g.members.len() == b {
            return Err("not found".into());
        }
        g.pending_rekey = true;
        Ok(())
    }
    fn seal(&self, msg: &MessageEnvelope) -> Result<MlsCiphertextEnvelope, String> {
        let g = self.g(&msg.conversation_id)?;
        if !g.members.iter().any(|m| {
            m.identity_id == msg.sender
                && m.device_id.as_ref().is_none_or(|d| d == &msg.sender_device)
        }) {
            return Err("sender not in group".into());
        }
        let pt = postcard::to_allocvec(msg).map_err(|e| format!("enc: {e}"))?;
        let uk = aead::UnboundKey::new(&aead::AES_256_GCM, &g.group_key)
            .map_err(|e| format!("key: {e}"))?;
        let sk = aead::LessSafeKey::new(uk);
        let nb = derive_nonce(&g.group_id, g.epoch, msg.timestamp_ms as u64);
        let mut ct = pt;
        sk.seal_in_place_append_tag(
            aead::Nonce::assume_unique_for_key(nb),
            aead::Aad::empty(),
            &mut ct,
        )
        .map_err(|e| format!("seal: {e}"))?;
        let mut framed = nb.to_vec();
        framed.append(&mut ct);
        Ok(MlsCiphertextEnvelope {
            group_id: g.group_id.clone(),
            conversation_id: msg.conversation_id.clone(),
            epoch: g.epoch,
            sender: msg.sender.clone(),
            sender_device: msg.sender_device.clone(),
            payload_type: msg.payload_type.clone(),
            wire_format: MlsWireFormat::Aes256Gcm,
            ciphertext: framed,
        })
    }
    #[allow(clippy::possible_missing_else)]
    fn open(&self, env: &MlsCiphertextEnvelope) -> Result<MessageEnvelope, String> {
        let g = self.g(&env.conversation_id)?;
        if g.group_id != env.group_id {
            return Err("group mismatch".into());
        }
        if env.wire_format == MlsWireFormat::SkeletonPostcard {
            let m: MessageEnvelope =
                postcard::from_bytes(&env.ciphertext).map_err(|e| format!("dec: {e}"))?;
            if m.conversation_id != env.conversation_id {
                return Err("conv mismatch".into());
            }
            return Ok(m);
        }
        if env.ciphertext.len() < NONCE_LEN + 16 {
            return Err("too short".into());
        }
        let nb: [u8; NONCE_LEN] = env.ciphertext[..NONCE_LEN]
            .try_into()
            .map_err(|_| "nonce".to_string())?;
        let uk = aead::UnboundKey::new(&aead::AES_256_GCM, &g.group_key)
            .map_err(|e| format!("key: {e}"))?;
        let ok = aead::LessSafeKey::new(uk);
        let mut pt = env.ciphertext[NONCE_LEN..].to_vec();
        let dec = ok
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nb),
                aead::Aad::empty(),
                &mut pt,
            )
            .map_err(|e| format!("open: {e}"))?;
        let m: MessageEnvelope = postcard::from_bytes(dec).map_err(|e| format!("dec: {e}"))?;
        if m.conversation_id != env.conversation_id {
            return Err("conv mismatch".into());
        }
        Ok(m)
    }
}

#[deprecated]
pub type InMemorySecureSessionManager = SkeletonSecureSessionManager;

#[cfg(test)]
mod tests {
    use super::*;
    use chat_core::{ClientProfile, MessageBody, MessageId};
    fn dm() -> Vec<MlsMember> {
        vec![
            MlsMember::device("rsaga", "d1"),
            MlsMember::device("builder", "d2"),
        ]
    }
    fn msg() -> MessageEnvelope {
        MessageEnvelope {
            message_id: MessageId("m1".into()),
            conversation_id: ConversationId("dm:rsaga:builder".into()),
            sender: IdentityId("rsaga".into()),
            reply_to_message_id: None,
            sender_device: DeviceId("d1".into()),
            sender_profile: ClientProfile::desktop_terminal(),
            payload_type: PayloadType::Text,
            body: MessageBody {
                preview: "h".into(),
                plain_text: "hello".into(),
                language_tag: "en".into(),
            },
            ciphertext: vec![],
            timestamp_ms: 1_763_560_000_000,
            ephemeral: false,
        }
    }
    #[test]
    fn crypto_key_helpers_do_not_panic_on_rng_or_hkdf_failure() {
        let source = include_str!("lib.rs");
        for pattern in [
            format!(".expect({:?})", "RNG"),
            format!(".expect({:?})", "HKDF"),
            format!(".expect({:?})", "fill"),
        ] {
            assert!(
                !source.contains(&pattern),
                "production crypto helper should return errors instead of panicking on {pattern}"
            );
        }
    }
    #[test]
    fn t_direct() {
        let mut m = SkeletonSecureSessionManager::new();
        let g = m
            .bootstrap_direct(&ConversationId("dm:rsaga:builder".into()), dm())
            .unwrap();
        assert_eq!(g.kind, MlsGroupKind::Direct);
        assert_eq!(g.members.len(), 2);
    }
    #[test]
    fn t_epoch() {
        let mut m = SkeletonSecureSessionManager::new();
        let c = ConversationId("room:o".into());
        m.bootstrap_room(
            &c,
            ConversationScope::CityPublic,
            vec![
                MlsMember::identity("a"),
                MlsMember::identity("b"),
                MlsMember::identity("c"),
            ],
        )
        .unwrap();
        m.add_member(&c, MlsMember::identity("d")).unwrap();
        assert!(m.group_state(&c).unwrap().pending_rekey);
        assert_eq!(m.rotate_epoch(&c).unwrap(), 2);
    }
    #[test]
    fn t_seal() {
        let mut m = SkeletonSecureSessionManager::new();
        let c = ConversationId("dm:rsaga:builder".into());
        m.bootstrap_direct(&c, dm()).unwrap();
        let s = m.seal(&msg()).unwrap();
        let o = m.open(&s).unwrap();
        assert_eq!(o, msg());
        assert_eq!(s.epoch, 1);
    }
    #[test]
    fn t_outsider() {
        let mut m = SkeletonSecureSessionManager::new();
        m.bootstrap_direct(&ConversationId("dm:rsaga:builder".into()), dm())
            .unwrap();
        let mut x = msg();
        x.sender = IdentityId("bad".into());
        assert!(m.seal(&x).is_err());
    }
    #[test]
    fn t_snap() {
        let mut m = SkeletonSecureSessionManager::new();
        m.bootstrap_direct(&ConversationId("dm:rsaga:builder".into()), dm())
            .unwrap();
        let s = m.snapshot();
        let mut r = SkeletonSecureSessionManager::new();
        r.restore(s);
        assert_eq!(
            r.group_state(&ConversationId("dm:rsaga:builder".into()))
                .unwrap()
                .members
                .len(),
            2
        );
    }
    #[test]
    fn public_group_view_never_serializes_group_key() {
        let mut manager = SkeletonSecureSessionManager::new();
        let group = manager
            .bootstrap_direct(&ConversationId("dm:rsaga:builder".into()), dm())
            .unwrap();
        let serialized = serde_json::to_value(MlsGroupView::from(&group)).unwrap();
        assert!(serialized.get("group_key").is_none());
        assert_eq!(serialized["group_id"], "mls:dm:rsaga:builder");
    }
    #[test]
    fn sealed_snapshot_roundtrips_without_plaintext_group_key() {
        let mut manager = SkeletonSecureSessionManager::new();
        let conversation = ConversationId("dm:rsaga:builder".into());
        manager.bootstrap_direct(&conversation, dm()).unwrap();
        let expected_key = manager
            .group_state(&conversation)
            .unwrap()
            .group_key
            .clone();
        let storage_key =
            SecureSessionStorageKey::from_secret("unit-test-secure-session-storage-secret-0001")
                .unwrap();

        let sealed = manager.seal_snapshot(&storage_key).unwrap();
        let serialized = serde_json::to_string(&sealed).unwrap();
        assert!(!serialized.contains("group_key"));
        assert!(!serialized.contains(&hex::encode(&expected_key)));

        let mut restored = SkeletonSecureSessionManager::new();
        restored
            .restore_sealed_snapshot(&sealed, &storage_key)
            .unwrap();
        assert_eq!(
            restored.group_state(&conversation).unwrap().group_key,
            expected_key
        );
    }
    #[test]
    fn sealed_snapshot_rejects_wrong_storage_key() {
        let mut manager = SkeletonSecureSessionManager::new();
        manager
            .bootstrap_direct(&ConversationId("dm:rsaga:builder".into()), dm())
            .unwrap();
        let storage_key =
            SecureSessionStorageKey::from_secret("unit-test-secure-session-storage-secret-0001")
                .unwrap();
        let wrong_key =
            SecureSessionStorageKey::from_secret("unit-test-secure-session-storage-secret-0002")
                .unwrap();
        let sealed = manager.seal_snapshot(&storage_key).unwrap();

        let mut restored = SkeletonSecureSessionManager::new();
        assert!(
            restored
                .restore_sealed_snapshot(&sealed, &wrong_key)
                .is_err()
        );
    }
    #[test]
    fn t_enc() {
        let mut m = SkeletonSecureSessionManager::new();
        let c = ConversationId("dm:rsaga:builder".into());
        m.bootstrap_direct(&c, dm()).unwrap();
        let x = msg();
        let s = m.seal(&x).unwrap();
        assert_eq!(s.wire_format, MlsWireFormat::Aes256Gcm);
        let pl = postcard::to_allocvec(&x).unwrap();
        assert!(s.ciphertext.len() > pl.len());
    }
    #[test]
    fn t_rot() {
        let mut m = SkeletonSecureSessionManager::new();
        let c = ConversationId("dm:rsaga:builder".into());
        m.bootstrap_direct(&c, dm()).unwrap();
        let x = msg();
        let s1 = m.seal(&x).unwrap();
        m.rotate_epoch(&c).unwrap();
        let s2 = m.seal(&x).unwrap();
        assert_ne!(s1.ciphertext, s2.ciphertext);
        let o2 = m.open(&s2).unwrap();
        assert_eq!(o2.body.plain_text, x.body.plain_text);
    }
    #[test]
    fn t_pfs() {
        let mut m = SkeletonSecureSessionManager::new();
        let c = ConversationId("dm:rsaga:builder".into());
        m.bootstrap_direct(&c, dm()).unwrap();
        let ok = m.group_state(&c).unwrap().group_key.clone();
        let x = msg();
        let s = m.seal(&x).unwrap();
        m.rotate_epoch(&c).unwrap();
        assert_ne!(ok, m.group_state(&c).unwrap().group_key);
        assert!(m.open(&s).is_err());
    }
    #[test]
    fn t_wrong() {
        let mut m = SkeletonSecureSessionManager::new();
        m.bootstrap_direct(&ConversationId("dm:rsaga:builder".into()), dm())
            .unwrap();
        m.bootstrap_direct(
            &ConversationId("dm:a:b".into()),
            vec![MlsMember::device("a", "x"), MlsMember::device("b", "y")],
        )
        .unwrap();
        let mut x = msg();
        x.conversation_id = ConversationId("dm:a:b".into());
        x.sender = IdentityId("a".into());
        x.sender_device = DeviceId("x".into());
        let s = m.seal(&x).unwrap();
        let mut w = s;
        w.conversation_id = ConversationId("dm:rsaga:builder".into());
        assert!(m.open(&w).is_err());
    }
    #[test]
    fn t_det() {
        let m = vec![
            MlsMember::device("a", "x").with_public_key("p1"),
            MlsMember::device("b", "y").with_public_key("p2"),
        ];
        assert_eq!(derive_group_key(&m).unwrap(), derive_group_key(&m).unwrap());
    }
    #[test]
    fn t_diff() {
        let a = vec![
            MlsMember::device("a", "x").with_public_key("aa"),
            MlsMember::device("b", "y").with_public_key("bb"),
        ];
        let b = vec![
            MlsMember::device("a", "x").with_public_key("cc"),
            MlsMember::device("b", "y").with_public_key("dd"),
        ];
        assert_ne!(derive_group_key(&a).unwrap(), derive_group_key(&b).unwrap());
    }
    #[test]
    fn t_ord() {
        let k1 = derive_group_key(&[
            MlsMember::device("a", "x").with_public_key("p1"),
            MlsMember::device("b", "y").with_public_key("p2"),
        ])
        .unwrap();
        let k2 = derive_group_key(&[
            MlsMember::device("b", "y").with_public_key("p2"),
            MlsMember::device("a", "x").with_public_key("p1"),
        ])
        .unwrap();
        assert_eq!(k1, k2);
    }
    #[test]
    fn t_cross() {
        let ms = vec![
            MlsMember::device("alice", "p1").with_public_key("ak"),
            MlsMember::device("bob", "p2").with_public_key("bk"),
        ];
        let c = ConversationId("dm:alice:bob".into());
        let mut a = SkeletonSecureSessionManager::new();
        let mut b = SkeletonSecureSessionManager::new();
        a.bootstrap_direct(&c, ms.clone()).unwrap();
        b.bootstrap_direct(&c, ms).unwrap();
        assert_eq!(
            a.group_state(&c).unwrap().group_key,
            b.group_state(&c).unwrap().group_key
        );
        let mut x = msg();
        x.conversation_id = c;
        x.sender = IdentityId("alice".into());
        x.sender_device = DeviceId("p1".into());
        let s = a.seal(&x).unwrap();
        let o = b.open(&s).unwrap();
        assert_eq!(o.body.plain_text, x.body.plain_text);
    }
    #[test]
    fn t_empty() {
        let mut m = SkeletonSecureSessionManager::new();
        assert!(
            m.bootstrap_direct(&ConversationId("dm:a:b".into()), vec![])
                .is_err()
        );
    }
    #[test]
    fn t_three_direct() {
        let mut m = SkeletonSecureSessionManager::new();
        assert!(
            m.bootstrap_direct(
                &ConversationId("dm:a:b".into()),
                vec![
                    MlsMember::identity("a"),
                    MlsMember::identity("b"),
                    MlsMember::identity("c")
                ]
            )
            .is_err()
        );
    }
    #[test]
    fn t_three_room() {
        let mut m = SkeletonSecureSessionManager::new();
        assert!(
            m.bootstrap_room(
                &ConversationId("room:t".into()),
                ConversationScope::CityPublic,
                vec![
                    MlsMember::identity("a"),
                    MlsMember::identity("b"),
                    MlsMember::identity("c")
                ]
            )
            .is_ok()
        );
    }
    #[test]
    fn t_dup() {
        let mut m = SkeletonSecureSessionManager::new();
        let c = ConversationId("room:t".into());
        m.bootstrap_room(
            &c,
            ConversationScope::CityPublic,
            vec![MlsMember::identity("a"), MlsMember::identity("b")],
        )
        .unwrap();
        assert!(m.add_member(&c, MlsMember::identity("a")).is_err());
    }
    #[test]
    fn t_rm() {
        let mut m = SkeletonSecureSessionManager::new();
        let c = ConversationId("room:t".into());
        m.bootstrap_room(
            &c,
            ConversationScope::CityPublic,
            vec![MlsMember::identity("a")],
        )
        .unwrap();
        assert!(m.remove_member(&c, &IdentityId("x".into())).is_err());
    }
    #[test]
    fn t_private_room() {
        let mut m = SkeletonSecureSessionManager::new();
        assert!(
            m.bootstrap_room(
                &ConversationId("room:t".into()),
                ConversationScope::Private,
                vec![MlsMember::identity("a")]
            )
            .is_err()
        );
    }
    #[test]
    fn t_five_member_room() {
        let mut m = SkeletonSecureSessionManager::new();
        let c = ConversationId("room:big".into());
        m.bootstrap_room(
            &c,
            ConversationScope::CityPublic,
            vec![
                MlsMember::identity("a"),
                MlsMember::identity("b"),
                MlsMember::identity("c"),
                MlsMember::identity("d"),
                MlsMember::identity("e"),
            ],
        )
        .unwrap();
        m.rotate_epoch(&c).unwrap();
        m.add_member(&c, MlsMember::identity("f")).unwrap();
        assert_eq!(m.group_state(&c).unwrap().members.len(), 6);
    }
}
