use std::sync::{Arc, Mutex};

use serde::Deserialize;
use tiny_http::{Request, Response, StatusCode};

use crate::{
    AdminDeviceRequest, GatewayRuntime, GatewayStateNotifier,
    http_support::{ResponseHeaderExt, json_header},
    http_write_routes::{
        admin_session_actor, require_admin_actor_capability, require_admin_auth,
        require_admin_capability,
    },
};

type HttpResponse = Response<std::io::Cursor<Vec<u8>>>;

fn with_runtime<T>(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    action: impl FnOnce(&mut GatewayRuntime) -> T,
) -> Result<T, HttpResponse> {
    match runtime.lock() {
        Ok(mut runtime) => Ok(action(&mut runtime)),
        Err(poisoned) => Ok(action(&mut poisoned.into_inner())),
    }
}

pub(crate) fn handle_get_admin_devices(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let devices = match with_runtime(runtime, |runtime| runtime.admin_list_devices()) {
        Ok(devices) => devices,
        Err(response) => return response,
    };
    let body = serde_json::to_string(&devices).unwrap_or_else(|_| "[]".into());
    Response::from_string(body)
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_post_admin_add_device(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_auth(runtime, request) {
        return resp;
    }
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string(r#"{"error":"read body failed"}"#)
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    match serde_json::from_str::<AdminDeviceRequest>(&body) {
        Ok(req) => {
            let actor = if let Some(actor_id) = req.actor_id.as_deref() {
                if let Some(resp) = require_admin_actor_capability(
                    runtime,
                    request,
                    Some(actor_id),
                    crate::CAP_ADMIN_DIAGNOSTICS,
                ) {
                    return resp;
                }
                actor_id.to_string()
            } else {
                if let Some(resp) =
                    require_admin_capability(runtime, request, crate::CAP_ADMIN_DIAGNOSTICS)
                {
                    return resp;
                }
                match admin_session_actor(runtime, request) {
                    Ok(Some(actor)) => actor,
                    Ok(None) => "admin".into(),
                    Err(response) => return response,
                }
            };
            let result = match with_runtime(runtime, |runtime| {
                runtime.admin_add_device(req.address, req.label.unwrap_or_default(), actor)
            }) {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(record) => Response::from_string(
                    serde_json::to_string(&record).unwrap_or_else(|_| "{}".into()),
                )
                .with_status_code(StatusCode(200))
                .with_optional_header(json_header()),
                Err(msg) => Response::from_string(format!("{{\"error\":\"{msg}\"}}"))
                    .with_status_code(StatusCode(400))
                    .with_optional_header(json_header()),
            }
        }
        Err(e) => Response::from_string(format!("{{\"error\":\"decode: {e}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_admin_remove_device(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_auth(runtime, request) {
        return resp;
    }
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string(r#"{"error":"read body failed"}"#)
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    #[derive(Deserialize)]
    struct RemoveReq {
        address: String,
    }
    match serde_json::from_str::<RemoveReq>(&body) {
        Ok(req) => {
            if let Some(resp) =
                require_admin_capability(runtime, request, crate::CAP_ADMIN_DIAGNOSTICS)
            {
                return resp;
            }
            let result =
                match with_runtime(runtime, |runtime| runtime.admin_remove_device(&req.address)) {
                    Ok(result) => result,
                    Err(response) => return response,
                };
            match result {
                Ok(()) => Response::from_string(r#"{"ok":true}"#)
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header()),
                Err(msg) => Response::from_string(format!("{{\"error\":\"{msg}\"}}"))
                    .with_status_code(StatusCode(400))
                    .with_optional_header(json_header()),
            }
        }
        Err(e) => Response::from_string(format!("{{\"error\":\"decode: {e}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_admin_block_device(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_auth(runtime, request) {
        return resp;
    }
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string(r#"{"error":"read body failed"}"#)
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    #[derive(Deserialize)]
    struct BlockReq {
        address: String,
    }
    match serde_json::from_str::<BlockReq>(&body) {
        Ok(req) => {
            if let Some(resp) =
                require_admin_capability(runtime, request, crate::CAP_ADMIN_DIAGNOSTICS)
            {
                return resp;
            }
            let result =
                match with_runtime(runtime, |runtime| runtime.admin_block_device(&req.address)) {
                    Ok(result) => result,
                    Err(response) => return response,
                };
            match result {
                Ok(()) => Response::from_string(r#"{"ok":true}"#)
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header()),
                Err(msg) => Response::from_string(format!("{{\"error\":\"{msg}\"}}"))
                    .with_status_code(StatusCode(400))
                    .with_optional_header(json_header()),
            }
        }
        Err(e) => Response::from_string(format!("{{\"error\":\"decode: {e}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_admin_unblock_device(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_auth(runtime, request) {
        return resp;
    }
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string(r#"{"error":"read body failed"}"#)
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    #[derive(Deserialize)]
    struct UnblockReq {
        address: String,
    }
    match serde_json::from_str::<UnblockReq>(&body) {
        Ok(req) => {
            if let Some(resp) =
                require_admin_capability(runtime, request, crate::CAP_ADMIN_DIAGNOSTICS)
            {
                return resp;
            }
            let result = match with_runtime(runtime, |runtime| {
                runtime.admin_unblock_device(&req.address)
            }) {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(()) => Response::from_string(r#"{"ok":true}"#)
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header()),
                Err(msg) => Response::from_string(format!("{{\"error\":\"{msg}\"}}"))
                    .with_status_code(StatusCode(400))
                    .with_optional_header(json_header()),
            }
        }
        Err(e) => Response::from_string(format!("{{\"error\":\"decode: {e}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}
