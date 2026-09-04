#[path = "../src/native_rest.rs"]
mod native_rest;

use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use native_rest::{NativeRestError, NativeWakuRestClient, OpaqueEncryptedEnvelope, ReceivedBatch};
use serde::Serialize;

const NODE_A_URL_ENV: &str = "LOBSTER_WAKU_LAB_NODE_A_URL";
const NODE_B_URL_ENV: &str = "LOBSTER_WAKU_LAB_NODE_B_URL";
const STATE_DIR_ENV: &str = "LOBSTER_WAKU_LAB_STATE_DIR";
const MESSAGE_COUNT_ENV: &str = "LOBSTER_WAKU_LAB_MESSAGE_COUNT";
const POLL_TIMEOUT_ENV: &str = "LOBSTER_WAKU_LAB_POLL_TIMEOUT_SECS";
const DEFAULT_MESSAGE_COUNT: usize = 100;
const DEFAULT_POLL_TIMEOUT_SECS: u64 = 60;
const MAX_MESSAGE_COUNT: usize = 10_000;
const MAX_POLL_TIMEOUT_SECS: u64 = 3_600;
const TEST_CIPHERTEXT_BYTES: usize = 128;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SuccessSummary {
    status: &'static str,
    requested_per_node: usize,
    published_total: usize,
    send_acknowledged_total: usize,
    cross_received_total: usize,
    duplicate_total: usize,
    self_echo_total: usize,
    unrelated_total: usize,
    pending_after_completion: usize,
    elapsed_millis: u128,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorSummary {
    status: &'static str,
    error_code: &'static str,
}

enum LabError {
    MissingEnvironment,
    InvalidEnvironment,
    RandomSource,
    PollTimeout,
    UnexpectedEnvelope,
    CiphertextVerification,
    Native(NativeRestError),
}

impl LabError {
    fn code(&self) -> &'static str {
        match self {
            Self::MissingEnvironment => "missing_environment",
            Self::InvalidEnvironment => "invalid_environment",
            Self::RandomSource => "random_source",
            Self::PollTimeout => "poll_timeout",
            Self::UnexpectedEnvelope => "unexpected_envelope",
            Self::CiphertextVerification => "ciphertext_verification",
            Self::Native(error) => error.code(),
        }
    }
}

impl From<NativeRestError> for LabError {
    fn from(error: NativeRestError) -> Self {
        Self::Native(error)
    }
}

struct PendingLabRequest {
    request_id: String,
    acknowledged: bool,
}

#[derive(Default)]
struct ReceiveCounters {
    cross_received: usize,
    duplicates: usize,
    self_echoes: usize,
    unrelated: usize,
}

fn main() {
    let (output, failed) = match run() {
        Ok(summary) => (serde_json::to_string(&summary), false),
        Err(error) => (
            serde_json::to_string(&ErrorSummary {
                status: "error",
                error_code: error.code(),
            }),
            true,
        ),
    };
    match output {
        Ok(ref json) => println!("{json}"),
        Err(_) => println!("{}", r#"{"status":"error","errorCode":"json_encoding"}"#),
    }
    if failed || output.is_err() {
        std::process::exit(1);
    }
}

fn run() -> Result<SuccessSummary, LabError> {
    let started = Instant::now();
    let node_a_url = required_env(NODE_A_URL_ENV)?;
    let node_b_url = required_env(NODE_B_URL_ENV)?;
    let state_dir = PathBuf::from(required_env(STATE_DIR_ENV)?);
    let message_count = bounded_env_usize(
        MESSAGE_COUNT_ENV,
        DEFAULT_MESSAGE_COUNT,
        1,
        MAX_MESSAGE_COUNT,
    )?;
    let poll_timeout_secs = bounded_env_u64(
        POLL_TIMEOUT_ENV,
        DEFAULT_POLL_TIMEOUT_SECS,
        1,
        MAX_POLL_TIMEOUT_SECS,
    )?;

    let mut node_a = NativeWakuRestClient::open(&node_a_url, state_dir.join("node-a"))?;
    let mut node_b = NativeWakuRestClient::open(&node_b_url, state_dir.join("node-b"))?;
    node_a.healthcheck()?;
    node_b.healthcheck()?;

    let routing_token = random_bytes(32)?;
    let routing_hash = *OpaqueEncryptedEnvelope::new(
        &routing_token,
        random_bytes(16)?,
        random_bytes(TEST_CIPHERTEXT_BYTES)?,
    )?
    .routing_token_sha256();
    let topic = native_rest::content_topic_for_routing_token(&routing_token)?;
    node_a.subscribe(std::slice::from_ref(&topic))?;
    node_b.subscribe(std::slice::from_ref(&topic))?;

    let mut expected_at_a = BTreeMap::<Vec<u8>, [u8; 32]>::new();
    let mut expected_at_b = BTreeMap::<Vec<u8>, [u8; 32]>::new();
    let mut own_at_a = BTreeMap::<Vec<u8>, [u8; 32]>::new();
    let mut own_at_b = BTreeMap::<Vec<u8>, [u8; 32]>::new();
    let mut pending_a = Vec::with_capacity(message_count);
    let mut pending_b = Vec::with_capacity(message_count);
    let mut counters_a = ReceiveCounters::default();
    let mut counters_b = ReceiveCounters::default();

    for _ in 0..message_count {
        let envelope_a = random_envelope(&routing_token)?;
        own_at_a.insert(
            envelope_a.opaque_message_id().to_vec(),
            *envelope_a.ciphertext_sha256(),
        );
        expected_at_b.insert(
            envelope_a.opaque_message_id().to_vec(),
            *envelope_a.ciphertext_sha256(),
        );
        let published_a = node_a.publish(&envelope_a, false, None)?;
        pending_a.push(PendingLabRequest {
            request_id: published_a.request_id().to_owned(),
            acknowledged: false,
        });

        let envelope_b = random_envelope(&routing_token)?;
        own_at_b.insert(
            envelope_b.opaque_message_id().to_vec(),
            *envelope_b.ciphertext_sha256(),
        );
        expected_at_a.insert(
            envelope_b.opaque_message_id().to_vec(),
            *envelope_b.ciphertext_sha256(),
        );
        let published_b = node_b.publish(&envelope_b, false, None)?;
        pending_b.push(PendingLabRequest {
            request_id: published_b.request_id().to_owned(),
            acknowledged: false,
        });

        process_received(
            node_a.poll_received()?,
            &routing_hash,
            &mut expected_at_a,
            &own_at_a,
            &mut counters_a,
        )?;
        process_received(
            node_b.poll_received()?,
            &routing_hash,
            &mut expected_at_b,
            &own_at_b,
            &mut counters_b,
        )?;
    }

    let deadline = Instant::now() + Duration::from_secs(poll_timeout_secs);
    loop {
        let mut progress = false;
        progress |= poll_send_requests(&mut node_a, &mut pending_a)?;
        progress |= poll_send_requests(&mut node_b, &mut pending_b)?;

        let before_a = counters_a.cross_received + counters_a.duplicates;
        let before_b = counters_b.cross_received + counters_b.duplicates;
        process_received(
            node_a.poll_received()?,
            &routing_hash,
            &mut expected_at_a,
            &own_at_a,
            &mut counters_a,
        )?;
        process_received(
            node_b.poll_received()?,
            &routing_hash,
            &mut expected_at_b,
            &own_at_b,
            &mut counters_b,
        )?;
        progress |= before_a != counters_a.cross_received + counters_a.duplicates;
        progress |= before_b != counters_b.cross_received + counters_b.duplicates;

        let all_acknowledged = pending_a.iter().all(|item| item.acknowledged)
            && pending_b.iter().all(|item| item.acknowledged);
        if all_acknowledged && expected_at_a.is_empty() && expected_at_b.is_empty() {
            break;
        }
        if Instant::now() >= deadline {
            return Err(LabError::PollTimeout);
        }
        if !progress {
            thread::sleep(Duration::from_millis(100));
        }
    }

    for request in &pending_a {
        node_a.complete_pending(&request.request_id)?;
    }
    for request in &pending_b {
        node_b.complete_pending(&request.request_id)?;
    }
    let pending_after_completion = node_a.pending_request_count() + node_b.pending_request_count();
    if pending_after_completion != 0 {
        return Err(LabError::UnexpectedEnvelope);
    }

    Ok(SuccessSummary {
        status: "ok",
        requested_per_node: message_count,
        published_total: pending_a.len() + pending_b.len(),
        send_acknowledged_total: pending_a.iter().filter(|item| item.acknowledged).count()
            + pending_b.iter().filter(|item| item.acknowledged).count(),
        cross_received_total: counters_a.cross_received + counters_b.cross_received,
        duplicate_total: counters_a.duplicates + counters_b.duplicates,
        self_echo_total: counters_a.self_echoes + counters_b.self_echoes,
        unrelated_total: counters_a.unrelated + counters_b.unrelated,
        pending_after_completion,
        elapsed_millis: started.elapsed().as_millis(),
    })
}

fn poll_send_requests(
    client: &mut NativeWakuRestClient,
    pending: &mut [PendingLabRequest],
) -> Result<bool, LabError> {
    let mut progress = false;
    for request in pending.iter_mut().filter(|item| !item.acknowledged) {
        let status = client.poll_send_events(&request.request_id)?;
        if status.acknowledged() {
            request.acknowledged = true;
            progress = true;
        }
    }
    Ok(progress)
}

fn process_received(
    batch: ReceivedBatch,
    routing_hash: &[u8; 32],
    expected: &mut BTreeMap<Vec<u8>, [u8; 32]>,
    own: &BTreeMap<Vec<u8>, [u8; 32]>,
    counters: &mut ReceiveCounters,
) -> Result<(), LabError> {
    counters.duplicates += batch.duplicate_count;
    for received in batch.messages {
        let envelope = received.envelope();
        if envelope.routing_token_sha256() != routing_hash {
            counters.unrelated += 1;
            continue;
        }
        if let Some(expected_digest) = expected.remove(envelope.opaque_message_id()) {
            if expected_digest != *envelope.ciphertext_sha256() {
                return Err(LabError::CiphertextVerification);
            }
            counters.cross_received += 1;
            continue;
        }
        if own.contains_key(envelope.opaque_message_id()) {
            counters.self_echoes += 1;
            continue;
        }
        return Err(LabError::UnexpectedEnvelope);
    }
    Ok(())
}

fn random_envelope(routing_token: &[u8]) -> Result<OpaqueEncryptedEnvelope, LabError> {
    Ok(OpaqueEncryptedEnvelope::new(
        routing_token,
        random_bytes(16)?,
        random_bytes(TEST_CIPHERTEXT_BYTES)?,
    )?)
}

fn random_bytes(length: usize) -> Result<Vec<u8>, LabError> {
    let mut bytes = vec![0; length];
    getrandom::getrandom(&mut bytes).map_err(|_| LabError::RandomSource)?;
    Ok(bytes)
}

fn required_env(name: &str) -> Result<String, LabError> {
    let value = env::var(name).map_err(|_| LabError::MissingEnvironment)?;
    if value.is_empty() {
        return Err(LabError::InvalidEnvironment);
    }
    Ok(value)
}

fn bounded_env_usize(
    name: &str,
    default: usize,
    minimum: usize,
    maximum: usize,
) -> Result<usize, LabError> {
    match env::var(name) {
        Ok(value) => value
            .parse::<usize>()
            .ok()
            .filter(|parsed| (minimum..=maximum).contains(parsed))
            .ok_or(LabError::InvalidEnvironment),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => Err(LabError::InvalidEnvironment),
    }
}

fn bounded_env_u64(name: &str, default: u64, minimum: u64, maximum: u64) -> Result<u64, LabError> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .ok()
            .filter(|parsed| (minimum..=maximum).contains(parsed))
            .ok_or(LabError::InvalidEnvironment),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => Err(LabError::InvalidEnvironment),
    }
}
