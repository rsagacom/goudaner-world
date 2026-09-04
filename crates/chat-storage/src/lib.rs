use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use chat_core::{
    ArchivePolicy, Conversation, ConversationId, ConversationKind, ConversationScope,
    DeliveryState, IdentityId, MessageEnvelope, MessageId, TimelineEntry,
};
use serde::{Deserialize, Serialize};

mod timeline_journal;

pub type StorageResult<T> = Result<T, String>;

static ATOMIC_WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn atomic_write(path: &Path, bytes: &[u8]) -> StorageResult<()> {
    let sequence = ATOMIC_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("snapshot");
    let temp_path = path.with_file_name(format!(
        ".{file_name}.{}.{}.{}.tmp",
        std::process::id(),
        timestamp,
        sequence,
    ));

    let write_result = (|| -> StorageResult<()> {
        let mut file = File::create(&temp_path)
            .map_err(|error| format!("create temp snapshot failed: {error}"))?;
        file.write_all(bytes)
            .map_err(|error| format!("write temp snapshot failed: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("sync temp snapshot failed: {error}"))?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    fs::rename(&temp_path, path).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        format!("replace snapshot failed: {error}")
    })?;

    Ok(())
}

pub fn atomic_write_file(path: &Path, bytes: &[u8]) -> StorageResult<()> {
    atomic_write(path, bytes)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyConversationV1 {
    conversation_id: ConversationId,
    kind: ConversationKind,
    content_topic: String,
    participants: Vec<IdentityId>,
    created_at_ms: i64,
    last_active_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyConversationV2 {
    conversation_id: ConversationId,
    kind: ConversationKind,
    scope: ConversationScope,
    content_topic: String,
    participants: Vec<IdentityId>,
    created_at_ms: i64,
    last_active_at_ms: i64,
}

impl From<LegacyConversationV1> for Conversation {
    fn from(value: LegacyConversationV1) -> Self {
        Self {
            conversation_id: value.conversation_id,
            kind: value.kind,
            scope: ConversationScope::Private,
            scene: None,
            content_topic: value.content_topic,
            participants: value.participants,
            created_at_ms: value.created_at_ms,
            last_active_at_ms: value.last_active_at_ms,
        }
    }
}

impl From<LegacyConversationV2> for Conversation {
    fn from(value: LegacyConversationV2) -> Self {
        Self {
            conversation_id: value.conversation_id,
            kind: value.kind,
            scope: value.scope,
            scene: None,
            content_topic: value.content_topic,
            participants: value.participants,
            created_at_ms: value.created_at_ms,
            last_active_at_ms: value.last_active_at_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyTimelineEntryV1 {
    envelope: MessageEnvelope,
    delivery_state: DeliveryState,
    archived_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyTimelineEntryV2 {
    envelope: MessageEnvelope,
    delivery_state: DeliveryState,
    archived_at_ms: Option<i64>,
    pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyTimelineEntryV3 {
    envelope: MessageEnvelope,
    delivery_state: DeliveryState,
    archived_at_ms: Option<i64>,
    pinned: bool,
    recalled_at_ms: Option<i64>,
    recalled_by: Option<IdentityId>,
}

impl From<LegacyTimelineEntryV1> for TimelineEntry {
    fn from(value: LegacyTimelineEntryV1) -> Self {
        Self {
            envelope: value.envelope,
            delivery_state: value.delivery_state,
            archived_at_ms: value.archived_at_ms,
            pinned: false,
            recalled_at_ms: None,
            recalled_by: None,
            edited_at_ms: None,
            edited_by: None,
        }
    }
}

impl From<LegacyTimelineEntryV2> for TimelineEntry {
    fn from(value: LegacyTimelineEntryV2) -> Self {
        Self {
            envelope: value.envelope,
            delivery_state: value.delivery_state,
            archived_at_ms: value.archived_at_ms,
            pinned: value.pinned,
            recalled_at_ms: None,
            recalled_by: None,
            edited_at_ms: None,
            edited_by: None,
        }
    }
}

impl From<LegacyTimelineEntryV3> for TimelineEntry {
    fn from(value: LegacyTimelineEntryV3) -> Self {
        Self {
            envelope: value.envelope,
            delivery_state: value.delivery_state,
            archived_at_ms: value.archived_at_ms,
            pinned: value.pinned,
            recalled_at_ms: value.recalled_at_ms,
            recalled_by: value.recalled_by,
            edited_at_ms: None,
            edited_by: None,
        }
    }
}

pub trait TimelineStore {
    fn upsert_conversation(&mut self, conversation: Conversation) -> StorageResult<()>;
    fn append_message(&mut self, message: MessageEnvelope) -> StorageResult<()>;
    fn recent_messages(&self, conversation_id: &ConversationId, limit: usize)
    -> Vec<TimelineEntry>;
    fn active_conversations(&self) -> Vec<Conversation>;
}

pub trait ArchiveStore {
    fn archive_policy(&self) -> ArchivePolicy;
    fn archive_expired_messages(&mut self, now_ms: i64) -> StorageResult<usize>;
}

#[derive(Debug, Clone)]
pub struct InMemoryTimelineStore {
    archive_policy: ArchivePolicy,
    conversations: HashMap<ConversationId, Conversation>,
    timelines: HashMap<ConversationId, Vec<TimelineEntry>>,
}

impl InMemoryTimelineStore {
    pub fn new(archive_policy: ArchivePolicy) -> Self {
        Self {
            archive_policy,
            conversations: HashMap::new(),
            timelines: HashMap::new(),
        }
    }

    pub fn archived_count(&self, conversation_id: &ConversationId) -> usize {
        self.timelines
            .get(conversation_id)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|entry| entry.archived_at_ms.is_some())
                    .count()
            })
            .unwrap_or(0)
    }
}

impl TimelineStore for InMemoryTimelineStore {
    fn upsert_conversation(&mut self, conversation: Conversation) -> StorageResult<()> {
        self.conversations
            .insert(conversation.conversation_id.clone(), conversation);
        Ok(())
    }

    fn append_message(&mut self, message: MessageEnvelope) -> StorageResult<()> {
        if let Some(conversation) = self.conversations.get_mut(&message.conversation_id) {
            conversation.touch(message.timestamp_ms);
        }
        self.timelines
            .entry(message.conversation_id.clone())
            .or_default()
            .push(TimelineEntry {
                envelope: message,
                delivery_state: DeliveryState::LocalOnly,
                archived_at_ms: None,
                pinned: false,
                recalled_at_ms: None,
                recalled_by: None,
                edited_at_ms: None,
                edited_by: None,
            });
        Ok(())
    }

    fn recent_messages(
        &self,
        conversation_id: &ConversationId,
        limit: usize,
    ) -> Vec<TimelineEntry> {
        self.timelines
            .get(conversation_id)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|entry| entry.archived_at_ms.is_none())
                    .rev()
                    .take(limit)
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn active_conversations(&self) -> Vec<Conversation> {
        let mut items = self.conversations.values().cloned().collect::<Vec<_>>();
        items.sort_by_key(|conversation| conversation.last_active_at_ms);
        items.reverse();
        items
    }
}

impl ArchiveStore for InMemoryTimelineStore {
    fn archive_policy(&self) -> ArchivePolicy {
        self.archive_policy.clone()
    }

    fn archive_expired_messages(&mut self, now_ms: i64) -> StorageResult<usize> {
        let policy = self.archive_policy.clone();
        let mut count = 0;
        for entries in self.timelines.values_mut() {
            for entry in entries.iter_mut() {
                if entry.is_active_at(now_ms, &policy) {
                    continue;
                }
                if entry.archived_at_ms.is_none() {
                    entry.archived_at_ms = Some(now_ms);
                    entry.delivery_state = DeliveryState::ArchivedLocal;
                    count += 1;
                }
            }
        }
        Ok(count)
    }
}

#[derive(Debug, Clone)]
pub struct FileTimelineStore {
    root_dir: PathBuf,
    inner: InMemoryTimelineStore,
    /// Frames accumulated in the per-conversation journal since its snapshot
    /// was last rewritten. Authoritative for compaction timing.
    journal_frame_counts: HashMap<ConversationId, usize>,
}

impl FileTimelineStore {
    pub fn open(
        root_dir: impl Into<PathBuf>,
        archive_policy: ArchivePolicy,
    ) -> StorageResult<Self> {
        let root_dir = root_dir.into();
        fs::create_dir_all(root_dir.join("timelines"))
            .map_err(|error| format!("create storage directories failed: {error}"))?;

        let mut store = Self {
            root_dir,
            inner: InMemoryTimelineStore::new(archive_policy),
            journal_frame_counts: HashMap::new(),
        };
        store.load_from_disk()?;
        Ok(store)
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn archived_count(&self, conversation_id: &ConversationId) -> usize {
        self.inner.archived_count(conversation_id)
    }

    pub fn export_messages(&self, conversation_id: &ConversationId) -> Vec<TimelineEntry> {
        self.inner
            .timelines
            .get(conversation_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn merge_message(
        &mut self,
        message: MessageEnvelope,
        delivery_state: DeliveryState,
    ) -> StorageResult<bool> {
        if let Some(conversation) = self.inner.conversations.get_mut(&message.conversation_id) {
            conversation.touch(message.timestamp_ms);
        }

        let entries = self
            .inner
            .timelines
            .entry(message.conversation_id.clone())
            .or_default();

        if let Some(index) = entries
            .iter()
            .position(|entry| entry.envelope.message_id == message.message_id)
        {
            let mut changed = false;
            if entries[index].delivery_state != delivery_state {
                entries[index].delivery_state = delivery_state;
                changed = true;
            }
            if entries[index].envelope != message {
                entries[index].envelope = message;
                changed = true;
            }
            if changed {
                let conversation_id = entries[index].envelope.conversation_id.clone();
                self.persist_timeline(&conversation_id)?;
            }
            return Ok(false);
        }

        entries.push(TimelineEntry {
            envelope: message.clone(),
            delivery_state,
            archived_at_ms: None,
            pinned: false,
            recalled_at_ms: None,
            recalled_by: None,
            edited_at_ms: None,
            edited_by: None,
        });
        self.persist_timeline(&message.conversation_id)?;
        Ok(true)
    }

    pub fn recall_message(
        &mut self,
        conversation_id: &ConversationId,
        message_id: &MessageId,
        actor: IdentityId,
        recalled_at_ms: i64,
    ) -> StorageResult<Option<TimelineEntry>> {
        let Some(entries) = self.inner.timelines.get_mut(conversation_id) else {
            return Ok(None);
        };
        let Some(entry) = entries
            .iter_mut()
            .find(|entry| entry.envelope.message_id == *message_id)
        else {
            return Ok(None);
        };
        if entry.envelope.sender != actor {
            return Err("only the original sender can recall this message".into());
        }
        entry.recalled_at_ms = Some(recalled_at_ms);
        entry.recalled_by = Some(actor);
        let recalled = entry.clone();
        self.persist_timeline(conversation_id)?;
        Ok(Some(recalled))
    }

    pub fn edit_message(
        &mut self,
        conversation_id: &ConversationId,
        message_id: &MessageId,
        actor: IdentityId,
        text: String,
        edited_at_ms: i64,
    ) -> StorageResult<Option<TimelineEntry>> {
        let Some(entries) = self.inner.timelines.get_mut(conversation_id) else {
            return Ok(None);
        };
        let Some(entry) = entries
            .iter_mut()
            .find(|entry| entry.envelope.message_id == *message_id)
        else {
            return Ok(None);
        };
        if entry.envelope.sender != actor {
            return Err("only the original sender can edit this message".into());
        }
        if entry.recalled_at_ms.is_some() {
            return Err("recalled messages cannot be edited".into());
        }
        entry.envelope.body.preview = text.clone();
        entry.envelope.body.plain_text = text;
        entry.edited_at_ms = Some(edited_at_ms);
        entry.edited_by = Some(actor);
        let edited = entry.clone();
        self.persist_timeline(conversation_id)?;
        Ok(Some(edited))
    }

    fn load_from_disk(&mut self) -> StorageResult<()> {
        let conversations = self.load_conversations()?;
        for conversation in conversations {
            let conversation_id = conversation.conversation_id.clone();
            self.inner
                .conversations
                .insert(conversation_id.clone(), conversation);
            let entries = self.load_timeline(&conversation_id)?;
            if !entries.is_empty() {
                self.inner.timelines.insert(conversation_id, entries);
            }
        }
        Ok(())
    }

    fn conversations_path(&self) -> PathBuf {
        self.root_dir.join("conversations.postcard")
    }

    fn timeline_path(&self, conversation_id: &ConversationId) -> PathBuf {
        let key = conversation_id
            .0
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        self.root_dir
            .join("timelines")
            .join(format!("{key}.postcard"))
    }

    fn quarantine_corrupt_snapshot(
        &self,
        path: &Path,
        label: &str,
        decode_error: &str,
    ) -> StorageResult<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(label);
        let quarantine_path = path.with_file_name(format!("{file_name}.corrupt-{timestamp}"));
        fs::rename(path, &quarantine_path).map_err(|error| {
            format!("quarantine {label} failed after decode error ({decode_error}): {error}")
        })?;
        eprintln!(
            "chat-storage: quarantined unreadable {label}: {} -> {} ({decode_error})",
            path.display(),
            quarantine_path.display()
        );
        Ok(())
    }

    fn load_conversations(&self) -> StorageResult<Vec<Conversation>> {
        let path = self.conversations_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&path)
            .map_err(|error| format!("read conversations snapshot failed: {error}"))?;
        if bytes.is_empty() {
            return Ok(Vec::new());
        }
        match postcard::from_bytes(&bytes) {
            Ok(conversations) => Ok(conversations),
            Err(current_error) => {
                if let Ok(legacy) = postcard::from_bytes::<Vec<LegacyConversationV2>>(&bytes) {
                    return Ok(legacy.into_iter().map(Conversation::from).collect());
                }
                if let Ok(legacy) = postcard::from_bytes::<Vec<LegacyConversationV1>>(&bytes) {
                    return Ok(legacy.into_iter().map(Conversation::from).collect());
                }
                let decode_error = current_error.to_string();
                self.quarantine_corrupt_snapshot(&path, "conversations snapshot", &decode_error)?;
                Ok(Vec::new())
            }
        }
    }

    fn load_timeline(
        &mut self,
        conversation_id: &ConversationId,
    ) -> StorageResult<Vec<TimelineEntry>> {
        let path = self.timeline_path(conversation_id);
        // A conversation may legitimately have a journal but no snapshot yet
        // (nothing compacted since creation) — fall through and replay it.
        let snapshot: Vec<TimelineEntry> = if !path.exists() {
            Vec::new()
        } else {
            let bytes = fs::read(&path)
                .map_err(|error| format!("read timeline snapshot failed: {error}"))?;
            if bytes.is_empty() {
                Vec::new()
            } else {
                self.decode_timeline_snapshot(&bytes, &path)?
            }
        };
        self.merge_journal(conversation_id, snapshot)
    }

    /// Decode a snapshot in the current or any legacy codec; unrecoverable
    /// payloads are quarantined and treated as empty (existing contract).
    fn decode_timeline_snapshot(
        &self,
        bytes: &[u8],
        path: &Path,
    ) -> StorageResult<Vec<TimelineEntry>> {
        type LegacyDecoder = fn(&[u8]) -> Result<Vec<TimelineEntry>, postcard::Error>;
        if let Ok(entries) = postcard::from_bytes(bytes) {
            return Ok(entries);
        }
        let legacy_candidates: [LegacyDecoder; 3] = [
            |bytes| {
                postcard::from_bytes::<Vec<LegacyTimelineEntryV3>>(bytes)
                    .map(|items| items.into_iter().map(TimelineEntry::from).collect())
            },
            |bytes| {
                postcard::from_bytes::<Vec<LegacyTimelineEntryV2>>(bytes)
                    .map(|items| items.into_iter().map(TimelineEntry::from).collect())
            },
            |bytes| {
                postcard::from_bytes::<Vec<LegacyTimelineEntryV1>>(bytes)
                    .map(|items| items.into_iter().map(TimelineEntry::from).collect())
            },
        ];
        for candidate in legacy_candidates {
            if let Ok(entries) = candidate(bytes) {
                return Ok(entries);
            }
        }
        let decode_error = "unknown timeline snapshot codec".to_string();
        self.quarantine_corrupt_snapshot(path, "timeline snapshot", &decode_error)?;
        Ok(Vec::new())
    }

    /// Fold the journal into snapshot entries (deduped by message id), record
    /// the surviving frame count, and repair any torn journal tail on disk.
    fn merge_journal(
        &mut self,
        conversation_id: &ConversationId,
        snapshot: Vec<TimelineEntry>,
    ) -> StorageResult<Vec<TimelineEntry>> {
        let journal = timeline_journal::journal_path(&self.root_dir, conversation_id);
        let entries = timeline_journal::load_with_journal(&journal, snapshot)?;
        let frames = timeline_journal::journal_frame_count(&journal);
        self.journal_frame_counts
            .insert(conversation_id.clone(), frames);
        Ok(entries)
    }

    fn persist_conversations(&self) -> StorageResult<()> {
        let conversations = self.inner.active_conversations();
        let bytes = postcard::to_allocvec(&conversations)
            .map_err(|error| format!("encode conversations snapshot failed: {error}"))?;
        atomic_write(&self.conversations_path(), &bytes)
            .map_err(|error| format!("write conversations snapshot failed: {error}"))?;
        Ok(())
    }

    /// Rewrite the snapshot as the single authority and drop the journal.
    /// Used by mutating paths (edit/recall/archive) and journal compaction.
    fn persist_timeline(&mut self, conversation_id: &ConversationId) -> StorageResult<()> {
        let entries = self
            .inner
            .timelines
            .get(conversation_id)
            .cloned()
            .unwrap_or_default();
        let bytes = postcard::to_allocvec(&entries)
            .map_err(|error| format!("encode timeline snapshot failed: {error}"))?;
        atomic_write(&self.timeline_path(conversation_id), &bytes)
            .map_err(|error| format!("write timeline snapshot failed: {error}"))?;
        timeline_journal::journal_remove(&timeline_journal::journal_path(
            &self.root_dir,
            conversation_id,
        ))?;
        self.journal_frame_counts.remove(conversation_id);
        Ok(())
    }

    fn persist_all_timelines(&mut self) -> StorageResult<()> {
        let conversation_ids: Vec<ConversationId> =
            self.inner.conversations.keys().cloned().collect();
        for conversation_id in conversation_ids {
            self.persist_timeline(&conversation_id)?;
        }
        Ok(())
    }
}

impl TimelineStore for FileTimelineStore {
    fn upsert_conversation(&mut self, conversation: Conversation) -> StorageResult<()> {
        self.inner.upsert_conversation(conversation)?;
        self.persist_conversations()
    }

    fn append_message(&mut self, message: MessageEnvelope) -> StorageResult<()> {
        let conversation_id = message.conversation_id.clone();
        self.inner.append_message(message)?;
        // Durability: journal the appended entry (O(frame) IO) instead of
        // rewriting the whole timeline snapshot. Atomicity: if either persist
        // step fails, roll the journal back to its pre-append length and remove
        // the in-memory message to keep RAM and disk consistent (avoid silent
        // data divergence).
        let journal_path = timeline_journal::journal_path(&self.root_dir, &conversation_id);
        let journal_pre_len = std::fs::metadata(&journal_path)
            .map(|meta| meta.len())
            .unwrap_or(0);
        let frames_before = *self
            .journal_frame_counts
            .entry(conversation_id.clone())
            .or_insert(timeline_journal::journal_frame_count(&journal_path));
        let appended = self
            .inner
            .timelines
            .get(&conversation_id)
            .and_then(|entries| entries.last())
            .cloned();
        let journal_result = match &appended {
            Some(entry) => timeline_journal::journal_append(&journal_path, entry),
            None => Ok(()),
        };
        if let Err(e) = journal_result.and_then(|_| self.persist_conversations()) {
            // Undo the in-memory append to keep state consistent with disk
            let timeline = self.inner.timelines.get_mut(&conversation_id);
            if let Some(entries) = timeline {
                entries.pop();
            }
            // Roll the journal back to its intact pre-append prefix so the
            // frames of earlier messages stay durable.
            if let Ok(file) = std::fs::OpenOptions::new().write(true).open(&journal_path) {
                let _ = file.set_len(journal_pre_len);
                let _ = file.sync_all();
            }
            self.journal_frame_counts
                .insert(conversation_id, frames_before);
            return Err(format!("append message persist failed, rolled back: {e}"));
        }
        // Compaction: fold the journal into the snapshot once it grows past
        // the threshold so replay cost stays bounded.
        let frames = self
            .journal_frame_counts
            .entry(conversation_id.clone())
            .or_insert(frames_before);
        *frames += 1;
        if *frames >= timeline_journal::JOURNAL_COMPACT_THRESHOLD {
            self.persist_timeline(&conversation_id)?;
        }
        Ok(())
    }

    fn recent_messages(
        &self,
        conversation_id: &ConversationId,
        limit: usize,
    ) -> Vec<TimelineEntry> {
        self.inner.recent_messages(conversation_id, limit)
    }

    fn active_conversations(&self) -> Vec<Conversation> {
        self.inner.active_conversations()
    }
}

impl ArchiveStore for FileTimelineStore {
    fn archive_policy(&self) -> ArchivePolicy {
        self.inner.archive_policy()
    }

    fn archive_expired_messages(&mut self, now_ms: i64) -> StorageResult<usize> {
        let archived = self.inner.archive_expired_messages(now_ms)?;
        self.persist_conversations()?;
        self.persist_all_timelines()?;
        Ok(archived)
    }
}

#[cfg(test)]
mod tests {
    use chat_core::{
        AgentSceneSlot, AgentScope, AgentUseCase, ClientProfile, ConversationKind,
        ConversationScope, DeviceId, IdentityId, MessageBody, MessageId, PayloadType,
        SceneImageLayer, SceneLandmark, SceneMetadata, SceneRenderStyle, SceneScope,
    };
    use tempfile::tempdir;

    use super::*;

    fn sample_conversation() -> Conversation {
        Conversation {
            conversation_id: ConversationId("dm:alice:bob".into()),
            kind: ConversationKind::Direct,
            scope: ConversationScope::Private,
            scene: None,
            content_topic: "/lobster-chat/dm/alice-bob/1".into(),
            participants: vec![IdentityId("alice".into()), IdentityId("bob".into())],
            created_at_ms: 1_000,
            last_active_at_ms: 1_000,
        }
    }

    fn sample_scene() -> SceneMetadata {
        SceneMetadata {
            scope: SceneScope::DirectRoom,
            render_style: SceneRenderStyle::SfcPixel,
            title_banner: Some("Alice and Bob".into()),
            background_preset: "residence-night".into(),
            ambiance: "quiet".into(),
            owner_editable: true,
            avatar_editable: true,
            primary_avatar: None,
            assistant_slots: vec![AgentSceneSlot {
                slot_id: "caretaker".into(),
                display_name: "Caretaker".into(),
                scope: AgentScope::Room,
                use_cases: vec![AgentUseCase::Caretaking],
                appearance_hint: "warm light".into(),
                can_leave_messages: true,
                can_edit_scene: false,
                can_trade_goods: false,
            }],
            image_layer: Some(SceneImageLayer {
                layer_id: "image-layer".into(),
                preset: "private-room-loft".into(),
                asset_hint: "private-room-loft".into(),
                aspect_ratio_permyriad: 5_625,
                owner_editable: true,
                day_image_url: None,
                night_image_url: None,
            }),
            hotspot_layer: None,
            landmarks: vec![SceneLandmark {
                slot_id: "desk".into(),
                label: "Desk".into(),
                sprite_hint: "oak desk".into(),
                interaction_hint: "Open notes".into(),
            }],
        }
    }

    fn sample_message(timestamp_ms: i64) -> MessageEnvelope {
        MessageEnvelope {
            message_id: MessageId(format!("m-{timestamp_ms}")),
            conversation_id: ConversationId("dm:alice:bob".into()),
            sender: IdentityId("alice".into()),
            reply_to_message_id: None,
            sender_device: DeviceId("alice-desktop".into()),
            sender_profile: ClientProfile::desktop_terminal(),
            payload_type: PayloadType::Text,
            body: MessageBody {
                preview: "hello".into(),
                plain_text: "hello".into(),
                language_tag: "en".into(),
            },
            ciphertext: vec![1, 2, 3],
            timestamp_ms,
            ephemeral: false,
        }
    }

    fn archive_policy() -> ArchivePolicy {
        ArchivePolicy {
            active_window_hours: 1,
            local_retention_days: Some(7),
            allow_user_pinned_archive: true,
            archive_when_idle_hours: 1,
        }
    }

    #[test]
    fn archives_messages_outside_window() {
        let mut store = InMemoryTimelineStore::new(archive_policy());
        store.upsert_conversation(sample_conversation()).unwrap();
        store.append_message(sample_message(0)).unwrap();
        let archived = store.archive_expired_messages(4_000_000).unwrap();
        assert_eq!(archived, 1);
        assert_eq!(
            store.archived_count(&ConversationId("dm:alice:bob".into())),
            1
        );
    }

    #[test]
    fn file_store_restores_conversations_and_messages() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("storage");

        {
            let mut store = FileTimelineStore::open(&root, archive_policy()).expect("open store");
            store.upsert_conversation(sample_conversation()).unwrap();
            store.append_message(sample_message(10)).unwrap();
            store.append_message(sample_message(20)).unwrap();
            assert_eq!(
                store
                    .recent_messages(&ConversationId("dm:alice:bob".into()), 10)
                    .len(),
                2
            );
        }

        let restored = FileTimelineStore::open(&root, archive_policy()).expect("restore store");
        assert_eq!(restored.active_conversations().len(), 1);
        assert_eq!(
            restored
                .recent_messages(&ConversationId("dm:alice:bob".into()), 10)
                .len(),
            2
        );
    }

    #[test]
    fn file_store_restores_conversations_with_scene_metadata() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("storage");
        let mut conversation = sample_conversation();
        conversation.scene = Some(sample_scene());

        {
            let mut store = FileTimelineStore::open(&root, archive_policy()).expect("open store");
            store.upsert_conversation(conversation.clone()).unwrap();
        }

        let restored = FileTimelineStore::open(&root, archive_policy()).expect("restore store");
        let restored_conversation = restored
            .active_conversations()
            .into_iter()
            .find(|item| item.conversation_id == conversation.conversation_id)
            .expect("restored conversation");
        assert_eq!(restored_conversation.scene, conversation.scene);
    }

    #[test]
    fn file_store_merge_message_deduplicates_existing_message_id_and_upgrades_delivery_state() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("storage");
        let conversation_id = ConversationId("dm:alice:bob".into());
        let message = sample_message(10);

        let mut store = FileTimelineStore::open(&root, archive_policy()).expect("open store");
        store.upsert_conversation(sample_conversation()).unwrap();
        store.append_message(message.clone()).unwrap();

        let merged = store
            .merge_message(message.clone(), DeliveryState::Delivered)
            .expect("merge message");
        assert!(!merged, "existing message id should not append a duplicate");

        let entries = store.recent_messages(&conversation_id, 10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].envelope.message_id, message.message_id);
        assert_eq!(entries[0].delivery_state, DeliveryState::Delivered);
    }

    #[test]
    fn file_store_merge_message_appends_remote_message_as_delivered() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("storage");
        let conversation_id = ConversationId("dm:alice:bob".into());
        let message = sample_message(42);

        let mut store = FileTimelineStore::open(&root, archive_policy()).expect("open store");
        store.upsert_conversation(sample_conversation()).unwrap();

        let merged = store
            .merge_message(message.clone(), DeliveryState::Delivered)
            .expect("merge remote message");
        assert!(merged, "new remote message should be appended");

        let entries = store.recent_messages(&conversation_id, 10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].envelope.message_id, message.message_id);
        assert_eq!(entries[0].delivery_state, DeliveryState::Delivered);
    }

    #[test]
    fn file_store_persists_archive_state() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("storage");
        let conversation_id = ConversationId("dm:alice:bob".into());

        {
            let mut store = FileTimelineStore::open(&root, archive_policy()).expect("open store");
            store.upsert_conversation(sample_conversation()).unwrap();
            store.append_message(sample_message(0)).unwrap();
            let archived = store.archive_expired_messages(4_000_000).unwrap();
            assert_eq!(archived, 1);
            assert_eq!(store.archived_count(&conversation_id), 1);
        }

        let restored = FileTimelineStore::open(&root, archive_policy()).expect("restore store");
        assert_eq!(restored.archived_count(&conversation_id), 1);
    }

    #[test]
    fn export_messages_keeps_archived_entries() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("storage");
        let conversation_id = ConversationId("dm:alice:bob".into());

        let exported = {
            let mut store = FileTimelineStore::open(&root, archive_policy()).expect("open store");
            store.upsert_conversation(sample_conversation()).unwrap();
            store.append_message(sample_message(0)).unwrap();
            store.archive_expired_messages(4_000_000).unwrap();
            store.export_messages(&conversation_id)
        };

        assert_eq!(exported.len(), 1);
        assert!(exported[0].archived_at_ms.is_some());
    }

    #[test]
    fn file_store_restores_scope_only_legacy_conversations() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("storage");
        fs::create_dir_all(root.join("timelines")).expect("create timelines");
        let legacy = vec![LegacyConversationV2 {
            conversation_id: ConversationId("room:city:core-harbor:lobby".into()),
            kind: ConversationKind::Room,
            scope: ConversationScope::CityPublic,
            content_topic: "/lobster-chat/1/conversation/room:city:core-harbor:lobby".into(),
            participants: vec![IdentityId("rsaga".into()), IdentityId("builder".into())],
            created_at_ms: 1_000,
            last_active_at_ms: 2_000,
        }];
        let bytes = postcard::to_allocvec(&legacy).expect("encode legacy conversations");
        fs::write(root.join("conversations.postcard"), bytes).expect("write legacy conversations");

        let restored = FileTimelineStore::open(&root, archive_policy()).expect("restore store");
        let conversations = restored.active_conversations();
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].scope, ConversationScope::CityPublic);
        assert!(conversations[0].scene.is_none());
    }

    #[test]
    fn file_store_restores_legacy_timelines_without_pinned() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("storage");
        let conversation = sample_conversation();
        let conversation_id = conversation.conversation_id.clone();
        fs::create_dir_all(root.join("timelines")).expect("create timelines");
        let conversations =
            postcard::to_allocvec(&vec![conversation.clone()]).expect("encode conversations");
        fs::write(root.join("conversations.postcard"), conversations).expect("write conversations");
        let legacy_entries = vec![LegacyTimelineEntryV1 {
            envelope: sample_message(42),
            delivery_state: DeliveryState::Delivered,
            archived_at_ms: None,
        }];
        let bytes = postcard::to_allocvec(&legacy_entries).expect("encode legacy timeline");
        let timeline_path = root
            .join("timelines")
            .join("646d3a616c6963653a626f62.postcard");
        fs::write(timeline_path, bytes).expect("write legacy timeline");

        let restored = FileTimelineStore::open(&root, archive_policy()).expect("restore store");
        let entries = restored.recent_messages(&conversation_id, 10);
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].pinned);
        assert_eq!(entries[0].delivery_state, DeliveryState::Delivered);
    }

    #[test]
    fn file_store_restores_legacy_timelines_with_pinned_before_recall_fields() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("storage");
        let conversation = sample_conversation();
        let conversation_id = conversation.conversation_id.clone();
        fs::create_dir_all(root.join("timelines")).expect("create timelines");
        let conversations =
            postcard::to_allocvec(&vec![conversation.clone()]).expect("encode conversations");
        fs::write(root.join("conversations.postcard"), conversations).expect("write conversations");
        let legacy_entries = vec![LegacyTimelineEntryV2 {
            envelope: sample_message(42),
            delivery_state: DeliveryState::Delivered,
            archived_at_ms: None,
            pinned: true,
        }];
        let bytes = postcard::to_allocvec(&legacy_entries).expect("encode legacy timeline");
        let timeline_path = root
            .join("timelines")
            .join("646d3a616c6963653a626f62.postcard");
        fs::write(timeline_path, bytes).expect("write legacy timeline");

        let restored = FileTimelineStore::open(&root, archive_policy()).expect("restore store");
        let entries = restored.recent_messages(&conversation_id, 10);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].pinned);
        assert_eq!(entries[0].recalled_at_ms, None);
        assert_eq!(entries[0].recalled_by, None);
    }

    #[test]
    fn file_store_restores_legacy_timelines_with_recall_before_edit_fields() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("storage");
        let conversation = sample_conversation();
        let conversation_id = conversation.conversation_id.clone();
        fs::create_dir_all(root.join("timelines")).expect("create timelines");
        let conversations =
            postcard::to_allocvec(&vec![conversation.clone()]).expect("encode conversations");
        fs::write(root.join("conversations.postcard"), conversations).expect("write conversations");
        let legacy_entries = vec![LegacyTimelineEntryV3 {
            envelope: sample_message(42),
            delivery_state: DeliveryState::Delivered,
            archived_at_ms: None,
            pinned: true,
            recalled_at_ms: Some(99),
            recalled_by: Some(IdentityId("alice".into())),
        }];
        let bytes = postcard::to_allocvec(&legacy_entries).expect("encode legacy timeline");
        let timeline_path = root
            .join("timelines")
            .join("646d3a616c6963653a626f62.postcard");
        fs::write(timeline_path, bytes).expect("write legacy timeline");

        let restored = FileTimelineStore::open(&root, archive_policy()).expect("restore store");
        let entries = restored.recent_messages(&conversation_id, 10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].recalled_at_ms, Some(99));
        assert_eq!(entries[0].recalled_by, Some(IdentityId("alice".into())));
        assert_eq!(entries[0].edited_at_ms, None);
        assert_eq!(entries[0].edited_by, None);
    }

    #[test]
    fn file_store_quarantines_unreadable_conversations_snapshot() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("storage");
        fs::create_dir_all(root.join("timelines")).expect("create timelines");
        fs::write(root.join("conversations.postcard"), b"not postcard bytes")
            .expect("write bad conversations");

        let restored = FileTimelineStore::open(&root, archive_policy()).expect("restore store");
        assert!(restored.active_conversations().is_empty());
        assert!(!root.join("conversations.postcard").exists());
        let quarantined = fs::read_dir(&root)
            .expect("read root")
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("conversations.postcard.corrupt-")
            });
        assert!(quarantined);
    }

    #[test]
    fn atomic_write_replaces_existing_file_without_leaving_tmp_artifact() {
        let temp = tempdir().expect("temp dir");
        let path = temp.path().join("snapshot.postcard");

        fs::write(&path, b"old").expect("seed snapshot");
        atomic_write(&path, b"new").expect("atomic write");

        assert_eq!(fs::read(&path).expect("read snapshot"), b"new");
        assert!(
            !path.with_extension("postcard.tmp").exists(),
            "temp artifact should be removed after atomic replace"
        );
    }

    #[test]
    fn atomic_write_supports_concurrent_replacements() {
        let temp = tempdir().expect("temp dir");
        let path = std::sync::Arc::new(temp.path().join("snapshot.postcard"));
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let workers = (0..8)
            .map(|worker| {
                let path = std::sync::Arc::clone(&path);
                let barrier = std::sync::Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    for round in 0..32 {
                        let payload = format!("worker-{worker}-round-{round}");
                        atomic_write(&path, payload.as_bytes()).expect("concurrent atomic write");
                    }
                })
            })
            .collect::<Vec<_>>();

        for worker in workers {
            worker.join().expect("concurrent writer should finish");
        }

        let payload = fs::read_to_string(&*path).expect("read concurrent snapshot");
        assert!(payload.starts_with("worker-") && payload.contains("-round-"));
        assert!(!path.with_extension("postcard.tmp").exists());
    }

    #[test]
    fn empty_timeline_returns_no_messages() {
        let dir = tempdir().unwrap();
        let store =
            FileTimelineStore::open(dir.path().join("store"), ArchivePolicy::default()).unwrap();
        let msgs = store.recent_messages(&ConversationId("dm:a:b".into()), 10);
        assert!(msgs.is_empty());
    }

    #[test]
    fn open_empty_store_yields_no_conversations() {
        let dir = tempdir().unwrap();
        let store =
            FileTimelineStore::open(dir.path().join("store"), ArchivePolicy::default()).unwrap();
        assert!(store.active_conversations().is_empty());
    }

    #[test]
    fn append_and_read_single_message() {
        let dir = tempdir().unwrap();
        let mut store =
            FileTimelineStore::open(dir.path().join("store"), ArchivePolicy::default()).unwrap();
        let msg = sample_message(1000);
        store.append_message(msg).unwrap();
        let loaded = store.recent_messages(&ConversationId("dm:alice:bob".into()), 10);
        assert_eq!(loaded.len(), 1);
    }
    #[test]
    fn consecutive_opens_within_same_tempdir() {
        // Verify store can be opened twice at different paths without conflict
        let dir = tempdir().unwrap();
        let s1 = FileTimelineStore::open(dir.path().join("s1"), ArchivePolicy::default()).unwrap();
        let s2 = FileTimelineStore::open(dir.path().join("s2"), ArchivePolicy::default()).unwrap();
        assert!(s1.active_conversations().is_empty());
        assert!(s2.active_conversations().is_empty());
    }

    // ---- R2 append-only journal ----

    fn journal_path_for(root: &Path, id: &ConversationId) -> PathBuf {
        timeline_journal::journal_path(root, id)
    }

    #[test]
    fn journal_only_timeline_survives_reopen_without_snapshot_rewrite() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("store");
        let conversation_id = ConversationId("dm:alice:bob".into());

        let (snapshot_before, journal_before) = {
            let mut store = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
            store.upsert_conversation(sample_conversation()).unwrap();
            store.append_message(sample_message(10)).unwrap();
            store.append_message(sample_message(20)).unwrap();
            let snapshot_bytes =
                fs::read(store.timeline_path(&conversation_id)).unwrap_or_default();
            let journal_bytes = fs::read(journal_path_for(&root, &conversation_id)).unwrap();
            (snapshot_bytes, journal_bytes)
        };
        // 关键合同：追加不再重写快照（快照保持不存在），数据在 journal 里
        assert!(
            snapshot_before.is_empty(),
            "snapshot must not be rewritten per append"
        );
        assert!(!journal_before.is_empty());

        let restored = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
        let entries = restored.recent_messages(&conversation_id, 10);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].envelope.timestamp_ms, 10);
        assert_eq!(entries[1].envelope.timestamp_ms, 20);
    }

    #[test]
    fn journal_compacts_into_snapshot_at_threshold() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("store");
        let conversation_id = ConversationId("dm:alice:bob".into());
        let mut store = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
        store.upsert_conversation(sample_conversation()).unwrap();
        for index in 0..timeline_journal::JOURNAL_COMPACT_THRESHOLD as i64 {
            store.append_message(sample_message(1_000 + index)).unwrap();
        }

        let journal = journal_path_for(&root, &conversation_id);
        assert!(
            !journal.exists(),
            "journal must fold into snapshot at threshold"
        );
        assert!(store.timeline_path(&conversation_id).exists());
        assert_eq!(
            store.recent_messages(&conversation_id, 1_000).len(),
            timeline_journal::JOURNAL_COMPACT_THRESHOLD
        );

        drop(store);
        let restored = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
        assert_eq!(
            restored.recent_messages(&conversation_id, 1_000).len(),
            timeline_journal::JOURNAL_COMPACT_THRESHOLD
        );
    }

    #[test]
    fn torn_journal_tail_is_repaired_and_good_frames_survive() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("store");
        let conversation_id = ConversationId("dm:alice:bob".into());
        {
            let mut store = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
            store.upsert_conversation(sample_conversation()).unwrap();
            for timestamp in [10, 20, 30] {
                store.append_message(sample_message(timestamp)).unwrap();
            }
        }
        let journal = journal_path_for(&root, &conversation_id);
        let intact = fs::read(&journal).unwrap();
        // 模拟追加中途断电：尾部多出半帧
        let mut torn = intact.clone();
        torn.extend_from_slice(&[0x2a, 0x00, 0x00]);
        fs::write(&journal, &torn).unwrap();

        let restored = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
        assert_eq!(restored.recent_messages(&conversation_id, 10).len(), 3);
        assert_eq!(
            fs::read(&journal).unwrap(),
            intact,
            "repair must truncate back to the intact prefix"
        );
    }

    #[test]
    fn corrupted_frame_stops_replay_at_last_good_frame() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("store");
        let conversation_id = ConversationId("dm:alice:bob".into());
        {
            let mut store = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
            store.upsert_conversation(sample_conversation()).unwrap();
            for timestamp in [10, 20, 30] {
                store.append_message(sample_message(timestamp)).unwrap();
            }
        }
        let journal = journal_path_for(&root, &conversation_id);
        let mut bytes = fs::read(&journal).unwrap();
        // 翻转最后一帧 payload 的一个字节（CRC 不再匹配）
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        fs::write(&journal, &bytes).unwrap();

        let restored = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
        let entries = restored.recent_messages(&conversation_id, 10);
        assert_eq!(
            entries.len(),
            2,
            "only frames before the corrupt one survive"
        );
        assert_eq!(entries[1].envelope.timestamp_ms, 20);
    }

    #[test]
    fn edit_message_folds_journal_into_snapshot() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("store");
        let conversation_id = ConversationId("dm:alice:bob".into());
        let mut store = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
        store.upsert_conversation(sample_conversation()).unwrap();
        store.append_message(sample_message(10)).unwrap();
        store
            .edit_message(
                &conversation_id,
                &MessageId("m-10".into()),
                IdentityId("alice".into()),
                "编辑后".into(),
                99,
            )
            .unwrap();

        assert!(!journal_path_for(&root, &conversation_id).exists());
        drop(store);
        let restored = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
        let entries = restored.recent_messages(&conversation_id, 10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].envelope.body.plain_text, "编辑后");
        assert_eq!(entries[0].edited_at_ms, Some(99));
    }

    #[test]
    fn recall_message_folds_journal_into_snapshot() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("store");
        let conversation_id = ConversationId("dm:alice:bob".into());
        let mut store = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
        store.upsert_conversation(sample_conversation()).unwrap();
        store.append_message(sample_message(10)).unwrap();
        store.append_message(sample_message(20)).unwrap();
        store
            .recall_message(
                &conversation_id,
                &MessageId("m-20".into()),
                IdentityId("alice".into()),
                999,
            )
            .unwrap();

        assert!(!journal_path_for(&root, &conversation_id).exists());
        drop(store);
        let restored = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
        let entries = restored.recent_messages(&conversation_id, 10);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].recalled_at_ms, Some(999));
    }

    #[test]
    fn duplicated_journal_frames_after_crash_are_deduped() {
        // 崩溃窗口：新快照已写入、journal 尚未删除 —— 重放不得产生重复消息
        let dir = tempdir().unwrap();
        let root = dir.path().join("store");
        let conversation_id = ConversationId("dm:alice:bob".into());
        let mut store = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
        store.upsert_conversation(sample_conversation()).unwrap();
        store.append_message(sample_message(10)).unwrap();
        store.append_message(sample_message(20)).unwrap();
        // 手工把当前内存里的 entries 写成快照，同时保留 journal（模拟该窗口）
        let entries = store.export_messages(&conversation_id);
        let bytes = postcard::to_allocvec(&entries).unwrap();
        atomic_write(&store.timeline_path(&conversation_id), &bytes).unwrap();

        let restored = FileTimelineStore::open(&root, ArchivePolicy::default()).unwrap();
        let messages = restored.recent_messages(&conversation_id, 10);
        assert_eq!(
            messages.len(),
            2,
            "snapshot+journal duplicates must collapse"
        );
        assert_eq!(messages[0].envelope.message_id.0, "m-10");
        assert_eq!(messages[1].envelope.message_id.0, "m-20");
    }

    #[test]
    fn archive_expired_messages_truncates_journals() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("store");
        let conversation_id = ConversationId("dm:alice:bob".into());
        let mut store = FileTimelineStore::open(&root, archive_policy()).unwrap();
        store.upsert_conversation(sample_conversation()).unwrap();
        store.append_message(sample_message(0)).unwrap();
        assert!(journal_path_for(&root, &conversation_id).exists());

        store.archive_expired_messages(4_000_000).unwrap();
        assert!(!journal_path_for(&root, &conversation_id).exists());
        drop(store);
        let restored = FileTimelineStore::open(&root, archive_policy()).unwrap();
        assert_eq!(restored.archived_count(&conversation_id), 1);
    }
}
