//! Append-only timeline journal (R2 增量写).
//!
//! Every `append_message` used to rewrite the whole `<id>.postcard` snapshot,
//! which is O(timeline) IO per message — the first performance cliff as message
//! volume grows. The journal turns appends into O(frame) IO: each new entry is
//! a CRC-guarded frame appended to `<id>.journal`, replayed on load after the
//! snapshot. When the journal grows past [`JOURNAL_COMPACT_THRESHOLD`] frames,
//! the next append folds it back into the snapshot (amortised O(1)).
//!
//! Frame format: `[len: u32 LE][crc32: u32 LE][payload]`, payload = postcard
//! encoded `TimelineEntry`. The length prefix + CRC let replay stop at (and
//! truncate away) a torn tail from a crashed append without touching good
//! frames.
//!
//! Mutating paths that rewrite the snapshot (edit/recall/archive/compaction)
//! remove the journal afterwards: the snapshot becomes the single authority and
//! replay starts from an empty journal. Replays dedupe by `message_id`, so a
//! crash between "snapshot written" and "journal removed" cannot duplicate
//! entries.

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chat_core::{ConversationId, MessageId, TimelineEntry};

use crate::StorageResult;

/// Fold the journal back into the snapshot once it holds this many frames.
pub(crate) const JOURNAL_COMPACT_THRESHOLD: usize = 128;

const FRAME_HEADER_BYTES: usize = 8; // len: u32 LE + crc32: u32 LE

pub(crate) fn journal_path(root_dir: &Path, conversation_id: &ConversationId) -> PathBuf {
    let key: String = conversation_id
        .0
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    root_dir.join("timelines").join(format!("{key}.journal"))
}

/// CRC-32 (IEEE 802.3, same polynomial as zlib), table-free.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn fsync_parent_dir(path: &Path) -> StorageResult<()> {
    if let Some(parent) = path.parent()
        && let Ok(dir) = File::open(parent)
    {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// Append one entry as a durable frame. Creates the journal if missing.
///
/// Frame = `[len: u32 LE][crc32: u32 LE][payload]`, payload = postcard encoded
/// `TimelineEntry`. The envelope is hand-packed (not postcard) so the header
/// length is fixed and torn tails are trivially detectable.
pub(crate) fn journal_append(path: &Path, entry: &TimelineEntry) -> StorageResult<()> {
    let payload = postcard::to_allocvec(entry)
        .map_err(|error| format!("encode journal frame failed: {error}"))?;
    let mut bytes = Vec::with_capacity(FRAME_HEADER_BYTES + payload.len());
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&crc32(&payload).to_le_bytes());
    bytes.extend_from_slice(&payload);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("open journal failed: {error}"))?;
    let created = file.metadata().map(|meta| meta.len() == 0).unwrap_or(false);
    file.write_all(&bytes)
        .map_err(|error| format!("append journal frame failed: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("sync journal failed: {error}"))?;
    if created {
        fsync_parent_dir(path)?;
    }
    Ok(())
}

pub(crate) fn journal_remove(path: &Path) -> StorageResult<()> {
    if path.exists() {
        fs::remove_file(path).map_err(|error| format!("remove journal failed: {error}"))?;
    }
    Ok(())
}

/// Replay result: entries recovered plus the byte length of the intact prefix
/// (callers truncate the file to this when it has a torn tail).
pub(crate) struct JournalReplay {
    pub entries: Vec<TimelineEntry>,
    pub intact_bytes: usize,
    pub torn_tail: bool,
}

/// Decode frames from `bytes`, stopping at the first short/corrupt frame.
fn decode_frames(bytes: &[u8]) -> JournalReplay {
    fn read_le_u32(bytes: &[u8], start: usize) -> u32 {
        let mut buffer = [0u8; 4];
        buffer.copy_from_slice(&bytes[start..start + 4]);
        u32::from_le_bytes(buffer)
    }
    let mut entries = Vec::new();
    let mut offset = 0usize;
    while offset + FRAME_HEADER_BYTES <= bytes.len() {
        let len = read_le_u32(bytes, offset) as usize;
        let crc = read_le_u32(bytes, offset + 4);
        let payload_end = match offset
            .checked_add(FRAME_HEADER_BYTES)
            .and_then(|start| start.checked_add(len))
        {
            Some(end) if end <= bytes.len() => end,
            _ => break, // torn/oversized tail
        };
        let payload = &bytes[offset + FRAME_HEADER_BYTES..payload_end];
        if crc32(payload) != crc {
            break; // corrupted frame, treat as torn tail
        }
        match postcard::from_bytes::<TimelineEntry>(payload) {
            Ok(entry) => entries.push(entry),
            Err(_) => break,
        }
        offset = payload_end;
    }
    let torn_tail = offset != bytes.len();
    JournalReplay {
        entries,
        intact_bytes: offset,
        torn_tail,
    }
}

/// Load snapshot entries + journal frames, deduped by `message_id` (snapshot
/// wins; a crash between snapshot write and journal removal cannot duplicate).
/// A journal with a torn tail is truncated in place to its intact prefix.
pub(crate) fn load_with_journal(
    journal_path: &Path,
    snapshot_entries: Vec<TimelineEntry>,
) -> StorageResult<Vec<TimelineEntry>> {
    let mut entries = snapshot_entries;
    if !journal_path.exists() {
        return Ok(entries);
    }
    let mut bytes = Vec::new();
    File::open(journal_path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|error| format!("read journal failed: {error}"))?;
    if bytes.is_empty() {
        return Ok(entries);
    }
    let replay = decode_frames(&bytes);
    if !replay.entries.is_empty() {
        let seen: std::collections::HashSet<MessageId> = entries
            .iter()
            .map(|entry| entry.envelope.message_id.clone())
            .collect();
        let mut journaled: std::collections::HashSet<MessageId> = seen;
        for entry in replay.entries {
            if journaled.insert(entry.envelope.message_id.clone()) {
                entries.push(entry);
            }
        }
    }
    if replay.torn_tail {
        // Repair: keep only the intact prefix so future appends stay readable.
        let file = File::options()
            .write(true)
            .open(journal_path)
            .map_err(|error| format!("open journal for tail repair failed: {error}"))?;
        file.set_len(replay.intact_bytes as u64)
            .map_err(|error| format!("truncate torn journal tail failed: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("sync repaired journal failed: {error}"))?;
    }
    Ok(entries)
}

/// Number of frames currently in the journal file (0 when absent).
pub(crate) fn journal_frame_count(journal_path: &Path) -> usize {
    let mut bytes = Vec::new();
    if File::open(journal_path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .is_err()
    {
        return 0;
    }
    fn read_len(bytes: &[u8], start: usize) -> usize {
        let mut buffer = [0u8; 4];
        buffer.copy_from_slice(&bytes[start..start + 4]);
        u32::from_le_bytes(buffer) as usize
    }
    let mut count = 0usize;
    let mut offset = 0usize;
    while offset + FRAME_HEADER_BYTES <= bytes.len() {
        let payload_end = match offset
            .checked_add(FRAME_HEADER_BYTES)
            .and_then(|start| start.checked_add(read_len(&bytes, offset)))
        {
            Some(end) if end <= bytes.len() => end,
            _ => break,
        };
        count += 1;
        offset = payload_end;
    }
    count
}
