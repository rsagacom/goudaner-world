//! R2 append-only journal 压测基准（opt-in，不进 CI）。
//!
//! 运行：cargo test -p chat-storage --release --ignored -- --nocapture
//!
//! 量化两种落盘路径在 5,000 条消息上的单条成本：
//!   - journal 追加（R2 路径，append_message 实际走的）
//!   - 快照全量重写（旧路径单次成本，用 persist_timeline 在 5k 条上度量）
//!
//! 输出仅打印，不做时间断言（避免 CI 抖动），供蓝图记录参考值。

use std::time::Instant;

use chat_core::{
    ArchivePolicy, ClientProfile, Conversation, ConversationId, ConversationKind,
    ConversationScope, DeviceId, IdentityId, MessageBody, MessageEnvelope, MessageId, PayloadType,
};

use crate::{FileTimelineStore, TimelineStore};

#[test]
#[ignore = "benchmark: run explicitly with cargo test -p chat-storage --release --ignored"]
fn bench_journal_append_vs_snapshot_rewrite() {
    let dir = tempfile::tempdir().unwrap();
    let mut store =
        FileTimelineStore::open(dir.path().join("store"), ArchivePolicy::default()).unwrap();
    let conversation_id = ConversationId("dm:bench:bench".into());
    store
        .upsert_conversation(Conversation {
            conversation_id: conversation_id.clone(),
            kind: ConversationKind::Direct,
            scope: ConversationScope::Private,
            scene: None,
            content_topic: "/lobster-chat/dm/bench-bench/1".into(),
            participants: vec![IdentityId("bench".into())],
            created_at_ms: 1_000,
            last_active_at_ms: 1_000,
        })
        .unwrap();

    let total = 5_000;
    let message = |index: i64| MessageEnvelope {
        message_id: MessageId(format!("bench-{index}")),
        conversation_id: conversation_id.clone(),
        sender: IdentityId("bench".into()),
        reply_to_message_id: None,
        sender_device: DeviceId("bench".into()),
        sender_profile: ClientProfile::desktop_terminal(),
        payload_type: PayloadType::Text,
        body: MessageBody {
            preview: "benchmark payload".into(),
            plain_text: "benchmark payload body with realistic length padding ~".repeat(4),
            language_tag: "zh-CN".into(),
        },
        ciphertext: vec![],
        timestamp_ms: 1_000_000 + index,
        ephemeral: false,
    };

    let started = Instant::now();
    for index in 0..total as i64 {
        store.append_message(message(index)).unwrap();
    }
    let journal_elapsed = started.elapsed();

    // 旧路径等价成本：在同一 5k 条 timeline 上做一次全量快照重写
    let rewrite_started = Instant::now();
    store.persist_timeline(&conversation_id).unwrap();
    let rewrite_elapsed = rewrite_started.elapsed();

    let journal_us_per_message = journal_elapsed.as_micros() as f64 / total as f64;
    println!("\n=== R2 journal bench ({total} messages) ===");
    println!("journal append: total {journal_elapsed:?}, ~{journal_us_per_message:.1} µs/message");
    println!(
        "full snapshot rewrite (old per-message cost): {} ms/op",
        rewrite_elapsed.as_millis()
    );
    assert_eq!(store.recent_messages(&conversation_id, 1).len(), 1);
}
