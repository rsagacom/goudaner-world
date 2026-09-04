use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};

use tiny_http::{Request, Response, StatusCode};
use transport_waku::{WakuGatewayRequest, WakuGatewayResponse};

use crate::{
    AddWorldMirrorSourceRequest, AdminBanResidentRequest, AdminConfigPayload,
    AdminCreateInviteRequest, AdminCreateResidentRequest, AdminFreezeRoomRequest,
    AdminHandleLogRequest, AdminManageRoomMemberRequest, AdminModerateMessageRequest,
    AdminRevokeInviteRequest, AdminSetNicknameRequest, AdminUnbanResidentRequest,
    AdminUnfreezeRoomRequest, AdminUpdateSceneRequest, AssignPermissionGroupRequest, AuthSession,
    CliSendRequest, ConnectProviderRequest, ConversationId, CreatePermissionGroupRequest,
    EditShellMessageRequest, GatewayRuntime, GatewayStateNotifier, IdentityId,
    OpenDirectSessionRequest, PersonalRoomAccessPolicyRequest, PersonalRoomRequest,
    RecallShellMessageRequest, ResidentRelationshipRequest, SceneHotspotLayer, SceneImageLayer,
    ShellMarkReadRequest, ShellMessageRequest, ShellPresenceRequest, ShellSetNicknameRequest,
    UpdateShellSceneRequest,
    http_support::{ResponseHeaderExt, authorization_bearer_token, json_header, read_request_body},
};

pub(crate) type HttpResponse = Response<Cursor<Vec<u8>>>;

fn unauthorized(message: String) -> HttpResponse {
    Response::from_string(
        serde_json::to_string(&WakuGatewayResponse::Error { message })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
    )
    .with_status_code(StatusCode(401))
    .with_optional_header(json_header())
}

fn with_runtime<T>(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    action: impl FnOnce(&mut GatewayRuntime) -> T,
) -> Result<T, HttpResponse> {
    match runtime.lock() {
        Ok(mut runtime) => Ok(action(&mut runtime)),
        Err(poisoned) => Ok(action(&mut poisoned.into_inner())),
    }
}

fn missing_admin_actor_response() -> HttpResponse {
    Response::from_string("{\"error\":\"actor_id is required for admin operations\"}")
        .with_status_code(StatusCode(401))
        .with_optional_header(json_header())
}

fn required_admin_actor(actor_id: Option<String>) -> Result<String, HttpResponse> {
    actor_id
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(missing_admin_actor_response)
}

fn resolved_admin_actor(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &tiny_http::Request,
    actor_id: Option<String>,
) -> Result<String, HttpResponse> {
    if let Some(actor_id) = actor_id.filter(|actor_id| !actor_id.trim().is_empty()) {
        return Ok(actor_id);
    }
    match admin_session_actor(runtime, request)? {
        Some(actor_id) => Ok(actor_id),
        None => Ok("admin".into()),
    }
}

fn resolve_admin_session(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &tiny_http::Request,
) -> Result<Option<AuthSession>, HttpResponse> {
    let runtime = runtime
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let token = authorization_bearer_token(request);
    if runtime.dev_auth_bypass_enabled() && token.is_none() {
        return Ok(None);
    }
    let token = token
        .ok_or_else(|| unauthorized("admin operations require a valid Bearer token".into()))?;
    runtime
        .resolve_bearer_session(&token)
        .map(Some)
        .map_err(unauthorized)
}

pub(crate) fn admin_session_actor(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &tiny_http::Request,
) -> Result<Option<String>, HttpResponse> {
    resolve_admin_session(runtime, request)
        .map(|session| session.map(|session| session.resident_id.0))
}

pub(crate) fn require_admin_capability(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &tiny_http::Request,
    capability: &str,
) -> Option<HttpResponse> {
    let session = match resolve_admin_session(runtime, request) {
        Ok(Some(session)) => session,
        Ok(None) => return None,
        Err(response) => return Some(response),
    };
    let resident_id = session.resident_id.0;
    let runtime = match runtime.lock() {
        Ok(runtime) => runtime,
        Err(poisoned) => poisoned.into_inner(),
    };
    if !runtime.resident_has_capability(&resident_id, capability) {
        return Some(unauthorized(format!(
            "forbidden: resident {resident_id} lacks capability {capability}"
        )));
    }
    None
}

pub(crate) fn require_admin_actor_capability(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &tiny_http::Request,
    actor_id: Option<&str>,
    capability: &str,
) -> Option<HttpResponse> {
    let session = match resolve_admin_session(runtime, request) {
        Ok(Some(session)) => session,
        Ok(None) => return None,
        Err(response) => return Some(response),
    };
    let Some(actor_id) = actor_id
        .map(str::trim)
        .filter(|actor_id| !actor_id.is_empty())
    else {
        return Some(unauthorized(
            "actor_id is required for admin operations".into(),
        ));
    };
    if session.resident_id.0 != actor_id {
        return Some(unauthorized(format!(
            "actor_id {actor_id} does not match authenticated session {}",
            session.resident_id.0
        )));
    }
    let runtime = match runtime.lock() {
        Ok(runtime) => runtime,
        Err(poisoned) => poisoned.into_inner(),
    };
    if !runtime.resident_has_capability(actor_id, capability) {
        return Some(unauthorized(format!(
            "forbidden: resident {actor_id} lacks capability {capability}"
        )));
    }
    None
}

pub(crate) fn require_admin_actor_or_capability(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &tiny_http::Request,
    actor_id: Option<&str>,
    capability: &str,
) -> Option<HttpResponse> {
    if actor_id
        .map(str::trim)
        .is_some_and(|actor_id| !actor_id.is_empty())
    {
        require_admin_actor_capability(runtime, request, actor_id, capability)
    } else {
        require_admin_capability(runtime, request, capability)
    }
}

/// Require the request actor to be the resident represented by the Bearer session.
/// Development bypass keeps local fixtures and the explicit dev mode permissive.
pub(crate) fn require_authenticated_actor(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &tiny_http::Request,
    actor_id: &str,
) -> Option<HttpResponse> {
    let session = match resolve_admin_session(runtime, request) {
        Ok(Some(session)) => session,
        Ok(None) => return None,
        Err(response) => return Some(response),
    };
    let actor_id = actor_id.trim();
    if actor_id.is_empty() {
        return Some(unauthorized("actor_id is required".into()));
    }
    if session.resident_id.0 != actor_id {
        return Some(unauthorized(format!(
            "actor_id {actor_id} does not match authenticated session {}",
            session.resident_id.0
        )));
    }
    None
}

/// Authenticate the CLI sidecar sender without conflating `agent:<id>` with a
/// resident session. Agent sends use a token explicitly bound to that agent by
/// `LOBSTER_AGENT_TOKENS`; user sends continue to use the resident Bearer session.
pub(crate) fn require_cli_sender_auth(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &tiny_http::Request,
    sender: &str,
) -> Option<HttpResponse> {
    let token = authorization_bearer_token(request);
    if token.is_none() {
        let bypass = match runtime.lock() {
            Ok(runtime) => runtime.dev_auth_bypass_enabled(),
            Err(poisoned) => poisoned.into_inner().dev_auth_bypass_enabled(),
        };
        if bypass {
            return None;
        }
    }
    let Some(token) = token else {
        return Some(unauthorized(
            "cli sends require a valid Bearer token".into(),
        ));
    };
    let sender = sender.trim();
    if sender.starts_with("agent:") {
        let valid = match runtime.lock() {
            Ok(runtime) => runtime.validate_agent_token(sender, &token),
            Err(poisoned) => poisoned.into_inner().validate_agent_token(sender, &token),
        };
        if !valid {
            return Some(unauthorized(format!("invalid sidecar token for {sender}")));
        }
        return None;
    }
    let resident_id = sender.strip_prefix("user:").unwrap_or(sender);
    require_authenticated_actor(runtime, request, resident_id)
}

/// Authenticate legacy shell edit/recall requests while allowing CLI callers
/// to preserve the typed `user:<id>` / `agent:<id>` identity they used to send.
/// The optional address must agree with the legacy raw actor field.
fn require_shell_actor_auth(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &tiny_http::Request,
    actor: &str,
    actor_address: Option<&str>,
) -> Option<HttpResponse> {
    let Some(actor_address) = actor_address
        .map(str::trim)
        .filter(|address| !address.is_empty())
    else {
        return require_authenticated_actor(runtime, request, actor);
    };
    let Some((kind, identity)) = actor_address.split_once(':') else {
        return Some(unauthorized(
            "actor_address must be user:<id> or agent:<id>".into(),
        ));
    };
    if !matches!(kind, "user" | "agent") || identity.trim().is_empty() {
        return Some(unauthorized(
            "actor_address must be user:<id> or agent:<id>".into(),
        ));
    }
    if identity.trim() != actor.trim() {
        return Some(unauthorized(format!(
            "actor_address {actor_address} does not match actor {}",
            actor.trim()
        )));
    }
    require_cli_sender_auth(runtime, request, actor_address)
}

/// Require a valid Bearer token for admin operations. In dev mode, bypasses if LOBSTER_DEV_AUTH_BYPASS=1.
pub(crate) fn require_admin_auth(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &tiny_http::Request,
) -> Option<HttpResponse> {
    resolve_admin_session(runtime, request).err()
}

pub(crate) fn handle_post_world_mirror_sources(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }

    match serde_json::from_slice::<AddWorldMirrorSourceRequest>(&body) {
        Ok(payload) => {
            let result =
                match with_runtime(runtime, |runtime| runtime.add_world_mirror_source(payload)) {
                    Ok(result) => result,
                    Err(response) => return response,
                };
            match result {
                Ok(mirror_sources) => Response::from_string(
                    serde_json::to_string(&mirror_sources).unwrap_or_else(|_| "[]".into()),
                )
                .with_status_code(StatusCode(200))
                .with_optional_header(json_header()),
                Err(message) => Response::from_string(
                    serde_json::to_string(&WakuGatewayResponse::Error { message })
                        .unwrap_or_else(|_| "{\"error\":true}".into()),
                )
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header()),
            }
        }
        Err(error) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("decode world mirror source failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_provider_connect(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }

    match serde_json::from_slice::<ConnectProviderRequest>(&body) {
        Ok(payload) => {
            let result = match with_runtime(runtime, |runtime| runtime.connect_provider(payload)) {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(provider) => Response::from_string(
                    serde_json::to_string(&provider).unwrap_or_else(|_| "{}".into()),
                )
                .with_status_code(StatusCode(200))
                .with_optional_header(json_header()),
                Err(message) => Response::from_string(
                    serde_json::to_string(&WakuGatewayResponse::Error { message })
                        .unwrap_or_else(|_| "{\"error\":true}".into()),
                )
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header()),
            }
        }
        Err(error) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("decode connect provider failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_provider_disconnect(
    runtime: &Arc<Mutex<GatewayRuntime>>,
) -> HttpResponse {
    let result = match with_runtime(runtime, |runtime| runtime.disconnect_provider()) {
        Ok(result) => result,
        Err(response) => return response,
    };
    match result {
        Ok(provider) => {
            Response::from_string(serde_json::to_string(&provider).unwrap_or_else(|_| "{}".into()))
                .with_status_code(StatusCode(200))
                .with_optional_header(json_header())
        }
        Err(message) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error { message })
                .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_direct_open(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }

    match serde_json::from_slice::<OpenDirectSessionRequest>(&body) {
        Ok(payload) => {
            if let Some(resp) = require_authenticated_actor(runtime, request, &payload.requester_id)
            {
                return resp;
            }
            let result = match with_runtime(runtime, |runtime| runtime.open_direct_session(payload))
            {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(group) => {
                    notifier.notify_changed();
                    Response::from_string(
                        serde_json::to_string(&group).unwrap_or_else(|_| "{}".into()),
                    )
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header())
                }
                Err(message) => Response::from_string(
                    serde_json::to_string(&WakuGatewayResponse::Error { message })
                        .unwrap_or_else(|_| "{\"error\":true}".into()),
                )
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header()),
            }
        }
        Err(error) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("decode direct session request failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_personal_room(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    let auth_token = authorization_bearer_token(request);
    let body = match read_request_body(request) {
        Ok(body) => body,
        Err(error) => {
            return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };

    match serde_json::from_slice::<PersonalRoomRequest>(&body) {
        Ok(payload) => {
            let actor = IdentityId(payload.resident_id.trim().to_string());
            let result = match with_runtime(runtime, |runtime| {
                let Some(token) = auth_token.as_deref() else {
                    return Err((
                        StatusCode(401),
                        "personal room requires a valid Bearer token".to_string(),
                    ));
                };
                if let Err(message) = runtime.validate_bearer_session_actor(token, &actor) {
                    return Err((StatusCode(401), message));
                }
                runtime
                    .open_personal_room(payload)
                    .map_err(|message| (StatusCode(400), message))
            }) {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(response) => {
                    notifier.notify_changed();
                    Response::from_string(
                        serde_json::to_string(&response).unwrap_or_else(|_| "{}".into()),
                    )
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header())
                }
                Err((status, message)) => Response::from_string(
                    serde_json::to_string(&WakuGatewayResponse::Error { message })
                        .unwrap_or_else(|_| "{\"error\":true}".into()),
                )
                .with_status_code(status)
                .with_optional_header(json_header()),
            }
        }
        Err(error) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("decode personal room request failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_personal_room_access_policy(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    let auth_token = authorization_bearer_token(request);
    let body = match read_request_body(request) {
        Ok(body) => body,
        Err(error) => {
            return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };

    match serde_json::from_slice::<PersonalRoomAccessPolicyRequest>(&body) {
        Ok(payload) => {
            let actor = IdentityId(payload.resident_id.trim().to_string());
            let result = match with_runtime(runtime, |runtime| {
                let Some(token) = auth_token.as_deref() else {
                    return Err((
                        StatusCode(401),
                        "personal room access policy requires a valid Bearer token".to_string(),
                    ));
                };
                if let Err(message) = runtime.validate_bearer_session_actor(token, &actor) {
                    return Err((StatusCode(401), message));
                }
                runtime
                    .set_personal_room_access_policy(payload)
                    .map_err(|message| (StatusCode(400), message))
            }) {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(response) => {
                    notifier.notify_changed();
                    Response::from_string(
                        serde_json::to_string(&response).unwrap_or_else(|_| "{}".into()),
                    )
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header())
                }
                Err((status, message)) => Response::from_string(
                    serde_json::to_string(&WakuGatewayResponse::Error { message })
                        .unwrap_or_else(|_| "{\"error\":true}".into()),
                )
                .with_status_code(status)
                .with_optional_header(json_header()),
            }
        }
        Err(error) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("decode personal room access policy request failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

fn handle_resident_relationship_write(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
    action: &str,
) -> HttpResponse {
    let auth_token = authorization_bearer_token(request);
    let body = match read_request_body(request) {
        Ok(body) => body,
        Err(error) => {
            return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };

    match serde_json::from_slice::<ResidentRelationshipRequest>(&body) {
        Ok(payload) => {
            let actor = IdentityId(payload.actor_id.trim().to_string());
            let result = match with_runtime(runtime, |runtime| {
                let Some(token) = auth_token.as_deref() else {
                    return Err((
                        StatusCode(401),
                        "resident relationship requires a valid Bearer token".to_string(),
                    ));
                };
                if let Err(message) = runtime.validate_bearer_session_actor(token, &actor) {
                    return Err((StatusCode(401), message));
                }
                match action {
                    "request" => runtime
                        .request_resident_friendship(payload)
                        .map_err(|message| (StatusCode(400), message)),
                    "accept" => runtime
                        .accept_resident_friendship(payload)
                        .map_err(|message| (StatusCode(400), message)),
                    _ => Err((StatusCode(400), "unknown relationship action".into())),
                }
            }) {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(response) => {
                    notifier.notify_changed();
                    Response::from_string(
                        serde_json::to_string(&response).unwrap_or_else(|_| "{}".into()),
                    )
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header())
                }
                Err((status, message)) => Response::from_string(
                    serde_json::to_string(&WakuGatewayResponse::Error { message })
                        .unwrap_or_else(|_| "{\"error\":true}".into()),
                )
                .with_status_code(status)
                .with_optional_header(json_header()),
            }
        }
        Err(error) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("decode resident relationship request failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_resident_relationship_request(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    handle_resident_relationship_write(runtime, notifier, request, "request")
}

pub(crate) fn handle_post_resident_relationship_accept(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    handle_resident_relationship_write(runtime, notifier, request, "accept")
}

pub(crate) fn handle_post_waku(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(response) = require_federation_auth(runtime, request) {
        return response;
    }
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("read request body failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"Error\":\"read body failed\"}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header());
    }

    match serde_json::from_slice::<WakuGatewayRequest>(&body) {
        Ok(gateway_request) => {
            let gateway_response =
                match with_runtime(runtime, |runtime| runtime.handle(gateway_request)) {
                    Ok(gateway_response) => gateway_response,
                    Err(response) => return response,
                };
            let status = match gateway_response {
                WakuGatewayResponse::Error { .. } => StatusCode(400),
                _ => StatusCode(200),
            };
            Response::from_string(
                serde_json::to_string(&gateway_response)
                    .unwrap_or_else(|_| "{\"Error\":{\"message\":\"serialize failed\"}}".into()),
            )
            .with_status_code(status)
            .with_optional_header(json_header())
        }
        Err(error) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("decode gateway request failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"Error\":\"decode failed\"}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

fn require_federation_auth(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &tiny_http::Request,
) -> Option<HttpResponse> {
    let token = authorization_bearer_token(request);
    let runtime = match runtime.lock() {
        Ok(runtime) => runtime,
        Err(poisoned) => poisoned.into_inner(),
    };
    if runtime.dev_auth_bypass_enabled() && token.is_none() {
        return None;
    }
    if token
        .as_deref()
        .is_some_and(|token| runtime.validate_federation_token(token))
    {
        return None;
    }
    Some(
        Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: "gateway federation Bearer token required or invalid".into(),
            })
            .unwrap_or_else(|_| "{\"Error\":{\"message\":\"unauthorized\"}}".into()),
        )
        .with_status_code(StatusCode(401))
        .with_optional_header(json_header()),
    )
}

pub(crate) fn handle_post_shell_message(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }

    match serde_json::from_slice::<ShellMessageRequest>(&body) {
        Ok(payload) => {
            if let Some(resp) = require_authenticated_actor(runtime, request, &payload.sender) {
                return resp;
            }
            let result = match with_runtime(runtime, |runtime| {
                if let Some(retry_ms) = runtime.check_rate_limit(&payload.sender, 30) {
                    return Err(Response::from_string(format!(
                        "{{\"error\":\"rate_limited\",\"retry_after_ms\":{}}}",
                        retry_ms
                    ))
                    .with_status_code(StatusCode(429))
                    .with_optional_header(json_header()));
                }
                Ok(runtime.append_shell_message(payload))
            }) {
                Ok(Ok(result)) => result,
                Ok(Err(response)) | Err(response) => return response,
            };
            match result {
                Ok(response) => {
                    notifier.notify_changed();
                    Response::from_string(
                        serde_json::to_string(&response).unwrap_or_else(|_| "{\"ok\":true}".into()),
                    )
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header())
                }
                Err(message) => Response::from_string(
                    serde_json::to_string(&WakuGatewayResponse::Error { message })
                        .unwrap_or_else(|_| "{\"error\":true}".into()),
                )
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header()),
            }
        }
        Err(error) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("decode shell message failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_shell_scene(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }

    match serde_json::from_slice::<UpdateShellSceneRequest>(&body) {
        Ok(payload) => {
            if let Some(resp) = require_authenticated_actor(runtime, request, &payload.actor) {
                return resp;
            }
            let result =
                match with_runtime(runtime, |runtime| Ok(runtime.update_shell_scene(payload))) {
                    Ok(Ok(result)) => result,
                    Ok(Err(response)) | Err(response) => return response,
                };
            match result {
                Ok(response) => {
                    notifier.notify_changed();
                    Response::from_string(
                        serde_json::to_string(&response).unwrap_or_else(|_| "{\"ok\":true}".into()),
                    )
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header())
                }
                Err(message) => Response::from_string(
                    serde_json::to_string(&WakuGatewayResponse::Error { message })
                        .unwrap_or_else(|_| "{\"error\":true}".into()),
                )
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header()),
            }
        }
        Err(error) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("decode shell scene update failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_shell_message_recall(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }

    match serde_json::from_slice::<RecallShellMessageRequest>(&body) {
        Ok(payload) => {
            if let Some(resp) = require_shell_actor_auth(
                runtime,
                request,
                &payload.actor,
                payload.actor_address.as_deref(),
            ) {
                return resp;
            }
            let result =
                match with_runtime(runtime, |runtime| Ok(runtime.recall_shell_message(payload))) {
                    Ok(Ok(result)) => result,
                    Ok(Err(response)) | Err(response) => return response,
                };
            match result {
                Ok(response) => {
                    notifier.notify_changed();
                    Response::from_string(
                        serde_json::to_string(&response).unwrap_or_else(|_| "{\"ok\":true}".into()),
                    )
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header())
                }
                Err(message) => Response::from_string(
                    serde_json::to_string(&WakuGatewayResponse::Error { message })
                        .unwrap_or_else(|_| "{\"error\":true}".into()),
                )
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header()),
            }
        }
        Err(error) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("decode shell message recall failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_shell_message_edit(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }

    match serde_json::from_slice::<EditShellMessageRequest>(&body) {
        Ok(payload) => {
            if let Some(resp) = require_shell_actor_auth(
                runtime,
                request,
                &payload.actor,
                payload.actor_address.as_deref(),
            ) {
                return resp;
            }
            let result =
                match with_runtime(runtime, |runtime| Ok(runtime.edit_shell_message(payload))) {
                    Ok(Ok(result)) => result,
                    Ok(Err(response)) | Err(response) => return response,
                };
            match result {
                Ok(response) => {
                    notifier.notify_changed();
                    Response::from_string(
                        serde_json::to_string(&response).unwrap_or_else(|_| "{\"ok\":true}".into()),
                    )
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header())
                }
                Err(message) => Response::from_string(
                    serde_json::to_string(&WakuGatewayResponse::Error { message })
                        .unwrap_or_else(|_| "{\"error\":true}".into()),
                )
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header()),
            }
        }
        Err(error) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("decode shell message edit failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_cli_send(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }

    match serde_json::from_slice::<CliSendRequest>(&body) {
        Ok(payload) => {
            if let Some(resp) = require_cli_sender_auth(runtime, request, &payload.from) {
                return resp;
            }
            let result = match with_runtime(runtime, |runtime| runtime.send_cli_message(payload)) {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(response) => {
                    notifier.notify_changed();
                    Response::from_string(
                        serde_json::to_string(&response).unwrap_or_else(|_| "{\"ok\":true}".into()),
                    )
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header())
                }
                Err(message) => Response::from_string(
                    serde_json::to_string(&WakuGatewayResponse::Error { message })
                        .unwrap_or_else(|_| "{\"error\":true}".into()),
                )
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header()),
            }
        }
        Err(error) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error {
                message: format!("decode cli send request failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_shell_presence(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = String::new();
    if let Err(error) = request.as_reader().read_to_string(&mut body) {
        return Response::from_string(format!("{{\"error\":\"read body failed: {error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    match serde_json::from_str::<ShellPresenceRequest>(&body) {
        Ok(presence) => {
            let resident_id = presence.resident_id.trim().to_string();
            if resident_id.is_empty() {
                return Response::from_string("{\"error\":\"resident_id is required\"}")
                    .with_status_code(StatusCode(400))
                    .with_optional_header(json_header());
            }
            if let Some(resp) = require_authenticated_actor(runtime, request, &resident_id) {
                return resp;
            }
            let became_online =
                match with_runtime(runtime, |runtime| runtime.record_presence(&resident_id)) {
                    Ok(became_online) => became_online,
                    Err(response) => return response,
                };
            if became_online {
                notifier.notify_changed();
            }
            Response::from_string("{\"ok\":true}")
                .with_status_code(StatusCode(200))
                .with_optional_header(json_header())
        }
        Err(error) => Response::from_string(format!("{{\"error\":\"decode failed: {error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_shell_mark_read(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = String::new();
    if let Err(error) = request.as_reader().read_to_string(&mut body) {
        return Response::from_string(format!("{{\"error\":\"read body failed: {error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    match serde_json::from_str::<ShellMarkReadRequest>(&body) {
        Ok(read_req) => {
            let resident_id = IdentityId(read_req.resident_id.trim().to_string());
            let conversation_id = ConversationId(read_req.conversation_id.trim().to_string());
            if resident_id.0.is_empty() || conversation_id.0.is_empty() {
                return Response::from_string(
                    "{\"error\":\"resident_id and conversation_id are required\"}",
                )
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
            }
            if let Some(resp) = require_authenticated_actor(runtime, request, &resident_id.0) {
                return resp;
            }
            if let Err(response) = with_runtime(runtime, |runtime| {
                runtime.mark_read(&resident_id, &conversation_id);
            }) {
                return response;
            }
            Response::from_string("{\"ok\":true}")
                .with_status_code(StatusCode(200))
                .with_optional_header(json_header())
        }
        Err(error) => Response::from_string(format!("{{\"error\":\"decode failed: {error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_admin_ban_resident(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_auth(runtime, request) {
        return resp;
    }
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string("{\"error\":\"read body failed\"}")
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    match serde_json::from_str::<AdminBanResidentRequest>(&body) {
        Ok(req) => {
            let actor = match required_admin_actor(req.actor_id.clone()) {
                Ok(actor) => actor,
                Err(response) => return response,
            };
            if let Some(resp) = require_admin_actor_capability(
                runtime,
                request,
                req.actor_id.as_deref(),
                crate::CAP_BAN_RESIDENT,
            ) {
                return resp;
            }
            match with_runtime(runtime, |rt| {
                match rt.admin_ban_resident(&req.resident_id, &req.reason) {
                    Ok(()) => {
                        rt.log_audit_event(
                            &actor,
                            "admin:ban_resident",
                            &req.resident_id,
                            Some(&req.reason),
                        );
                        Response::from_string("{\"ok\":true}")
                            .with_status_code(StatusCode(200))
                            .with_optional_header(json_header())
                    }
                    Err(e) => Response::from_string(format!("{{\"error\":\"{e}\"}}"))
                        .with_status_code(StatusCode(400))
                        .with_optional_header(json_header()),
                }
            }) {
                Ok(response) => response,
                Err(response) => response,
            }
        }
        Err(e) => Response::from_string(format!("{{\"error\":\"decode failed: {e}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_admin_unban_resident(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_auth(runtime, request) {
        return resp;
    }
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string("{\"error\":\"read body failed\"}")
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    match serde_json::from_str::<AdminUnbanResidentRequest>(&body) {
        Ok(req) => {
            let actor = match required_admin_actor(req.actor_id.clone()) {
                Ok(actor) => actor,
                Err(response) => return response,
            };
            if let Some(resp) = require_admin_actor_capability(
                runtime,
                request,
                Some(actor.as_str()),
                crate::CAP_BAN_RESIDENT,
            ) {
                return resp;
            }
            match with_runtime(runtime, |rt| {
                match rt.admin_unban_resident(&req.resident_id) {
                    Ok(count) => {
                        rt.log_audit_event(&actor, "admin:unban_resident", &req.resident_id, None);
                        Response::from_string(
                            serde_json::to_string(
                                &serde_json::json!({"ok": true, "lifted_count": count}),
                            )
                            .unwrap_or_else(|_| "{}".into()),
                        )
                        .with_status_code(StatusCode(200))
                        .with_optional_header(json_header())
                    }
                    Err(e) => Response::from_string(format!("{{\"error\":\"{e}\"}}"))
                        .with_status_code(StatusCode(400))
                        .with_optional_header(json_header()),
                }
            }) {
                Ok(response) => response,
                Err(response) => response,
            }
        }
        Err(e) => Response::from_string(format!("{{\"error\":\"decode failed: {e}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_admin_set_nickname(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_auth(runtime, request) {
        return resp;
    }
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string("{\"error\":\"read body failed\"}")
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    match serde_json::from_str::<AdminSetNicknameRequest>(&body) {
        Ok(req) => {
            if let Some(resp) = require_admin_actor_or_capability(
                runtime,
                request,
                req.actor_id.as_deref(),
                crate::CAP_MANAGE_RESIDENT,
            ) {
                return resp;
            }
            match with_runtime(runtime, |rt| {
                match rt.admin_set_nickname(&req.resident_id, req.nickname.as_deref()) {
                    Ok(true) => Response::from_string(
                        serde_json::to_string(&serde_json::json!({"ok": true}))
                            .unwrap_or_else(|_| "{}".into()),
                    )
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header()),
                    Ok(false) => Response::from_string("{\"error\":\"resident not found\"}")
                        .with_status_code(StatusCode(404))
                        .with_optional_header(json_header()),
                    Err(e) => Response::from_string(format!("{{\"error\":\"{e}\"}}"))
                        .with_status_code(StatusCode(400))
                        .with_optional_header(json_header()),
                }
            }) {
                Ok(response) => response,
                Err(response) => response,
            }
        }
        Err(e) => Response::from_string(format!("{{\"error\":\"decode failed: {e}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_shell_set_nickname(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    let token = match authorization_bearer_token(request) {
        Some(t) => t,
        None => return unauthorized("authorization bearer token required".into()),
    };
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string("{\"error\":\"read body failed\"}")
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    let req: ShellSetNicknameRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return Response::from_string(format!("{{\"error\":\"decode failed: {e}\"}}"))
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };
    match with_runtime(runtime, |rt| {
        let session = match rt.resolve_bearer_session(&token) {
            Ok(session) => session,
            Err(e) => {
                return Response::from_string(format!("{{\"error\":\"{e}\"}}"))
                    .with_status_code(StatusCode(401))
                    .with_optional_header(json_header());
            }
        };
        match rt.shell_set_nickname(&session.resident_id.0, req.nickname.as_deref()) {
            Ok((true, nickname)) => Response::from_string(
                serde_json::to_string(&serde_json::json!({"ok": true, "nickname": nickname}))
                    .unwrap_or_else(|_| "{}".into()),
            )
            .with_status_code(StatusCode(200))
            .with_optional_header(json_header()),
            Ok((false, _)) => Response::from_string("{\"error\":\"registration not found\"}")
                .with_status_code(StatusCode(404))
                .with_optional_header(json_header()),
            Err(e) => Response::from_string(format!("{{\"error\":\"{e}\"}}"))
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header()),
        }
    }) {
        Ok(response) => response,
        Err(response) => response,
    }
}

pub(crate) fn handle_post_admin_freeze_room(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_auth(runtime, request) {
        return resp;
    }
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string("{\"error\":\"read body failed\"}")
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    match serde_json::from_str::<AdminFreezeRoomRequest>(&body) {
        Ok(req) => {
            let actor = match resolved_admin_actor(runtime, request, req.actor_id.clone()) {
                Ok(actor) => actor,
                Err(response) => return response,
            };
            if let Some(resp) = require_admin_actor_or_capability(
                runtime,
                request,
                req.actor_id.as_deref(),
                crate::CAP_FREEZE_ROOM,
            ) {
                return resp;
            }
            match with_runtime(runtime, |rt| match rt.admin_freeze_room(&req.room_id) {
                Ok(_) => {
                    rt.log_audit_event(&actor, "admin:freeze_room", &req.room_id, None);
                    Response::from_string("{\"ok\":true}")
                        .with_status_code(StatusCode(200))
                        .with_optional_header(json_header())
                }
                Err(e) => Response::from_string(format!("{{\"error\":\"{e}\"}}"))
                    .with_status_code(StatusCode(400))
                    .with_optional_header(json_header()),
            }) {
                Ok(response) => response,
                Err(response) => response,
            }
        }
        Err(e) => Response::from_string(format!("{{\"error\":\"decode failed: {e}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_admin_unfreeze_room(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_auth(runtime, request) {
        return resp;
    }
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string("{\"error\":\"read body failed\"}")
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    match serde_json::from_str::<AdminUnfreezeRoomRequest>(&body) {
        Ok(req) => {
            let actor = match resolved_admin_actor(runtime, request, req.actor_id.clone()) {
                Ok(actor) => actor,
                Err(response) => return response,
            };
            if let Some(resp) = require_admin_actor_or_capability(
                runtime,
                request,
                req.actor_id.as_deref(),
                crate::CAP_FREEZE_ROOM,
            ) {
                return resp;
            }
            match with_runtime(runtime, |rt| match rt.admin_unfreeze_room(&req.room_id) {
                Ok(_) => {
                    rt.log_audit_event(&actor, "admin:unfreeze_room", &req.room_id, None);
                    Response::from_string("{\"ok\":true}")
                        .with_status_code(StatusCode(200))
                        .with_optional_header(json_header())
                }
                Err(e) => Response::from_string(format!("{{\"error\":\"{e}\"}}"))
                    .with_status_code(StatusCode(400))
                    .with_optional_header(json_header()),
            }) {
                Ok(response) => response,
                Err(response) => response,
            }
        }
        Err(e) => Response::from_string(format!("{{\"error\":\"decode failed: {e}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_admin_config(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_auth(runtime, request) {
        return resp;
    }
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string("{\"error\":\"read body failed\"}")
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    match serde_json::from_str::<AdminConfigPayload>(&body) {
        Ok(payload) => {
            let actor = match resolved_admin_actor(runtime, request, payload.actor_id.clone()) {
                Ok(actor) => actor,
                Err(response) => return response,
            };
            if let Some(resp) = require_admin_actor_or_capability(
                runtime,
                request,
                payload.actor_id.as_deref(),
                crate::CAP_ADMIN_CONFIG,
            ) {
                return resp;
            }
            match with_runtime(runtime, |runtime| {
                runtime.admin_set_config(payload.config)?;
                runtime.log_audit_event(&actor, "admin:config", "app-config", None);
                Ok::<(), String>(())
            }) {
                Ok(Ok(())) => Response::from_string("{\"ok\":true}")
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header()),
                Ok(Err(error)) => {
                    Response::from_string(serde_json::json!({"error": error}).to_string())
                        .with_status_code(StatusCode(500))
                        .with_optional_header(json_header())
                }
                Err(response) => response,
            }
        }
        Err(e) => Response::from_string(format!("{{\"error\":\"decode failed: {e}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_admin_moderate_message(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_auth(runtime, request) {
        return resp;
    }
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string("{\"error\":\"read body failed\"}")
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    match serde_json::from_str::<AdminModerateMessageRequest>(&body) {
        Ok(req) => {
            let actor = match resolved_admin_actor(runtime, request, req.actor_id.clone()) {
                Ok(actor) => actor,
                Err(response) => return response,
            };
            if let Some(resp) = require_admin_actor_or_capability(
                runtime,
                request,
                req.actor_id.as_deref(),
                crate::CAP_MODERATE_MESSAGE,
            ) {
                return resp;
            }
            match with_runtime(runtime, |rt| {
                let msg_id = req.message_id.clone();
                let conv_id = req.conversation_id.clone();
                let action = req.action.clone();
                match rt.admin_moderate_message(&req.message_id, &req.conversation_id, &req.action)
                {
                    Ok(()) => {
                        let target = format!("msg:{}@{}", msg_id, conv_id);
                        rt.log_audit_event(
                            &actor,
                            &format!("admin:moderate_message:{}", action),
                            &target,
                            req.reason.as_deref(),
                        );
                        Response::from_string(
                            serde_json::to_string(&serde_json::json!({"ok": true, "message_id": msg_id, "action": action}))
                                .unwrap_or_else(|_| "{}".into()),
                        )
                        .with_status_code(StatusCode(200))
                        .with_optional_header(json_header())
                    }
                    Err(e) => Response::from_string(format!("{{\"error\":\"{e}\"}}"))
                        .with_status_code(StatusCode(400))
                        .with_optional_header(json_header()),
                }
            }) {
                Ok(response) => response,
                Err(response) => response,
            }
        }
        Err(e) => Response::from_string(format!("{{\"error\":\"decode failed: {e}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_admin_create_invite(
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
    let req: AdminCreateInviteRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return Response::from_string(serde_json::json!({"error": e.to_string()}).to_string())
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };
    if let Some(resp) = require_admin_actor_capability(
        runtime,
        request,
        Some(req.actor_id.as_str()),
        crate::CAP_INVITE_RESIDENT,
    ) {
        return resp;
    }
    let max_uses = req.max_uses.unwrap_or(10);
    let resp = match with_runtime(runtime, |rt| {
        rt.admin_create_invite(&req.actor_id, max_uses)
    }) {
        Ok(Ok(resp)) => resp,
        Ok(Err(error)) => {
            return Response::from_string(serde_json::json!({"error": error}).to_string())
                .with_status_code(StatusCode(500))
                .with_optional_header(json_header());
        }
        Err(response) => return response,
    };
    let json = serde_json::to_string(&resp).unwrap_or_default();
    Response::from_string(json)
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_post_admin_revoke_invite(
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
    let req: AdminRevokeInviteRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return Response::from_string(serde_json::json!({"error": e.to_string()}).to_string())
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };
    if let Some(resp) = require_admin_actor_capability(
        runtime,
        request,
        Some(req.actor_id.as_str()),
        crate::CAP_INVITE_RESIDENT,
    ) {
        return resp;
    }
    let ok = match with_runtime(runtime, |rt| rt.admin_revoke_invite(&req.code)) {
        Ok(Ok(ok)) => ok,
        Ok(Err(error)) => {
            return Response::from_string(serde_json::json!({"error": error}).to_string())
                .with_status_code(StatusCode(500))
                .with_optional_header(json_header());
        }
        Err(response) => return response,
    };
    let body = if ok {
        r#"{"ok":true}"#
    } else {
        r#"{"ok":false,"error":"not found"}"#
    };
    let code = if ok { StatusCode(200) } else { StatusCode(404) };
    Response::from_string(body)
        .with_status_code(code)
        .with_optional_header(json_header())
}

pub(crate) fn handle_post_admin_manage_room_member(
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
    let req: AdminManageRoomMemberRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return Response::from_string(serde_json::json!({"error": e.to_string()}).to_string())
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };
    if let Some(resp) = require_admin_actor_capability(
        runtime,
        request,
        Some(req.actor_id.as_str()),
        crate::CAP_MANAGE_RESIDENT,
    ) {
        return resp;
    }
    let ok = match with_runtime(runtime, |rt| {
        rt.admin_manage_room_member(&req.room_id, &req.resident_id, &req.action)
    }) {
        Ok(Ok(ok)) => ok,
        Ok(Err(error)) => {
            return Response::from_string(serde_json::json!({"error": error}).to_string())
                .with_status_code(StatusCode(500))
                .with_optional_header(json_header());
        }
        Err(response) => return response,
    };
    let resp = if ok {
        r#"{"ok":true}"#
    } else {
        r#"{"ok":false,"error":"room not found"}"#
    };
    let code = if ok { StatusCode(200) } else { StatusCode(404) };
    Response::from_string(resp)
        .with_status_code(code)
        .with_optional_header(json_header())
}

pub(crate) fn handle_post_admin_handle_log(
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
    let req: AdminHandleLogRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return Response::from_string(serde_json::json!({"error": e.to_string()}).to_string())
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };
    if let Some(resp) = require_admin_actor_capability(
        runtime,
        request,
        Some(req.actor_id.as_str()),
        crate::CAP_ADMIN_DIAGNOSTICS,
    ) {
        return resp;
    }
    match with_runtime(runtime, |rt| rt.admin_handle_log(&req.log_id)) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            return Response::from_string(serde_json::json!({"error": error}).to_string())
                .with_status_code(StatusCode(500))
                .with_optional_header(json_header());
        }
        Err(response) => return response,
    }
    Response::from_string(r#"{"ok":true}"#)
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_post_admin_clear_processed_logs(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_capability(runtime, request, crate::CAP_ADMIN_DIAGNOSTICS) {
        return resp;
    }
    let count = match with_runtime(runtime, |rt| rt.admin_clear_processed_logs()) {
        Ok(Ok(count)) => count,
        Ok(Err(error)) => {
            return Response::from_string(serde_json::json!({"error": error}).to_string())
                .with_status_code(StatusCode(500))
                .with_optional_header(json_header());
        }
        Err(response) => return response,
    };
    let body = serde_json::json!({"ok": true, "cleared": count}).to_string();
    Response::from_string(body)
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_post_admin_create_resident(
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
    let req: AdminCreateResidentRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return Response::from_string(serde_json::json!({"error": e.to_string()}).to_string())
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };
    if let Some(resp) = require_admin_actor_or_capability(
        runtime,
        request,
        req.actor_id.as_deref(),
        crate::CAP_MANAGE_RESIDENT,
    ) {
        return resp;
    }
    let ok = match with_runtime(runtime, |rt| {
        rt.admin_create_resident(&req.resident_id, &req.email)
    }) {
        Ok(Ok(ok)) => ok,
        Ok(Err(error)) => {
            return Response::from_string(serde_json::json!({"error": error}).to_string())
                .with_status_code(StatusCode(500))
                .with_optional_header(json_header());
        }
        Err(response) => return response,
    };
    let resp = if ok {
        r#"{"ok":true}"#
    } else {
        r#"{"ok":false,"error":"resident already exists"}"#
    };
    let code = if ok { StatusCode(200) } else { StatusCode(409) };
    Response::from_string(resp)
        .with_status_code(code)
        .with_optional_header(json_header())
}

pub(crate) fn handle_post_scene_validate(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    _notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Response::from_string("{\"error\":\"read body failed\"}")
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    let parsed: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            return Response::from_string(format!("{{\"error\":\"decode failed: {e}\"}}"))
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };
    let image_layer: Option<SceneImageLayer> = parsed
        .get("image_layer")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let hotspot_layer: Option<SceneHotspotLayer> = parsed
        .get("hotspot_layer")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let result = match with_runtime(runtime, |runtime| {
        runtime.validate_scene_config(&ConversationId("".into()), &image_layer, &hotspot_layer)
    }) {
        Ok(result) => result,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&result).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_post_create_permission_group(
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
    let req: CreatePermissionGroupRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return Response::from_string(serde_json::json!({"error": e.to_string()}).to_string())
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };
    if req.name.is_empty() {
        return Response::from_string(r#"{"error":"name is required"}"#)
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    if req.capabilities.is_empty() {
        return Response::from_string(r#"{"error":"at least one capability is required"}"#)
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    if let Some(resp) = require_admin_actor_capability(
        runtime,
        request,
        Some(req.actor_id.as_str()),
        crate::CAP_MANAGE_PERMISSIONS,
    ) {
        return resp;
    }
    let resp = match with_runtime(runtime, |rt| {
        let result = rt.admin_create_permission_group(
            &req.actor_id,
            &req.name,
            &req.description,
            req.capabilities,
        );
        if let Ok(result) = &result {
            rt.log_audit_event(
                &req.actor_id,
                "admin:create_permission_group",
                &result.group.id,
                None,
            );
        }
        result
    }) {
        Ok(Ok(resp)) => resp,
        Ok(Err(error)) => {
            return Response::from_string(serde_json::json!({"error": error}).to_string())
                .with_status_code(StatusCode(500))
                .with_optional_header(json_header());
        }
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&resp).unwrap_or_else(|_| r#"{"ok":false}"#.into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_post_assign_permission_group(
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
    let req: AssignPermissionGroupRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return Response::from_string(serde_json::json!({"error": e.to_string()}).to_string())
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };
    if req.resident_id.is_empty() || req.permission_group_id.is_empty() {
        return Response::from_string(
            r#"{"error":"resident_id and permission_group_id are required"}"#,
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header());
    }
    if let Some(resp) = require_admin_actor_capability(
        runtime,
        request,
        Some(req.actor_id.as_str()),
        crate::CAP_MANAGE_PERMISSIONS,
    ) {
        return resp;
    }
    let resp = match with_runtime(runtime, |rt| {
        let result = rt.admin_assign_permission_group(&req.resident_id, &req.permission_group_id);
        if result.is_ok() {
            let target = format!(
                "resident:{}→pg:{}",
                req.resident_id, req.permission_group_id
            );
            rt.log_audit_event(
                &req.actor_id,
                "admin:assign_permission_group",
                &target,
                None,
            );
        }
        result
    }) {
        Ok(Ok(resp)) => resp,
        Ok(Err(error)) => {
            return Response::from_string(serde_json::json!({"error": error}).to_string())
                .with_status_code(StatusCode(500))
                .with_optional_header(json_header());
        }
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&resp).unwrap_or_else(|_| r#"{"ok":false}"#.into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_post_admin_scene(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &mut Request,
) -> HttpResponse {
    if let Some(resp) = require_admin_auth(runtime, request) {
        return resp;
    }
    let mut body = Vec::new();
    if request.as_reader().read_to_end(&mut body).is_err() {
        return Response::from_string("{\"error\":\"read body failed\"}")
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }
    match serde_json::from_slice::<AdminUpdateSceneRequest>(&body) {
        Ok(req) => {
            if let Some(resp) = require_admin_actor_or_capability(
                runtime,
                request,
                req.actor_id.as_deref(),
                crate::CAP_ADMIN_SCENE,
            ) {
                return resp;
            }
            let result = match with_runtime(runtime, |rt| rt.admin_update_scene(req)) {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(response) => {
                    notifier.notify_changed();
                    Response::from_string(
                        serde_json::to_string(&response).unwrap_or_else(|_| "{\"ok\":true}".into()),
                    )
                    .with_status_code(StatusCode(200))
                    .with_optional_header(json_header())
                }
                Err(message) => Response::from_string(
                    serde_json::to_string(&WakuGatewayResponse::Error { message })
                        .unwrap_or_else(|_| "{\"error\":true}".into()),
                )
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header()),
            }
        }
        Err(e) => Response::from_string(format!("{{\"error\":\"decode failed: {e}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
    }
}
