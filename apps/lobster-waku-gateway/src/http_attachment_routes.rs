use std::io::Read;
use std::sync::{Arc, Mutex};

use tiny_http::{Header, Request, Response, StatusCode};

use crate::{
    AttachmentUploadResponse, GatewayRuntime,
    attachment_runtime::MAX_ATTACHMENT_BYTES,
    http_support::{ResponseHeaderExt, authorization_bearer_token, json_header},
};

/// POST /v1/shell/attachment — raw image body (png/jpeg/gif/webp), Bearer required.
pub(crate) fn handle_post_shell_attachment(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &mut Request,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(token) = authorization_bearer_token(request) else {
        return attachment_error(StatusCode(401), "authorization bearer token required");
    };

    let mut body = Vec::new();
    let mut limited = request.as_reader().take(MAX_ATTACHMENT_BYTES as u64 + 1);
    if let Err(error) = limited.read_to_end(&mut body) {
        return attachment_error(
            StatusCode(400),
            &format!("read attachment body failed: {error}"),
        );
    }
    if body.len() > MAX_ATTACHMENT_BYTES {
        return attachment_error(
            StatusCode(413),
            &format!("attachment too large: max {MAX_ATTACHMENT_BYTES} bytes"),
        );
    }

    let declared_mime = request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Content-Type"))
        .map(|header| {
            header
                .value
                .as_str()
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .filter(|value| !value.is_empty());

    let runtime = runtime
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // A valid resident session is required; dev bypass keeps local flows usable.
    if !runtime.dev_auth_bypass_enabled()
        && let Err(message) = runtime.resolve_bearer_session(&token)
    {
        return attachment_error(StatusCode(401), &message);
    }
    match runtime.save_image_attachment(body, declared_mime.as_deref()) {
        Ok((attachment_id, mime_type, byte_size)) => {
            let payload = AttachmentUploadResponse {
                ok: true,
                url: format!("/v1/shell/attachment/{attachment_id}"),
                attachment_id,
                mime_type,
                byte_size,
            };
            Response::from_string(
                serde_json::to_string(&payload).unwrap_or_else(|_| "{\"ok\":true}".into()),
            )
            .with_status_code(StatusCode(201))
            .with_optional_header(json_header())
        }
        Err(message) => attachment_error(StatusCode(400), &message),
    }
}

/// GET /v1/shell/attachment/<id> — capability URL, no Bearer required.
pub(crate) fn handle_get_shell_attachment(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    path: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(attachment_id) = path.strip_prefix("/v1/shell/attachment/") else {
        return attachment_error(StatusCode(404), "attachment not found");
    };
    let runtime = runtime
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match runtime.load_image_attachment(attachment_id) {
        Some((bytes, mime_type)) => {
            let content_type = match Header::from_bytes("Content-Type", mime_type.as_bytes())
                .or_else(|_| Header::from_bytes("Content-Type", &b"application/octet-stream"[..]))
            {
                Ok(header) => header,
                Err(_) => {
                    return attachment_error(StatusCode(500), "attachment header unavailable");
                }
            };
            Response::from_data(bytes)
                .with_status_code(StatusCode(200))
                .with_header(content_type)
        }
        None => attachment_error(StatusCode(404), "attachment not found"),
    }
}

fn attachment_error(status: StatusCode, message: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::to_string(&serde_json::json!({ "error": message }))
        .unwrap_or_else(|_| "{\"error\":true}".into());
    Response::from_string(body)
        .with_status_code(status)
        .with_optional_header(json_header())
}
