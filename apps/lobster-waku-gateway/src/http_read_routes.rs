use std::{
    collections::HashMap,
    io::Cursor,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use host_adapter::default_mobile_web_bootstrap;
use tiny_http::{Request, Response, StatusCode};
use transport_waku::WakuGatewayResponse;

use crate::{
    AdminModerationStatusResponse, ConversationId, GatewayRuntime, GatewayStateNotifier,
    IdentityId, capability_catalog,
    http_support::{
        ResponseHeaderExt, cli_missing_for_body, json_header, no_cache_header, parse_bool,
        parse_cli_address, parse_export_format, sse_header,
    },
    http_write_routes::{require_authenticated_actor, require_cli_sender_auth},
};

pub(crate) type HttpResponse = Response<Cursor<Vec<u8>>>;

fn with_runtime<T>(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    action: impl FnOnce(&mut GatewayRuntime) -> T,
) -> Result<T, HttpResponse> {
    match runtime.lock() {
        Ok(mut runtime) => Ok(action(&mut runtime)),
        Err(poisoned) => Ok(action(&mut poisoned.into_inner())),
    }
}

pub(crate) fn handle_get_provider(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let provider = match with_runtime(runtime, |runtime| runtime.provider_status()) {
        Ok(provider) => provider,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&provider).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_shell_bootstrap(listen_addr: &str) -> HttpResponse {
    let mut bootstrap = default_mobile_web_bootstrap();
    bootstrap.gateway_base_url = Some(format!("http://{listen_addr}"));
    Response::from_string(serde_json::to_string(&bootstrap).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_shell_state(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &Request,
    query_params: &HashMap<String, String>,
) -> HttpResponse {
    if let Some(raw_resident_id) = query_params
        .get("resident_id")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        && let Some(response) = require_authenticated_actor(runtime, request, raw_resident_id)
    {
        return response;
    }
    let resident_id = query_params
        .get("resident_id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| IdentityId(value.to_string()));
    let shell_state = match with_runtime(runtime, |runtime| {
        runtime.shell_state_for_viewer(resident_id.as_ref())
    }) {
        Ok(shell_state) => shell_state,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&shell_state).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_shell_events(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    request: &Request,
    query_params: &HashMap<String, String>,
) -> HttpResponse {
    if let Some(raw_resident_id) = query_params
        .get("resident_id")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        && let Some(response) = require_authenticated_actor(runtime, request, raw_resident_id)
    {
        return response;
    }
    let resident_id = query_params
        .get("resident_id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| IdentityId(value.to_string()));
    let after_version = query_params
        .get("after")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let wait_ms = parse_sse_wait_ms(query_params.get("wait_ms").map(String::as_str));
    let deadline = Instant::now() + Duration::from_millis(wait_ms);
    let shell_state = loop {
        let observed_generation = notifier.generation();
        let state = match with_runtime(runtime, |runtime| {
            runtime.shell_state_for_viewer(resident_id.as_ref())
        }) {
            Ok(state) => state,
            Err(response) => return response,
        };
        let has_new_state = after_version
            .map(|version| state.state_version != version)
            .unwrap_or(true);
        if has_new_state || wait_ms == 0 || Instant::now() >= deadline {
            break state;
        }
        notifier.wait_until_changed_since(observed_generation, deadline);
    };
    let data = serde_json::to_string(&shell_state).unwrap_or_else(|_| "{}".into());
    let heartbeat = serde_json::json!({
        "now_ms": GatewayRuntime::now_ms(),
        "resident_id": resident_id.as_ref().map(|item| item.0.as_str()),
    });
    let body = format!(
        "retry: 4000\n\
         event: shell-state\n\
         data: {data}\n\n\
         event: shell-heartbeat\n\
         data: {heartbeat}\n\n"
    );
    Response::from_string(body)
        .with_status_code(StatusCode(200))
        .with_optional_header(sse_header())
        .with_optional_header(no_cache_header())
}

fn parse_sse_wait_ms(raw: Option<&str>) -> u64 {
    raw.and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0)
        .min(5_000)
}

pub(crate) fn handle_get_world(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let snapshot = match with_runtime(runtime, |runtime| {
        runtime
            .federation_read_plan()
            .federated_governance_snapshot()
    }) {
        Ok(snapshot) => snapshot,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_cities(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let cities = match with_runtime(runtime, |runtime| {
        runtime
            .federation_read_plan()
            .federated_governance_snapshot()
            .cities
    }) {
        Ok(cities) => cities,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&cities).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_residents(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    query_params: &HashMap<String, String>,
) -> HttpResponse {
    let search = query_params
        .get("q")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase());
    let viewer = query_params
        .get("resident_id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| IdentityId(value.to_string()));
    let residents = match with_runtime(runtime, |runtime| {
        runtime.enrich_resident_directory_for_viewer(viewer.as_ref())
    }) {
        Ok(residents) => residents,
        Err(response) => return response,
    };
    let filtered: Vec<_> = if let Some(search) = search {
        residents
            .into_iter()
            .filter(|entry| {
                entry.resident_id.to_lowercase().contains(&search)
                    || entry
                        .active_cities
                        .iter()
                        .any(|city| city.to_lowercase().contains(&search))
                    || entry
                        .roles
                        .iter()
                        .any(|role| role.to_lowercase().contains(&search))
            })
            .collect()
    } else {
        residents
    };
    Response::from_string(serde_json::to_string(&filtered).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_world_square(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let notices = match with_runtime(runtime, |runtime| {
        runtime
            .federation_read_plan()
            .federated_governance_snapshot()
            .world_square_notices
    }) {
        Ok(notices) => notices,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&notices).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_world_safety(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let snapshot = match with_runtime(runtime, |runtime| {
        runtime.federation_read_plan().world_safety_snapshot()
    }) {
        Ok(snapshot) => snapshot,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_world_safety_reports(
    runtime: &Arc<Mutex<GatewayRuntime>>,
) -> HttpResponse {
    let reports = match with_runtime(runtime, |runtime| {
        runtime
            .federation_read_plan()
            .world_safety_snapshot()
            .reports
    }) {
        Ok(reports) => reports,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&reports).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_world_safety_residents(
    runtime: &Arc<Mutex<GatewayRuntime>>,
) -> HttpResponse {
    let snapshot = match with_runtime(runtime, |runtime| {
        runtime.federation_read_plan().world_safety_snapshot()
    }) {
        Ok(snapshot) => snapshot,
        Err(response) => return response,
    };
    Response::from_string(
        serde_json::to_string(&serde_json::json!({
            "resident_sanctions": snapshot.resident_sanctions,
            "registration_blacklist": snapshot.registration_blacklist,
        }))
        .unwrap_or_else(|_| "{}".into()),
    )
    .with_status_code(StatusCode(200))
    .with_optional_header(json_header())
}

pub(crate) fn handle_get_world_directory(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let snapshot = match with_runtime(runtime, |runtime| {
        runtime.federation_read_plan().world_directory_snapshot()
    }) {
        Ok(snapshot) => snapshot,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_world_entry(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let entry = match with_runtime(runtime, |runtime| runtime.world_entry_state()) {
        Ok(entry) => entry,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&entry).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_world_snapshot(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let snapshot = match with_runtime(runtime, |runtime| {
        runtime.federation_read_plan().world_snapshot_bundle()
    }) {
        Ok(snapshot) => snapshot,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_world_mirrors(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let mirrors = match with_runtime(runtime, |runtime| {
        runtime.federation_read_plan().world_directory_mirrors()
    }) {
        Ok(mirrors) => mirrors,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&mirrors).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_world_mirror_sources(
    runtime: &Arc<Mutex<GatewayRuntime>>,
) -> HttpResponse {
    let mirror_sources = match with_runtime(runtime, |runtime| {
        runtime
            .federation_read_plan()
            .world_mirror_source_statuses()
    }) {
        Ok(mirror_sources) => mirror_sources,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&mirror_sources).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_export(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &Request,
    query_params: &HashMap<String, String>,
) -> HttpResponse {
    let resident_id = match query_params
        .get("resident_id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        Some(resident_id) => resident_id.to_string(),
        None => {
            return Response::from_string(
                serde_json::to_string(&WakuGatewayResponse::Error {
                    message: "resident_id query parameter required".into(),
                })
                .unwrap_or_else(|_| "{\"error\":true}".into()),
            )
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
        }
    };

    if let Some(response) = require_authenticated_actor(runtime, request, &resident_id) {
        return response;
    }

    let conversation_id = query_params.get("conversation_id").map(String::as_str);
    let format = parse_export_format(query_params.get("format").map(String::as_str));
    let include_public = parse_bool(query_params.get("include_public").map(String::as_str));
    let result = match with_runtime(runtime, |runtime| {
        runtime.export_history(
            IdentityId(resident_id),
            conversation_id,
            format,
            include_public,
        )
    }) {
        Ok(result) => result,
        Err(response) => return response,
    };
    match result {
        Ok(exported) => {
            Response::from_string(serde_json::to_string(&exported).unwrap_or_else(|_| "{}".into()))
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

pub(crate) fn handle_get_admin_summary(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let summary = match with_runtime(runtime, |runtime| runtime.admin_summary()) {
        Ok(summary) => summary,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&summary).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_admin_conversations(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let conversations = match with_runtime(runtime, |runtime| runtime.admin_conversations()) {
        Ok(conversations) => conversations,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&conversations).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_admin_messages(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    query_params: &HashMap<String, String>,
) -> HttpResponse {
    let conversation_id = match query_params.get("conversation_id") {
        Some(id) if !id.trim().is_empty() => ConversationId(id.clone()),
        _ => {
            return Response::from_string(
                serde_json::to_string(&serde_json::json!({
                    "error": "conversation_id query parameter required"
                }))
                .unwrap_or_else(|_| "{}".into()),
            )
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
        }
    };
    let limit = query_params
        .get("limit")
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(50)
        .min(200);
    let audit = match with_runtime(runtime, |runtime| {
        runtime.admin_message_audit(&conversation_id, limit)
    }) {
        Ok(audit) => audit,
        Err(response) => return response,
    };
    match audit {
        Some(audit) => {
            Response::from_string(serde_json::to_string(&audit).unwrap_or_else(|_| "{}".into()))
                .with_status_code(StatusCode(200))
                .with_optional_header(json_header())
        }
        None => Response::from_string("{\"messages\":[],\"total_count\":0,\"returned_count\":0}")
            .with_status_code(StatusCode(200))
            .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_get_admin_residents(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let residents = match with_runtime(runtime, |runtime| runtime.admin_residents()) {
        Ok(residents) => residents,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&residents).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_admin_rooms(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let rooms = match with_runtime(runtime, |runtime| runtime.admin_rooms_detail()) {
        Ok(rooms) => rooms,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&rooms).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_admin_config(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let config = match with_runtime(runtime, |runtime| runtime.admin_get_config()) {
        Ok(config) => config,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&config).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_admin_logs(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let logs = match with_runtime(runtime, |runtime| runtime.admin_logs()) {
        Ok(logs) => logs,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&logs).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_admin_messages_moderation(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    query_params: &HashMap<String, String>,
) -> HttpResponse {
    let message_id = match query_params.get("message_id") {
        Some(id) if !id.trim().is_empty() => id.clone(),
        _ => {
            return Response::from_string(r#"{"error":"message_id query parameter required"}"#)
                .with_status_code(StatusCode(400))
                .with_optional_header(json_header());
        }
    };
    let status = match with_runtime(runtime, |runtime| {
        runtime
            .admin_message_moderation_status(&message_id)
            .map(|s| s.to_string())
    }) {
        Ok(status) => status,
        Err(response) => return response,
    };
    let resp = AdminModerationStatusResponse { message_id, status };
    Response::from_string(serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_message_search(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &Request,
    query_params: &HashMap<String, String>,
) -> HttpResponse {
    let resident_id = match query_params
        .get("resident_id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        Some(resident_id) => resident_id.to_string(),
        None => {
            return Response::from_string(
                serde_json::to_string(&WakuGatewayResponse::Error {
                    message: "resident_id query parameter required".into(),
                })
                .unwrap_or_else(|_| "{\"error\":true}".into()),
            )
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
        }
    };
    if let Some(response) = require_authenticated_actor(runtime, request, &resident_id) {
        return response;
    }
    let q = match query_params.get("q") {
        Some(q) if !q.trim().is_empty() => q.trim().to_string(),
        _ => {
            return Response::from_string(
                serde_json::to_string(&WakuGatewayResponse::Error {
                    message: "q query parameter required".into(),
                })
                .unwrap_or_else(|_| "{\"error\":true}".into()),
            )
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
        }
    };
    let room_id = query_params
        .get("room_id")
        .map(|v| v.trim())
        .filter(|v| !v.is_empty());
    let limit = query_params
        .get("limit")
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(50)
        .min(200);
    let results = match with_runtime(runtime, |runtime| {
        runtime.search_messages_for_viewer(&IdentityId(resident_id), &q, room_id, limit)
    }) {
        Ok(results) => results,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&results).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_admin_invites(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let invites = match with_runtime(runtime, |runtime| runtime.admin_list_invites()) {
        Ok(invites) => invites,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&invites).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_permission_groups(runtime: &Arc<Mutex<GatewayRuntime>>) -> HttpResponse {
    let groups = match with_runtime(runtime, |runtime| runtime.admin_list_permission_groups()) {
        Ok(groups) => groups,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&groups).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_capability_catalog() -> HttpResponse {
    let catalog = capability_catalog();
    Response::from_string(serde_json::to_string(&catalog).unwrap_or_else(|_| "[]".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_audit_log(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    query_params: &HashMap<String, String>,
) -> HttpResponse {
    let limit = query_params
        .get("limit")
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(100)
        .min(500);
    let response = match with_runtime(runtime, |runtime| runtime.admin_list_audit_events(limit)) {
        Ok(response) => response,
        Err(response) => return response,
    };
    Response::from_string(serde_json::to_string(&response).unwrap_or_else(|_| "{}".into()))
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

pub(crate) fn handle_get_cli_inbox(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &Request,
    query_params: &HashMap<String, String>,
) -> HttpResponse {
    if let Some(raw_for) = query_params.get("for") {
        if let Some(response) = require_cli_sender_auth(runtime, request, raw_for) {
            return response;
        }
        match parse_cli_address(raw_for) {
            Ok(viewer) => {
                let result = match with_runtime(runtime, |runtime| runtime.cli_inbox_for(&viewer)) {
                    Ok(result) => result,
                    Err(response) => return response,
                };
                match result {
                    Ok(response) => Response::from_string(
                        serde_json::to_string(&response)
                            .unwrap_or_else(|_| "{\"identity\":\"\"}".into()),
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
            Err(message) => Response::from_string(
                serde_json::to_string(&WakuGatewayResponse::Error { message })
                    .unwrap_or_else(|_| "{\"error\":true}".into()),
            )
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
        }
    } else {
        Response::from_string(cli_missing_for_body())
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header())
    }
}

pub(crate) fn handle_get_cli_rooms(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &Request,
    query_params: &HashMap<String, String>,
) -> HttpResponse {
    if let Some(raw_for) = query_params.get("for") {
        if let Some(response) = require_cli_sender_auth(runtime, request, raw_for) {
            return response;
        }
        match parse_cli_address(raw_for) {
            Ok(viewer) => {
                let result = match with_runtime(runtime, |runtime| runtime.cli_rooms_for(&viewer)) {
                    Ok(result) => result,
                    Err(response) => return response,
                };
                match result {
                    Ok(response) => Response::from_string(
                        serde_json::to_string(&response)
                            .unwrap_or_else(|_| "{\"identity\":\"\"}".into()),
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
            Err(message) => Response::from_string(
                serde_json::to_string(&WakuGatewayResponse::Error { message })
                    .unwrap_or_else(|_| "{\"error\":true}".into()),
            )
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
        }
    } else {
        Response::from_string(cli_missing_for_body())
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header())
    }
}

pub(crate) fn handle_get_cli_tail(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &Request,
    query_params: &HashMap<String, String>,
) -> HttpResponse {
    if let Some(raw_for) = query_params.get("for") {
        if let Some(response) = require_cli_sender_auth(runtime, request, raw_for) {
            return response;
        }
        match parse_cli_address(raw_for) {
            Ok(viewer) => {
                let conversation_id = query_params
                    .get("conversation_id")
                    .map(|value| ConversationId(value.clone()));
                let result = match with_runtime(runtime, |runtime| {
                    runtime.cli_tail_for(&viewer, conversation_id.as_ref())
                }) {
                    Ok(result) => result,
                    Err(response) => return response,
                };
                match result {
                    Ok(response) => Response::from_string(
                        serde_json::to_string(&response)
                            .unwrap_or_else(|_| "{\"identity\":\"\"}".into()),
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
            Err(message) => Response::from_string(
                serde_json::to_string(&WakuGatewayResponse::Error { message })
                    .unwrap_or_else(|_| "{\"error\":true}".into()),
            )
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header()),
        }
    } else {
        Response::from_string(cli_missing_for_body())
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header())
    }
}

pub(crate) fn handle_get_cli_search(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &Request,
    query_params: &HashMap<String, String>,
) -> HttpResponse {
    let Some(raw_for) = query_params.get("for") else {
        return Response::from_string(cli_missing_for_body())
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    };
    if let Some(response) = require_cli_sender_auth(runtime, request, raw_for) {
        return response;
    }
    let q = match query_params.get("q") {
        Some(q) if !q.trim().is_empty() => q.trim().to_string(),
        _ => {
            return Response::from_string(
                serde_json::to_string(&WakuGatewayResponse::Error {
                    message: "q query parameter required".into(),
                })
                .unwrap_or_else(|_| "{\"error\":true}".into()),
            )
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
        }
    };
    let room_id = query_params
        .get("room_id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| ConversationId(value.to_string()));
    let limit = query_params
        .get("limit")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(50)
        .min(200);
    match parse_cli_address(raw_for) {
        Ok(viewer) => {
            let result = match with_runtime(runtime, |runtime| {
                runtime.cli_search_for(&viewer, &q, room_id.as_ref(), limit)
            }) {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(response) => Response::from_string(
                    serde_json::to_string(&response).unwrap_or_else(|_| "[]".into()),
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
        Err(message) => Response::from_string(
            serde_json::to_string(&WakuGatewayResponse::Error { message })
                .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}
