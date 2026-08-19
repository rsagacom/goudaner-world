use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use prost::Message as _;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _, PermissionsExt as _};

const ENVELOPE_VERSION: u32 = 1;
const LEDGER_VERSION: u32 = 1;
const MIN_OPAQUE_ID_BYTES: usize = 16;
const MAX_OPAQUE_ID_BYTES: usize = 64;
const MIN_ROUTING_TOKEN_BYTES: usize = 16;
const MAX_ROUTING_TOKEN_BYTES: usize = 64;
const MAX_CIPHERTEXT_BYTES: usize = 1024 * 1024;
const MAX_META_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_LEDGER_MESSAGES: usize = 100_000;
const MAX_PENDING_REQUESTS: usize = 10_000;
const MAX_REQUEST_ID_BYTES: usize = 128;
const MAX_MESSAGE_HASH_BYTES: usize = 256;
const MAX_STORE_CURSOR_BYTES: usize = 4096;
const MAX_PEER_ADDRESS_BYTES: usize = 2048;
const LEDGER_FILE_NAME: &str = "native-waku-ledger.json";

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, PartialEq, prost::Message)]
struct WireOpaqueEncryptedEnvelope {
    #[prost(uint32, tag = "1")]
    protocol_version: u32,
    #[prost(bytes = "vec", tag = "2")]
    routing_token_sha256: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    opaque_message_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    ciphertext: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    ciphertext_sha256: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct OpaqueEncryptedEnvelope {
    protocol_version: u32,
    routing_token_sha256: [u8; 32],
    opaque_message_id: Vec<u8>,
    ciphertext: Vec<u8>,
    ciphertext_sha256: [u8; 32],
}

impl fmt::Debug for OpaqueEncryptedEnvelope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpaqueEncryptedEnvelope")
            .field("protocol_version", &self.protocol_version)
            .field("routing", &"[REDACTED]")
            .field("opaque_message_id", &"[REDACTED]")
            .field("ciphertext", &"[REDACTED]")
            .field("ciphertext_len", &self.ciphertext.len())
            .finish()
    }
}

impl OpaqueEncryptedEnvelope {
    pub fn new(
        routing_token: &[u8],
        opaque_message_id: Vec<u8>,
        ciphertext: Vec<u8>,
    ) -> Result<Self, NativeRestError> {
        validate_bounded_bytes(
            routing_token,
            MIN_ROUTING_TOKEN_BYTES,
            MAX_ROUTING_TOKEN_BYTES,
            NativeRestError::InvalidRoutingToken,
        )?;
        validate_bounded_bytes(
            &opaque_message_id,
            MIN_OPAQUE_ID_BYTES,
            MAX_OPAQUE_ID_BYTES,
            NativeRestError::InvalidOpaqueMessageId,
        )?;
        validate_bounded_bytes(
            &ciphertext,
            1,
            MAX_CIPHERTEXT_BYTES,
            NativeRestError::InvalidCiphertext,
        )?;

        let routing_token_sha256 = sha256_array(routing_token);
        let ciphertext_sha256 = sha256_array(&ciphertext);
        Ok(Self {
            protocol_version: ENVELOPE_VERSION,
            routing_token_sha256,
            opaque_message_id,
            ciphertext,
            ciphertext_sha256,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        self.to_wire().encode_to_vec()
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, NativeRestError> {
        if encoded.is_empty() || encoded.len() > MAX_CIPHERTEXT_BYTES + 512 {
            return Err(NativeRestError::InvalidProtobuf);
        }
        let wire = WireOpaqueEncryptedEnvelope::decode(encoded)
            .map_err(|_| NativeRestError::InvalidProtobuf)?;
        let envelope = Self::from_wire(wire)?;
        if envelope.encode() != encoded {
            return Err(NativeRestError::NonCanonicalProtobuf);
        }
        Ok(envelope)
    }

    pub fn content_topic(&self) -> String {
        topic_from_routing_hash(&self.routing_token_sha256)
    }

    pub fn opaque_message_id(&self) -> &[u8] {
        &self.opaque_message_id
    }

    pub fn routing_token_sha256(&self) -> &[u8; 32] {
        &self.routing_token_sha256
    }

    pub fn ciphertext_sha256(&self) -> &[u8; 32] {
        &self.ciphertext_sha256
    }

    fn to_wire(&self) -> WireOpaqueEncryptedEnvelope {
        WireOpaqueEncryptedEnvelope {
            protocol_version: self.protocol_version,
            routing_token_sha256: self.routing_token_sha256.to_vec(),
            opaque_message_id: self.opaque_message_id.clone(),
            ciphertext: self.ciphertext.clone(),
            ciphertext_sha256: self.ciphertext_sha256.to_vec(),
        }
    }

    fn from_wire(wire: WireOpaqueEncryptedEnvelope) -> Result<Self, NativeRestError> {
        if wire.protocol_version != ENVELOPE_VERSION {
            return Err(NativeRestError::UnsupportedEnvelopeVersion);
        }
        let routing_token_sha256 =
            vec_to_digest(wire.routing_token_sha256).ok_or(NativeRestError::InvalidRoutingHash)?;
        validate_bounded_bytes(
            &wire.opaque_message_id,
            MIN_OPAQUE_ID_BYTES,
            MAX_OPAQUE_ID_BYTES,
            NativeRestError::InvalidOpaqueMessageId,
        )?;
        validate_bounded_bytes(
            &wire.ciphertext,
            1,
            MAX_CIPHERTEXT_BYTES,
            NativeRestError::InvalidCiphertext,
        )?;
        let ciphertext_sha256 =
            vec_to_digest(wire.ciphertext_sha256).ok_or(NativeRestError::InvalidCiphertextHash)?;
        if sha256_array(&wire.ciphertext) != ciphertext_sha256 {
            return Err(NativeRestError::CiphertextHashMismatch);
        }
        Ok(Self {
            protocol_version: wire.protocol_version,
            routing_token_sha256,
            opaque_message_id: wire.opaque_message_id,
            ciphertext: wire.ciphertext,
            ciphertext_sha256,
        })
    }
}

pub fn content_topic_for_routing_token(routing_token: &[u8]) -> Result<String, NativeRestError> {
    validate_bounded_bytes(
        routing_token,
        MIN_ROUTING_TOKEN_BYTES,
        MAX_ROUTING_TOKEN_BYTES,
        NativeRestError::InvalidRoutingToken,
    )?;
    Ok(topic_from_routing_hash(&sha256_array(routing_token)))
}

fn topic_from_routing_hash(routing_hash: &[u8; 32]) -> String {
    format!("/goudaner-world/1/messages-{:02x}/proto", routing_hash[0])
}

fn validate_content_topic(topic: &str) -> Result<(), NativeRestError> {
    let bytes = topic.as_bytes();
    let prefix = b"/goudaner-world/1/messages-";
    let suffix = b"/proto";
    if bytes.len() != prefix.len() + 2 + suffix.len()
        || !bytes.starts_with(prefix)
        || !bytes.ends_with(suffix)
        || !bytes[prefix.len()..prefix.len() + 2]
            .iter()
            .all(u8::is_ascii_hexdigit)
        || bytes[prefix.len()..prefix.len() + 2]
            .iter()
            .any(u8::is_ascii_uppercase)
    {
        return Err(NativeRestError::InvalidContentTopic);
    }
    Ok(())
}

fn validate_bounded_bytes(
    value: &[u8],
    minimum: usize,
    maximum: usize,
    error: NativeRestError,
) -> Result<(), NativeRestError> {
    if (minimum..=maximum).contains(&value.len()) {
        Ok(())
    } else {
        Err(error)
    }
}

#[derive(Clone, PartialEq, Eq)]
struct LoopbackBaseUrl(String);

impl LoopbackBaseUrl {
    fn parse(raw: &str) -> Result<Self, NativeRestError> {
        if raw.trim() != raw {
            return Err(NativeRestError::InvalidBaseUrl);
        }
        let authority = raw
            .strip_prefix("http://")
            .ok_or(NativeRestError::InvalidBaseUrl)?;
        let authority = authority.strip_suffix('/').unwrap_or(authority);
        if authority.is_empty()
            || authority.contains('/')
            || authority.contains('?')
            || authority.contains('#')
            || authority.contains('@')
        {
            return Err(NativeRestError::InvalidBaseUrl);
        }
        let socket: SocketAddr = authority
            .parse()
            .map_err(|_| NativeRestError::InvalidBaseUrl)?;
        if socket.port() == 0 || !socket.ip().is_loopback() {
            return Err(NativeRestError::InvalidBaseUrl);
        }
        Ok(Self(format!("http://{socket}")))
    }

    fn endpoint(&self, path: &str) -> String {
        debug_assert!(path.starts_with('/'));
        format!("{}{path}", self.0)
    }
}

impl fmt::Debug for LoopbackBaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LoopbackBaseUrl([LOOPBACK])")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NativeRestError {
    InvalidBaseUrl,
    InsecureStateDirectory,
    InsecureLedgerPermissions,
    StateIo,
    LedgerJson,
    InvalidLedger,
    LedgerCapacity,
    HttpTransport(&'static str),
    HttpStatus(&'static str, u16),
    InvalidJson(&'static str),
    OversizedResponse(&'static str),
    UnhealthyNode,
    InvalidRoutingToken,
    InvalidRoutingHash,
    InvalidContentTopic,
    TopicMismatch,
    InvalidOpaqueMessageId,
    InvalidCiphertext,
    InvalidCiphertextHash,
    CiphertextHashMismatch,
    UnsupportedEnvelopeVersion,
    InvalidProtobuf,
    NonCanonicalProtobuf,
    InvalidBase64,
    InvalidRequestId,
    UnknownRequestId,
    InvalidMessageHash,
    MessageHashMismatch,
    MessageIdCollision,
    InvalidSendEvent,
    RemoteSendError,
    InvalidStoreCursor,
    InvalidStoreQuery,
    InvalidStoreStatus,
    InvalidStoreMessage,
    InvalidMeta,
}

impl NativeRestError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidBaseUrl => "invalid_base_url",
            Self::InsecureStateDirectory => "insecure_state_directory",
            Self::InsecureLedgerPermissions => "insecure_ledger_permissions",
            Self::StateIo => "state_io",
            Self::LedgerJson => "ledger_json",
            Self::InvalidLedger => "invalid_ledger",
            Self::LedgerCapacity => "ledger_capacity",
            Self::HttpTransport(_) => "http_transport",
            Self::HttpStatus(_, _) => "http_status",
            Self::InvalidJson(_) => "invalid_json",
            Self::OversizedResponse(_) => "oversized_response",
            Self::UnhealthyNode => "unhealthy_node",
            Self::InvalidRoutingToken => "invalid_routing_token",
            Self::InvalidRoutingHash => "invalid_routing_hash",
            Self::InvalidContentTopic => "invalid_content_topic",
            Self::TopicMismatch => "topic_mismatch",
            Self::InvalidOpaqueMessageId => "invalid_opaque_message_id",
            Self::InvalidCiphertext => "invalid_ciphertext",
            Self::InvalidCiphertextHash => "invalid_ciphertext_hash",
            Self::CiphertextHashMismatch => "ciphertext_hash_mismatch",
            Self::UnsupportedEnvelopeVersion => "unsupported_envelope_version",
            Self::InvalidProtobuf => "invalid_protobuf",
            Self::NonCanonicalProtobuf => "non_canonical_protobuf",
            Self::InvalidBase64 => "invalid_base64",
            Self::InvalidRequestId => "invalid_request_id",
            Self::UnknownRequestId => "unknown_request_id",
            Self::InvalidMessageHash => "invalid_message_hash",
            Self::MessageHashMismatch => "message_hash_mismatch",
            Self::MessageIdCollision => "message_id_collision",
            Self::InvalidSendEvent => "invalid_send_event",
            Self::RemoteSendError => "remote_send_error",
            Self::InvalidStoreCursor => "invalid_store_cursor",
            Self::InvalidStoreQuery => "invalid_store_query",
            Self::InvalidStoreStatus => "invalid_store_status",
            Self::InvalidStoreMessage => "invalid_store_message",
            Self::InvalidMeta => "invalid_meta",
        }
    }
}

impl fmt::Display for NativeRestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HttpStatus(operation, status) => {
                write!(formatter, "{}:{operation}:{status}", self.code())
            }
            Self::HttpTransport(operation)
            | Self::InvalidJson(operation)
            | Self::OversizedResponse(operation) => {
                write!(formatter, "{}:{operation}", self.code())
            }
            _ => formatter.write_str(self.code()),
        }
    }
}

impl fmt::Debug for NativeRestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl std::error::Error for NativeRestError {}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LedgerSnapshot {
    version: u32,
    seen_message_hashes: BTreeMap<String, SeenMessage>,
    seen_message_ids: BTreeMap<String, String>,
    received_envelopes: Vec<StoredReceivedEnvelope>,
    pending_requests: BTreeMap<String, PendingRequest>,
    store_cursor: Option<String>,
}

impl Default for LedgerSnapshot {
    fn default() -> Self {
        Self {
            version: LEDGER_VERSION,
            seen_message_hashes: BTreeMap::new(),
            seen_message_ids: BTreeMap::new(),
            received_envelopes: Vec::new(),
            pending_requests: BTreeMap::new(),
            store_cursor: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SeenMessage {
    opaque_message_id: String,
    payload_sha256: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredReceivedEnvelope {
    message_hash: String,
    opaque_message_id: String,
    content_topic: String,
    payload_sha256: String,
    envelope: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PendingRequest {
    opaque_message_id: String,
    content_topic: String,
    payload_sha256: String,
    network_message_hash: Option<String>,
}

struct PersistentLedger {
    path: PathBuf,
    snapshot: LedgerSnapshot,
}

impl fmt::Debug for PersistentLedger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PersistentLedger")
            .field("received_count", &self.snapshot.received_envelopes.len())
            .field("pending_count", &self.snapshot.pending_requests.len())
            .field("has_store_cursor", &self.snapshot.store_cursor.is_some())
            .finish()
    }
}

impl PersistentLedger {
    fn open(state_dir: &Path) -> Result<Self, NativeRestError> {
        ensure_secure_state_dir(state_dir)?;
        let path = state_dir.join(LEDGER_FILE_NAME);
        let snapshot = if path.exists() {
            validate_ledger_file(&path)?;
            let encoded = fs::read(&path).map_err(|_| NativeRestError::StateIo)?;
            let snapshot: LedgerSnapshot =
                serde_json::from_slice(&encoded).map_err(|_| NativeRestError::LedgerJson)?;
            validate_snapshot(&snapshot)?;
            snapshot
        } else {
            LedgerSnapshot::default()
        };
        let ledger = Self { path, snapshot };
        if !ledger.path.exists() {
            ledger.persist_snapshot(&ledger.snapshot)?;
        }
        Ok(ledger)
    }

    fn persist_snapshot(&self, snapshot: &LedgerSnapshot) -> Result<(), NativeRestError> {
        validate_snapshot(snapshot)?;
        let encoded = serde_json::to_vec(snapshot).map_err(|_| NativeRestError::LedgerJson)?;
        let parent = self.path.parent().ok_or(NativeRestError::StateIo)?;
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp_path = parent.join(format!(
            ".{LEDGER_FILE_NAME}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));

        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let result = (|| -> Result<(), NativeRestError> {
            let mut file = options
                .open(&temp_path)
                .map_err(|_| NativeRestError::StateIo)?;
            file.write_all(&encoded)
                .map_err(|_| NativeRestError::StateIo)?;
            file.sync_all().map_err(|_| NativeRestError::StateIo)?;
            fs::rename(&temp_path, &self.path).map_err(|_| NativeRestError::StateIo)?;
            #[cfg(unix)]
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600))
                .map_err(|_| NativeRestError::StateIo)?;
            File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| NativeRestError::StateIo)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        result
    }

    fn transact(
        &mut self,
        update: impl FnOnce(&mut LedgerSnapshot) -> Result<(), NativeRestError>,
    ) -> Result<(), NativeRestError> {
        let mut next = self.snapshot.clone();
        update(&mut next)?;
        self.persist_snapshot(&next)?;
        self.snapshot = next;
        Ok(())
    }

    fn record_pending(
        &mut self,
        request_id: &str,
        envelope: &OpaqueEncryptedEnvelope,
        payload_sha256: &str,
    ) -> Result<(), NativeRestError> {
        if self.snapshot.pending_requests.len() >= MAX_PENDING_REQUESTS {
            return Err(NativeRestError::LedgerCapacity);
        }
        let request_id = request_id.to_owned();
        let pending = PendingRequest {
            opaque_message_id: BASE64_STANDARD.encode(envelope.opaque_message_id()),
            content_topic: envelope.content_topic(),
            payload_sha256: payload_sha256.to_owned(),
            network_message_hash: None,
        };
        self.transact(move |snapshot| {
            if snapshot.pending_requests.contains_key(&request_id) {
                return Err(NativeRestError::InvalidRequestId);
            }
            snapshot.pending_requests.insert(request_id, pending);
            Ok(())
        })
    }

    fn record_send_hash(
        &mut self,
        request_id: &str,
        message_hash: &str,
    ) -> Result<(), NativeRestError> {
        let request_id = request_id.to_owned();
        let message_hash = message_hash.to_owned();
        self.transact(move |snapshot| {
            let pending = snapshot
                .pending_requests
                .get_mut(&request_id)
                .ok_or(NativeRestError::UnknownRequestId)?;
            if let Some(existing) = pending.network_message_hash.as_deref()
                && existing != message_hash
            {
                return Err(NativeRestError::MessageHashMismatch);
            }
            pending.network_message_hash = Some(message_hash);
            Ok(())
        })
    }

    fn complete_pending(&mut self, request_id: &str) -> Result<(), NativeRestError> {
        let request_id = request_id.to_owned();
        self.transact(move |snapshot| {
            if snapshot.pending_requests.remove(&request_id).is_none() {
                return Err(NativeRestError::UnknownRequestId);
            }
            Ok(())
        })
    }

    fn record_received(
        &mut self,
        candidates: &[ReceivedCandidate],
    ) -> Result<(Vec<bool>, usize), NativeRestError> {
        self.record_received_with_cursor(candidates, StoreCursorUpdate::Keep)
    }

    fn record_store_page(
        &mut self,
        candidates: &[ReceivedCandidate],
        pagination_cursor: Option<String>,
    ) -> Result<(Vec<bool>, usize), NativeRestError> {
        self.record_received_with_cursor(candidates, StoreCursorUpdate::Set(pagination_cursor))
    }

    fn record_received_with_cursor(
        &mut self,
        candidates: &[ReceivedCandidate],
        cursor_update: StoreCursorUpdate,
    ) -> Result<(Vec<bool>, usize), NativeRestError> {
        if self.snapshot.received_envelopes.len() + candidates.len() > MAX_LEDGER_MESSAGES {
            return Err(NativeRestError::LedgerCapacity);
        }
        let mut accepted = Vec::with_capacity(candidates.len());
        let mut duplicate_count = 0;
        let candidates = candidates.to_vec();
        let mut next = self.snapshot.clone();
        for candidate in &candidates {
            let message_id = BASE64_STANDARD.encode(candidate.envelope.opaque_message_id());
            if let Some(existing) = next.seen_message_hashes.get(&candidate.message_hash) {
                if existing.opaque_message_id != message_id
                    || existing.payload_sha256 != candidate.payload_sha256
                {
                    return Err(NativeRestError::MessageHashMismatch);
                }
                duplicate_count += 1;
                accepted.push(false);
                continue;
            }
            if let Some(existing_hash) = next.seen_message_ids.get(&message_id) {
                if existing_hash != &candidate.message_hash {
                    return Err(NativeRestError::MessageIdCollision);
                }
                return Err(NativeRestError::InvalidLedger);
            }
            next.seen_message_hashes.insert(
                candidate.message_hash.clone(),
                SeenMessage {
                    opaque_message_id: message_id.clone(),
                    payload_sha256: candidate.payload_sha256.clone(),
                },
            );
            next.seen_message_ids
                .insert(message_id.clone(), candidate.message_hash.clone());
            next.received_envelopes.push(StoredReceivedEnvelope {
                message_hash: candidate.message_hash.clone(),
                opaque_message_id: message_id,
                content_topic: candidate.content_topic.clone(),
                payload_sha256: candidate.payload_sha256.clone(),
                envelope: BASE64_STANDARD.encode(&candidate.encoded_envelope),
            });
            accepted.push(true);
        }
        if let StoreCursorUpdate::Set(cursor) = cursor_update {
            if let Some(value) = cursor.as_deref() {
                validate_store_cursor(value)?;
            }
            next.store_cursor = cursor;
        }
        if accepted.iter().any(|value| *value) || next.store_cursor != self.snapshot.store_cursor {
            self.persist_snapshot(&next)?;
            self.snapshot = next;
        }
        Ok((accepted, duplicate_count))
    }

    fn set_store_cursor(&mut self, cursor: Option<&str>) -> Result<(), NativeRestError> {
        if let Some(value) = cursor {
            validate_store_cursor(value)?;
        }
        let cursor = cursor.map(str::to_owned);
        self.transact(move |snapshot| {
            snapshot.store_cursor = cursor;
            Ok(())
        })
    }
}

enum StoreCursorUpdate {
    Keep,
    Set(Option<String>),
}

#[derive(Clone)]
struct ReceivedCandidate {
    message_hash: String,
    content_topic: String,
    payload_sha256: String,
    encoded_envelope: Vec<u8>,
    envelope: OpaqueEncryptedEnvelope,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublishWire<'a> {
    payload: String,
    content_topic: &'a str,
    ephemeral: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SendResponseWire {
    request_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HealthReportWire {
    node_health: String,
    #[serde(default)]
    protocols_health: Vec<BTreeMap<String, String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SendStatusWire {
    request_id: String,
    events: Vec<SendEventWire>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SendEventWire {
    kind: String,
    message_hash: String,
    #[serde(default)]
    error: Option<String>,
    timestamp: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReceivedRecordWire {
    message_hash: String,
    message: ReceivedMessageWire,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReceivedMessageWire {
    payload: String,
    content_topic: Option<String>,
    #[serde(default)]
    version: Option<f64>,
    #[serde(default)]
    timestamp: Option<f64>,
    #[serde(default)]
    ephemeral: Option<bool>,
    #[serde(default)]
    meta: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoreResponseWire {
    request_id: String,
    status_code: u32,
    status_desc: String,
    messages: Vec<StoreMessageItemWire>,
    #[serde(default)]
    pagination_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreMessageItemWire {
    #[serde(rename = "message_hash", alias = "messageHash")]
    message_hash: String,
    message: StoreMessageWire,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoreMessageWire {
    payload: String,
    content_topic: String,
    timestamp: i64,
    #[serde(default)]
    version: Option<i32>,
    #[serde(default)]
    ephemeral: Option<bool>,
    #[serde(default)]
    proof: Option<String>,
    #[serde(default)]
    meta: Option<String>,
}

pub struct PublishedRequest {
    request_id: String,
}

impl PublishedRequest {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
}

impl fmt::Debug for PublishedRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PublishedRequest")
            .field("request_id", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SendPollSummary {
    pub event_count: usize,
    pub sent_count: usize,
    pub propagated_count: usize,
}

impl SendPollSummary {
    pub fn acknowledged(self) -> bool {
        self.sent_count > 0 || self.propagated_count > 0
    }
}

pub struct ReceivedOpaqueEnvelope {
    envelope: OpaqueEncryptedEnvelope,
}

impl ReceivedOpaqueEnvelope {
    pub fn envelope(&self) -> &OpaqueEncryptedEnvelope {
        &self.envelope
    }
}

impl fmt::Debug for ReceivedOpaqueEnvelope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReceivedOpaqueEnvelope")
            .field("envelope", &"[REDACTED]")
            .finish()
    }
}

pub struct ReceivedBatch {
    pub messages: Vec<ReceivedOpaqueEnvelope>,
    pub duplicate_count: usize,
}

pub struct StoreRecoveryPage {
    pub messages: Vec<ReceivedOpaqueEnvelope>,
    pub duplicate_count: usize,
    pub has_next_cursor: bool,
}

impl fmt::Debug for StoreRecoveryPage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoreRecoveryPage")
            .field("message_count", &self.messages.len())
            .field("duplicate_count", &self.duplicate_count)
            .field("has_next_cursor", &self.has_next_cursor)
            .finish()
    }
}

impl fmt::Debug for ReceivedBatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReceivedBatch")
            .field("message_count", &self.messages.len())
            .field("duplicate_count", &self.duplicate_count)
            .finish()
    }
}

pub struct NativeWakuRestClient {
    base_url: LoopbackBaseUrl,
    agent: ureq::Agent,
    ledger: PersistentLedger,
}

impl fmt::Debug for NativeWakuRestClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeWakuRestClient")
            .field("base_url", &self.base_url)
            .field("ledger", &self.ledger)
            .finish()
    }
}

impl NativeWakuRestClient {
    pub fn open(base_url: &str, state_dir: impl AsRef<Path>) -> Result<Self, NativeRestError> {
        let base_url = LoopbackBaseUrl::parse(base_url)?;
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(30))
            .timeout_write(Duration::from_secs(10))
            .redirects(0)
            .build();
        let ledger = PersistentLedger::open(state_dir.as_ref())?;
        Ok(Self {
            base_url,
            agent,
            ledger,
        })
    }

    pub fn healthcheck(&self) -> Result<(), NativeRestError> {
        let response = call_request(
            self.agent.get(&self.base_url.endpoint("/health")).call(),
            "health",
        )?;
        let report: HealthReportWire = read_json(response, "health")?;
        if report.node_health != "Ready"
            || report
                .protocols_health
                .iter()
                .flat_map(BTreeMap::values)
                .any(|status| status == "Not Ready" || status == "Shutting Down")
        {
            return Err(NativeRestError::UnhealthyNode);
        }
        Ok(())
    }

    pub fn subscribe(&self, content_topics: &[String]) -> Result<(), NativeRestError> {
        if content_topics.is_empty() {
            return Err(NativeRestError::InvalidContentTopic);
        }
        let mut unique = BTreeSet::new();
        for topic in content_topics {
            validate_content_topic(topic)?;
            if !unique.insert(topic) {
                return Err(NativeRestError::InvalidContentTopic);
            }
        }
        let body = serde_json::to_value(content_topics)
            .map_err(|_| NativeRestError::InvalidJson("subscribe_request"))?;
        let response = call_request(
            self.agent
                .post(&self.base_url.endpoint("/messaging/v1/subscriptions"))
                .send_json(body),
            "subscribe",
        )?;
        read_bounded_body(response, "subscribe")?;
        Ok(())
    }

    pub fn publish(
        &mut self,
        envelope: &OpaqueEncryptedEnvelope,
        ephemeral: bool,
        meta: Option<&[u8]>,
    ) -> Result<PublishedRequest, NativeRestError> {
        if let Some(meta) = meta
            && meta.len() > MAX_META_BYTES
        {
            return Err(NativeRestError::InvalidMeta);
        }
        let encoded = envelope.encode();
        let content_topic = envelope.content_topic();
        validate_content_topic(&content_topic)?;
        let payload_sha256 = sha256_hex(&encoded);
        let body = PublishWire {
            payload: BASE64_STANDARD.encode(&encoded),
            content_topic: &content_topic,
            ephemeral,
            meta: meta.map(|value| BASE64_STANDARD.encode(value)),
        };
        let value = serde_json::to_value(&body)
            .map_err(|_| NativeRestError::InvalidJson("publish_request"))?;
        let response = call_request(
            self.agent
                .post(&self.base_url.endpoint("/messaging/v1/messages"))
                .send_json(value),
            "publish",
        )?;
        let response: SendResponseWire = read_json(response, "publish")?;
        validate_request_id(&response.request_id)?;
        self.ledger
            .record_pending(&response.request_id, envelope, &payload_sha256)?;
        Ok(PublishedRequest {
            request_id: response.request_id,
        })
    }

    pub fn poll_send_events(
        &mut self,
        request_id: &str,
    ) -> Result<SendPollSummary, NativeRestError> {
        validate_request_id(request_id)?;
        if !self
            .ledger
            .snapshot
            .pending_requests
            .contains_key(request_id)
        {
            return Err(NativeRestError::UnknownRequestId);
        }
        let path = format!("/messaging/v1/events/send/{request_id}");
        let response = match self.agent.get(&self.base_url.endpoint(&path)).call() {
            Ok(response) if response.status() == 200 => response,
            Ok(response) => {
                return Err(NativeRestError::HttpStatus(
                    "send_events",
                    response.status(),
                ));
            }
            Err(ureq::Error::Status(404, _)) => return Ok(SendPollSummary::default()),
            Err(ureq::Error::Status(status, _)) => {
                return Err(NativeRestError::HttpStatus("send_events", status));
            }
            Err(ureq::Error::Transport(_)) => {
                return Err(NativeRestError::HttpTransport("send_events"));
            }
        };
        let status: SendStatusWire = read_json(response, "send_events")?;
        if status.request_id != request_id {
            return Err(NativeRestError::InvalidRequestId);
        }
        let mut summary = SendPollSummary {
            event_count: status.events.len(),
            ..SendPollSummary::default()
        };
        let mut observed_hash: Option<&str> = None;
        for event in &status.events {
            validate_message_hash(&event.message_hash)?;
            if event.timestamp < 0 {
                return Err(NativeRestError::InvalidSendEvent);
            }
            if let Some(existing) = observed_hash
                && existing != event.message_hash
            {
                return Err(NativeRestError::MessageHashMismatch);
            }
            observed_hash = Some(&event.message_hash);
            match event.kind.as_str() {
                "sent" if event.error.is_none() => summary.sent_count += 1,
                "propagated" if event.error.is_none() => summary.propagated_count += 1,
                "error"
                    if event
                        .error
                        .as_deref()
                        .is_some_and(|value| !value.is_empty()) =>
                {
                    return Err(NativeRestError::RemoteSendError);
                }
                _ => return Err(NativeRestError::InvalidSendEvent),
            }
        }
        if let Some(message_hash) = observed_hash {
            self.ledger.record_send_hash(request_id, message_hash)?;
        }
        Ok(summary)
    }

    pub fn poll_received(&mut self) -> Result<ReceivedBatch, NativeRestError> {
        let response = call_request(
            self.agent
                .get(&self.base_url.endpoint("/messaging/v1/events/received"))
                .call(),
            "received_events",
        )?;
        let records: Vec<ReceivedRecordWire> = read_json(response, "received_events")?;
        let mut candidates = Vec::with_capacity(records.len());
        for record in records {
            validate_message_hash(&record.message_hash)?;
            let content_topic = record
                .message
                .content_topic
                .ok_or(NativeRestError::InvalidContentTopic)?;
            validate_content_topic(&content_topic)?;
            let encoded_envelope = decode_canonical_base64(&record.message.payload)?;
            if let Some(meta) = record.message.meta.as_deref() {
                let decoded = decode_canonical_base64(meta)?;
                if decoded.len() > MAX_META_BYTES {
                    return Err(NativeRestError::InvalidMeta);
                }
            }
            if record
                .message
                .version
                .is_some_and(|value| !value.is_finite())
                || record
                    .message
                    .timestamp
                    .is_some_and(|value| !value.is_finite())
            {
                return Err(NativeRestError::InvalidJson("received_events"));
            }
            let _ephemeral = record.message.ephemeral.unwrap_or(false);
            let envelope = OpaqueEncryptedEnvelope::decode(&encoded_envelope)?;
            if envelope.content_topic() != content_topic {
                return Err(NativeRestError::TopicMismatch);
            }
            candidates.push(ReceivedCandidate {
                message_hash: record.message_hash,
                content_topic,
                payload_sha256: sha256_hex(&encoded_envelope),
                encoded_envelope,
                envelope,
            });
        }
        let (accepted, duplicate_count) = self.ledger.record_received(&candidates)?;
        let messages = candidates
            .into_iter()
            .zip(accepted)
            .filter_map(|(candidate, accepted)| {
                accepted.then_some(ReceivedOpaqueEnvelope {
                    envelope: candidate.envelope,
                })
            })
            .collect();
        Ok(ReceivedBatch {
            messages,
            duplicate_count,
        })
    }

    pub fn recover_store_page(
        &mut self,
        peer_addr: &str,
        content_topics: &[String],
        page_size: u8,
    ) -> Result<StoreRecoveryPage, NativeRestError> {
        validate_peer_addr(peer_addr)?;
        if content_topics.is_empty() || page_size == 0 || page_size > 100 {
            return Err(NativeRestError::InvalidStoreQuery);
        }
        let mut unique_topics = BTreeSet::new();
        for topic in content_topics {
            validate_content_topic(topic)?;
            if !unique_topics.insert(topic) {
                return Err(NativeRestError::InvalidStoreQuery);
            }
        }
        let joined_topics = content_topics.join(",");
        let mut path = format!(
            "/store/v3/messages?peerAddr={}&includeData=true&contentTopics={}&pageSize={page_size}&ascending=true",
            percent_encode(peer_addr),
            percent_encode(&joined_topics)
        );
        if let Some(cursor) = self.ledger.snapshot.store_cursor.as_deref() {
            validate_store_cursor(cursor)?;
            path.push_str("&cursor=");
            path.push_str(&percent_encode(cursor));
        }
        let response = call_request(
            self.agent.get(&self.base_url.endpoint(&path)).call(),
            "store_recovery",
        )?;
        let response: StoreResponseWire = read_json(response, "store_recovery")?;
        validate_request_id(&response.request_id)?;
        if response.status_code != 200 || response.status_desc.trim().is_empty() {
            return Err(NativeRestError::InvalidStoreStatus);
        }
        if let Some(cursor) = response.pagination_cursor.as_deref() {
            validate_store_cursor(cursor)?;
        }
        let mut candidates = Vec::with_capacity(response.messages.len());
        for item in response.messages {
            validate_message_hash(&item.message_hash)?;
            validate_content_topic(&item.message.content_topic)?;
            if item.message.timestamp < 0 {
                return Err(NativeRestError::InvalidStoreMessage);
            }
            if let Some(meta) = item.message.meta.as_deref() {
                let decoded = decode_canonical_base64(meta)?;
                if decoded.len() > MAX_META_BYTES {
                    return Err(NativeRestError::InvalidMeta);
                }
            }
            if item
                .message
                .proof
                .as_deref()
                .is_some_and(|proof| proof.len() > MAX_MESSAGE_HASH_BYTES)
            {
                return Err(NativeRestError::InvalidStoreMessage);
            }
            let _version = item.message.version;
            let _ephemeral = item.message.ephemeral.unwrap_or(false);
            let encoded_envelope = decode_canonical_base64(&item.message.payload)?;
            let envelope = OpaqueEncryptedEnvelope::decode(&encoded_envelope)?;
            if envelope.content_topic() != item.message.content_topic {
                return Err(NativeRestError::TopicMismatch);
            }
            candidates.push(ReceivedCandidate {
                message_hash: item.message_hash,
                content_topic: item.message.content_topic,
                payload_sha256: sha256_hex(&encoded_envelope),
                encoded_envelope,
                envelope,
            });
        }
        let next_cursor = response.pagination_cursor;
        let has_next_cursor = next_cursor.is_some();
        let (accepted, duplicate_count) =
            self.ledger.record_store_page(&candidates, next_cursor)?;
        let messages = candidates
            .into_iter()
            .zip(accepted)
            .filter_map(|(candidate, accepted)| {
                accepted.then_some(ReceivedOpaqueEnvelope {
                    envelope: candidate.envelope,
                })
            })
            .collect();
        Ok(StoreRecoveryPage {
            messages,
            duplicate_count,
            has_next_cursor,
        })
    }

    pub fn complete_pending(&mut self, request_id: &str) -> Result<(), NativeRestError> {
        validate_request_id(request_id)?;
        self.ledger.complete_pending(request_id)
    }

    pub fn pending_request_count(&self) -> usize {
        self.ledger.snapshot.pending_requests.len()
    }

    pub fn received_envelope_count(&self) -> usize {
        self.ledger.snapshot.received_envelopes.len()
    }

    pub fn store_cursor(&self) -> Option<&str> {
        self.ledger.snapshot.store_cursor.as_deref()
    }

    pub fn set_store_cursor(&mut self, cursor: Option<&str>) -> Result<(), NativeRestError> {
        self.ledger.set_store_cursor(cursor)
    }
}

fn call_request(
    result: Result<ureq::Response, ureq::Error>,
    operation: &'static str,
) -> Result<ureq::Response, NativeRestError> {
    match result {
        Ok(response) if response.status() == 200 => Ok(response),
        Ok(response) => Err(NativeRestError::HttpStatus(operation, response.status())),
        Err(ureq::Error::Status(status, _)) => Err(NativeRestError::HttpStatus(operation, status)),
        Err(ureq::Error::Transport(_)) => Err(NativeRestError::HttpTransport(operation)),
    }
}

fn read_json<T: DeserializeOwned>(
    response: ureq::Response,
    operation: &'static str,
) -> Result<T, NativeRestError> {
    let content_type = response.header("Content-Type").unwrap_or_default();
    if !content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
    {
        return Err(NativeRestError::InvalidJson(operation));
    }
    let bytes = read_bounded_body(response, operation)?;
    serde_json::from_slice(&bytes).map_err(|_| NativeRestError::InvalidJson(operation))
}

fn read_bounded_body(
    response: ureq::Response,
    operation: &'static str,
) -> Result<Vec<u8>, NativeRestError> {
    let mut reader = response.into_reader().take(MAX_RESPONSE_BYTES + 1);
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| NativeRestError::HttpTransport(operation))?;
    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(NativeRestError::OversizedResponse(operation));
    }
    Ok(bytes)
}

fn ensure_secure_state_dir(path: &Path) -> Result<(), NativeRestError> {
    if path.as_os_str().is_empty() {
        return Err(NativeRestError::InsecureStateDirectory);
    }
    if path.exists() {
        let metadata = fs::symlink_metadata(path).map_err(|_| NativeRestError::StateIo)?;
        if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
            return Err(NativeRestError::InsecureStateDirectory);
        }
    } else {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        builder.mode(0o700);
        builder.create(path).map_err(|_| NativeRestError::StateIo)?;
    }
    #[cfg(unix)]
    {
        let permissions = fs::metadata(path)
            .map_err(|_| NativeRestError::StateIo)?
            .permissions()
            .mode()
            & 0o777;
        if permissions & 0o077 != 0 {
            return Err(NativeRestError::InsecureStateDirectory);
        }
    }
    Ok(())
}

fn validate_ledger_file(path: &Path) -> Result<(), NativeRestError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| NativeRestError::StateIo)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(NativeRestError::InvalidLedger);
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o777 != 0o600 {
        return Err(NativeRestError::InsecureLedgerPermissions);
    }
    Ok(())
}

fn validate_snapshot(snapshot: &LedgerSnapshot) -> Result<(), NativeRestError> {
    if snapshot.version != LEDGER_VERSION
        || snapshot.received_envelopes.len() > MAX_LEDGER_MESSAGES
        || snapshot.pending_requests.len() > MAX_PENDING_REQUESTS
        || snapshot.seen_message_hashes.len() != snapshot.received_envelopes.len()
        || snapshot.seen_message_ids.len() != snapshot.received_envelopes.len()
    {
        return Err(NativeRestError::InvalidLedger);
    }
    if let Some(cursor) = snapshot.store_cursor.as_deref() {
        validate_store_cursor(cursor)?;
    }
    for (request_id, pending) in &snapshot.pending_requests {
        validate_request_id(request_id)?;
        validate_content_topic(&pending.content_topic)?;
        validate_hex_digest(&pending.payload_sha256)?;
        let message_id = decode_canonical_base64(&pending.opaque_message_id)?;
        validate_bounded_bytes(
            &message_id,
            MIN_OPAQUE_ID_BYTES,
            MAX_OPAQUE_ID_BYTES,
            NativeRestError::InvalidLedger,
        )?;
        if let Some(hash) = pending.network_message_hash.as_deref() {
            validate_message_hash(hash)?;
        }
    }
    for stored in &snapshot.received_envelopes {
        validate_message_hash(&stored.message_hash)?;
        validate_content_topic(&stored.content_topic)?;
        validate_hex_digest(&stored.payload_sha256)?;
        let encoded = decode_canonical_base64(&stored.envelope)?;
        if sha256_hex(&encoded) != stored.payload_sha256 {
            return Err(NativeRestError::InvalidLedger);
        }
        let envelope = OpaqueEncryptedEnvelope::decode(&encoded)?;
        if envelope.content_topic() != stored.content_topic
            || BASE64_STANDARD.encode(envelope.opaque_message_id()) != stored.opaque_message_id
        {
            return Err(NativeRestError::InvalidLedger);
        }
        let seen = snapshot
            .seen_message_hashes
            .get(&stored.message_hash)
            .ok_or(NativeRestError::InvalidLedger)?;
        if seen.opaque_message_id != stored.opaque_message_id
            || seen.payload_sha256 != stored.payload_sha256
            || snapshot.seen_message_ids.get(&stored.opaque_message_id)
                != Some(&stored.message_hash)
        {
            return Err(NativeRestError::InvalidLedger);
        }
    }
    Ok(())
}

fn validate_request_id(value: &str) -> Result<(), NativeRestError> {
    if value.is_empty()
        || value.len() > MAX_REQUEST_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(NativeRestError::InvalidRequestId);
    }
    Ok(())
}

fn validate_message_hash(value: &str) -> Result<(), NativeRestError> {
    if value.is_empty()
        || value.len() > MAX_MESSAGE_HASH_BYTES
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(NativeRestError::InvalidMessageHash);
    }
    Ok(())
}

fn validate_store_cursor(value: &str) -> Result<(), NativeRestError> {
    if value.is_empty()
        || value.len() > MAX_STORE_CURSOR_BYTES
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(NativeRestError::InvalidStoreCursor);
    }
    Ok(())
}

fn validate_peer_addr(value: &str) -> Result<(), NativeRestError> {
    if value.is_empty()
        || value.len() > MAX_PEER_ADDRESS_BYTES
        || !value.starts_with('/')
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
        || !value.contains("/p2p/")
    {
        return Err(NativeRestError::InvalidStoreQuery);
    }
    Ok(())
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use fmt::Write as _;
            write!(&mut encoded, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    encoded
}

fn decode_canonical_base64(value: &str) -> Result<Vec<u8>, NativeRestError> {
    let decoded = BASE64_STANDARD
        .decode(value)
        .map_err(|_| NativeRestError::InvalidBase64)?;
    if BASE64_STANDARD.encode(&decoded) != value {
        return Err(NativeRestError::InvalidBase64);
    }
    Ok(decoded)
}

fn validate_hex_digest(value: &str) -> Result<(), NativeRestError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(NativeRestError::InvalidLedger);
    }
    Ok(())
}

fn sha256_array(value: &[u8]) -> [u8; 32] {
    Sha256::digest(value).into()
}

fn sha256_hex(value: &[u8]) -> String {
    let digest = sha256_array(value);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        use fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn vec_to_digest(value: Vec<u8>) -> Option<[u8; 32]> {
    value.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::thread;

    fn temp_state_dir(label: &str) -> PathBuf {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "lobster-waku-{label}-{}-{sequence}",
            std::process::id()
        ))
    }

    fn remove_temp_dir(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    fn sample_envelope() -> OpaqueEncryptedEnvelope {
        OpaqueEncryptedEnvelope::new(&[7; 32], vec![8; 16], vec![9; 64]).expect("sample envelope")
    }

    #[test]
    fn loopback_url_requires_http_ip_and_explicit_port() {
        assert!(LoopbackBaseUrl::parse("http://127.0.0.1:8645").is_ok());
        assert!(LoopbackBaseUrl::parse("http://127.1.2.3:1/").is_ok());
        assert!(LoopbackBaseUrl::parse("http://[::1]:8645").is_ok());
        for invalid in [
            "https://127.0.0.1:8645",
            "http://127.0.0.1",
            "http://localhost:8645",
            "http://0.0.0.0:8645",
            "http://192.168.1.2:8645",
            "http://127.0.0.1:0",
            "http://127.0.0.1:8645/path",
            " http://127.0.0.1:8645",
        ] {
            assert_eq!(
                LoopbackBaseUrl::parse(invalid),
                Err(NativeRestError::InvalidBaseUrl)
            );
        }
    }

    #[test]
    fn topic_is_only_sha256_bucket_and_token_is_bounded() {
        let token = [42; 32];
        let topic = content_topic_for_routing_token(&token).expect("topic");
        assert!(topic.starts_with("/goudaner-world/1/messages-"));
        assert!(topic.ends_with("/proto"));
        assert_eq!(topic.len(), "/goudaner-world/1/messages-xx/proto".len());
        assert_eq!(topic, topic_from_routing_hash(&sha256_array(&token)));
        assert_eq!(
            content_topic_for_routing_token(&[0; 15]),
            Err(NativeRestError::InvalidRoutingToken)
        );
        assert_eq!(
            content_topic_for_routing_token(&[0; 65]),
            Err(NativeRestError::InvalidRoutingToken)
        );
    }

    #[test]
    fn protobuf_roundtrip_is_canonical_and_integrity_checked() {
        let envelope = sample_envelope();
        let encoded = envelope.encode();
        let decoded = OpaqueEncryptedEnvelope::decode(&encoded).expect("decode");
        assert_eq!(decoded, envelope);

        let mut wire = envelope.to_wire();
        wire.ciphertext[0] ^= 1;
        assert_eq!(
            OpaqueEncryptedEnvelope::decode(&wire.encode_to_vec()),
            Err(NativeRestError::CiphertextHashMismatch)
        );

        let mut wire = envelope.to_wire();
        wire.protocol_version += 1;
        assert_eq!(
            OpaqueEncryptedEnvelope::decode(&wire.encode_to_vec()),
            Err(NativeRestError::UnsupportedEnvelopeVersion)
        );
    }

    #[test]
    fn debug_output_redacts_sensitive_envelope_fields() {
        let debug = format!("{:?}", sample_envelope());
        assert!(!debug.contains("090909"));
        assert!(!debug.contains("080808"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn ledger_is_0600_atomic_and_recovers_all_state() {
        let state_dir = temp_state_dir("ledger-recovery");
        let envelope = sample_envelope();
        let encoded = envelope.encode();
        let candidate = ReceivedCandidate {
            message_hash: "network-hash-1".into(),
            content_topic: envelope.content_topic(),
            payload_sha256: sha256_hex(&encoded),
            encoded_envelope: encoded,
            envelope: envelope.clone(),
        };
        {
            let mut ledger = PersistentLedger::open(&state_dir).expect("open ledger");
            ledger
                .record_pending("request-1", &envelope, &sha256_hex(&envelope.encode()))
                .expect("pending");
            ledger
                .record_received(std::slice::from_ref(&candidate))
                .expect("received");
            ledger.set_store_cursor(Some("cursor-1")).expect("cursor");
        }
        let path = state_dir.join(LEDGER_FILE_NAME);
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&path).expect("metadata").permissions().mode() & 0o777,
            0o600
        );
        let mut recovered = PersistentLedger::open(&state_dir).expect("recover ledger");
        assert_eq!(recovered.snapshot.pending_requests.len(), 1);
        assert_eq!(recovered.snapshot.received_envelopes.len(), 1);
        assert_eq!(recovered.snapshot.store_cursor.as_deref(), Some("cursor-1"));
        let (accepted, duplicates) = recovered
            .record_received(&[candidate])
            .expect("dedupe after restart");
        assert_eq!(accepted, vec![false]);
        assert_eq!(duplicates, 1);
        remove_temp_dir(&state_dir);
    }

    #[cfg(unix)]
    #[test]
    fn ledger_rejects_broad_permissions() {
        let state_dir = temp_state_dir("ledger-mode");
        let ledger = PersistentLedger::open(&state_dir).expect("create ledger");
        let path = ledger.path.clone();
        drop(ledger);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("chmod");
        assert!(matches!(
            PersistentLedger::open(&state_dir),
            Err(NativeRestError::InsecureLedgerPermissions)
        ));
        remove_temp_dir(&state_dir);
    }

    #[derive(Clone)]
    struct MockResponse {
        method: &'static str,
        path: String,
        content_type: &'static str,
        body: String,
        inspect_body: Option<fn(&[u8])>,
    }

    fn spawn_mock_server(responses: Vec<MockResponse>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
        let address = listener.local_addr().expect("mock address");
        let handle = thread::spawn(move || {
            for expected in responses {
                let (stream, _) = listener.accept().expect("accept");
                let mut reader = BufReader::new(stream);
                let mut request_line = String::new();
                reader.read_line(&mut request_line).expect("request line");
                let mut parts = request_line.split_whitespace();
                assert_eq!(parts.next(), Some(expected.method));
                assert_eq!(parts.next(), Some(expected.path.as_str()));
                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).expect("header");
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        content_length = value.trim().parse().expect("content length");
                    }
                }
                let mut body = vec![0; content_length];
                reader.read_exact(&mut body).expect("body");
                if let Some(inspect) = expected.inspect_body {
                    inspect(&body);
                }
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    expected.content_type,
                    expected.body.len(),
                    expected.body
                );
                reader
                    .get_mut()
                    .write_all(response.as_bytes())
                    .expect("response");
            }
        });
        (format!("http://{address}"), handle)
    }

    fn inspect_subscription_body(body: &[u8]) {
        let value: serde_json::Value = serde_json::from_slice(body).expect("subscription JSON");
        let topics = value.as_array().expect("topic array");
        assert_eq!(topics.len(), 1);
        validate_content_topic(topics[0].as_str().expect("topic string")).expect("topic format");
    }

    fn inspect_publish_body(body: &[u8]) {
        let value: serde_json::Value = serde_json::from_slice(body).expect("publish JSON");
        let object = value.as_object().expect("publish object");
        assert_eq!(
            object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            BTreeSet::from(["contentTopic", "ephemeral", "payload"])
        );
        let encoded = object["payload"].as_str().expect("payload base64");
        let bytes = decode_canonical_base64(encoded).expect("canonical base64");
        OpaqueEncryptedEnvelope::decode(&bytes).expect("opaque protobuf");
        assert_eq!(object["ephemeral"], false);
    }

    #[test]
    fn official_rest_contract_roundtrip_and_restart_dedupe() {
        let envelope = sample_envelope();
        let encoded = envelope.encode();
        let topic = envelope.content_topic();
        let received_body = serde_json::json!([{
            "messageHash": "network-hash-1",
            "message": {
                "payload": BASE64_STANDARD.encode(&encoded),
                "contentTopic": topic,
                "ephemeral": false
            }
        }])
        .to_string();
        let (base_url, server) = spawn_mock_server(vec![
            MockResponse {
                method: "GET",
                path: "/health".into(),
                content_type: "application/json",
                body: r#"{"nodeHealth":"Ready","protocolsHealth":[]}"#.into(),
                inspect_body: None,
            },
            MockResponse {
                method: "POST",
                path: "/messaging/v1/subscriptions".into(),
                content_type: "text/plain",
                body: "OK".into(),
                inspect_body: Some(inspect_subscription_body),
            },
            MockResponse {
                method: "POST",
                path: "/messaging/v1/messages".into(),
                content_type: "application/json",
                body: r#"{"requestId":"request-1"}"#.into(),
                inspect_body: Some(inspect_publish_body),
            },
            MockResponse {
                method: "GET",
                path: "/messaging/v1/events/send/request-1".into(),
                content_type: "application/json",
                body: r#"{"requestId":"request-1","events":[{"kind":"sent","messageHash":"network-hash-1","timestamp":1}]}"#.into(),
                inspect_body: None,
            },
            MockResponse {
                method: "GET",
                path: "/messaging/v1/events/received".into(),
                content_type: "application/json",
                body: received_body.clone(),
                inspect_body: None,
            },
            MockResponse {
                method: "GET",
                path: "/messaging/v1/events/received".into(),
                content_type: "application/json",
                body: received_body,
                inspect_body: None,
            },
        ]);
        let state_dir = temp_state_dir("rest-contract");
        let mut client = NativeWakuRestClient::open(&base_url, &state_dir).expect("client");
        client.healthcheck().expect("health");
        client
            .subscribe(std::slice::from_ref(&topic))
            .expect("subscribe");
        let published = client.publish(&envelope, false, None).expect("publish");
        assert_eq!(client.pending_request_count(), 1);
        let send = client
            .poll_send_events(published.request_id())
            .expect("send status");
        assert!(send.acknowledged());
        let received = client.poll_received().expect("received");
        assert_eq!(received.messages.len(), 1);
        assert_eq!(received.duplicate_count, 0);
        drop(client);
        let mut client = NativeWakuRestClient::open(&base_url, &state_dir).expect("restart");
        let duplicate = client.poll_received().expect("duplicate poll");
        assert!(duplicate.messages.is_empty());
        assert_eq!(duplicate.duplicate_count, 1);
        server.join().expect("mock server");
        remove_temp_dir(&state_dir);
    }

    #[test]
    fn store_cursor_recovers_after_restart_and_dedupes_with_received_events() {
        let envelope = sample_envelope();
        let encoded = envelope.encode();
        let topic = envelope.content_topic();
        let peer_addr = "/ip4/127.0.0.1/tcp/60001/p2p/peer-1";
        let first_path = format!(
            "/store/v3/messages?peerAddr={}&includeData=true&contentTopics={}&pageSize=100&ascending=true",
            percent_encode(peer_addr),
            percent_encode(&topic)
        );
        let second_path = format!("{first_path}&cursor={}", percent_encode("cursor+/="));
        let first_body = serde_json::json!({
            "requestId": "store-request-1",
            "statusCode": 200,
            "statusDesc": "OK",
            "messages": [{
                "message_hash": "store-network-hash-1",
                "message": {
                    "payload": BASE64_STANDARD.encode(&encoded),
                    "contentTopic": topic,
                    "timestamp": 1,
                    "ephemeral": false
                }
            }],
            "paginationCursor": "cursor+/="
        })
        .to_string();
        let second_body = serde_json::json!({
            "requestId": "store-request-2",
            "statusCode": 200,
            "statusDesc": "OK",
            "messages": [{
                "messageHash": "store-network-hash-1",
                "message": {
                    "payload": BASE64_STANDARD.encode(&encoded),
                    "contentTopic": topic,
                    "timestamp": 1
                }
            }]
        })
        .to_string();
        let received_body = serde_json::json!([{
            "messageHash": "store-network-hash-1",
            "message": {
                "payload": BASE64_STANDARD.encode(&encoded),
                "contentTopic": topic
            }
        }])
        .to_string();
        let (base_url, server) = spawn_mock_server(vec![
            MockResponse {
                method: "GET",
                path: first_path,
                content_type: "application/json",
                body: first_body,
                inspect_body: None,
            },
            MockResponse {
                method: "GET",
                path: second_path,
                content_type: "application/json",
                body: second_body,
                inspect_body: None,
            },
            MockResponse {
                method: "GET",
                path: "/messaging/v1/events/received".into(),
                content_type: "application/json",
                body: received_body,
                inspect_body: None,
            },
        ]);
        let state_dir = temp_state_dir("store-recovery");
        {
            let mut client = NativeWakuRestClient::open(&base_url, &state_dir).expect("client");
            let first = client
                .recover_store_page(peer_addr, std::slice::from_ref(&topic), 100)
                .expect("first store page");
            assert_eq!(first.messages.len(), 1);
            assert_eq!(first.duplicate_count, 0);
            assert!(first.has_next_cursor);
            assert_eq!(client.store_cursor(), Some("cursor+/="));
        }
        let mut client = NativeWakuRestClient::open(&base_url, &state_dir).expect("restart");
        assert_eq!(client.store_cursor(), Some("cursor+/="));
        let second = client
            .recover_store_page(peer_addr, std::slice::from_ref(&topic), 100)
            .expect("second store page");
        assert!(second.messages.is_empty());
        assert_eq!(second.duplicate_count, 1);
        assert!(!second.has_next_cursor);
        assert_eq!(client.store_cursor(), None);
        let received = client.poll_received().expect("received duplicate");
        assert!(received.messages.is_empty());
        assert_eq!(received.duplicate_count, 1);
        assert_eq!(client.received_envelope_count(), 1);
        server.join().expect("mock server");
        remove_temp_dir(&state_dir);
    }

    #[test]
    fn store_query_requires_peer_topics_and_bounded_page_size() {
        let state_dir = temp_state_dir("store-query");
        let mut client =
            NativeWakuRestClient::open("http://127.0.0.1:9", &state_dir).expect("client");
        let topic = sample_envelope().content_topic();
        for result in [
            client.recover_store_page("", std::slice::from_ref(&topic), 100),
            client.recover_store_page("/ip4/127.0.0.1/tcp/1", std::slice::from_ref(&topic), 100),
            client.recover_store_page("/ip4/127.0.0.1/tcp/1/p2p/peer", &[], 100),
            client.recover_store_page(
                "/ip4/127.0.0.1/tcp/1/p2p/peer",
                std::slice::from_ref(&topic),
                101,
            ),
        ] {
            assert!(matches!(result, Err(NativeRestError::InvalidStoreQuery)));
        }
        remove_temp_dir(&state_dir);
    }

    #[test]
    fn received_event_rejects_topic_hash_mismatch() {
        let envelope = sample_envelope();
        let body = serde_json::json!([{
            "messageHash": "network-hash-2",
            "message": {
                "payload": BASE64_STANDARD.encode(envelope.encode()),
                "contentTopic": "/goudaner-world/1/messages-00/proto"
            }
        }])
        .to_string();
        let (base_url, server) = spawn_mock_server(vec![MockResponse {
            method: "GET",
            path: "/messaging/v1/events/received".into(),
            content_type: "application/json",
            body,
            inspect_body: None,
        }]);
        let state_dir = temp_state_dir("topic-mismatch");
        let mut client = NativeWakuRestClient::open(&base_url, &state_dir).expect("client");
        assert!(matches!(
            client.poll_received(),
            Err(NativeRestError::TopicMismatch)
        ));
        server.join().expect("mock server");
        remove_temp_dir(&state_dir);
    }

    #[test]
    fn json_and_base64_fail_closed() {
        let (base_url, server) = spawn_mock_server(vec![
            MockResponse {
                method: "GET",
                path: "/health".into(),
                content_type: "text/plain",
                body: "not-json".into(),
                inspect_body: None,
            },
            MockResponse {
                method: "GET",
                path: "/messaging/v1/events/received".into(),
                content_type: "application/json",
                body: r#"[{"messageHash":"hash","message":{"payload":"***","contentTopic":"/goudaner-world/1/messages-00/proto"}}]"#.into(),
                inspect_body: None,
            },
        ]);
        let state_dir = temp_state_dir("invalid-wire");
        let mut client = NativeWakuRestClient::open(&base_url, &state_dir).expect("client");
        assert!(matches!(
            client.healthcheck(),
            Err(NativeRestError::InvalidJson("health"))
        ));
        assert!(matches!(
            client.poll_received(),
            Err(NativeRestError::InvalidBase64)
        ));
        server.join().expect("mock server");
        remove_temp_dir(&state_dir);
    }
}
