use std::sync::{Arc, Mutex};

use tiny_http::{Request, Response, StatusCode};

use crate::{
    GatewayRuntime,
    http_support::{ResponseHeaderExt, authorization_bearer_token, json_header},
};

/// GET /v1/push/vapid-public-key — public: clients need it before they can
/// authenticate; the key itself is not a secret.
pub(crate) fn handle_get_push_vapid_public_key(
    runtime: &Arc<Mutex<GatewayRuntime>>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let runtime = runtime
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(public_key) = runtime.vapid_public_key_base64url() else {
        return push_error(StatusCode(503), "push is not available");
    };
    let body = serde_json::json!({ "ok": true, "public_key": public_key }).to_string();
    Response::from_string(body).with_optional_header(json_header())
}

/// POST /v1/push/subscribe — resident Bearer required.
pub(crate) fn handle_post_push_subscribe(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &mut Request,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(token) = authorization_bearer_token(request) else {
        return push_error(StatusCode(401), "authorization bearer token required");
    };
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return push_error(StatusCode(400), &format!("read request failed: {error}"));
    }
    let Ok(subscribe) = serde_json::from_slice::<crate::PushSubscribeRequest>(&body) else {
        return push_error(StatusCode(400), "endpoint and keys are required");
    };
    let mut runtime = runtime
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Ok(session) = runtime.resolve_bearer_session(&token) else {
        return push_error(StatusCode(401), "invalid or expired session");
    };
    match runtime.push_subscribe(
        &session.resident_id,
        &subscribe.endpoint,
        &subscribe.keys.p256dh,
        &subscribe.keys.auth,
    ) {
        Ok(()) => {
            let payload = serde_json::json!({ "ok": true }).to_string();
            Response::from_string(payload)
                .with_status_code(StatusCode(201))
                .with_optional_header(json_header())
        }
        Err(message) => push_error(StatusCode(400), &message),
    }
}

/// POST /v1/push/unsubscribe — resident Bearer required. Only the owner of a
/// subscription may remove it; unknown endpoints report success (idempotent).
pub(crate) fn handle_post_push_unsubscribe(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &mut Request,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(token) = authorization_bearer_token(request) else {
        return push_error(StatusCode(401), "authorization bearer token required");
    };
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return push_error(StatusCode(400), &format!("read request failed: {error}"));
    }
    let Ok(unsubscribe) = serde_json::from_slice::<crate::PushUnsubscribeRequest>(&body) else {
        return push_error(StatusCode(400), "endpoint is required");
    };
    let mut runtime = runtime
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Ok(session) = runtime.resolve_bearer_session(&token) else {
        return push_error(StatusCode(401), "invalid or expired session");
    };
    let owner_matches = runtime
        .push_subscription_resident(&unsubscribe.endpoint)
        .is_none_or(|resident| *resident == session.resident_id);
    if !owner_matches {
        return push_error(StatusCode(403), "subscription belongs to another resident");
    }
    match runtime.push_unsubscribe(&unsubscribe.endpoint) {
        Ok(_) => {
            let payload = serde_json::json!({ "ok": true }).to_string();
            Response::from_string(payload).with_optional_header(json_header())
        }
        Err(message) => push_error(StatusCode(500), &message),
    }
}

fn push_error(status: StatusCode, message: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::to_string(&serde_json::json!({ "error": message }))
        .unwrap_or_else(|_| "{\"error\":true}".into());
    Response::from_string(body)
        .with_status_code(status)
        .with_optional_header(json_header())
}
