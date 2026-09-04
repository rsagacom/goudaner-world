use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};

use tiny_http::{Method, Request, Response, StatusCode};

use crate::{
    GatewayRuntime, GatewayStateNotifier,
    http_attachment_routes::{handle_get_shell_attachment, handle_post_shell_attachment},
    http_auth_routes::{
        handle_get_auth_session, handle_post_auth_logout, handle_post_auth_preflight,
        handle_post_request_email_otp, handle_post_request_mobile_otp,
        handle_post_verify_email_otp, handle_post_verify_mobile_otp,
    },
    http_city_write_routes::{
        handle_post_approve_city_join, handle_post_create_city, handle_post_create_public_room,
        handle_post_freeze_public_room, handle_post_join_city,
        handle_post_update_federation_policy, handle_post_update_steward,
    },
    http_device_routes::{
        handle_get_admin_devices, handle_post_admin_add_device, handle_post_admin_block_device,
        handle_post_admin_remove_device, handle_post_admin_unblock_device,
    },
    http_governance_write_routes::{
        handle_post_publish_safety_advisory, handle_post_publish_world_notice,
        handle_post_review_safety_report, handle_post_sanction_resident,
        handle_post_submit_safety_report, handle_post_unsanction_resident,
        handle_post_update_city_trust,
    },
    http_push_routes::{
        handle_get_push_vapid_public_key, handle_post_push_subscribe, handle_post_push_unsubscribe,
    },
    http_read_routes::{
        handle_get_admin_config, handle_get_admin_conversations, handle_get_admin_invites,
        handle_get_admin_logs, handle_get_admin_messages, handle_get_admin_messages_moderation,
        handle_get_admin_residents, handle_get_admin_rooms, handle_get_admin_summary,
        handle_get_audit_log, handle_get_capability_catalog, handle_get_cities,
        handle_get_cli_inbox, handle_get_cli_rooms, handle_get_cli_search, handle_get_cli_tail,
        handle_get_export, handle_get_message_search, handle_get_permission_groups,
        handle_get_provider, handle_get_residents, handle_get_shell_bootstrap,
        handle_get_shell_events, handle_get_shell_state, handle_get_world,
        handle_get_world_directory, handle_get_world_entry, handle_get_world_mirror_sources,
        handle_get_world_mirrors, handle_get_world_safety, handle_get_world_safety_reports,
        handle_get_world_safety_residents, handle_get_world_snapshot, handle_get_world_square,
    },
    http_support::{ResponseHeaderExt, split_path_and_query, text_header},
    http_write_routes::{
        handle_post_admin_ban_resident, handle_post_admin_clear_processed_logs,
        handle_post_admin_config, handle_post_admin_create_invite,
        handle_post_admin_create_resident, handle_post_admin_freeze_room,
        handle_post_admin_handle_log, handle_post_admin_manage_room_member,
        handle_post_admin_moderate_message, handle_post_admin_revoke_invite,
        handle_post_admin_scene, handle_post_admin_set_nickname, handle_post_admin_unban_resident,
        handle_post_admin_unfreeze_room, handle_post_assign_permission_group, handle_post_cli_send,
        handle_post_create_permission_group, handle_post_direct_open, handle_post_personal_room,
        handle_post_personal_room_access_policy, handle_post_provider_connect,
        handle_post_provider_disconnect, handle_post_resident_relationship_accept,
        handle_post_resident_relationship_request, handle_post_scene_validate,
        handle_post_shell_mark_read, handle_post_shell_message, handle_post_shell_message_edit,
        handle_post_shell_message_recall, handle_post_shell_presence, handle_post_shell_scene,
        handle_post_shell_set_nickname, handle_post_waku, handle_post_world_mirror_sources,
        require_admin_auth,
    },
    release_info::handle_get_version,
};

pub(crate) type HttpResponse = Response<Cursor<Vec<u8>>>;

fn dispatch_admin_read(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &Request,
    handler: impl FnOnce() -> HttpResponse,
) -> HttpResponse {
    if let Some(response) = require_admin_auth(runtime, request) {
        return response;
    }
    handler()
}

fn dispatch_admin_write(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &mut Request,
    handler: impl FnOnce(&mut Request) -> HttpResponse,
) -> HttpResponse {
    if let Some(response) = require_admin_auth(runtime, request) {
        return response;
    }
    handler(request)
}

pub(crate) fn dispatch_http_request(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    notifier: &Arc<GatewayStateNotifier>,
    listen_addr: &str,
    request: &mut Request,
) -> HttpResponse {
    let method = request.method().clone();
    let url = request.url().to_string();
    let (path, query_params) = split_path_and_query(&url);

    match (method, path) {
        (Method::Options, _) => Response::from_string("")
            .with_status_code(StatusCode(204))
            .with_optional_header(text_header()),
        (Method::Get, "/health") | (Method::Head, "/health") => Response::from_string("ok")
            .with_status_code(StatusCode(200))
            .with_optional_header(text_header()),
        (Method::Get, "/v1/version") => handle_get_version(),
        (Method::Get, "/v1/admin/summary") => {
            dispatch_admin_read(runtime, request, || handle_get_admin_summary(runtime))
        }
        (Method::Get, "/v1/admin/conversations") => {
            dispatch_admin_read(runtime, request, || handle_get_admin_conversations(runtime))
        }
        (Method::Get, "/v1/admin/messages") => dispatch_admin_read(runtime, request, || {
            handle_get_admin_messages(runtime, &query_params)
        }),
        (Method::Get, "/v1/admin/residents") => {
            dispatch_admin_read(runtime, request, || handle_get_admin_residents(runtime))
        }
        (Method::Get, "/v1/admin/rooms") => {
            dispatch_admin_read(runtime, request, || handle_get_admin_rooms(runtime))
        }
        (Method::Get, "/v1/admin/config") => {
            dispatch_admin_read(runtime, request, || handle_get_admin_config(runtime))
        }
        (Method::Get, "/v1/admin/logs") => {
            dispatch_admin_read(runtime, request, || handle_get_admin_logs(runtime))
        }
        (Method::Get, "/v1/admin/messages/moderation") => {
            dispatch_admin_read(runtime, request, || {
                handle_get_admin_messages_moderation(runtime, &query_params)
            })
        }
        (Method::Get, "/v1/admin/invites") => {
            dispatch_admin_read(runtime, request, || handle_get_admin_invites(runtime))
        }
        (Method::Get, "/v1/admin/permission-groups") => {
            dispatch_admin_read(runtime, request, || handle_get_permission_groups(runtime))
        }
        (Method::Get, "/v1/admin/capabilities") => handle_get_capability_catalog(),
        (Method::Get, "/v1/admin/audit-log") => dispatch_admin_read(runtime, request, || {
            handle_get_audit_log(runtime, &query_params)
        }),
        (Method::Post, "/v1/admin/residents/ban") => {
            handle_post_admin_ban_resident(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/residents/unban") => {
            handle_post_admin_unban_resident(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/residents/nickname") => {
            handle_post_admin_set_nickname(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/rooms/freeze") => {
            handle_post_admin_freeze_room(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/rooms/unfreeze") => {
            handle_post_admin_unfreeze_room(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/config") => handle_post_admin_config(runtime, notifier, request),
        (Method::Post, "/v1/admin/messages/moderate") => {
            handle_post_admin_moderate_message(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/residents") => {
            handle_post_admin_create_resident(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/invites") => {
            handle_post_admin_create_invite(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/invites/revoke") => {
            handle_post_admin_revoke_invite(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/rooms/members") => {
            handle_post_admin_manage_room_member(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/logs/handle") => {
            handle_post_admin_handle_log(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/logs/clear") => {
            handle_post_admin_clear_processed_logs(runtime, notifier, request)
        }
        (Method::Get, "/v1/admin/devices") => {
            dispatch_admin_read(runtime, request, || handle_get_admin_devices(runtime))
        }
        (Method::Post, "/v1/admin/devices/add") => {
            handle_post_admin_add_device(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/devices/remove") => {
            handle_post_admin_remove_device(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/devices/block") => {
            handle_post_admin_block_device(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/devices/unblock") => {
            handle_post_admin_unblock_device(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/permission-groups") => {
            handle_post_create_permission_group(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/permission-groups/assign") => {
            handle_post_assign_permission_group(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/scene") => handle_post_admin_scene(runtime, notifier, request),
        (Method::Post, "/v1/shell/scene/validate") => {
            handle_post_scene_validate(runtime, notifier, request)
        }
        (Method::Get, "/v1/provider") => handle_get_provider(runtime),
        (Method::Post, "/v1/provider/connect") => {
            dispatch_admin_write(runtime, request, |request| {
                handle_post_provider_connect(runtime, request)
            })
        }
        (Method::Post, "/v1/provider/disconnect") => dispatch_admin_write(runtime, request, |_| {
            handle_post_provider_disconnect(runtime)
        }),
        (Method::Get, "/v1/shell/bootstrap") => handle_get_shell_bootstrap(listen_addr),
        (Method::Get, "/v1/shell/events") => {
            handle_get_shell_events(runtime, notifier, request, &query_params)
        }
        (Method::Get, "/v1/shell/state") => handle_get_shell_state(runtime, request, &query_params),
        (Method::Get, "/v1/shell/messages/search") => {
            handle_get_message_search(runtime, request, &query_params)
        }
        (Method::Get, path) if path.starts_with("/v1/shell/attachment/") => {
            handle_get_shell_attachment(runtime, path)
        }
        (Method::Get, "/v1/world") => handle_get_world(runtime),
        (Method::Get, "/v1/cities") => handle_get_cities(runtime),
        (Method::Get, "/v1/residents") => handle_get_residents(runtime, &query_params),
        (Method::Get, "/v1/auth/session") => handle_get_auth_session(runtime, request),
        (Method::Post, "/v1/auth/preflight") => handle_post_auth_preflight(runtime, request),
        (Method::Post, "/v1/auth/email-otp/request") => {
            handle_post_request_email_otp(runtime, request)
        }
        (Method::Post, "/v1/auth/email-otp/verify") => {
            handle_post_verify_email_otp(runtime, notifier, request)
        }
        (Method::Post, "/v1/auth/mobile-otp/request") => {
            handle_post_request_mobile_otp(runtime, request)
        }
        (Method::Post, "/v1/auth/mobile-otp/verify") => {
            handle_post_verify_mobile_otp(runtime, notifier, request)
        }
        (Method::Post, "/v1/auth/logout") => handle_post_auth_logout(runtime, request),
        (Method::Get, "/v1/export") => handle_get_export(runtime, request, &query_params),
        (Method::Get, "/v1/world-square") => handle_get_world_square(runtime),
        (Method::Get, "/v1/world-safety") => handle_get_world_safety(runtime),
        (Method::Get, "/v1/world-safety/reports") => handle_get_world_safety_reports(runtime),
        (Method::Get, "/v1/world-safety/residents") => handle_get_world_safety_residents(runtime),
        (Method::Get, "/v1/world-directory") => handle_get_world_directory(runtime),
        (Method::Get, "/v1/world-entry") => handle_get_world_entry(runtime),
        (Method::Get, "/v1/world-snapshot") => handle_get_world_snapshot(runtime),
        (Method::Get, "/v1/world-mirrors") => handle_get_world_mirrors(runtime),
        (Method::Get, "/v1/world-mirror-sources") => handle_get_world_mirror_sources(runtime),
        (Method::Post, "/v1/world-mirror-sources") => {
            dispatch_admin_write(runtime, request, |request| {
                handle_post_world_mirror_sources(runtime, request)
            })
        }
        (Method::Post, "/v1/shell/message") => {
            handle_post_shell_message(runtime, notifier, request)
        }
        (Method::Post, "/v1/shell/attachment") => handle_post_shell_attachment(runtime, request),
        (Method::Get, "/v1/push/vapid-public-key") => handle_get_push_vapid_public_key(runtime),
        (Method::Post, "/v1/push/subscribe") => handle_post_push_subscribe(runtime, request),
        (Method::Post, "/v1/push/unsubscribe") => handle_post_push_unsubscribe(runtime, request),
        (Method::Post, "/v1/shell/scene") => handle_post_shell_scene(runtime, notifier, request),
        (Method::Post, "/v1/shell/message/recall") => {
            handle_post_shell_message_recall(runtime, notifier, request)
        }
        (Method::Post, "/v1/shell/message/edit") => {
            handle_post_shell_message_edit(runtime, notifier, request)
        }
        (Method::Post, "/v1/cli/send") => handle_post_cli_send(runtime, notifier, request),
        (Method::Get, "/v1/cli/inbox") => handle_get_cli_inbox(runtime, request, &query_params),
        (Method::Get, "/v1/cli/rooms") => handle_get_cli_rooms(runtime, request, &query_params),
        (Method::Get, "/v1/cli/search") => handle_get_cli_search(runtime, request, &query_params),
        (Method::Get, "/v1/cli/tail") => handle_get_cli_tail(runtime, request, &query_params),
        (Method::Post, "/v1/direct/open") => handle_post_direct_open(runtime, notifier, request),
        (Method::Post, "/v1/personal-room") => {
            handle_post_personal_room(runtime, notifier, request)
        }
        (Method::Post, "/v1/personal-room/access-policy") => {
            handle_post_personal_room_access_policy(runtime, notifier, request)
        }
        (Method::Post, "/v1/resident-relationships/request") => {
            handle_post_resident_relationship_request(runtime, notifier, request)
        }
        (Method::Post, "/v1/resident-relationships/accept") => {
            handle_post_resident_relationship_accept(runtime, notifier, request)
        }
        (Method::Post, "/v1/cities") => handle_post_create_city(runtime, notifier, request),
        (Method::Post, "/v1/cities/join") => handle_post_join_city(runtime, notifier, request),
        (Method::Post, "/v1/cities/approve") => {
            handle_post_approve_city_join(runtime, notifier, request)
        }
        (Method::Post, "/v1/cities/stewards") => {
            handle_post_update_steward(runtime, notifier, request)
        }
        (Method::Post, "/v1/cities/federation-policy") => {
            handle_post_update_federation_policy(runtime, notifier, request)
        }
        (Method::Post, "/v1/cities/rooms") => {
            handle_post_create_public_room(runtime, notifier, request)
        }
        (Method::Post, "/v1/cities/rooms/freeze") => {
            handle_post_freeze_public_room(runtime, notifier, request)
        }
        (Method::Post, "/v1/world-square/notices") => {
            handle_post_publish_world_notice(runtime, notifier, request)
        }
        (Method::Post, "/v1/world-safety/cities/trust") => {
            handle_post_update_city_trust(runtime, notifier, request)
        }
        (Method::Post, "/v1/world-safety/reports") => {
            handle_post_submit_safety_report(runtime, notifier, request)
        }
        (Method::Post, "/v1/world-safety/reports/review") => {
            handle_post_review_safety_report(runtime, notifier, request)
        }
        (Method::Post, "/v1/world-safety/advisories") => {
            handle_post_publish_safety_advisory(runtime, notifier, request)
        }
        (Method::Post, "/v1/world-safety/residents/sanction") => {
            handle_post_sanction_resident(runtime, notifier, request)
        }
        (Method::Post, "/v1/admin/residents/unsanction") => {
            handle_post_unsanction_resident(runtime, notifier, request)
        }
        (Method::Post, "/v1/shell/presence") => {
            handle_post_shell_presence(runtime, notifier, request)
        }
        (Method::Post, "/v1/shell/read") => handle_post_shell_mark_read(runtime, notifier, request),
        (Method::Post, "/v1/shell/nickname") => {
            handle_post_shell_set_nickname(runtime, notifier, request)
        }
        (Method::Post, "/v1/waku") => handle_post_waku(runtime, request),
        _ => Response::from_string("not found")
            .with_status_code(StatusCode(404))
            .with_optional_header(text_header()),
    }
}
