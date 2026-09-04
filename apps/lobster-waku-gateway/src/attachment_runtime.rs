//! Image attachments for shell messages.
//!
//! Storage model: files live under `<state_dir>/attachments/<id>.<ext>` and are
//! referenced from message envelopes via an `attachment://<id>` prefix inside
//! `MessageBody::plain_text`. Keeping the reference in the existing text field
//! avoids any postcard schema migration for stored timelines; projections split
//! the prefix back into a structured `ShellMessageAttachment` field.
//!
//! Access model: `GET /v1/shell/attachment/<id>` is a capability URL. The id is
//! 128 bits of CSPRNG entropy, so knowledge of the id is the authorization.
//! Recalled messages stop projecting the attachment, but the file itself is
//! retained (deletion hygiene is a follow-up hardening item).

use std::{fs, path::PathBuf};

use crate::{GatewayRuntime, gateway_models::ShellMessageAttachment};
pub(crate) use chat_core::{
    ATTACHMENT_URL_PREFIX, attachment_reference, split_attachment_text, validate_attachment_id,
};

pub(crate) const MAX_ATTACHMENT_BYTES: usize = 5 * 1024 * 1024;
const ATTACHMENT_EXTENSIONS: [&str; 4] = ["png", "jpg", "gif", "webp"];

/// Sniff the canonical image mime type from magic bytes.
fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

fn extension_for_mime(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        _ => "webp",
    }
}

fn mime_for_extension(extension: &str) -> &'static str {
    match extension {
        "png" => "image/png",
        "jpg" => "image/jpeg",
        "gif" => "image/gif",
        _ => "image/webp",
    }
}

fn generate_attachment_id() -> Result<String, String> {
    let mut entropy = [0u8; 16];
    getrandom::getrandom(&mut entropy)
        .map_err(|error| format!("attachment id entropy failed: {error}"))?;
    Ok(entropy.iter().map(|byte| format!("{byte:02x}")).collect())
}

impl GatewayRuntime {
    fn attachment_path(&self, id: &str) -> Option<PathBuf> {
        if !validate_attachment_id(id) {
            return None;
        }
        ATTACHMENT_EXTENSIONS
            .iter()
            .map(|extension| self.attachments_root.join(format!("{id}.{extension}")))
            .find(|path| path.is_file())
    }

    /// Metadata (mime, byte size) for a stored attachment, if present.
    pub(crate) fn attachment_metadata(&self, id: &str) -> Option<(String, u64)> {
        let path = self.attachment_path(id)?;
        let extension = path.extension()?.to_str()?;
        let metadata = fs::metadata(&path).ok()?;
        Some((mime_for_extension(extension).to_string(), metadata.len()))
    }

    pub(crate) fn attachment_projection(&self, id: &str) -> Option<ShellMessageAttachment> {
        let (mime_type, byte_size) = self.attachment_metadata(id)?;
        Some(ShellMessageAttachment {
            url: format!("/v1/shell/attachment/{id}"),
            mime_type,
            byte_size,
        })
    }

    /// Validate and persist an uploaded image, returning its id and metadata.
    pub(crate) fn save_image_attachment(
        &self,
        bytes: Vec<u8>,
        declared_mime: Option<&str>,
    ) -> Result<(String, String, u64), String> {
        if bytes.is_empty() {
            return Err("attachment body required".into());
        }
        if bytes.len() > MAX_ATTACHMENT_BYTES {
            return Err(format!(
                "attachment too large: max {MAX_ATTACHMENT_BYTES} bytes"
            ));
        }
        let Some(detected) = sniff_image_mime(&bytes) else {
            return Err("unsupported attachment: only png/jpeg/gif/webp images are allowed".into());
        };
        if let Some(declared) = declared_mime {
            let normalized = if declared == "image/jpg" {
                "image/jpeg"
            } else {
                declared
            };
            if normalized != detected {
                return Err(format!(
                    "attachment content-type mismatch: declared {declared}, detected {detected}"
                ));
            }
        }
        let id = generate_attachment_id()?;
        fs::create_dir_all(&self.attachments_root)
            .map_err(|error| format!("attachment directory unavailable: {error}"))?;
        let path = self
            .attachments_root
            .join(format!("{id}.{}", extension_for_mime(detected)));
        fs::write(&path, &bytes).map_err(|error| format!("attachment write failed: {error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        }
        Ok((id, detected.to_string(), bytes.len() as u64))
    }

    /// Load attachment bytes for a capability-URL download.
    pub(crate) fn load_image_attachment(&self, id: &str) -> Option<(Vec<u8>, String)> {
        let path = self.attachment_path(id)?;
        let extension = path.extension()?.to_str()?;
        let bytes = fs::read(&path).ok()?;
        Some((bytes, mime_for_extension(extension).to_string()))
    }

    /// Project a stored plain_text into shell `(text, attachment)` fields.
    /// Recalled messages keep the recall notice and drop the image.
    pub(crate) fn shell_projection_fields(
        &self,
        plain_text: &str,
        recalled: bool,
    ) -> (String, Option<ShellMessageAttachment>) {
        if recalled {
            return ("消息已撤回".into(), None);
        }
        let (attachment_id, caption) = split_attachment_text(plain_text);
        match attachment_id
            .as_deref()
            .and_then(|id| self.attachment_projection(id))
        {
            Some(attachment) => (caption, Some(attachment)),
            None => (plain_text.to_string(), None),
        }
    }
}
