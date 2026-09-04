use chat_core::{ATTACHMENT_URL_PREFIX, TimelineEntry, attachment_display_text};

pub(crate) fn timeline_entry_text(entry: &TimelineEntry) -> String {
    if entry.recalled_at_ms.is_some() {
        return "消息已撤回".into();
    }
    // Attachment references carry `attachment://<id>` instead of displayable
    // text; render the shared human fallback so raw hex ids never leak into
    // the transcript.
    let plain_text = &entry.envelope.body.plain_text;
    attachment_display_text(plain_text).unwrap_or_else(|| plain_text.clone())
}

pub(crate) fn timeline_entry_preview(entry: &TimelineEntry) -> String {
    if entry.recalled_at_ms.is_some() {
        return "消息已撤回".into();
    }
    let preview = &entry.envelope.body.preview;
    // Defensive: projections synced from peers may carry the raw reference in
    // the preview field; normalize it instead of showing hex ids.
    if preview.contains(ATTACHMENT_URL_PREFIX) {
        attachment_display_text(preview).unwrap_or_else(|| "[图片]".into())
    } else {
        preview.clone()
    }
}

pub(crate) fn timeline_entry_status_label(entry: &TimelineEntry) -> Option<&'static str> {
    if entry.recalled_at_ms.is_some() {
        Some("已撤回")
    } else if entry.edited_at_ms.is_some() {
        Some("已编辑")
    } else {
        None
    }
}
