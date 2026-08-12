use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use crate::gateway_test_support::{
    http_json, http_json_with_headers, http_raw, http_raw_with_headers, register_resident,
    sample_frame, sample_frame_with, start_local_gateway_http_server, start_mock_upstream_gateway,
};
use crate::http_auth_routes::handle_get_auth_session;
use crate::http_city_write_routes::handle_post_create_city;
use crate::http_device_routes::handle_get_admin_devices;
use crate::http_governance_write_routes::handle_post_publish_world_notice;
use crate::http_read_routes::{
    handle_get_provider, handle_get_world_entry, handle_get_world_square,
};
use crate::http_write_routes::handle_post_provider_disconnect;
use tempfile::tempdir;
use tiny_http::{Header, StatusCode, TestRequest};
use transport_waku::WakuGatewayClient;

use crate::email_otp_mailer::{EmailOtpDelivery, EmailOtpMailerConfig, deliver_email_otp};

fn poison_runtime_mutex(runtime: &Arc<Mutex<GatewayRuntime>>) {
    let poisoned_runtime = Arc::clone(runtime);
    let _ = thread::spawn(move || {
        let _guard = poisoned_runtime.lock().expect("lock runtime");
        panic!("poison gateway runtime mutex");
    })
    .join();
}

#[test]
fn runtime_publishes_and_polls_frames() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let frame = sample_frame("dm:test:poll");
    let topic = frame.content_topic.clone();

    let connected = runtime.handle(WakuGatewayRequest::Connect {
        endpoint: WakuEndpointConfig {
            peer_mode: WakuPeerMode::DesktopLight,
            relay_urls: vec!["http://127.0.0.1:8787".into()],
            use_filter: true,
            use_store: true,
            use_light_push: true,
        },
    });
    assert!(matches!(connected, WakuGatewayResponse::Connected));

    let subscribed = runtime.handle(WakuGatewayRequest::Subscribe {
        subscriptions: vec![TopicSubscription {
            content_topic: topic.clone(),
            recover_history: true,
        }],
    });
    assert!(matches!(subscribed, WakuGatewayResponse::Subscribed));

    let published = runtime.handle(WakuGatewayRequest::Publish { frame });
    assert!(matches!(published, WakuGatewayResponse::Published));

    let polled = runtime.handle(WakuGatewayRequest::Poll {
        subscriptions: vec![],
        limit: 16,
    });
    match polled {
        WakuGatewayResponse::Frames { frames } => assert_eq!(frames.len(), 1),
        other => panic!("expected frames response, got {other:?}"),
    }
}

#[test]
fn admin_devices_returns_500_when_runtime_lock_poisoned() {
    let temp = tempdir().expect("temp dir");
    let runtime = Arc::new(Mutex::new(
        GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime"),
    ));
    poison_runtime_mutex(&runtime);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle_get_admin_devices(&runtime)
    }));

    assert!(result.is_ok(), "device list route should not panic");
    assert_eq!(result.unwrap().status_code(), StatusCode(500));
}

#[test]
fn auth_session_returns_500_when_runtime_lock_poisoned() {
    let temp = tempdir().expect("temp dir");
    let runtime = Arc::new(Mutex::new(
        GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime"),
    ));
    poison_runtime_mutex(&runtime);
    let request: tiny_http::Request = TestRequest::new()
        .with_header(Header::from_bytes("Authorization", "Bearer test-token").expect("header"))
        .into();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle_get_auth_session(&runtime, &request)
    }));

    assert!(result.is_ok(), "auth session route should not panic");
    assert_eq!(result.unwrap().status_code(), StatusCode(500));
}

#[test]
fn create_city_returns_500_when_runtime_lock_poisoned() {
    let temp = tempdir().expect("temp dir");
    let runtime = Arc::new(Mutex::new(
        GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime"),
    ));
    poison_runtime_mutex(&runtime);
    let notifier = Arc::new(GatewayStateNotifier::new());
    let mut request: tiny_http::Request = TestRequest::new().with_body("{}").into();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle_post_create_city(&runtime, &notifier, &mut request)
    }));

    assert!(result.is_ok(), "create city route should not panic");
    assert_eq!(result.unwrap().status_code(), StatusCode(500));
}

#[test]
fn city_write_routes_do_not_depend_on_runtime_lock_expect() {
    let source = include_str!("http_city_write_routes.rs");

    assert!(
        !source.contains("gateway runtime mutex poisoned"),
        "city write routes should return JSON 500 when runtime lock is poisoned"
    );
}

#[test]
fn publish_world_notice_returns_500_when_runtime_lock_poisoned() {
    let temp = tempdir().expect("temp dir");
    let runtime = Arc::new(Mutex::new(
        GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime"),
    ));
    poison_runtime_mutex(&runtime);
    let notifier = Arc::new(GatewayStateNotifier::new());
    let mut request: tiny_http::Request = TestRequest::new()
        .with_body(
            r#"{"actor_id":"rsaga","title":"Mirror sync","body":"Maintenance window","severity":"info","tags":["world"]}"#,
        )
        .into();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle_post_publish_world_notice(&runtime, &notifier, &mut request)
    }));

    assert!(
        result.is_ok(),
        "publish world notice route should not panic"
    );
    assert_eq!(result.unwrap().status_code(), StatusCode(500));
}

#[test]
fn governance_write_routes_do_not_depend_on_runtime_lock_expect() {
    let source = include_str!("http_governance_write_routes.rs");

    assert!(
        !source.contains("gateway runtime mutex poisoned"),
        "governance write routes should return JSON 500 when runtime lock is poisoned"
    );
}

#[test]
fn provider_status_returns_500_when_runtime_lock_poisoned() {
    let temp = tempdir().expect("temp dir");
    let runtime = Arc::new(Mutex::new(
        GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime"),
    ));
    poison_runtime_mutex(&runtime);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle_get_provider(&runtime)
    }));

    assert!(result.is_ok(), "provider status route should not panic");
    assert_eq!(result.unwrap().status_code(), StatusCode(500));
}

#[test]
fn read_routes_do_not_depend_on_runtime_lock_expect() {
    let source = include_str!("http_read_routes.rs");

    assert!(
        !source.contains("gateway runtime mutex poisoned"),
        "read routes should return JSON 500 when runtime lock is poisoned"
    );
}

#[test]
fn provider_disconnect_returns_500_when_runtime_lock_poisoned() {
    let temp = tempdir().expect("temp dir");
    let runtime = Arc::new(Mutex::new(
        GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime"),
    ));
    poison_runtime_mutex(&runtime);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle_post_provider_disconnect(&runtime)
    }));

    assert!(result.is_ok(), "provider disconnect route should not panic");
    assert_eq!(result.unwrap().status_code(), StatusCode(500));
}

#[test]
fn write_routes_do_not_depend_on_runtime_lock_expect() {
    let source = include_str!("http_write_routes.rs");

    assert!(
        !source.contains("expect(\"gateway runtime mutex"),
        "write routes should return JSON 500 when runtime lock is poisoned"
    );
}

#[test]
fn write_routes_do_not_depend_on_actor_unwrap() {
    let source = include_str!("http_write_routes.rs");

    assert!(
        !source.contains("actor.unwrap()"),
        "write routes should handle missing admin actors without production unwrap"
    );
}

#[test]
fn core_runtime_now_ms_does_not_depend_on_system_time_expect() {
    let source = include_str!("core_runtime.rs");

    assert!(
        !source.contains("system time should be after unix epoch"),
        "gateway runtime time helper should not panic on system clock errors"
    );
}

#[test]
fn gateway_main_does_not_depend_on_runtime_lock_expect() {
    let source = include_str!("main.rs");

    assert!(
        !source.contains("expect(\"gateway runtime mutex"),
        "gateway main should not panic when runtime lock is poisoned"
    );
}

#[test]
fn gateway_notifier_recovers_from_poisoned_mutex() {
    let notifier = Arc::new(GatewayStateNotifier::new());
    let poisoned_notifier = Arc::clone(&notifier);
    let _ = thread::spawn(move || {
        let _guard = poisoned_notifier
            .generation
            .lock()
            .expect("lock notifier generation");
        panic!("poison gateway notifier mutex");
    })
    .join();

    let generation =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| notifier.generation()));
    assert!(generation.is_ok(), "notifier generation should not panic");
    assert_eq!(generation.unwrap(), 0);

    let notify = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        notifier.notify_changed();
    }));
    assert!(notify.is_ok(), "notifier notify should not panic");
    assert_eq!(notifier.generation(), 1);

    let wait = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        notifier.wait_until_changed_since(1, Instant::now());
    }));
    assert!(wait.is_ok(), "notifier wait should not panic");
}

#[test]
fn gateway_notifier_does_not_depend_on_poison_expect() {
    let source = include_str!("main.rs");

    assert!(
        !source.contains("expect(\"gateway notifier"),
        "gateway notifier should recover from poisoned synchronization primitives"
    );
}

#[test]
fn waku_http_route_roundtrips_connect_subscribe_publish_and_poll_contract() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);
    let frame = sample_frame("dm:http:waku");
    let topic = frame.content_topic.clone();

    let (connect_status, connected) = http_json(
        "POST",
        &server.base_url,
        "/v1/waku",
        Some(
            &serde_json::to_value(WakuGatewayRequest::Connect {
                endpoint: WakuEndpointConfig {
                    peer_mode: WakuPeerMode::DesktopLight,
                    relay_urls: vec!["http://127.0.0.1:8787".into()],
                    use_filter: true,
                    use_store: true,
                    use_light_push: true,
                },
            })
            .expect("encode connect request"),
        ),
    );
    assert_eq!(connect_status, 200);
    assert_eq!(connected, serde_json::json!("Connected"));

    let (subscribe_status, subscribed) = http_json(
        "POST",
        &server.base_url,
        "/v1/waku",
        Some(
            &serde_json::to_value(WakuGatewayRequest::Subscribe {
                subscriptions: vec![TopicSubscription {
                    content_topic: topic.clone(),
                    recover_history: true,
                }],
            })
            .expect("encode subscribe request"),
        ),
    );
    assert_eq!(subscribe_status, 200);
    assert_eq!(subscribed, serde_json::json!("Subscribed"));

    let (publish_status, published) = http_json(
        "POST",
        &server.base_url,
        "/v1/waku",
        Some(
            &serde_json::to_value(WakuGatewayRequest::Publish {
                frame: frame.clone(),
            })
            .expect("encode publish request"),
        ),
    );
    assert_eq!(publish_status, 200);
    assert_eq!(published, serde_json::json!("Published"));

    let (poll_status, poll) = http_json(
        "POST",
        &server.base_url,
        "/v1/waku",
        Some(
            &serde_json::to_value(WakuGatewayRequest::Poll {
                subscriptions: vec![TopicSubscription {
                    content_topic: topic,
                    recover_history: false,
                }],
                limit: 10,
            })
            .expect("encode poll request"),
        ),
    );
    assert_eq!(poll_status, 200);
    assert_eq!(
        poll["Frames"]["frames"][0]["content_topic"],
        frame.content_topic
    );
    assert!(
        !poll["Frames"]["frames"][0]["payload"]
            .as_array()
            .expect("encoded frame payload")
            .is_empty()
    );
}

#[test]
fn waku_http_route_requires_dedicated_federation_bearer_in_production_mode() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    runtime.set_federation_token_for_tests("federation-test-token");
    let server = start_local_gateway_http_server(runtime);
    let request = serde_json::to_value(WakuGatewayRequest::Connect {
        endpoint: WakuEndpointConfig {
            peer_mode: WakuPeerMode::DesktopLight,
            relay_urls: vec![server.base_url.clone()],
            use_filter: true,
            use_store: true,
            use_light_push: true,
        },
    })
    .expect("encode connect request");

    let (missing_status, missing) = http_json("POST", &server.base_url, "/v1/waku", Some(&request));
    assert_eq!(missing_status, 401);
    assert!(
        missing["Error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("federation Bearer token"))
    );

    let (invalid_status, _) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/waku",
        &[("Authorization", "Bearer wrong-token")],
        Some(&request),
    );
    assert_eq!(invalid_status, 401);

    let (valid_status, valid) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/waku",
        &[("Authorization", "Bearer federation-test-token")],
        Some(&request),
    );
    assert_eq!(valid_status, 200);
    assert_eq!(valid, serde_json::json!("Connected"));
}

#[test]
fn http_waku_gateway_client_sends_federation_bearer_without_exposing_it_in_debug() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    runtime.set_federation_token_for_tests("federation-client-token");
    let server = start_local_gateway_http_server(runtime);
    let mut client = HttpWakuGatewayClient::with_bearer_token(
        server.base_url.clone(),
        "federation-client-token",
    );

    assert!(!format!("{client:?}").contains("federation-client-token"));
    client
        .connect_gateway(&WakuEndpointConfig {
            peer_mode: WakuPeerMode::DesktopLight,
            relay_urls: vec![server.base_url.clone()],
            use_filter: true,
            use_store: true,
            use_light_push: true,
        })
        .expect("authenticated gateway connect");
}

#[test]
fn http_boundary_routes_return_health_cors_and_not_found_contract() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (health_status, health_headers, health_body) =
        http_raw("GET", &server.base_url, "/health", None);
    assert_eq!(health_status, 200);
    assert!(health_headers.contains("Content-Type: text/plain; charset=utf-8"));
    assert!(health_headers.contains("Access-Control-Allow-Origin: *"));
    assert_eq!(health_body, "ok");

    let (version_status, version) = http_json("GET", &server.base_url, "/v1/version", None);
    assert_eq!(version_status, 200);
    assert_eq!(version["schema_version"], 1);
    assert_eq!(version["package_version"], env!("CARGO_PKG_VERSION"));
    assert!(
        version["git_sha"]
            .as_str()
            .is_some_and(|value| value.len() == 40)
    );

    let (head_status, head_headers, _head_body) =
        http_raw("HEAD", &server.base_url, "/health", None);
    assert_eq!(head_status, 200);
    assert!(head_headers.contains("Access-Control-Allow-Origin: *"));

    let (options_status, options_headers, options_body) =
        http_raw("OPTIONS", &server.base_url, "/v1/shell/message", None);
    assert_eq!(options_status, 204);
    assert!(options_headers.contains("Access-Control-Allow-Origin: *"));
    assert!(options_headers.contains("Access-Control-Allow-Methods: GET, POST, OPTIONS"));
    assert!(options_headers.contains("Access-Control-Allow-Headers: Content-Type"));
    assert!(options_body.is_empty());

    let (missing_status, missing_headers, missing_body) =
        http_raw("GET", &server.base_url, "/v1/does-not-exist", None);
    assert_eq!(missing_status, 404);
    assert!(missing_headers.contains("Content-Type: text/plain; charset=utf-8"));
    assert!(missing_headers.contains("Access-Control-Allow-Origin: *"));
    assert_eq!(missing_body, "not found");
}

#[test]
fn resident_scoped_shell_state_requires_matching_bearer_session() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    register_resident(&mut runtime, "alice");
    register_resident(&mut runtime, "bob");
    let alice = IdentityId("alice".into());
    let (alice_token, _) = runtime.issue_auth_session(
        &alice,
        "test-scoped-shell-state-alice",
        GatewayRuntime::now_ms(),
    );
    let server = start_local_gateway_http_server(runtime);
    let auth_header = format!("Bearer {alice_token}");

    let (missing_status, _) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=alice",
        None,
    );
    assert_eq!(missing_status, 401);

    let (matching_status, _) = http_json_with_headers(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=alice",
        &[("Authorization", auth_header.as_str())],
        None,
    );
    assert_eq!(matching_status, 200);

    let (mismatched_status, _) = http_json_with_headers(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=bob",
        &[("Authorization", auth_header.as_str())],
        None,
    );
    assert_eq!(mismatched_status, 401);

    let (missing_events_status, _, _) = http_raw(
        "GET",
        &server.base_url,
        "/v1/shell/events?resident_id=alice&wait_ms=0",
        None,
    );
    assert_eq!(missing_events_status, 401);
    let (matching_events_status, _, matching_events_body) = http_raw_with_headers(
        "GET",
        &server.base_url,
        "/v1/shell/events?resident_id=alice&wait_ms=0",
        &[("Authorization", auth_header.as_str())],
        None,
    );
    assert_eq!(matching_events_status, 200);
    assert!(matching_events_body.contains("event: shell-state"));
}

#[test]
fn shell_events_route_returns_sse_shell_state_snapshot() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (status, headers, body) = http_raw(
        "GET",
        &server.base_url,
        "/v1/shell/events?resident_id=qa-a",
        None,
    );

    assert_eq!(status, 200);
    assert!(headers.contains("Content-Type: text/event-stream; charset=utf-8"));
    assert!(headers.contains("Cache-Control: no-cache"));
    assert!(headers.contains("Access-Control-Allow-Origin: *"));
    assert!(body.starts_with("retry: 4000\n"));
    assert!(body.contains("event: shell-state\ndata: "));
    assert!(body.contains("\n\nevent: shell-heartbeat\ndata: "));
    assert!(body.ends_with("\n\n"));

    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert!(payload["rooms"].is_array());
    assert!(payload["conversation_shell"]["conversations"].is_array());
    assert!(payload["scene_render"]["scenes"].is_array());

    let heartbeat_data = body
        .split("event: shell-heartbeat\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse heartbeat payload");
    let heartbeat: serde_json::Value =
        serde_json::from_str(heartbeat_data).expect("heartbeat json");
    assert_eq!(heartbeat["resident_id"], "qa-a");
    assert!(heartbeat["now_ms"].as_i64().unwrap_or_default() > 0);
}

#[test]
fn shell_state_version_changes_after_message_append() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version");

    let (sent_status, _sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "version bump",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);

    let (updated_status, updated_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(updated_status, 200);
    let updated_version = updated_state["state_version"]
        .as_str()
        .expect("updated state version");
    assert_ne!(updated_version, initial_version);
}

#[test]
fn shell_events_can_wait_until_state_version_changes() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version")
        .to_string();

    let events_base_url = server.base_url.clone();
    let events_path =
        format!("/v1/shell/events?resident_id=qa-a&after={initial_version}&wait_ms=1000");
    let events_thread =
        thread::spawn(move || http_raw("GET", &events_base_url, &events_path, None));

    thread::sleep(Duration::from_millis(100));
    let (sent_status, _sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "wake waiting events",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);

    let (events_status, _headers, body) = events_thread.join().expect("events thread");
    assert_eq!(events_status, 200);
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    let updated_version = payload["state_version"]
        .as_str()
        .expect("updated state version");
    assert_ne!(updated_version, initial_version);
}

#[test]
fn shell_events_wait_returns_current_snapshot_when_state_is_unchanged() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version");

    let (events_status, headers, body) = http_raw(
        "GET",
        &server.base_url,
        &format!("/v1/shell/events?resident_id=qa-a&after={initial_version}&wait_ms=10"),
        None,
    );
    assert_eq!(events_status, 200);
    assert!(headers.contains("Content-Type: text/event-stream; charset=utf-8"));

    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert_eq!(payload["state_version"], initial_version);
    assert!(body.contains("\n\nevent: shell-heartbeat\ndata: "));
}

#[test]
fn shell_events_wait_ms_zero_returns_immediately_without_waiting() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version");

    let start = std::time::Instant::now();
    let (events_status, _headers, body) = http_raw(
        "GET",
        &server.base_url,
        &format!("/v1/shell/events?resident_id=qa-a&after={initial_version}&wait_ms=0"),
        None,
    );
    let elapsed = start.elapsed();

    assert_eq!(events_status, 200);
    assert!(
        elapsed < Duration::from_millis(100),
        "wait_ms=0 should return immediately, took {:?}",
        elapsed
    );
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert_eq!(payload["state_version"], initial_version);
}

#[test]
fn shell_events_missing_wait_ms_defaults_to_zero() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let start = std::time::Instant::now();
    let (status, _headers, _body) = http_raw(
        "GET",
        &server.base_url,
        "/v1/shell/events?resident_id=qa-a",
        None,
    );
    let elapsed = start.elapsed();

    assert_eq!(status, 200);
    assert!(
        elapsed < Duration::from_millis(100),
        "missing wait_ms should default to 0 and return immediately, took {:?}",
        elapsed
    );
}

#[test]
fn shell_events_wait_ms_capped_at_5000() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version");

    let start = std::time::Instant::now();
    let (events_status, _headers, _body) = http_raw(
        "GET",
        &server.base_url,
        &format!("/v1/shell/events?resident_id=qa-a&after={initial_version}&wait_ms=99999"),
        None,
    );
    let elapsed = start.elapsed();

    assert_eq!(events_status, 200);
    assert!(
        elapsed < Duration::from_millis(5500),
        "wait_ms should be capped at 5000ms, took {:?}",
        elapsed
    );
    assert!(
        elapsed >= Duration::from_millis(100),
        "should have waited at least a bit, took {:?}",
        elapsed
    );
}

#[test]
fn shell_events_invalid_wait_ms_defaults_to_zero() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let start = std::time::Instant::now();
    let (status, _headers, _body) = http_raw(
        "GET",
        &server.base_url,
        "/v1/shell/events?resident_id=qa-a&wait_ms=abc",
        None,
    );
    let elapsed = start.elapsed();

    assert_eq!(status, 200);
    assert!(
        elapsed < Duration::from_millis(100),
        "invalid wait_ms should default to 0, took {:?}",
        elapsed
    );
}

#[test]
fn shell_events_negative_wait_ms_defaults_to_zero() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let start = std::time::Instant::now();
    let (status, _headers, _body) = http_raw(
        "GET",
        &server.base_url,
        "/v1/shell/events?resident_id=qa-a&wait_ms=-100",
        None,
    );
    let elapsed = start.elapsed();

    assert_eq!(status, 200);
    assert!(
        elapsed < Duration::from_millis(100),
        "negative wait_ms should default to 0, took {:?}",
        elapsed
    );
}

#[test]
fn shell_events_missing_after_returns_current_state_immediately() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let start = std::time::Instant::now();
    let (status, _headers, body) = http_raw(
        "GET",
        &server.base_url,
        "/v1/shell/events?resident_id=qa-a&wait_ms=5000",
        None,
    );
    let elapsed = start.elapsed();

    assert_eq!(status, 200);
    assert!(
        elapsed < Duration::from_millis(100),
        "missing after should return immediately, took {:?}",
        elapsed
    );
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert!(payload["state_version"].is_string());
}

#[test]
fn resubscribe_with_recover_history_resets_gateway_cursor() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let first = sample_frame_with(
        "room:test:restart-cursor",
        "msg-reset-1",
        "hello reset one",
        1_763_560_000_100,
    );
    let second = sample_frame_with(
        "room:test:restart-cursor",
        "msg-reset-2",
        "hello reset two",
        1_763_560_000_200,
    );
    let topic = first.content_topic.clone();

    let connected = runtime.handle(WakuGatewayRequest::Connect {
        endpoint: WakuEndpointConfig {
            peer_mode: WakuPeerMode::DesktopLight,
            relay_urls: vec!["http://127.0.0.1:8787".into()],
            use_filter: true,
            use_store: true,
            use_light_push: true,
        },
    });
    assert!(matches!(connected, WakuGatewayResponse::Connected));

    let subscribe = || WakuGatewayRequest::Subscribe {
        subscriptions: vec![TopicSubscription {
            content_topic: topic.clone(),
            recover_history: true,
        }],
    };

    assert!(matches!(
        runtime.handle(subscribe()),
        WakuGatewayResponse::Subscribed
    ));
    assert!(matches!(
        runtime.handle(WakuGatewayRequest::Publish { frame: first }),
        WakuGatewayResponse::Published
    ));
    match runtime.handle(WakuGatewayRequest::Poll {
        subscriptions: vec![],
        limit: 16,
    }) {
        WakuGatewayResponse::Frames { frames } => assert_eq!(frames.len(), 1),
        other => panic!("expected frames response, got {other:?}"),
    }

    assert!(matches!(
        runtime.handle(WakuGatewayRequest::Publish { frame: second }),
        WakuGatewayResponse::Published
    ));

    assert!(matches!(
        runtime.handle(subscribe()),
        WakuGatewayResponse::Subscribed
    ));
    assert_eq!(
        runtime.cursors.get(&topic),
        Some(&WakuSyncCursor::default())
    );
    assert_eq!(
        transport_waku::WakuGatewayClient::recover_frames(
            &runtime.node,
            &topic,
            &WakuSyncCursor::default(),
            16,
        )
        .expect("recover frames after reset")
        .len(),
        2
    );

    match runtime.handle(WakuGatewayRequest::Poll {
        subscriptions: vec![],
        limit: 16,
    }) {
        WakuGatewayResponse::Frames { frames } => assert_eq!(frames.len(), 2),
        other => panic!("expected frames response, got {other:?}"),
    }
}

#[test]
fn shell_messages_publish_to_upstream_provider() {
    let temp = tempdir().expect("temp dir");
    let (base_url, state, running, handle) = start_mock_upstream_gateway();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut runtime =
            GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
        let provider = runtime
            .connect_provider(ConnectProviderRequest {
                provider_url: base_url.clone(),
            })
            .expect("connect provider");
        assert_eq!(provider.mode, "remote-gateway");
        assert!(provider.reachable);

        let connected = runtime.handle(WakuGatewayRequest::Connect {
            endpoint: WakuEndpointConfig {
                peer_mode: WakuPeerMode::DesktopLight,
                relay_urls: vec!["http://127.0.0.1:8787".into()],
                use_filter: true,
                use_store: true,
                use_light_push: true,
            },
        });
        assert!(matches!(connected, WakuGatewayResponse::Connected));

        runtime
            .append_shell_message(ShellMessageRequest {
                room_id: "room:world:lobby".into(),
                sender: "rsaga".into(),
                text: "hello upstream".into(),
                reply_to_message_id: None,
                device_id: Some("browser".into()),
                language_tag: Some("zh-CN".into()),
            })
            .expect("append shell message");

        let upstream_frame = {
            let shared = state.lock().expect("lock mock upstream state");
            assert!(shared.healthcheck_count >= 1);
            assert_eq!(shared.connect_requests.len(), 1);
            shared
                .published_frames
                .last()
                .cloned()
                .expect("shell message published upstream")
        };
        let decoded = transport_waku::WakuFrameCodec::decode(&upstream_frame.payload)
            .expect("decode upstream frame");
        assert_eq!(decoded.body.plain_text, "hello upstream");

        let recovered = runtime.handle(WakuGatewayRequest::Recover {
            content_topic: upstream_frame.content_topic.clone(),
            cursor: WakuSyncCursor::default(),
            limit: 16,
        });
        match recovered {
            WakuGatewayResponse::Frames { frames } => {
                let hello_count = frames
                    .iter()
                    .filter_map(|frame| transport_waku::WakuFrameCodec::decode(&frame.payload).ok())
                    .filter(|message| message.body.plain_text == "hello upstream")
                    .count();
                assert_eq!(hello_count, 1);
            }
            other => panic!("expected frames response, got {other:?}"),
        }
    }));

    running.store(false, Ordering::SeqCst);
    handle.join().expect("stop mock upstream gateway");
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}

#[test]
fn shell_state_exposes_seeded_rooms() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let state = runtime.shell_state();
    assert!(!state.rooms.is_empty());
    assert!(state.rooms.iter().any(|room| room.id == "room:world:lobby"));
    assert_eq!(
        state.conversation_shell.active_conversation_id,
        state.rooms.first().map(|room| room.id.clone())
    );
    assert_eq!(
        state.conversation_shell.conversations.len(),
        state.rooms.len()
    );
    assert_eq!(state.scene_render.scenes.len(), state.rooms.len());
    assert!(
        state
            .scene_render
            .scenes
            .iter()
            .any(|scene| scene.conversation_id == "room:world:lobby")
    );
}

#[test]
fn shell_state_contract_exposes_detail_workflow_and_caretaker() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let state = runtime.shell_state();
    let json = serde_json::to_value(&state).expect("serialize shell state");

    let lobby = json["conversation_shell"]["conversations"]
        .as_array()
        .expect("conversation shell array")
        .iter()
        .find(|conversation| conversation["conversation_id"] == "room:world:lobby")
        .expect("lobby conversation");

    assert_eq!(lobby["kind"], "public");
    assert_eq!(lobby["scope"], "cross_city_shared");
    assert_eq!(lobby["title"], "世界广场");
    assert_eq!(lobby["participant_label"], "跨城共响回廊");
    assert_eq!(lobby["route_label"], "跨城共响线");
    assert_eq!(lobby["list_summary"], "世界广场 · 2 人 · 2 条消息");
    assert_eq!(lobby["status_line"], "跨城共响线 · 消息数：2");
    assert_eq!(lobby["thread_headline"], "跨城共响回廊 · 群聊");
    assert_eq!(lobby["chat_status_summary"], "群聊当前比较安静");
    assert_eq!(
        lobby["queue_summary"],
        "1 条访客提醒待处理 · 1 条巡视提醒待看"
    );
    assert_eq!(
        lobby["preview_text"],
        "Local gateway online. H5 shell can now poll and post through localhost."
    );
    assert_eq!(lobby["activity_time_label"], "5m ago");
    assert!(
        lobby["last_activity_label"]
            .as_str()
            .expect("last activity label")
            .starts_with("builder · ")
    );
    assert_eq!(lobby["overview_summary"], "跨城共响回廊 · 群聊");
    assert_eq!(
        lobby["context_summary"],
        "公共房间 · 公共频道、公告板与像素座位区"
    );
    assert_eq!(lobby["caretaker"]["name"], "巡逻犬");
    assert_eq!(lobby["detail_card"]["title"], "巡逻犬 / 频道状态");
    assert_eq!(lobby["workflow"]["action"], "委托");
    assert_eq!(lobby["workflow"]["state"], "待回执");
    assert_eq!(lobby["inline_actions"][0]["label"], "跟进委托");
    let search_terms = lobby["search_terms"]
        .as_array()
        .expect("search terms array")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
    assert!(search_terms.contains(&"跨城共响线"));
    assert!(search_terms.contains(&"当前委托正在等待第一轮回执。"));
    assert!(search_terms.contains(&"先确认需求是否被接住。"));
    assert!(search_terms.contains(&"巡逻犬 / 频道状态"));
    assert_eq!(lobby["messages"][0]["sender"], "system");

    let direct = json["conversation_shell"]["conversations"]
        .as_array()
        .expect("conversation shell array")
        .iter()
        .find(|conversation| conversation["conversation_id"] == "dm:builder:rsaga")
        .expect("direct conversation");
    assert_eq!(direct["kind"], "direct");
    assert_eq!(direct["scope"], "private");
    assert_eq!(direct["self_label"], "rsaga");
    assert_eq!(direct["peer_label"], "builder");
    assert_eq!(direct["title"], "正在与 builder 聊天");
    assert_eq!(direct["subtitle"], "居所直达 · 你与 builder");
    assert_eq!(direct["participant_label"], "你与 builder");
    assert_eq!(direct["thread_headline"], "正在与 builder 聊天");
    assert_eq!(direct["overview_summary"], "正在与 builder 聊天");
    assert_eq!(direct["detail_card"]["meta"][0]["label"], "住户");
    assert_eq!(direct["detail_card"]["meta"][0]["value"], "rsaga");
    assert_eq!(direct["detail_card"]["meta"][1]["label"], "对端");
    assert_eq!(direct["detail_card"]["meta"][1]["value"], "builder");
    assert_eq!(
        direct["detail_card"]["title"],
        "旺财 / 与 builder 的房内状态"
    );
    assert_eq!(
        direct["detail_card"]["summary_copy"],
        "旺财 会帮你记住与 builder 的留言和提醒，适合续聊、记任务和直接追问。"
    );
    let direct_search_terms = direct["search_terms"]
        .as_array()
        .expect("direct search terms array")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
    assert!(direct_search_terms.contains(&"builder"));
    assert!(direct_search_terms.contains(&"rsaga"));

    let lobby_scene = json["scene_render"]["scenes"]
        .as_array()
        .expect("scene render array")
        .iter()
        .find(|scene| scene["conversation_id"] == "room:world:lobby")
        .expect("lobby scene");

    assert_eq!(lobby_scene["scene_banner"], "世界广场");
    assert_eq!(lobby_scene["stage"]["title"], "世界广场");
    assert_eq!(lobby_scene["stage"]["badge"], "世界广场");
    assert_eq!(lobby_scene["portrait"]["title"], "巡逻犬");
    assert_eq!(lobby_scene["portrait"]["badge"], "频道巡视");
    assert_eq!(lobby_scene["portrait"]["status"], "在线巡视");
    assert_eq!(lobby_scene["portrait"]["monogram"], "巡");

    let direct_scene = json["scene_render"]["scenes"]
        .as_array()
        .expect("scene render array")
        .iter()
        .find(|scene| scene["conversation_id"] == "dm:builder:rsaga")
        .expect("direct scene");
    assert_eq!(direct_scene["scene_banner"], "个人房间");
    assert_eq!(direct_scene["image_layer"]["preset"], "private-room-loft");
    assert_eq!(
        direct_scene["hotspot_layer"]["coordinate_system"],
        "scene-permyriad"
    );
    assert_eq!(
        direct_scene["hotspot_layer"]["hotspots"][0]["label"],
        "工作台"
    );
    assert_eq!(direct_scene["stage"]["title"], "正在与 builder 聊天");
    assert_eq!(
        direct_scene["stage"]["summary"],
        "旺财 会帮你记住与 builder 的留言和提醒，适合续聊、记任务和直接追问。"
    );
}

#[test]
fn shell_state_contract_exposes_action_templates() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let state = runtime.shell_state();
    let json = serde_json::to_value(&state).expect("serialize shell state");

    let action_templates = json["conversation_shell"]["action_templates"]
        .as_array()
        .expect("conversation shell action_templates array");
    let entrust = action_templates
        .iter()
        .find(|item| item["action"] == "委托")
        .expect("委托 action template");

    assert_eq!(
        entrust["draft_template"],
        "委托：\n- 需求：\n- 截止：\n- 交付："
    );
    assert_eq!(entrust["send_label"], "发出委托");
    let replied = entrust["state_templates"]
        .as_array()
        .expect("委托 state templates")
        .iter()
        .find(|item| item["state"] == "已回执")
        .expect("委托 已回执 template");
    assert_eq!(
        replied["draft_template"],
        "委托：\n- 回执：\n- 待确认：\n- 下一步："
    );
}

#[test]
fn shell_state_anchors_direct_identity_to_registered_resident() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    register_resident(&mut runtime, "guest-03");
    runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "guest-03".into(),
            requester_device_id: Some("browser".into()),
            peer_id: "rsaga".into(),
            peer_device_id: Some("desktop-1".into()),
        })
        .expect("open direct session");

    let state = runtime.shell_state();
    let json = serde_json::to_value(&state).expect("serialize shell state");
    let direct = json["conversation_shell"]["conversations"]
        .as_array()
        .expect("conversation shell array")
        .iter()
        .find(|conversation| conversation["conversation_id"] == "dm:guest-03:rsaga")
        .expect("guest direct conversation");

    assert_eq!(direct["self_label"], "guest-03");
    assert_eq!(direct["peer_label"], "rsaga");
    assert_eq!(direct["participant_label"], "你与 rsaga");
    assert_eq!(direct["thread_headline"], "正在与 rsaga 聊天");
    assert_eq!(direct["detail_card"]["meta"][0]["label"], "住户");
    assert_eq!(direct["detail_card"]["meta"][0]["value"], "guest-03");
    assert_eq!(direct["detail_card"]["meta"][1]["label"], "对端");
    assert_eq!(direct["detail_card"]["meta"][1]["value"], "rsaga");
    assert_eq!(direct["detail_card"]["title"], "旺财 / 与 rsaga 的房内状态");
    assert_eq!(
        direct["detail_card"]["summary_copy"],
        "旺财 会帮你记住与 rsaga 的留言和提醒，适合续聊、记任务和直接追问。"
    );
}

#[test]
fn shell_state_for_viewer_filters_private_threads_and_labels_counterpart() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    register_resident(&mut runtime, "guest-03");
    runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "guest-03".into(),
            requester_device_id: Some("browser".into()),
            peer_id: "rsaga".into(),
            peer_device_id: Some("desktop-1".into()),
        })
        .expect("open guest direct session");

    let guest = IdentityId("guest-03".into());
    let guest_state = runtime.shell_state_for_viewer(Some(&guest));
    let guest_ids = guest_state
        .conversation_shell
        .conversations
        .iter()
        .map(|conversation| conversation.conversation_id.as_str())
        .collect::<Vec<_>>();
    assert!(guest_ids.contains(&"dm:guest-03:rsaga"));
    assert!(!guest_ids.contains(&"dm:builder:rsaga"));

    let guest_direct = guest_state
        .conversation_shell
        .conversations
        .iter()
        .find(|conversation| conversation.conversation_id == "dm:guest-03:rsaga")
        .expect("guest direct conversation");
    assert_eq!(guest_direct.self_label.as_deref(), Some("guest-03"));
    assert_eq!(guest_direct.peer_label.as_deref(), Some("rsaga"));
    assert_eq!(
        guest_direct.participant_label.as_deref(),
        Some("你与 rsaga")
    );
    assert_eq!(guest_direct.title, "正在与 rsaga 聊天");

    let rsaga = IdentityId("rsaga".into());
    let rsaga_state = runtime.shell_state_for_viewer(Some(&rsaga));
    let rsaga_ids = rsaga_state
        .conversation_shell
        .conversations
        .iter()
        .map(|conversation| conversation.conversation_id.as_str())
        .collect::<Vec<_>>();
    assert!(rsaga_ids.contains(&"dm:guest-03:rsaga"));
    assert!(rsaga_ids.contains(&"dm:builder:rsaga"));

    let rsaga_direct = rsaga_state
        .conversation_shell
        .conversations
        .iter()
        .find(|conversation| conversation.conversation_id == "dm:guest-03:rsaga")
        .expect("rsaga direct conversation");
    assert_eq!(rsaga_direct.self_label.as_deref(), Some("rsaga"));
    assert_eq!(rsaga_direct.peer_label.as_deref(), Some("guest-03"));
    assert_eq!(
        rsaga_direct.participant_label.as_deref(),
        Some("你与 guest-03")
    );
    assert_eq!(rsaga_direct.title, "正在与 guest-03 聊天");
}

#[test]
fn personal_room_defaults_to_owner_only_shell_visibility() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    register_resident(&mut runtime, "alice");
    register_resident(&mut runtime, "bob");
    runtime
        .open_personal_room(PersonalRoomRequest {
            resident_id: "alice".into(),
        })
        .expect("open alice personal room");

    let owner = IdentityId("alice".into());
    let owner_state = runtime.shell_state_for_viewer(Some(&owner));
    let owner_room = owner_state
        .rooms
        .iter()
        .find(|room| room.id == "home:alice")
        .expect("owner should see own personal room");
    assert_eq!(owner_room.owner_resident_id.as_deref(), Some("alice"));
    assert_eq!(
        owner_room.personal_room_access_policy,
        Some(PersonalRoomAccessPolicy::FriendsOnly),
        "owner shell state should expose the current personal room access policy"
    );

    let visitor = IdentityId("bob".into());
    let visitor_state = runtime.shell_state_for_viewer(Some(&visitor));
    assert!(
        visitor_state
            .rooms
            .iter()
            .all(|room| room.id != "home:alice"),
        "registered visitors must not see another resident's personal room without owner policy"
    );

    let anonymous_state = runtime.shell_state_for_viewer(None);
    assert!(
        anonymous_state
            .rooms
            .iter()
            .all(|room| room.id != "home:alice"),
        "anonymous shell state must not expose personal rooms"
    );
}

#[test]
fn personal_room_requires_registered_owner() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let err = runtime
        .open_personal_room(PersonalRoomRequest {
            resident_id: "unregistered-owner".into(),
        })
        .expect_err("unregistered owner must not create a personal room");
    assert!(
        err.contains("registered"),
        "error should mention registration, got {err}"
    );

    register_resident(&mut runtime, "registered-owner");
    let response = runtime
        .open_personal_room(PersonalRoomRequest {
            resident_id: "registered-owner".into(),
        })
        .expect("registered owner can create a personal room");
    assert_eq!(response.room_id, "home:registered-owner");
}

#[test]
fn personal_room_http_route_requires_matching_bearer_owner() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    register_resident(&mut runtime, "alice");
    register_resident(&mut runtime, "bob");
    runtime
        .join_city(JoinCityRequest {
            city: "core-harbor".into(),
            resident_id: "alice".into(),
        })
        .expect("alice joins default city");
    runtime
        .join_city(JoinCityRequest {
            city: "core-harbor".into(),
            resident_id: "bob".into(),
        })
        .expect("bob joins default city");
    let alice = IdentityId("alice".into());
    let bob = IdentityId("bob".into());
    let (alice_token, _) =
        runtime.issue_auth_session(&alice, "test-personal-room-alice", GatewayRuntime::now_ms());
    let (bob_token, _) =
        runtime.issue_auth_session(&bob, "test-personal-room-bob", GatewayRuntime::now_ms());
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({ "resident_id": "alice" });

    let (missing_status, _missing) =
        http_json("POST", &server.base_url, "/v1/personal-room", Some(&body));
    assert_eq!(missing_status, 401);

    let bob_auth = format!("Bearer {bob_token}");
    let (mismatch_status, mismatch) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/personal-room",
        &[("Authorization", bob_auth.as_str())],
        Some(&body),
    );
    assert_eq!(mismatch_status, 401);
    assert!(
        mismatch["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("does not match authenticated session")
    );

    let alice_auth = format!("Bearer {alice_token}");
    let (owner_status, owner_response) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/personal-room",
        &[("Authorization", alice_auth.as_str())],
        Some(&body),
    );
    assert_eq!(owner_status, 200);
    assert_eq!(owner_response["room_id"], "home:alice");
}

#[test]
fn personal_room_access_policy_http_route_requires_matching_bearer_owner() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    register_resident(&mut runtime, "alice");
    register_resident(&mut runtime, "bob");
    runtime
        .open_personal_room(PersonalRoomRequest {
            resident_id: "alice".into(),
        })
        .expect("open alice personal room");
    let alice = IdentityId("alice".into());
    let bob = IdentityId("bob".into());
    let (alice_token, _) =
        runtime.issue_auth_session(&alice, "test-access-policy-alice", GatewayRuntime::now_ms());
    let (bob_token, _) =
        runtime.issue_auth_session(&bob, "test-access-policy-bob", GatewayRuntime::now_ms());
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({
        "resident_id": "alice",
        "policy": "registered_all"
    });

    let (missing_status, _missing) = http_json(
        "POST",
        &server.base_url,
        "/v1/personal-room/access-policy",
        Some(&body),
    );
    assert_eq!(missing_status, 401);

    let bob_auth = format!("Bearer {bob_token}");
    let (mismatch_status, mismatch) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/personal-room/access-policy",
        &[("Authorization", bob_auth.as_str())],
        Some(&body),
    );
    assert_eq!(mismatch_status, 401);
    assert!(
        mismatch["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("does not match authenticated session")
    );

    let alice_auth = format!("Bearer {alice_token}");
    let (owner_status, owner_response) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/personal-room/access-policy",
        &[("Authorization", alice_auth.as_str())],
        Some(&body),
    );
    assert_eq!(owner_status, 200);
    assert_eq!(owner_response["resident_id"], "alice");
    assert_eq!(owner_response["policy"], "registered_all");

    let (bob_state_status, bob_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=bob",
        None,
    );
    assert_eq!(bob_state_status, 200);
    assert!(
        bob_state["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .any(|room| room["id"] == "home:alice"),
        "registered_all policy set through HTTP should expose the personal room scene"
    );
}

#[test]
fn personal_room_registered_all_allows_registered_scene_without_message_history() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    register_resident(&mut runtime, "alice");
    register_resident(&mut runtime, "bob");
    runtime
        .open_personal_room(PersonalRoomRequest {
            resident_id: "alice".into(),
        })
        .expect("open alice personal room");
    runtime
        .append_shell_message(ShellMessageRequest {
            room_id: "home:alice".into(),
            sender: "alice".into(),
            text: "owner private note".into(),
            device_id: Some("browser-a".into()),
            language_tag: Some("zh-CN".into()),
            reply_to_message_id: None,
        })
        .expect("owner writes private room note");

    let bob = IdentityId("bob".into());
    let hidden_state = runtime.shell_state_for_viewer(Some(&bob));
    assert!(
        hidden_state
            .rooms
            .iter()
            .all(|room| room.id != "home:alice"),
        "default friends_only policy should not expose the room before the owner opts in"
    );

    runtime
        .set_personal_room_access_policy(PersonalRoomAccessPolicyRequest {
            resident_id: "alice".into(),
            policy: PersonalRoomAccessPolicy::RegisteredAll,
        })
        .expect("owner enables registered_all access");

    let visible_state = runtime.shell_state_for_viewer(Some(&bob));
    let room = visible_state
        .rooms
        .iter()
        .find(|room| room.id == "home:alice")
        .expect("registered_all should expose the personal room scene to registered visitors");
    assert_eq!(room.owner_resident_id.as_deref(), Some("alice"));
    assert_eq!(
        room.personal_room_access_policy,
        Some(PersonalRoomAccessPolicy::RegisteredAll)
    );
    assert!(
        room.messages.is_empty(),
        "registered_all scene access must not expose the owner's private room message history"
    );
}

#[test]
fn personal_room_friends_only_allows_accepted_friend_scene_without_message_history() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    register_resident(&mut runtime, "alice");
    register_resident(&mut runtime, "bob");
    runtime
        .open_personal_room(PersonalRoomRequest {
            resident_id: "alice".into(),
        })
        .expect("open alice personal room");
    runtime
        .append_shell_message(ShellMessageRequest {
            room_id: "home:alice".into(),
            sender: "alice".into(),
            text: "owner private note".into(),
            device_id: Some("browser-a".into()),
            language_tag: Some("zh-CN".into()),
            reply_to_message_id: None,
        })
        .expect("owner writes private room note");

    let bob = IdentityId("bob".into());
    runtime
        .request_resident_friendship(ResidentRelationshipRequest {
            actor_id: "bob".into(),
            peer_id: "alice".into(),
        })
        .expect("bob requests alice friendship");
    assert!(
        runtime
            .shell_state_for_viewer(Some(&bob))
            .rooms
            .iter()
            .all(|room| room.id != "home:alice"),
        "pending friendship request must not unlock a friends_only personal room"
    );

    runtime
        .accept_resident_friendship(ResidentRelationshipRequest {
            actor_id: "alice".into(),
            peer_id: "bob".into(),
        })
        .expect("alice accepts bob friendship");
    let room = runtime
        .shell_state_for_viewer(Some(&bob))
        .rooms
        .into_iter()
        .find(|room| room.id == "home:alice")
        .expect("accepted friends should see the friends_only personal room scene");
    assert_eq!(room.owner_resident_id.as_deref(), Some("alice"));
    assert_eq!(
        room.personal_room_access_policy,
        Some(PersonalRoomAccessPolicy::FriendsOnly)
    );
    assert!(
        room.messages.is_empty(),
        "friends_only scene access must not expose the owner's private room message history"
    );
}

#[test]
fn resident_relationship_http_routes_require_matching_bearer_actor_and_unlock_friends_only() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    register_resident(&mut runtime, "alice");
    register_resident(&mut runtime, "bob");
    runtime
        .open_personal_room(PersonalRoomRequest {
            resident_id: "alice".into(),
        })
        .expect("open alice personal room");
    let alice = IdentityId("alice".into());
    let bob = IdentityId("bob".into());
    let (alice_token, _) =
        runtime.issue_auth_session(&alice, "test-relationship-alice", GatewayRuntime::now_ms());
    let (bob_token, _) =
        runtime.issue_auth_session(&bob, "test-relationship-bob", GatewayRuntime::now_ms());
    let server = start_local_gateway_http_server(runtime);
    let request_body = serde_json::json!({
        "actor_id": "bob",
        "peer_id": "alice"
    });

    let (missing_status, _missing) = http_json(
        "POST",
        &server.base_url,
        "/v1/resident-relationships/request",
        Some(&request_body),
    );
    assert_eq!(missing_status, 401);

    let alice_auth = format!("Bearer {alice_token}");
    let (mismatch_status, mismatch) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/resident-relationships/request",
        &[("Authorization", alice_auth.as_str())],
        Some(&request_body),
    );
    assert_eq!(mismatch_status, 401);
    assert!(
        mismatch["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("does not match authenticated session")
    );

    let bob_auth = format!("Bearer {bob_token}");
    let (request_status, request_response) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/resident-relationships/request",
        &[("Authorization", bob_auth.as_str())],
        Some(&request_body),
    );
    assert_eq!(request_status, 200);
    assert_eq!(request_response["state"], "pending");

    let (pending_status, pending_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=bob",
        None,
    );
    assert_eq!(pending_status, 200);
    assert!(
        pending_state["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .all(|room| room["id"] != "home:alice"),
        "pending relationship must not unlock friends_only personal room"
    );

    let accept_body = serde_json::json!({
        "actor_id": "alice",
        "peer_id": "bob"
    });
    let (accept_status, accept_response) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/resident-relationships/accept",
        &[("Authorization", alice_auth.as_str())],
        Some(&accept_body),
    );
    assert_eq!(accept_status, 200);
    assert_eq!(accept_response["state"], "friends");

    let (visible_status, visible_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=bob",
        None,
    );
    assert_eq!(visible_status, 200);
    assert!(
        visible_state["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .any(|room| room["id"] == "home:alice"),
        "accepted friends should see friends_only personal room scene through HTTP"
    );
}

#[test]
fn residents_endpoint_projects_relationship_state_for_viewer() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    register_resident(&mut runtime, "alice");
    register_resident(&mut runtime, "bob");
    runtime
        .join_city(JoinCityRequest {
            city: "core-harbor".into(),
            resident_id: "alice".into(),
        })
        .expect("alice joins default city");
    runtime
        .join_city(JoinCityRequest {
            city: "core-harbor".into(),
            resident_id: "bob".into(),
        })
        .expect("bob joins default city");
    let alice = IdentityId("alice".into());
    let bob = IdentityId("bob".into());
    let (alice_token, _) = runtime.issue_auth_session(
        &alice,
        "test-resident-projection-alice",
        GatewayRuntime::now_ms(),
    );
    let (bob_token, _) = runtime.issue_auth_session(
        &bob,
        "test-resident-projection-bob",
        GatewayRuntime::now_ms(),
    );
    let server = start_local_gateway_http_server(runtime);

    let bob_auth = format!("Bearer {bob_token}");
    let request_body = serde_json::json!({
        "actor_id": "bob",
        "peer_id": "alice"
    });
    let (request_status, _request_response) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/resident-relationships/request",
        &[("Authorization", bob_auth.as_str())],
        Some(&request_body),
    );
    assert_eq!(request_status, 200);

    let (alice_view_status, alice_view) = http_json(
        "GET",
        &server.base_url,
        "/v1/residents?resident_id=alice",
        None,
    );
    assert_eq!(alice_view_status, 200);
    let bob_row = alice_view
        .as_array()
        .expect("residents")
        .iter()
        .find(|row| row["resident_id"] == "bob")
        .expect("bob row");
    assert_eq!(bob_row["relationship_state"], "pending");
    assert_eq!(bob_row["relationship_requested_by"], "bob");

    let alice_auth = format!("Bearer {alice_token}");
    let accept_body = serde_json::json!({
        "actor_id": "alice",
        "peer_id": "bob"
    });
    let (accept_status, _accept_response) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/resident-relationships/accept",
        &[("Authorization", alice_auth.as_str())],
        Some(&accept_body),
    );
    assert_eq!(accept_status, 200);

    let (bob_view_status, bob_view) = http_json(
        "GET",
        &server.base_url,
        "/v1/residents?resident_id=bob",
        None,
    );
    assert_eq!(bob_view_status, 200);
    let alice_row = bob_view
        .as_array()
        .expect("residents")
        .iter()
        .find(|row| row["resident_id"] == "alice")
        .expect("alice row");
    assert_eq!(alice_row["relationship_state"], "friends");
    assert_eq!(alice_row["relationship_requested_by"], "bob");
}

#[test]
fn resident_relationships_persist_across_restart() {
    let temp = tempdir().expect("temp dir");
    {
        let mut runtime =
            GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
        register_resident(&mut runtime, "alice");
        register_resident(&mut runtime, "bob");
        runtime
            .request_resident_friendship(ResidentRelationshipRequest {
                actor_id: "bob".into(),
                peer_id: "alice".into(),
            })
            .expect("request relationship");
        runtime
            .accept_resident_friendship(ResidentRelationshipRequest {
                actor_id: "alice".into(),
                peer_id: "bob".into(),
            })
            .expect("accept relationship");
    }

    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    assert!(runtime.residents_are_friends(&IdentityId("alice".into()), &IdentityId("bob".into())));
}

#[test]
fn personal_room_access_policy_persists_across_restart() {
    let temp = tempdir().expect("temp dir");
    {
        let mut runtime =
            GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
        register_resident(&mut runtime, "alice");
        runtime
            .set_personal_room_access_policy(PersonalRoomAccessPolicyRequest {
                resident_id: "alice".into(),
                policy: PersonalRoomAccessPolicy::RegisteredAll,
            })
            .expect("set access policy");
    }

    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let alice = IdentityId("alice".into());
    assert_eq!(
        runtime.personal_room_access_policy(&alice),
        PersonalRoomAccessPolicy::RegisteredAll
    );
}

#[test]
fn shell_state_formats_unanchored_direct_threads_without_raw_dm_fallback() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .ensure_direct_conversation(
            &ConversationId("dm:alice:bob".into()),
            &[IdentityId("alice".into()), IdentityId("bob".into())],
        )
        .expect("ensure direct conversation");

    let state = runtime.shell_state();
    let json = serde_json::to_value(&state).expect("serialize shell state");
    let direct = json["conversation_shell"]["conversations"]
        .as_array()
        .expect("conversation shell array")
        .iter()
        .find(|conversation| conversation["conversation_id"] == "dm:alice:bob")
        .expect("unanchored direct conversation");

    assert_eq!(direct["title"], "alice 与 bob 的私聊");
    assert_eq!(direct["subtitle"], "居所直达 · alice 与 bob");
    assert_eq!(direct["participant_label"], "alice 与 bob");
    assert_eq!(direct["thread_headline"], "alice 与 bob 的私聊");
    assert_eq!(direct["overview_summary"], "alice 与 bob 的私聊");
    assert_eq!(direct["detail_card"]["title"], "旺财 / alice 与 bob 的私聊");
    assert_eq!(
        direct["detail_card"]["summary_copy"],
        "旺财 会帮你记住 alice 与 bob 的留言和提醒，适合续聊、记任务和直接追问。"
    );
    assert_eq!(direct["detail_card"]["meta"][0]["label"], "会话");
    assert_eq!(direct["detail_card"]["meta"][0]["value"], "alice 与 bob");

    let direct_scene = json["scene_render"]["scenes"]
        .as_array()
        .expect("scene render array")
        .iter()
        .find(|scene| scene["conversation_id"] == "dm:alice:bob")
        .expect("unanchored direct scene");
    assert_eq!(direct_scene["stage"]["title"], "alice 与 bob 的私聊");
}

#[test]
fn shell_state_formats_empty_direct_threads_without_direct_prefix() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .timeline_store
        .upsert_conversation(Conversation {
            conversation_id: ConversationId("dm:".into()),
            kind: ConversationKind::Direct,
            scope: ConversationScope::Private,
            scene: Some(GatewayRuntime::default_direct_scene(&[])),
            content_topic: transport_waku::WakuFrameCodec::content_topic_for(&ConversationId(
                "dm:".into(),
            )),
            participants: Vec::new(),
            created_at_ms: GatewayRuntime::now_ms(),
            last_active_at_ms: GatewayRuntime::now_ms(),
        })
        .expect("insert empty direct conversation");

    let state = runtime.shell_state();
    let json = serde_json::to_value(&state).expect("serialize shell state");
    let direct = json["conversation_shell"]["conversations"]
        .as_array()
        .expect("conversation shell array")
        .iter()
        .find(|conversation| conversation["conversation_id"] == "dm:")
        .expect("empty direct conversation");

    assert_eq!(direct["title"], "私聊会话");
    assert_eq!(direct["subtitle"], "居所直达 · 私聊会话");
    assert_eq!(direct["participant_label"], "私聊会话");
    assert_eq!(direct["thread_headline"], "私聊会话");
}

#[test]
fn shell_state_formats_half_anchored_direct_threads_without_current_resident_placeholder() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .timeline_store
        .upsert_conversation(Conversation {
            conversation_id: ConversationId("dm:rsaga".into()),
            kind: ConversationKind::Direct,
            scope: ConversationScope::Private,
            scene: Some(GatewayRuntime::default_direct_scene(&[IdentityId(
                "rsaga".into(),
            )])),
            content_topic: transport_waku::WakuFrameCodec::content_topic_for(&ConversationId(
                "dm:rsaga".into(),
            )),
            participants: vec![IdentityId("rsaga".into())],
            created_at_ms: GatewayRuntime::now_ms(),
            last_active_at_ms: GatewayRuntime::now_ms(),
        })
        .expect("insert half anchored direct conversation");

    let state = runtime.shell_state();
    let json = serde_json::to_value(&state).expect("serialize shell state");
    let direct = json["conversation_shell"]["conversations"]
        .as_array()
        .expect("conversation shell array")
        .iter()
        .find(|conversation| conversation["conversation_id"] == "dm:rsaga")
        .expect("half anchored direct conversation");

    assert_eq!(direct["title"], "rsaga 的私聊");
    assert_eq!(direct["detail_card"]["meta"][0]["label"], "会话");
    assert_eq!(direct["detail_card"]["meta"][0]["value"], "rsaga");
}

#[test]
fn shell_state_formats_unknown_room_threads_without_raw_room_prefix() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .timeline_store
        .upsert_conversation(Conversation {
            conversation_id: ConversationId("room:city:delta-hub:plaza".into()),
            kind: ConversationKind::Room,
            scope: ConversationScope::CityPublic,
            scene: Some(GatewayRuntime::default_public_room_scene(
                "shared",
                "channel",
                "room:city:delta-hub:plaza",
            )),
            content_topic: transport_waku::WakuFrameCodec::content_topic_for(&ConversationId(
                "room:city:delta-hub:plaza".into(),
            )),
            participants: vec![IdentityId("rsaga".into())],
            created_at_ms: GatewayRuntime::now_ms(),
            last_active_at_ms: GatewayRuntime::now_ms(),
        })
        .expect("insert unknown room conversation");

    let state = runtime.shell_state();
    let json = serde_json::to_value(&state).expect("serialize shell state");
    let room = json["conversation_shell"]["conversations"]
        .as_array()
        .expect("conversation shell array")
        .iter()
        .find(|conversation| conversation["conversation_id"] == "room:city:delta-hub:plaza")
        .expect("unknown room conversation");

    assert_eq!(room["title"], "城邦门牌 · city:delta-hub:plaza");
}

#[test]
fn runtime_persists_shell_messages_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime
            .append_shell_message(ShellMessageRequest {
                room_id: "room:world:lobby".into(),
                sender: "rsaga".into(),
                text: "persist me".into(),
                reply_to_message_id: None,
                device_id: Some("browser".into()),
                language_tag: Some("en".into()),
            })
            .expect("append shell message");
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
    let state = runtime.shell_state();
    let lobby = state
        .rooms
        .into_iter()
        .find(|room| room.id == "room:world:lobby")
        .expect("lobby room");
    assert!(
        lobby
            .messages
            .iter()
            .any(|message| message.text == "persist me")
    );
}

#[test]
fn seeded_conversations_persist_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");

    {
        let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        assert!(
            runtime
                .timeline_store
                .active_conversations()
                .iter()
                .any(|conversation| conversation.conversation_id.0 == "room:world:lobby")
        );
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
    assert!(
        runtime
            .timeline_store
            .active_conversations()
            .iter()
            .any(|conversation| conversation.conversation_id.0 == "room:world:lobby")
    );
    assert!(
        std::fs::read_dir(&root)
            .expect("read gateway storage")
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .all(|file_name| !file_name.starts_with("conversations.postcard.corrupt-")),
        "seeded conversations snapshot should decode without quarantine"
    );
}

#[test]
fn create_city_grants_lord_membership() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let city = runtime
        .create_city(CreateCityRequest {
            slug: Some("signal-bay".into()),
            title: "Signal Bay".into(),
            description: "A city for relay experiments".into(),
            lord_id: "alice".into(),
            approval_required: Some(false),
            public_room_discovery_enabled: Some(true),
            federation_policy: None,
        })
        .expect("create city");

    assert_eq!(city.profile.slug, "signal-bay");
    let lord = runtime
        .memberships
        .iter()
        .find(|membership| membership.city_id == city.profile.city_id)
        .expect("lord membership");
    assert_eq!(lord.role, CityRole::Lord);
    assert_eq!(lord.resident_id.0, "alice");
}

#[test]
fn city_lord_can_create_public_room_but_resident_cannot() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let denied = runtime.create_public_room(CreatePublicRoomRequest {
        city: "core-harbor".into(),
        creator_id: "visitor".into(),
        slug: Some("resident-corner".into()),
        title: "Resident Corner".into(),
        description: "should not work".into(),
    });
    assert!(denied.is_err());

    let created = runtime
        .create_public_room(CreatePublicRoomRequest {
            city: "core-harbor".into(),
            creator_id: "rsaga".into(),
            slug: Some("ops-room".into()),
            title: "Ops Room".into(),
            description: "public operations room".into(),
        })
        .expect("lord creates room");
    assert_eq!(created.room_id.0, "room:city:core-harbor:ops-room");
}

#[test]
fn city_http_routes_roundtrip_membership_room_policy_and_freeze_contract() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    register_resident(&mut runtime, "guest-04");
    let server = start_local_gateway_http_server(runtime);

    let (city_status, city) = http_json(
        "POST",
        &server.base_url,
        "/v1/cities",
        Some(&serde_json::json!({
            "slug": "approval-harbor",
            "title": "Approval Harbor",
            "description": "HTTP city contract fixture",
            "lord_id": "rsaga",
            "approval_required": true,
            "public_room_discovery_enabled": true,
            "federation_policy": "Open"
        })),
    );
    assert_eq!(city_status, 200);
    assert_eq!(city["profile"]["slug"], "approval-harbor");
    assert_eq!(city["profile"]["approval_required"], true);

    let (join_status, pending) = http_json(
        "POST",
        &server.base_url,
        "/v1/cities/join",
        Some(&serde_json::json!({
            "city": "approval-harbor",
            "resident_id": "guest-04"
        })),
    );
    assert_eq!(join_status, 200);
    assert_eq!(pending["resident_id"], "guest-04");
    assert_eq!(pending["state"], "PendingApproval");

    let (approve_status, approved) = http_json(
        "POST",
        &server.base_url,
        "/v1/cities/approve",
        Some(&serde_json::json!({
            "city": "approval-harbor",
            "actor_id": "rsaga",
            "resident_id": "guest-04"
        })),
    );
    assert_eq!(approve_status, 200);
    assert_eq!(approved["state"], "Active");
    assert_eq!(approved["added_by"], "rsaga");

    let (steward_status, steward) = http_json(
        "POST",
        &server.base_url,
        "/v1/cities/stewards",
        Some(&serde_json::json!({
            "city": "approval-harbor",
            "actor_id": "rsaga",
            "resident_id": "guest-04",
            "grant": true
        })),
    );
    assert_eq!(steward_status, 200);
    assert_eq!(steward["role"], "Steward");

    let (resident_status, resident) = http_json(
        "POST",
        &server.base_url,
        "/v1/cities/stewards",
        Some(&serde_json::json!({
            "city": "approval-harbor",
            "actor_id": "rsaga",
            "resident_id": "guest-04",
            "grant": false
        })),
    );
    assert_eq!(resident_status, 200);
    assert_eq!(resident["role"], "Resident");

    let (policy_status, policy_city) = http_json(
        "POST",
        &server.base_url,
        "/v1/cities/federation-policy",
        Some(&serde_json::json!({
            "city": "approval-harbor",
            "actor_id": "rsaga",
            "policy": "Selective"
        })),
    );
    assert_eq!(policy_status, 200);
    assert_eq!(policy_city["profile"]["federation_policy"], "Selective");

    let (room_status, room) = http_json(
        "POST",
        &server.base_url,
        "/v1/cities/rooms",
        Some(&serde_json::json!({
            "city": "approval-harbor",
            "creator_id": "rsaga",
            "slug": "qa-room",
            "title": "QA Room",
            "description": "Room created through HTTP contract"
        })),
    );
    assert_eq!(room_status, 200);
    assert_eq!(room["room_id"], "room:city:approval-harbor:qa-room");
    assert_eq!(room["frozen"], false);

    let (freeze_status, frozen) = http_json(
        "POST",
        &server.base_url,
        "/v1/cities/rooms/freeze",
        Some(&serde_json::json!({
            "city": "approval-harbor",
            "actor_id": "rsaga",
            "room": "qa-room",
            "frozen": true
        })),
    );
    assert_eq!(freeze_status, 200);
    assert_eq!(frozen["room_id"], "room:city:approval-harbor:qa-room");
    assert_eq!(frozen["frozen"], true);

    let (message_status, message_error) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:city:approval-harbor:qa-room",
            "sender": "guest-04",
            "text": "frozen room should reject this",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(message_status, 400);
    assert_eq!(message_error["Error"]["message"], "room qa-room is frozen");
}

#[test]
fn governance_state_persists_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime
            .create_city(CreateCityRequest {
                slug: Some("aurora".into()),
                title: "Aurora".into(),
                description: "northern city".into(),
                lord_id: "rsaga".into(),
                approval_required: Some(true),
                public_room_discovery_enabled: Some(true),
                federation_policy: None,
            })
            .expect("create city");
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
    assert!(
        runtime
            .cities
            .values()
            .any(|city| city.profile.slug == "aurora")
    );
}

#[test]
fn admin_room_freeze_persists_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");
    let room_id = "room:city:core-harbor:lobby";

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        assert!(runtime.admin_freeze_room(room_id).expect("freeze room"));
    }

    {
        let runtime = GatewayRuntime::open(&root, 64, None).expect("reopened runtime");
        let room = runtime
            .admin_rooms_detail()
            .into_iter()
            .find(|room| room.id == room_id)
            .expect("frozen room");
        assert!(
            room.is_frozen,
            "admin freeze must survive a gateway restart"
        );
    }

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        assert!(runtime.admin_unfreeze_room(room_id).expect("unfreeze room"));
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("reopened runtime");
    let room = runtime
        .admin_rooms_detail()
        .into_iter()
        .find(|room| room.id == room_id)
        .expect("unfrozen room");
    assert!(
        !room.is_frozen,
        "admin unfreeze must survive a gateway restart"
    );
}

#[test]
fn city_public_room_create_wakes_shell_events_without_waiting_for_timeout() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=rsaga",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version")
        .to_string();

    let events_base_url = server.base_url.clone();
    let events_path =
        format!("/v1/shell/events?resident_id=rsaga&after={initial_version}&wait_ms=5000");
    let started_at = Instant::now();
    let events_thread =
        thread::spawn(move || http_raw("GET", &events_base_url, &events_path, None));

    thread::sleep(Duration::from_millis(100));
    let (room_status, room) = http_json(
        "POST",
        &server.base_url,
        "/v1/cities/rooms",
        Some(&serde_json::json!({
            "city": "core-harbor",
            "creator_id": "rsaga",
            "slug": "sse-room",
            "title": "SSE Room",
            "description": "Room creation should wake shell listeners"
        })),
    );
    assert_eq!(room_status, 200);
    assert_eq!(room["room_id"], "room:city:core-harbor:sse-room");

    let (events_status, _headers, body) = events_thread.join().expect("events thread");
    let elapsed = started_at.elapsed();
    assert_eq!(events_status, 200);
    assert!(
        elapsed < Duration::from_millis(1500),
        "public room create should notify shell events promptly, elapsed {elapsed:?}"
    );
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert_ne!(payload["state_version"], initial_version);
    assert!(
        payload["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .any(|room| room["id"] == "room:city:core-harbor:sse-room")
    );
}

#[test]
fn city_public_room_freeze_wakes_shell_events_without_waiting_for_timeout() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=rsaga",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version")
        .to_string();
    assert!(
        initial_state["rooms"]
            .as_array()
            .expect("initial rooms")
            .iter()
            .any(|room| room["id"] == "room:city:core-harbor:lobby")
    );

    let events_base_url = server.base_url.clone();
    let events_path =
        format!("/v1/shell/events?resident_id=rsaga&after={initial_version}&wait_ms=5000");
    let started_at = Instant::now();
    let events_thread =
        thread::spawn(move || http_raw("GET", &events_base_url, &events_path, None));

    thread::sleep(Duration::from_millis(100));
    let (freeze_status, room) = http_json(
        "POST",
        &server.base_url,
        "/v1/cities/rooms/freeze",
        Some(&serde_json::json!({
            "city": "core-harbor",
            "actor_id": "rsaga",
            "room": "lobby",
            "frozen": true
        })),
    );
    assert_eq!(freeze_status, 200);
    assert_eq!(room["room_id"], "room:city:core-harbor:lobby");
    assert_eq!(room["frozen"], true);

    let (events_status, _headers, body) = events_thread.join().expect("events thread");
    let elapsed = started_at.elapsed();
    assert_eq!(events_status, 200);
    assert!(
        elapsed < Duration::from_millis(1500),
        "public room freeze should notify shell events promptly, elapsed {elapsed:?}"
    );
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert_ne!(payload["state_version"], initial_version);
    let lobby = payload["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "room:city:core-harbor:lobby")
        .expect("lobby room");
    assert_eq!(lobby["is_frozen"], true);
    assert_eq!(lobby["chat_status_summary"], "房间已冻结，仅管理员可发言");
}

#[test]
fn city_trust_update_wakes_shell_events_without_waiting_for_timeout() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=outside-reader",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version")
        .to_string();
    assert!(
        initial_state["rooms"]
            .as_array()
            .expect("initial rooms")
            .iter()
            .any(|room| room["id"] == "room:city:aurora-hub:announcements")
    );

    let events_base_url = server.base_url.clone();
    let events_path =
        format!("/v1/shell/events?resident_id=outside-reader&after={initial_version}&wait_ms=5000");
    let started_at = Instant::now();
    let events_thread =
        thread::spawn(move || http_raw("GET", &events_base_url, &events_path, None));

    thread::sleep(Duration::from_millis(100));
    let (trust_status, trust) = http_json(
        "POST",
        &server.base_url,
        "/v1/world-safety/cities/trust",
        Some(&serde_json::json!({
            "actor_id": "rsaga",
            "city": "aurora-hub",
            "state": "Isolated",
            "reason": "SSE trust update should refresh city directory"
        })),
    );
    assert_eq!(trust_status, 200);
    assert_eq!(trust["city_id"], "city:aurora-hub");
    assert_eq!(trust["state"], "Isolated");

    let (events_status, _headers, body) = events_thread.join().expect("events thread");
    let elapsed = started_at.elapsed();
    assert_eq!(events_status, 200);
    assert!(
        elapsed < Duration::from_millis(1500),
        "city trust update should notify shell events promptly, elapsed {elapsed:?}"
    );
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert_ne!(payload["state_version"], initial_version);
    assert!(
        !payload["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .any(|room| room["id"] == "room:city:aurora-hub:announcements")
    );
}

#[test]
fn lord_can_approve_pending_join() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .create_city(CreateCityRequest {
            slug: Some("approval-bay".into()),
            title: "Approval Bay".into(),
            description: "approval on".into(),
            lord_id: "rsaga".into(),
            approval_required: Some(true),
            public_room_discovery_enabled: Some(true),
            federation_policy: None,
        })
        .expect("create city");

    register_resident(&mut runtime, "guest-01");
    let pending = runtime
        .join_city(JoinCityRequest {
            city: "approval-bay".into(),
            resident_id: "guest-01".into(),
        })
        .expect("join city");
    assert_eq!(pending.state, MembershipState::PendingApproval);

    let approved = runtime
        .approve_city_join(ApproveCityJoinRequest {
            city: "approval-bay".into(),
            actor_id: "rsaga".into(),
            resident_id: "guest-01".into(),
        })
        .expect("approve join");
    assert_eq!(approved.state, MembershipState::Active);
    assert_eq!(approved.added_by.expect("added by").0, "rsaga");
}

#[test]
fn lord_can_grant_steward_role() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    register_resident(&mut runtime, "helper");
    runtime
        .join_city(JoinCityRequest {
            city: "core-harbor".into(),
            resident_id: "helper".into(),
        })
        .expect("join core harbor");

    let steward = runtime
        .update_steward(UpdateStewardRequest {
            city: "core-harbor".into(),
            actor_id: "rsaga".into(),
            resident_id: "helper".into(),
            grant: true,
        })
        .expect("grant steward");
    assert_eq!(steward.role, CityRole::Steward);
}

#[test]
fn frozen_public_room_blocks_resident_posts() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    register_resident(&mut runtime, "guest-02");
    runtime
        .join_city(JoinCityRequest {
            city: "core-harbor".into(),
            resident_id: "guest-02".into(),
        })
        .expect("join core harbor");

    runtime
        .freeze_public_room(FreezePublicRoomRequest {
            city: "core-harbor".into(),
            actor_id: "rsaga".into(),
            room: "lobby".into(),
            frozen: true,
        })
        .expect("freeze lobby");

    let blocked = runtime.append_shell_message(ShellMessageRequest {
        room_id: "room:city:core-harbor:lobby".into(),
        sender: "guest-02".into(),
        text: "let me in".into(),
        reply_to_message_id: None,
        device_id: Some("browser".into()),
        language_tag: Some("en".into()),
    });
    assert!(blocked.is_err());

    let allowed = runtime.append_shell_message(ShellMessageRequest {
        room_id: "room:city:core-harbor:lobby".into(),
        sender: "rsaga".into(),
        text: "maintenance window".into(),
        reply_to_message_id: None,
        device_id: Some("browser".into()),
        language_tag: Some("en".into()),
    });
    assert!(allowed.is_ok());
}

#[test]
fn shell_message_rejects_visitor_sender_before_login() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let room_id = ConversationId("room:world:lobby".into());
    let before = runtime.timeline_store.recent_messages(&room_id, 64).len();

    let error = runtime
        .append_shell_message(ShellMessageRequest {
            room_id: room_id.0.clone(),
            sender: "访客".into(),
            text: "I should not be able to post before login".into(),
            reply_to_message_id: None,
            device_id: Some("browser".into()),
            language_tag: Some("zh-CN".into()),
        })
        .expect_err("visitor shell sender should be rejected before login");

    assert!(error.contains("login"));
    assert_eq!(
        runtime.timeline_store.recent_messages(&room_id, 64).len(),
        before
    );
}

#[test]
fn shell_message_response_and_projection_expose_stable_message_contract() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let response = runtime
        .append_shell_message(ShellMessageRequest {
            room_id: "room:world:lobby".into(),
            sender: "rsaga".into(),
            text: "  交互回执稳定  ".into(),
            reply_to_message_id: None,
            device_id: Some("browser".into()),
            language_tag: Some("zh-CN".into()),
        })
        .expect("append shell message");

    assert!(response.ok);
    assert_eq!(response.conversation_id, "room:world:lobby");
    assert!(!response.message_id.is_empty());
    assert!(response.delivered_at_ms > 0);

    let state = runtime.shell_state();
    let lobby = state
        .rooms
        .iter()
        .find(|room| room.id == "room:world:lobby")
        .expect("lobby room");
    let message = lobby
        .messages
        .iter()
        .find(|message| message.message_id == response.message_id)
        .expect("projected message");

    assert_eq!(message.message_id, response.message_id);
    assert_eq!(message.text, "交互回执稳定");
    assert_eq!(message.delivery_status, "delivered");

    let blank = runtime
        .append_shell_message(ShellMessageRequest {
            room_id: "room:world:lobby".into(),
            sender: "rsaga".into(),
            text: "   ".into(),
            reply_to_message_id: None,
            device_id: Some("browser".into()),
            language_tag: Some("zh-CN".into()),
        })
        .expect_err("blank shell message should be rejected");
    assert!(blank.contains("text"));
}

#[test]
fn email_otp_mailer_posts_authenticated_delivery_without_leaking_token_into_body() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mailer capture");
    let address = listener.local_addr().expect("mailer capture address");
    let (captured_tx, captured_rx) = std::sync::mpsc::channel();
    let capture = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept mailer request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("mailer read timeout");
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; 2048];
        loop {
            let read = stream.read(&mut chunk).expect("read mailer request");
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..read]);
            let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
        captured_tx.send(bytes).expect("capture mailer request");
        stream
            .write_all(
                b"HTTP/1.1 202 Accepted\r\nContent-Type: application/json\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"ok\":true}",
            )
            .expect("respond to mailer request");
    });

    let config = EmailOtpMailerConfig::new(
        format!("http://{address}/deliver"),
        "mailer-secret".into(),
        Some("我和狗蛋儿的家 <no-reply@example.com>".into()),
    )
    .expect("localhost mailer config");
    deliver_email_otp(
        &config,
        &EmailOtpDelivery {
            to: "reader@example.com".into(),
            code: "483921".into(),
            challenge_id: "otp:test".into(),
            expires_at_ms: 1_800_000_000_000,
        },
    )
    .expect("deliver otp");

    capture.join().expect("join mailer capture");
    let request = String::from_utf8(captured_rx.recv().expect("captured request"))
        .expect("utf8 mailer request");
    assert!(request.starts_with("POST /deliver HTTP/1.1"));
    assert!(request.contains("Authorization: Bearer mailer-secret"));
    assert!(request.contains("\"to\":\"reader@example.com\""));
    assert!(request.contains("\"code\":\"483921\""));
    assert!(request.contains("\"challenge_id\":\"otp:test\""));
    assert!(!request.contains("\"token\":\"mailer-secret\""));
}

#[test]
fn email_otp_mailer_requires_https_except_for_local_test_endpoints() {
    assert!(
        EmailOtpMailerConfig::new(
            "http://mailer.example.com/send".into(),
            "secret".into(),
            None,
        )
        .is_err()
    );
    assert!(
        EmailOtpMailerConfig::new("https://mailer.example.com/send".into(), "".into(), None,)
            .is_err()
    );
}

#[test]
fn email_otp_delivery_does_not_hold_gateway_runtime_lock() {
    let temp = tempdir().expect("temp dir");
    let runtime = Arc::new(Mutex::new(
        GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime"),
    ));
    let worker_runtime = Arc::clone(&runtime);
    let (delivery_started_tx, delivery_started_rx) = std::sync::mpsc::channel();
    let (release_delivery_tx, release_delivery_rx) = std::sync::mpsc::channel();

    let worker = thread::spawn(move || {
        crate::http_auth_routes::request_email_otp_with_delivery(
            &worker_runtime,
            RequestEmailOtpRequest {
                email: "slow-mailer@example.com".into(),
                mobile: None,
                device_physical_address: None,
                resident_id: Some("slow-mailer".into()),
                nickname: None,
            },
            false,
            |_delivery| {
                delivery_started_tx.send(()).expect("signal delivery start");
                release_delivery_rx
                    .recv_timeout(Duration::from_secs(2))
                    .expect("release delivery");
                Ok(())
            },
        )
    });

    delivery_started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("delivery should start");
    assert!(
        runtime.try_lock().is_ok(),
        "slow external mail delivery must not block unrelated gateway requests"
    );
    release_delivery_tx.send(()).expect("release delivery");
    let response = worker
        .join()
        .expect("join delivery worker")
        .unwrap_or_else(|_| panic!("runtime should be available"))
        .expect("otp request should succeed");
    assert_eq!(response.delivery_mode, "mailer-webhook");
    assert!(response.dev_code.is_none());
}

#[test]
fn email_otp_delivery_failure_rolls_back_challenge_and_rate_limit() {
    let temp = tempdir().expect("temp dir");
    let runtime = Arc::new(Mutex::new(
        GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime"),
    ));
    let request = RequestEmailOtpRequest {
        email: "mailer-failure@example.com".into(),
        mobile: None,
        device_physical_address: None,
        resident_id: Some("mailer-failure".into()),
        nickname: None,
    };

    let result = crate::http_auth_routes::request_email_otp_with_delivery(
        &runtime,
        request.clone(),
        false,
        |_delivery| Err("mailer unavailable".into()),
    )
    .unwrap_or_else(|_| panic!("runtime should be available"));
    assert_eq!(
        result.expect_err("delivery should fail"),
        "email otp delivery failed",
        "resident-facing error must be generic; delivery detail stays in server logs"
    );

    let retry = runtime
        .lock()
        .expect("lock runtime")
        .request_email_otp(request)
        .expect("failed delivery must not consume the request rate limit");
    assert_eq!(retry.delivery_mode, "inline-dev");
}

#[test]
fn auth_credentials_use_fresh_os_random_values() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let first_code = runtime
        .generate_email_otp_code()
        .expect("secure otp generation");
    let second_code = runtime
        .generate_email_otp_code()
        .expect("secure otp generation");
    assert_eq!(first_code.len(), 6);
    assert_eq!(second_code.len(), 6);
    assert!(first_code.bytes().all(|byte| byte.is_ascii_digit()));
    assert!(second_code.bytes().all(|byte| byte.is_ascii_digit()));

    let resident = IdentityId("random-auth-user".into());
    let (first_token, _) =
        runtime.issue_auth_session(&resident, "same-challenge", GatewayRuntime::now_ms());
    let (second_token, _) =
        runtime.issue_auth_session(&resident, "same-challenge", GatewayRuntime::now_ms());
    assert_eq!(first_token.len(), 69);
    assert_eq!(second_token.len(), 69);
    assert_ne!(first_token, second_token);
}

#[test]
fn auth_http_routes_roundtrip_email_otp_registration() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (preflight_status, preflight) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/preflight",
        Some(&serde_json::json!({
            "email": "Novel.Reader@Example.COM",
            "mobile": "+86 13800138000",
            "device_physical_address": "66:55:44:33:22:11"
        })),
    );
    assert_eq!(preflight_status, 200);
    assert_eq!(preflight["allowed"], true);
    assert_eq!(preflight["normalized_email"], "novel.reader@example.com");

    let (request_status, challenge) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/request",
        Some(&serde_json::json!({
            "email": "novel.reader@example.com",
            "mobile": "+86 13800138000",
            "device_physical_address": "66:55:44:33:22:11",
            "resident_id": "novel-reader"
        })),
    );
    assert_eq!(request_status, 200);
    assert!(
        challenge["challenge_id"]
            .as_str()
            .unwrap_or_default()
            .starts_with("otp:")
    );
    assert_eq!(challenge["delivery_mode"], "inline-dev");
    let code = challenge["dev_code"]
        .as_str()
        .expect("test gateway should expose dev otp");

    let (verify_status, verified) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/verify",
        Some(&serde_json::json!({
            "challenge_id": challenge["challenge_id"],
            "code": code,
            "resident_id": "novel-reader"
        })),
    );
    assert_eq!(verify_status, 200);
    assert_eq!(verified["resident_id"], "novel-reader");
    assert_eq!(verified["email"], "novel.reader@example.com");
    assert_eq!(verified["token_type"], "Bearer");
    assert_eq!(verified["session"]["resident_id"], "novel-reader");
    assert!(
        verified["session_token"]
            .as_str()
            .expect("verify response should include session token")
            .starts_with("lbst_")
    );

    let (state_status, state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=novel-reader",
        None,
    );
    assert_eq!(state_status, 200);
    assert!(
        state["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .any(|room| room["id"] == "dm:guide:novel-reader")
    );
}

#[test]
fn auth_http_routes_roundtrip_mobile_otp_registration() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (request_status, challenge) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/mobile-otp/request",
        Some(&serde_json::json!({
            "mobile": "+86 13800138000",
            "email": "mobile-user@example.com",
            "device_physical_address": "AA:BB:CC:DD:EE:FF",
            "resident_id": "mobile-reader",
            "nickname": "手机用户"
        })),
    );
    assert_eq!(request_status, 200);
    assert!(
        challenge["challenge_id"]
            .as_str()
            .unwrap_or_default()
            .starts_with("mobile-otp:")
    );
    assert_eq!(challenge["delivery_mode"], "inline-dev");
    let code = challenge["dev_code"]
        .as_str()
        .expect("test gateway should expose dev mobile otp");

    let (verify_status, verified) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/mobile-otp/verify",
        Some(&serde_json::json!({
            "challenge_id": challenge["challenge_id"],
            "code": code,
            "resident_id": "mobile-reader"
        })),
    );
    assert_eq!(verify_status, 200);
    assert_eq!(verified["resident_id"], "mobile-reader");
    assert_eq!(verified["mobile"], "8613800138000");
    assert!(
        verified["mobile_masked"]
            .as_str()
            .unwrap_or_default()
            .contains("****")
    );
    assert_eq!(verified["nickname"], "手机用户");
    assert_eq!(verified["token_type"], "Bearer");
    assert_eq!(verified["session"]["resident_id"], "mobile-reader");
    assert!(
        verified["session_token"]
            .as_str()
            .expect("verify response should include session token")
            .starts_with("lbst_")
    );
    // session token works for authenticated endpoint
    let session_token = verified["session_token"].as_str().expect("session token");
    let auth_header = format!("Bearer {session_token}");
    let (session_status, session_resp) = http_json_with_headers(
        "GET",
        &server.base_url,
        "/v1/auth/session",
        &[("Authorization", auth_header.as_str())],
        None,
    );
    assert_eq!(session_status, 200);
    assert_eq!(session_resp["resident_id"], "mobile-reader");
}

#[test]
fn auth_mobile_otp_rejects_invalid_code() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (request_status, challenge) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/mobile-otp/request",
        Some(&serde_json::json!({
            "mobile": "13900139000"
        })),
    );
    assert_eq!(request_status, 200);

    let (verify_status, verify_resp) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/mobile-otp/verify",
        Some(&serde_json::json!({
            "challenge_id": challenge["challenge_id"],
            "code": "000000"
        })),
    );
    assert_eq!(verify_status, 400);
    assert!(
        verify_resp["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("invalid otp code")
    );
}

#[test]
fn auth_mobile_otp_rejects_expired_challenge() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (request_status, challenge) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/mobile-otp/request",
        Some(&serde_json::json!({
            "mobile": "13700137000"
        })),
    );
    assert_eq!(request_status, 200);
    let code = challenge["dev_code"].as_str().expect("dev otp");

    // Manually expire the challenge
    {
        let mut rt = server.runtime.lock().expect("lock");
        for ch in rt.email_otp_challenges.iter_mut() {
            if ch.challenge_id == challenge["challenge_id"].as_str().unwrap_or("") {
                ch.expires_at_ms = 0;
            }
        }
    }

    let (verify_status, verify_resp) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/mobile-otp/verify",
        Some(&serde_json::json!({
            "challenge_id": challenge["challenge_id"],
            "code": code
        })),
    );
    assert_eq!(verify_status, 400);
    // purge_expired runs first, so the challenge is already removed
    assert!(
        verify_resp["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("unknown")
    );
}

#[test]
fn auth_mobile_otp_blacklisted_mobile_rejected() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let blacklisted_mobile = "13800000001";
    let mobile_hash = GatewayRuntime::hash_registration_handle("mobile", blacklisted_mobile);
    runtime
        .registration_blacklist
        .push(RegistrationBlacklistEntry {
            entry_id: "blacklist:test".into(),
            resident_id: IdentityId("system".into()),
            report_id: None,
            handle_kind: "mobile".into(),
            hash_sha256: mobile_hash,
            reason: "test blacklisted mobile".into(),
            added_by: IdentityId("system".into()),
            added_at_ms: GatewayRuntime::now_ms(),
        });
    let server = start_local_gateway_http_server(runtime);

    let (request_status, _resp) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/mobile-otp/request",
        Some(&serde_json::json!({
            "mobile": "13800000001"
        })),
    );
    assert_eq!(request_status, 400);
}

#[test]
fn bearer_session_rejects_shell_message_sender_mismatch() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (request_status, challenge) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/request",
        Some(&serde_json::json!({
            "email": "qa-a@example.com",
            "resident_id": "qa-a"
        })),
    );
    assert_eq!(request_status, 200);
    let code = challenge["dev_code"].as_str().expect("dev otp");

    let (verify_status, verified) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/verify",
        Some(&serde_json::json!({
            "challenge_id": challenge["challenge_id"],
            "code": code,
            "resident_id": "qa-a"
        })),
    );
    assert_eq!(verify_status, 200);
    let session_token = verified["session_token"].as_str().expect("session token");
    let auth_header = format!("Bearer {session_token}");

    let (mismatch_status, mismatch) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        &[("Authorization", auth_header.as_str())],
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-b",
            "text": "冒用 qa-a token"
        })),
    );
    assert_eq!(mismatch_status, 401);
    assert!(
        mismatch["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("does not match authenticated session")
    );

    let (send_status, sent) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        &[("Authorization", auth_header.as_str())],
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "token sender matches"
        })),
    );
    assert_eq!(send_status, 200);
    assert_eq!(sent["delivery_status"], "delivered");
    assert_eq!(sent["sender"], "qa-a");
    let message_id = sent["message_id"].as_str().expect("message id");

    let (edit_mismatch_status, edit_mismatch) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        &[("Authorization", auth_header.as_str())],
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "qa-b",
            "text": "qa-b should not edit with qa-a token"
        })),
    );
    assert_eq!(edit_mismatch_status, 401);
    assert!(
        edit_mismatch["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("does not match authenticated session")
    );

    let (recall_mismatch_status, recall_mismatch) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/shell/message/recall",
        &[("Authorization", auth_header.as_str())],
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "qa-b"
        })),
    );
    assert_eq!(recall_mismatch_status, 401);
    assert!(
        recall_mismatch["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("does not match authenticated session")
    );

    let (notice_mismatch_status, notice_mismatch) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/world-square/notices",
        &[("Authorization", auth_header.as_str())],
        Some(&serde_json::json!({
            "actor_id": "qa-b",
            "title": "冒用治理身份",
            "body": "不应允许 qa-b 用 qa-a token 发布",
            "severity": "Info"
        })),
    );
    assert_eq!(notice_mismatch_status, 401);
    assert!(
        notice_mismatch["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("does not match authenticated session")
    );
}

#[test]
fn auth_session_route_projects_roles_and_capabilities_from_bearer_token() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (request_status, challenge) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/request",
        Some(&serde_json::json!({
            "email": "rsaga@example.com",
            "resident_id": "rsaga"
        })),
    );
    assert_eq!(request_status, 200);
    let code = challenge["dev_code"].as_str().expect("dev otp");

    let (verify_status, verified) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/verify",
        Some(&serde_json::json!({
            "challenge_id": challenge["challenge_id"],
            "code": code,
            "resident_id": "rsaga"
        })),
    );
    assert_eq!(verify_status, 200);
    let session_token = verified["session_token"].as_str().expect("session token");
    let auth_header = format!("Bearer {session_token}");

    let (session_status, session) = http_json_with_headers(
        "GET",
        &server.base_url,
        "/v1/auth/session",
        &[("Authorization", auth_header.as_str())],
        None,
    );
    assert_eq!(session_status, 200);
    assert_eq!(session["authenticated"], true);
    assert_eq!(session["resident_id"], "rsaga");
    assert!(
        session["roles"]
            .as_array()
            .expect("roles")
            .iter()
            .any(|role| role == "resident")
    );
    assert!(
        session["roles"]
            .as_array()
            .expect("roles")
            .iter()
            .any(|role| role == "world_steward")
    );
    assert!(
        session["capabilities"]
            .as_array()
            .expect("capabilities")
            .iter()
            .any(|capability| capability == "shell.message.send")
    );
    assert!(
        session["capabilities"]
            .as_array()
            .expect("capabilities")
            .iter()
            .any(|capability| capability == "world.safety.review")
    );

    let (missing_status, missing) = http_json("GET", &server.base_url, "/v1/auth/session", None);
    assert_eq!(missing_status, 401);
    assert!(
        missing["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("authorization bearer token required")
    );
}

#[test]
fn auth_email_otp_verify_wakes_shell_events_without_waiting_for_timeout() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (request_status, challenge) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/request",
        Some(&serde_json::json!({
            "email": "sse-login@example.com",
            "resident_id": "sse-login"
        })),
    );
    assert_eq!(request_status, 200);
    let code = challenge["dev_code"].as_str().expect("dev otp").to_string();
    let challenge_id = challenge["challenge_id"]
        .as_str()
        .expect("challenge id")
        .to_string();

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=sse-login",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version")
        .to_string();

    let events_base_url = server.base_url.clone();
    let events_path =
        format!("/v1/shell/events?resident_id=sse-login&after={initial_version}&wait_ms=5000");
    let started_at = Instant::now();
    let events_thread =
        thread::spawn(move || http_raw("GET", &events_base_url, &events_path, None));

    thread::sleep(Duration::from_millis(100));
    let (verify_status, verified) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/verify",
        Some(&serde_json::json!({
            "challenge_id": challenge_id,
            "code": code,
            "resident_id": "sse-login"
        })),
    );
    assert_eq!(verify_status, 200);
    assert_eq!(verified["resident_id"], "sse-login");

    let (events_status, _headers, body) = events_thread.join().expect("events thread");
    let elapsed = started_at.elapsed();
    assert_eq!(events_status, 200);
    assert!(
        elapsed < Duration::from_millis(1500),
        "otp verify should notify shell events promptly, elapsed {elapsed:?}"
    );
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert_ne!(payload["state_version"], initial_version);
    assert!(
        payload["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .any(|room| room["id"] == "dm:guide:sse-login")
    );
}

#[test]
fn shell_message_http_route_reports_real_delivery_and_rejects_visitors() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "rsaga",
            "text": "  HTTP 合同消息  ",
            "device_id": "browser",
            "language_tag": "zh-CN"
        })),
    );
    assert_eq!(sent_status, 200);
    assert_eq!(sent["ok"], true);
    assert_eq!(sent["conversation_id"], "room:world:lobby");
    assert_eq!(sent["delivery_status"], "delivered");
    assert_eq!(sent["sender"], "rsaga");
    assert_eq!(sent["text"], "HTTP 合同消息");
    let message_id = sent["message_id"].as_str().expect("message id");
    assert!(!message_id.is_empty());
    assert!(sent["delivered_at_ms"].as_i64().unwrap_or_default() > 0);

    let (state_status, state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=rsaga",
        None,
    );
    assert_eq!(state_status, 200);
    let world_lobby = state["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "room:world:lobby")
        .expect("world lobby");
    assert!(
        world_lobby["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .any(|message| message["message_id"] == message_id
                && message["text"] == "HTTP 合同消息"
                && message["delivery_status"] == "delivered")
    );

    let (visitor_status, visitor_error) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "访客",
            "text": "未登录不应入库",
            "device_id": "browser",
            "language_tag": "zh-CN"
        })),
    );
    assert_eq!(visitor_status, 400);
    assert!(
        visitor_error["Error"]["message"]
            .as_str()
            .expect("error message")
            .contains("login")
    );
}

#[test]
fn message_text_http_routes_share_trim_blank_and_length_contract() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (blank_shell_status, blank_shell) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "rsaga",
            "text": "   \n\t ",
            "device_id": "browser",
            "language_tag": "zh-CN"
        })),
    );
    assert_eq!(blank_shell_status, 400);
    assert_eq!(blank_shell["Error"]["message"], "message text required");

    let too_long_text = "长".repeat(2_001);
    let (too_long_shell_status, too_long_shell) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "rsaga",
            "text": too_long_text,
            "device_id": "browser",
            "language_tag": "zh-CN"
        })),
    );
    assert_eq!(too_long_shell_status, 400);
    assert_eq!(
        too_long_shell["Error"]["message"],
        "message text too long: max 2000 chars"
    );

    let (send_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "rsaga",
            "text": "  可编辑正文  ",
            "device_id": "browser",
            "language_tag": "zh-CN"
        })),
    );
    assert_eq!(send_status, 200);
    assert_eq!(sent["text"], "可编辑正文");
    let message_id = sent["message_id"].as_str().expect("message id");

    let (blank_edit_status, blank_edit) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "rsaga",
            "text": "   "
        })),
    );
    assert_eq!(blank_edit_status, 400);
    assert_eq!(blank_edit["Error"]["message"], "message text required");

    let (too_long_edit_status, too_long_edit) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "rsaga",
            "text": "长".repeat(2_001)
        })),
    );
    assert_eq!(too_long_edit_status, 400);
    assert_eq!(
        too_long_edit["Error"]["message"],
        "message text too long: max 2000 chars"
    );

    let (trimmed_cli_status, trimmed_cli) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "agent:openclaw",
            "to": "user:rsaga",
            "text": "  CLI 正文也要裁剪  ",
            "client_tag": "openclaw"
        })),
    );
    assert_eq!(trimmed_cli_status, 200);
    assert_eq!(trimmed_cli["ok"], true);
    assert_eq!(trimmed_cli["conversation_id"], "dm:openclaw:rsaga");
    let (trimmed_tail_status, trimmed_tail) = http_json(
        "GET",
        &server.base_url,
        "/v1/cli/tail?for=user%3Arsaga&conversation_id=dm%3Aopenclaw%3Arsaga",
        None,
    );
    assert_eq!(trimmed_tail_status, 200);
    assert!(
        trimmed_tail["messages"]
            .as_array()
            .expect("tail messages")
            .iter()
            .any(|entry| entry["text"] == "CLI 正文也要裁剪")
    );

    let (blank_cli_status, blank_cli) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "agent:openclaw",
            "to": "user:rsaga",
            "text": "   ",
            "client_tag": "openclaw"
        })),
    );
    assert_eq!(blank_cli_status, 400);
    assert_eq!(blank_cli["Error"]["message"], "message text required");

    let (too_long_cli_status, too_long_cli) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "agent:openclaw",
            "to": "user:rsaga",
            "text": "长".repeat(2_001),
            "client_tag": "openclaw"
        })),
    );
    assert_eq!(too_long_cli_status, 400);
    assert_eq!(
        too_long_cli["Error"]["message"],
        "message text too long: max 2000 chars"
    );
}

#[test]
fn shell_message_http_route_rejects_empty_body() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (status, body) = http_json("POST", &server.base_url, "/v1/shell/message", None);
    assert_eq!(status, 400);
    let message = body["Error"]["message"].as_str().expect("error message");
    assert!(
        message.contains("decode shell message failed"),
        "expected decode error, got: {message}"
    );
}

#[test]
fn shell_message_http_route_rejects_missing_room_id() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (status, body) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "sender": "rsaga",
            "text": "hello"
        })),
    );
    assert_eq!(status, 400);
    let message = body["Error"]["message"].as_str().expect("error message");
    assert!(
        message.contains("decode shell message failed"),
        "expected decode error, got: {message}"
    );
}

#[test]
fn shell_message_http_route_rejects_missing_sender() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (status, body) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "text": "hello"
        })),
    );
    assert_eq!(status, 400);
    let message = body["Error"]["message"].as_str().expect("error message");
    assert!(
        message.contains("decode shell message failed"),
        "expected decode error, got: {message}"
    );
}

#[test]
fn shell_message_http_route_rejects_missing_text() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (status, body) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "rsaga"
        })),
    );
    assert_eq!(status, 400);
    let message = body["Error"]["message"].as_str().expect("error message");
    assert!(
        message.contains("decode shell message failed"),
        "expected decode error, got: {message}"
    );
}

#[test]
fn shell_message_http_route_rejects_invalid_room_id() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (status, body) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:invalid:nonexistent",
            "sender": "rsaga",
            "text": "hello"
        })),
    );
    assert_eq!(status, 400);
    let message = body["Error"]["message"].as_str().expect("error message");
    assert!(
        message.contains("unknown public room"),
        "expected unknown room error, got: {message}"
    );
}

#[test]
fn shell_message_http_route_accepts_open_city_public_room_posts() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:city:core-harbor:lobby", "actor_id": "rsaga", "actor_id": "rsaga", "actor_id": "rsaga", "actor_id": "rsaga",
            "sender": "qa2",
            "text": "open city lobby should accept this",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);
    assert_eq!(sent["ok"], true);
    assert_eq!(sent["conversation_id"], "room:city:core-harbor:lobby");

    let (state_status, state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa2",
        None,
    );
    assert_eq!(state_status, 200);
    let city_lobby = state["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "room:city:core-harbor:lobby")
        .expect("city lobby");
    assert!(
        city_lobby["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .any(|message| message["sender"] == "qa2"
                && message["text"] == "open city lobby should accept this")
    );
}

#[test]
fn shell_message_http_route_persists_reply_reference() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (root_status, root) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "root message",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(root_status, 200);
    let root_message_id = root["message_id"].as_str().expect("root message id");

    let (reply_status, reply) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-b",
            "text": "reply message",
            "device_id": "browser",
            "language_tag": "en",
            "reply_to_message_id": root_message_id
        })),
    );
    assert_eq!(reply_status, 200);
    assert_eq!(reply["reply_to_message_id"], root_message_id);

    let (state_status, state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-b",
        None,
    );
    assert_eq!(state_status, 200);
    let world_lobby = state["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "room:world:lobby")
        .expect("world lobby");
    let reply_message = world_lobby["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .find(|message| message["text"] == "reply message")
        .expect("reply message");
    assert_eq!(reply_message["reply_to_message_id"], root_message_id);
}

#[test]
fn shell_message_http_route_rejects_unknown_reply_reference() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (reply_status, reply_error) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "invalid reply",
            "device_id": "browser-a",
            "language_tag": "en",
            "reply_to_message_id": "missing-message-id"
        })),
    );
    assert_eq!(reply_status, 400);
    assert_eq!(
        reply_error["Error"]["message"],
        "reply target missing-message-id not found in room:world:lobby"
    );
}

#[test]
fn shell_message_http_route_rejects_cross_room_reply_reference() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (direct_status, direct) = http_json(
        "POST",
        &server.base_url,
        "/v1/direct/open",
        Some(&serde_json::json!({
            "requester_id": "qa-a",
            "requester_device_id": "browser-a",
            "peer_id": "qa-b",
            "peer_device_id": "browser-b"
        })),
    );
    assert_eq!(direct_status, 200);
    let (direct_send_status, direct_sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": direct["conversation_id"],
            "sender": "qa-a",
            "text": "direct root",
            "device_id": "browser-a",
            "language_tag": "en"
        })),
    );
    assert_eq!(direct_send_status, 200);
    let direct_message_id = direct_sent["message_id"]
        .as_str()
        .expect("direct message id");

    let (reply_status, reply_error) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "cross room reply",
            "device_id": "browser-a",
            "language_tag": "en",
            "reply_to_message_id": direct_message_id
        })),
    );
    assert_eq!(reply_status, 400);
    assert_eq!(
        reply_error["Error"]["message"],
        format!("reply target {direct_message_id} not found in room:world:lobby")
    );
}

#[test]
fn shell_message_http_route_roundtrips_two_resident_public_chat() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (a_status, a_sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "qa-a says hello",
            "device_id": "browser-a",
            "language_tag": "en"
        })),
    );
    assert_eq!(a_status, 200);
    assert_eq!(a_sent["delivery_status"], "delivered");
    assert_eq!(a_sent["sender"], "qa-a");
    let a_message_id = a_sent["message_id"].as_str().expect("qa-a message id");

    let (b_status, b_sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-b",
            "text": "qa-b replies",
            "device_id": "browser-b",
            "language_tag": "en"
        })),
    );
    assert_eq!(b_status, 200);
    assert_eq!(b_sent["delivery_status"], "delivered");
    assert_eq!(b_sent["sender"], "qa-b");
    let b_message_id = b_sent["message_id"].as_str().expect("qa-b message id");

    for resident_id in ["qa-a", "qa-b"] {
        let (state_status, state) = http_json(
            "GET",
            &server.base_url,
            &format!("/v1/shell/state?resident_id={resident_id}"),
            None,
        );
        assert_eq!(state_status, 200);
        let world_lobby = state["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .find(|room| room["id"] == "room:world:lobby")
            .expect("world lobby");
        let messages = world_lobby["messages"].as_array().expect("messages");
        assert!(messages.iter().any(|message| {
            message["message_id"] == a_message_id
                && message["sender"] == "qa-a"
                && message["text"] == "qa-a says hello"
        }));
        assert!(messages.iter().any(|message| {
            message["message_id"] == b_message_id
                && message["sender"] == "qa-b"
                && message["text"] == "qa-b replies"
        }));
    }
}

#[test]
fn shell_direct_message_projection_is_visible_only_to_participants() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (direct_status, direct) = http_json(
        "POST",
        &server.base_url,
        "/v1/direct/open",
        Some(&serde_json::json!({
            "requester_id": "qa-a",
            "requester_device_id": "browser-a",
            "peer_id": "qa-b",
            "peer_device_id": "browser-b"
        })),
    );
    assert_eq!(direct_status, 200);
    assert_eq!(direct["conversation_id"], "dm:qa-a:qa-b");

    let (send_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "dm:qa-a:qa-b",
            "sender": "qa-a",
            "text": "private hello",
            "device_id": "browser-a",
            "language_tag": "en"
        })),
    );
    assert_eq!(send_status, 200);
    assert_eq!(sent["delivery_status"], "delivered");
    let message_id = sent["message_id"].as_str().expect("direct message id");

    for (resident_id, self_label, peer_label) in
        [("qa-a", "qa-a", "qa-b"), ("qa-b", "qa-b", "qa-a")]
    {
        let (state_status, state) = http_json(
            "GET",
            &server.base_url,
            &format!("/v1/shell/state?resident_id={resident_id}"),
            None,
        );
        assert_eq!(state_status, 200);
        let direct_room = state["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .find(|room| room["id"] == "dm:qa-a:qa-b")
            .expect("participant direct room");
        assert_eq!(direct_room["kind"], "direct");
        assert_eq!(direct_room["scope"], "private");
        assert_eq!(direct_room["self_label"], self_label);
        assert_eq!(direct_room["peer_label"], peer_label);
        assert!(
            direct_room["messages"]
                .as_array()
                .expect("messages")
                .iter()
                .any(|message| message["message_id"] == message_id
                    && message["sender"] == "qa-a"
                    && message["text"] == "private hello")
        );
    }

    let (outsider_status, outsider_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-c",
        None,
    );
    assert_eq!(outsider_status, 200);
    assert!(
        outsider_state["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .all(|room| room["id"] != "dm:qa-a:qa-b")
    );

    let (cli_tail_status, cli_tail) = http_json(
        "GET",
        &server.base_url,
        "/v1/cli/tail?for=user%3Aqa-c&conversation_id=dm%3Aqa-a%3Aqa-b",
        None,
    );
    assert_eq!(cli_tail_status, 400);
    assert!(
        cli_tail["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("not visible")
    );
}

#[test]
fn direct_open_http_route_rejects_visitor_or_blank_identities() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (visitor_status, visitor_error) = http_json(
        "POST",
        &server.base_url,
        "/v1/direct/open",
        Some(&serde_json::json!({
            "requester_id": "访客",
            "requester_device_id": "browser-a",
            "peer_id": "qa-b",
            "peer_device_id": "browser-b"
        })),
    );
    assert_eq!(visitor_status, 400);
    assert_eq!(
        visitor_error["Error"]["message"],
        "direct session requires authenticated residents"
    );

    let (blank_status, blank_error) = http_json(
        "POST",
        &server.base_url,
        "/v1/direct/open",
        Some(&serde_json::json!({
            "requester_id": "qa-a",
            "requester_device_id": "browser-a",
            "peer_id": "   ",
            "peer_device_id": "browser-b"
        })),
    );
    assert_eq!(blank_status, 400);
    assert_eq!(
        blank_error["Error"]["message"],
        "direct session requires authenticated residents"
    );
}

#[test]
fn shell_direct_message_route_rejects_non_participant_sender() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (direct_status, direct) = http_json(
        "POST",
        &server.base_url,
        "/v1/direct/open",
        Some(&serde_json::json!({
            "requester_id": "qa-a",
            "requester_device_id": "browser-a",
            "peer_id": "qa-b",
            "peer_device_id": "browser-b"
        })),
    );
    assert_eq!(direct_status, 200);
    assert_eq!(direct["conversation_id"], "dm:qa-a:qa-b");

    let (blocked_status, blocked) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "dm:qa-a:qa-b",
            "sender": "qa-c",
            "text": "outsider write should be rejected",
            "device_id": "browser-c",
            "language_tag": "en"
        })),
    );
    assert_eq!(blocked_status, 400);
    assert!(
        blocked["Error"]["message"]
            .as_str()
            .expect("blocked message")
            .contains("not a participant")
    );

    for resident_id in ["qa-a", "qa-b"] {
        let (state_status, state) = http_json(
            "GET",
            &server.base_url,
            &format!("/v1/shell/state?resident_id={resident_id}"),
            None,
        );
        assert_eq!(state_status, 200);
        let direct_room = state["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .find(|room| room["id"] == "dm:qa-a:qa-b")
            .expect("participant direct room");
        assert!(
            direct_room["messages"]
                .as_array()
                .expect("messages")
                .iter()
                .all(|message| message["text"] != "outsider write should be rejected")
        );
    }
}

#[test]
fn direct_open_wakes_peer_shell_events_without_waiting_for_timeout() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-b",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version")
        .to_string();

    let events_base_url = server.base_url.clone();
    let events_path =
        format!("/v1/shell/events?resident_id=qa-b&after={initial_version}&wait_ms=5000");
    let started_at = Instant::now();
    let events_thread =
        thread::spawn(move || http_raw("GET", &events_base_url, &events_path, None));

    thread::sleep(Duration::from_millis(100));
    let (direct_status, direct) = http_json(
        "POST",
        &server.base_url,
        "/v1/direct/open",
        Some(&serde_json::json!({
            "requester_id": "qa-a",
            "requester_device_id": "browser-a",
            "peer_id": "qa-b",
            "peer_device_id": "browser-b"
        })),
    );
    assert_eq!(direct_status, 200);
    assert_eq!(direct["conversation_id"], "dm:qa-a:qa-b");

    let (events_status, _headers, body) = events_thread.join().expect("events thread");
    let elapsed = started_at.elapsed();
    assert_eq!(events_status, 200);
    assert!(
        elapsed < Duration::from_millis(1500),
        "direct open should notify shell events promptly, elapsed {elapsed:?}"
    );
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert_ne!(payload["state_version"], initial_version);
    assert!(
        payload["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .any(|room| room["id"] == "dm:qa-a:qa-b")
    );
}

#[test]
fn shell_events_wait_returns_peer_message_after_send() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-b",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version")
        .to_string();

    let events_base_url = server.base_url.clone();
    let events_path =
        format!("/v1/shell/events?resident_id=qa-b&after={initial_version}&wait_ms=1000");
    let events_thread =
        thread::spawn(move || http_raw("GET", &events_base_url, &events_path, None));

    thread::sleep(Duration::from_millis(100));
    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "wake qa-b events",
            "device_id": "browser-a",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);
    let message_id = sent["message_id"].as_str().expect("message id");

    let (events_status, _headers, body) = events_thread.join().expect("events thread");
    assert_eq!(events_status, 200);
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert_ne!(payload["state_version"], initial_version);
    let world_lobby = payload["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "room:world:lobby")
        .expect("world lobby");
    assert!(
        world_lobby["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .any(|message| message["message_id"] == message_id
                && message["sender"] == "qa-a"
                && message["text"] == "wake qa-b events")
    );
}

#[test]
fn shell_events_wait_returns_direct_message_only_to_participant() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (direct_status, direct) = http_json(
        "POST",
        &server.base_url,
        "/v1/direct/open",
        Some(&serde_json::json!({
            "requester_id": "qa-a",
            "requester_device_id": "browser-a",
            "peer_id": "qa-b",
            "peer_device_id": "browser-b"
        })),
    );
    assert_eq!(direct_status, 200);
    assert_eq!(direct["conversation_id"], "dm:qa-a:qa-b");

    let (peer_initial_status, peer_initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-b",
        None,
    );
    assert_eq!(peer_initial_status, 200);
    let peer_initial_version = peer_initial_state["state_version"]
        .as_str()
        .expect("peer initial state version")
        .to_string();

    let (outsider_initial_status, outsider_initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-c",
        None,
    );
    assert_eq!(outsider_initial_status, 200);
    let outsider_initial_version = outsider_initial_state["state_version"]
        .as_str()
        .expect("outsider initial state version")
        .to_string();

    let events_base_url = server.base_url.clone();
    let events_path =
        format!("/v1/shell/events?resident_id=qa-b&after={peer_initial_version}&wait_ms=1000");
    let events_thread =
        thread::spawn(move || http_raw("GET", &events_base_url, &events_path, None));

    thread::sleep(Duration::from_millis(100));
    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "dm:qa-a:qa-b",
            "sender": "qa-a",
            "text": "direct wake qa-b only",
            "device_id": "browser-a",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);
    assert_eq!(sent["delivery_status"], "delivered");
    let message_id = sent["message_id"].as_str().expect("message id");

    let (events_status, _headers, body) = events_thread.join().expect("events thread");
    assert_eq!(events_status, 200);
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert_ne!(payload["state_version"], peer_initial_version);
    let direct_room = payload["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "dm:qa-a:qa-b")
        .expect("participant direct room");
    assert!(
        direct_room["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .any(|message| message["message_id"] == message_id
                && message["sender"] == "qa-a"
                && message["text"] == "direct wake qa-b only"
                && message["delivery_status"] == "delivered")
    );

    let (outsider_events_status, _outsider_headers, outsider_body) = http_raw(
        "GET",
        &server.base_url,
        &format!("/v1/shell/events?resident_id=qa-c&after={outsider_initial_version}&wait_ms=10"),
        None,
    );
    assert_eq!(outsider_events_status, 200);
    let outsider_data = outsider_body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("outsider sse shell-state payload");
    let outsider_payload: serde_json::Value =
        serde_json::from_str(outsider_data).expect("outsider shell state json");
    assert_eq!(outsider_payload["state_version"], outsider_initial_version);
    assert!(
        outsider_payload["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .all(|room| room["id"] != "dm:qa-a:qa-b")
    );
}

#[test]
fn shell_events_wait_keeps_direct_edit_and_recall_scoped_to_participants() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (direct_status, direct) = http_json(
        "POST",
        &server.base_url,
        "/v1/direct/open",
        Some(&serde_json::json!({
            "requester_id": "qa-a",
            "requester_device_id": "browser-a",
            "peer_id": "qa-b",
            "peer_device_id": "browser-b"
        })),
    );
    assert_eq!(direct_status, 200);
    assert_eq!(direct["conversation_id"], "dm:qa-a:qa-b");

    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "dm:qa-a:qa-b",
            "sender": "qa-a",
            "text": "direct before edit",
            "device_id": "browser-a",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);
    assert_eq!(sent["delivery_status"], "delivered");
    let message_id = sent["message_id"].as_str().expect("message id").to_string();

    let (peer_before_edit_status, peer_before_edit) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-b",
        None,
    );
    assert_eq!(peer_before_edit_status, 200);
    let peer_before_edit_version = peer_before_edit["state_version"]
        .as_str()
        .expect("peer before edit state version")
        .to_string();

    let (outsider_before_edit_status, outsider_before_edit) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-c",
        None,
    );
    assert_eq!(outsider_before_edit_status, 200);
    let outsider_before_edit_version = outsider_before_edit["state_version"]
        .as_str()
        .expect("outsider before edit state version")
        .to_string();

    let peer_edit_events_url =
        format!("/v1/shell/events?resident_id=qa-b&after={peer_before_edit_version}&wait_ms=1000");
    let peer_edit_events_base_url = server.base_url.clone();
    let peer_edit_events_thread = thread::spawn(move || {
        http_raw(
            "GET",
            &peer_edit_events_base_url,
            &peer_edit_events_url,
            None,
        )
    });
    thread::sleep(Duration::from_millis(50));

    let (edit_status, edit) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        Some(&serde_json::json!({
            "room_id": "dm:qa-a:qa-b",
            "message_id": message_id,
            "actor": "qa-a",
            "text": "direct after edit"
        })),
    );
    assert_eq!(edit_status, 200);
    assert_eq!(edit["edit_status"], "edited");
    let edited_at_ms = edit["edited_at_ms"].as_i64().expect("edited at ms");

    let (peer_edit_events_status, _headers, peer_edit_body) = peer_edit_events_thread
        .join()
        .expect("peer edit events thread");
    assert_eq!(peer_edit_events_status, 200);
    let peer_edit_data = peer_edit_body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("peer edit sse shell-state payload");
    let peer_edit_payload: serde_json::Value =
        serde_json::from_str(peer_edit_data).expect("peer edit shell state json");
    assert_ne!(peer_edit_payload["state_version"], peer_before_edit_version);
    let peer_direct_room = peer_edit_payload["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "dm:qa-a:qa-b")
        .expect("peer direct room");
    let peer_edited_message = peer_direct_room["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .find(|message| message["message_id"] == message_id)
        .expect("peer edited direct message");
    assert_eq!(peer_edited_message["text"], "direct after edit");
    assert_eq!(peer_edited_message["is_edited"], true);
    assert_eq!(peer_edited_message["edited_by"], "qa-a");
    assert_eq!(peer_edited_message["edited_at_ms"], edited_at_ms);

    let (outsider_edit_events_status, _outsider_edit_headers, outsider_edit_body) = http_raw(
        "GET",
        &server.base_url,
        &format!(
            "/v1/shell/events?resident_id=qa-c&after={outsider_before_edit_version}&wait_ms=10"
        ),
        None,
    );
    assert_eq!(outsider_edit_events_status, 200);
    let outsider_edit_data = outsider_edit_body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("outsider edit sse shell-state payload");
    let outsider_edit_payload: serde_json::Value =
        serde_json::from_str(outsider_edit_data).expect("outsider edit shell state json");
    assert_eq!(
        outsider_edit_payload["state_version"],
        outsider_before_edit_version
    );
    assert!(
        outsider_edit_payload["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .all(|room| room["id"] != "dm:qa-a:qa-b")
    );

    let (peer_before_recall_status, peer_before_recall) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-b",
        None,
    );
    assert_eq!(peer_before_recall_status, 200);
    let peer_before_recall_version = peer_before_recall["state_version"]
        .as_str()
        .expect("peer before recall state version")
        .to_string();

    let (outsider_before_recall_status, outsider_before_recall) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-c",
        None,
    );
    assert_eq!(outsider_before_recall_status, 200);
    let outsider_before_recall_version = outsider_before_recall["state_version"]
        .as_str()
        .expect("outsider before recall state version")
        .to_string();

    let peer_recall_events_url = format!(
        "/v1/shell/events?resident_id=qa-b&after={peer_before_recall_version}&wait_ms=1000"
    );
    let peer_recall_events_base_url = server.base_url.clone();
    let peer_recall_events_thread = thread::spawn(move || {
        http_raw(
            "GET",
            &peer_recall_events_base_url,
            &peer_recall_events_url,
            None,
        )
    });
    thread::sleep(Duration::from_millis(50));

    let (recall_status, recall) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/recall",
        Some(&serde_json::json!({
            "room_id": "dm:qa-a:qa-b",
            "message_id": message_id,
            "actor": "qa-a"
        })),
    );
    assert_eq!(recall_status, 200);
    assert_eq!(recall["recall_status"], "recalled");
    let recalled_at_ms = recall["recalled_at_ms"].as_i64().expect("recalled at ms");
    assert!(recalled_at_ms >= edited_at_ms);

    let (peer_recall_events_status, _headers, peer_recall_body) = peer_recall_events_thread
        .join()
        .expect("peer recall events thread");
    assert_eq!(peer_recall_events_status, 200);
    let peer_recall_data = peer_recall_body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("peer recall sse shell-state payload");
    let peer_recall_payload: serde_json::Value =
        serde_json::from_str(peer_recall_data).expect("peer recall shell state json");
    assert_ne!(
        peer_recall_payload["state_version"],
        peer_before_recall_version
    );
    let peer_direct_room = peer_recall_payload["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "dm:qa-a:qa-b")
        .expect("peer direct room after recall");
    let peer_recalled_message = peer_direct_room["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .find(|message| message["message_id"] == message_id)
        .expect("peer recalled direct message");
    assert_eq!(peer_recalled_message["text"], "消息已撤回");
    assert_eq!(peer_recalled_message["is_recalled"], true);
    assert_eq!(peer_recalled_message["recalled_by"], "qa-a");
    assert_eq!(peer_recalled_message["recalled_at_ms"], recalled_at_ms);
    assert_eq!(peer_recalled_message["is_edited"], true);
    assert_eq!(peer_recalled_message["edited_by"], "qa-a");
    assert_eq!(peer_recalled_message["edited_at_ms"], edited_at_ms);

    let (outsider_recall_events_status, _outsider_recall_headers, outsider_recall_body) = http_raw(
        "GET",
        &server.base_url,
        &format!(
            "/v1/shell/events?resident_id=qa-c&after={outsider_before_recall_version}&wait_ms=10"
        ),
        None,
    );
    assert_eq!(outsider_recall_events_status, 200);
    let outsider_recall_data = outsider_recall_body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("outsider recall sse shell-state payload");
    let outsider_recall_payload: serde_json::Value =
        serde_json::from_str(outsider_recall_data).expect("outsider recall shell state json");
    assert_eq!(
        outsider_recall_payload["state_version"],
        outsider_before_recall_version
    );
    assert!(
        outsider_recall_payload["rooms"]
            .as_array()
            .expect("rooms")
            .iter()
            .all(|room| room["id"] != "dm:qa-a:qa-b")
    );
}

#[test]
fn shell_events_wait_returns_peer_message_after_edit() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "before sse edit",
            "device_id": "browser-a",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);
    let message_id = sent["message_id"].as_str().expect("message id").to_string();

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-b",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version")
        .to_string();

    let events_url =
        format!("/v1/shell/events?resident_id=qa-b&after={initial_version}&wait_ms=1000");
    let events_base_url = server.base_url.clone();
    let events_thread = thread::spawn(move || http_raw("GET", &events_base_url, &events_url, None));
    thread::sleep(Duration::from_millis(50));

    let (edit_status, edit) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "qa-a",
            "text": "after sse edit"
        })),
    );
    assert_eq!(edit_status, 200);
    assert_eq!(edit["edit_status"], "edited");

    let (events_status, _headers, body) = events_thread.join().expect("events thread");
    assert_eq!(events_status, 200);
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert_ne!(payload["state_version"], initial_version);
    let world_lobby = payload["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "room:world:lobby")
        .expect("world lobby");
    let edited_message = world_lobby["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .find(|message| message["message_id"] == message_id)
        .expect("edited message");
    assert_eq!(edited_message["text"], "after sse edit");
    assert_eq!(edited_message["is_edited"], true);
    assert_eq!(edited_message["edited_by"], "qa-a");
}

#[test]
fn shell_events_wait_returns_peer_message_after_recall() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "before sse recall",
            "device_id": "browser-a",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);
    let message_id = sent["message_id"].as_str().expect("message id").to_string();

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-b",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version")
        .to_string();

    let events_url =
        format!("/v1/shell/events?resident_id=qa-b&after={initial_version}&wait_ms=1000");
    let events_base_url = server.base_url.clone();
    let events_thread = thread::spawn(move || http_raw("GET", &events_base_url, &events_url, None));
    thread::sleep(Duration::from_millis(50));

    let (recall_status, recall) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/recall",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "qa-a"
        })),
    );
    assert_eq!(recall_status, 200);
    assert_eq!(recall["recall_status"], "recalled");

    let (events_status, _headers, body) = events_thread.join().expect("events thread");
    assert_eq!(events_status, 200);
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert_ne!(payload["state_version"], initial_version);
    let world_lobby = payload["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "room:world:lobby")
        .expect("world lobby");
    let recalled_message = world_lobby["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .find(|message| message["message_id"] == message_id)
        .expect("recalled message");
    assert_eq!(recalled_message["text"], "消息已撤回");
    assert_eq!(recalled_message["is_recalled"], true);
    assert_eq!(recalled_message["recalled_by"], "qa-a");
}

#[test]
fn shell_events_wait_returns_recalled_state_after_edited_message() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "before edit then recall",
            "device_id": "browser-a",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);
    let message_id = sent["message_id"].as_str().expect("message id").to_string();

    let (edit_status, edit) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "qa-a",
            "text": "edited before recall"
        })),
    );
    assert_eq!(edit_status, 200);
    assert_eq!(edit["edit_status"], "edited");
    let edited_at_ms = edit["edited_at_ms"].as_i64().expect("edited at ms");

    let (initial_status, initial_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-b",
        None,
    );
    assert_eq!(initial_status, 200);
    let initial_version = initial_state["state_version"]
        .as_str()
        .expect("initial state version")
        .to_string();

    let events_url =
        format!("/v1/shell/events?resident_id=qa-b&after={initial_version}&wait_ms=1000");
    let events_base_url = server.base_url.clone();
    let events_thread = thread::spawn(move || http_raw("GET", &events_base_url, &events_url, None));
    thread::sleep(Duration::from_millis(50));

    let (recall_status, recall) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/recall",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "qa-a"
        })),
    );
    assert_eq!(recall_status, 200);
    assert_eq!(recall["recall_status"], "recalled");
    let recalled_at_ms = recall["recalled_at_ms"].as_i64().expect("recalled at ms");
    assert!(
        recalled_at_ms >= edited_at_ms,
        "recall should happen after edit in contract timestamps"
    );

    let (events_status, _headers, body) = events_thread.join().expect("events thread");
    assert_eq!(events_status, 200);
    let data = body
        .split("event: shell-state\ndata: ")
        .nth(1)
        .and_then(|value| value.split("\n\n").next())
        .expect("sse shell-state payload");
    let payload: serde_json::Value = serde_json::from_str(data).expect("shell state json");
    assert_ne!(payload["state_version"], initial_version);
    let world_lobby = payload["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "room:world:lobby")
        .expect("world lobby");
    let recalled_message = world_lobby["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .find(|message| message["message_id"] == message_id)
        .expect("recalled edited message");
    assert_eq!(recalled_message["text"], "消息已撤回");
    assert_eq!(recalled_message["is_recalled"], true);
    assert_eq!(recalled_message["recalled_by"], "qa-a");
    assert_eq!(recalled_message["recalled_at_ms"], recalled_at_ms);
    assert_eq!(recalled_message["is_edited"], true);
    assert_eq!(recalled_message["edited_by"], "qa-a");
    assert_eq!(recalled_message["edited_at_ms"], edited_at_ms);
}

#[test]
fn shell_message_http_route_recalls_own_message_without_deleting_audit_entry() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "recall me",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);
    let message_id = sent["message_id"].as_str().expect("message id");

    let (before_recall_status, before_recall_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(before_recall_status, 200);
    let before_recall_version = before_recall_state["state_version"]
        .as_str()
        .expect("before recall state version")
        .to_string();

    let (recall_status, recall) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/recall",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "qa-a"
        })),
    );
    assert_eq!(recall_status, 200);
    assert_eq!(recall["ok"], true);
    assert_eq!(recall["message_id"], message_id);
    assert_eq!(recall["recall_status"], "recalled");

    let (state_status, state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(state_status, 200);
    assert_ne!(
        state["state_version"]
            .as_str()
            .expect("after recall state version"),
        before_recall_version
    );
    let world_lobby = state["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "room:world:lobby")
        .expect("world lobby");
    let recalled_message = world_lobby["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .find(|message| message["message_id"] == message_id)
        .expect("recalled message");
    assert_eq!(recalled_message["is_recalled"], true);
    assert_eq!(recalled_message["recalled_by"], "qa-a");
    assert_eq!(recalled_message["text"], "消息已撤回");
}

#[test]
fn shell_message_http_route_edits_own_message_without_changing_message_id() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "before edit",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);
    let message_id = sent["message_id"].as_str().expect("message id");

    let (before_edit_status, before_edit_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(before_edit_status, 200);
    let before_edit_version = before_edit_state["state_version"]
        .as_str()
        .expect("before edit state version")
        .to_string();

    let (edit_status, edit) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "qa-a",
            "text": "after edit"
        })),
    );
    assert_eq!(edit_status, 200);
    assert_eq!(edit["ok"], true);
    assert_eq!(edit["message_id"], message_id);
    assert_eq!(edit["edit_status"], "edited");
    assert_eq!(edit["text"], "after edit");

    let (state_status, state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(state_status, 200);
    assert_ne!(
        state["state_version"]
            .as_str()
            .expect("after edit state version"),
        before_edit_version
    );
    let world_lobby = state["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "room:world:lobby")
        .expect("world lobby");
    let edited_message = world_lobby["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .find(|message| message["message_id"] == message_id)
        .expect("edited message");
    assert_eq!(edited_message["is_edited"], true);
    assert_eq!(edited_message["edited_by"], "qa-a");
    assert_eq!(edited_message["text"], "after edit");
}

#[test]
fn shell_message_http_route_rejects_edit_from_non_sender() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "owned by qa-a",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);
    let message_id = sent["message_id"].as_str().expect("message id");

    let (edit_status, edit_error) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "qa-b",
            "text": "qa-b should not overwrite"
        })),
    );
    assert_eq!(edit_status, 400);
    assert_eq!(
        edit_error["Error"]["message"],
        "edit message failed: only the original sender can edit this message"
    );

    let (state_status, state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(state_status, 200);
    let world_lobby = state["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "room:world:lobby")
        .expect("world lobby");
    let message = world_lobby["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .find(|message| message["message_id"] == message_id)
        .expect("original message");
    assert_eq!(message["text"], "owned by qa-a");
    assert_eq!(message["is_edited"], false);
}

#[test]
fn shell_message_http_route_rejects_edit_after_recall() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "recall before edit",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);
    let message_id = sent["message_id"].as_str().expect("message id");

    let (recall_status, recall) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/recall",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "qa-a"
        })),
    );
    assert_eq!(recall_status, 200);
    assert_eq!(recall["ok"], true);

    let (edit_status, edit_error) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "message_id": message_id,
            "actor": "qa-a",
            "text": "recalled message should not change"
        })),
    );
    assert_eq!(edit_status, 400);
    assert_eq!(
        edit_error["Error"]["message"],
        "edit message failed: recalled messages cannot be edited"
    );

    let (state_status, state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(state_status, 200);
    let world_lobby = state["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == "room:world:lobby")
        .expect("world lobby");
    let message = world_lobby["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .find(|message| message["message_id"] == message_id)
        .expect("recalled message");
    assert_eq!(message["is_recalled"], true);
    assert_eq!(message["is_edited"], false);
    assert_eq!(message["text"], "消息已撤回");
}

#[test]
fn read_http_routes_return_stable_gateway_projection_contract() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    register_resident(&mut runtime, "read-contract-user");
    runtime
        .join_city(JoinCityRequest {
            city: "core-harbor".into(),
            resident_id: "read-contract-user".into(),
        })
        .expect("join default city");
    let server = start_local_gateway_http_server(runtime);

    let (bootstrap_status, bootstrap) =
        http_json("GET", &server.base_url, "/v1/shell/bootstrap", None);
    assert_eq!(bootstrap_status, 200);
    assert_eq!(bootstrap["gateway_base_url"], server.base_url);

    let (world_status, world) = http_json("GET", &server.base_url, "/v1/world", None);
    assert_eq!(world_status, 200);
    assert_eq!(world["world"]["title"], "Lobster World");
    assert!(!world["cities"].as_array().expect("world cities").is_empty());
    assert!(
        world["public_rooms"]
            .as_array()
            .expect("world public rooms")
            .iter()
            .any(|room| room["room_id"] == "room:city:core-harbor:lobby")
    );

    let (cities_status, cities) = http_json("GET", &server.base_url, "/v1/cities", None);
    assert_eq!(cities_status, 200);
    assert!(
        cities
            .as_array()
            .expect("cities")
            .iter()
            .any(|city| city["profile"]["slug"] == "core-harbor")
    );

    let (residents_status, residents) = http_json("GET", &server.base_url, "/v1/residents", None);
    assert_eq!(residents_status, 200);
    assert!(
        residents
            .as_array()
            .expect("residents")
            .iter()
            .any(|resident| resident["resident_id"] == "read-contract-user")
    );

    let (directory_status, directory) =
        http_json("GET", &server.base_url, "/v1/world-directory", None);
    assert_eq!(directory_status, 200);
    assert_eq!(directory["title"], "Lobster World");
    assert!(
        directory["cities"]
            .as_array()
            .expect("directory cities")
            .iter()
            .any(|city| city["slug"] == "core-harbor")
    );

    let (snapshot_status, snapshot) =
        http_json("GET", &server.base_url, "/v1/world-snapshot", None);
    assert_eq!(snapshot_status, 200);
    assert!(
        snapshot["meta"]["checksum_sha256"]
            .as_str()
            .expect("snapshot checksum")
            .len()
            > 8
    );
    assert_eq!(
        snapshot["payload"]["governance"]["world"]["title"],
        "Lobster World"
    );

    let (mirrors_status, mirrors) = http_json("GET", &server.base_url, "/v1/world-mirrors", None);
    assert_eq!(mirrors_status, 200);
    assert!(!mirrors.as_array().expect("mirrors").is_empty());

    let (export_missing_status, export_missing) =
        http_json("GET", &server.base_url, "/v1/export", None);
    assert_eq!(export_missing_status, 400);
    assert!(
        export_missing["Error"]["message"]
            .as_str()
            .expect("missing export error")
            .contains("resident_id")
    );

    let (sent_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:city:core-harbor:lobby", "actor_id": "rsaga", "actor_id": "rsaga", "actor_id": "rsaga", "actor_id": "rsaga",
            "sender": "read-contract-user",
            "text": "read route export message",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(sent_status, 200);
    assert_eq!(sent["ok"], true);

    let (export_status, export) = http_json(
        "GET",
        &server.base_url,
        "/v1/export?resident_id=read-contract-user&include_public=true&format=jsonl",
        None,
    );
    assert_eq!(export_status, 200);
    assert_eq!(export["resident_id"], "read-contract-user");
    assert_eq!(export["format"], "jsonl");
    assert!(
        export["content"]
            .as_str()
            .expect("export content")
            .contains("read route export message")
    );
}

#[test]
fn merge_governance_snapshots_adds_upstream_city_catalog() {
    let temp = tempdir().expect("temp dir");
    let mut primary = GatewayRuntime::open(temp.path().join("gateway"), 64, None)
        .expect("runtime")
        .governance_snapshot();
    let mut secondary = primary.clone();

    primary
        .cities
        .retain(|city| city.profile.slug == "core-harbor");
    primary.public_rooms.retain(|room| room.slug == "lobby");

    let upstream_city_id = CityId("city:aurora".into());
    secondary.cities.push(CityState {
        profile: CityProfile {
            city_id: upstream_city_id.clone(),
            world_id: secondary.world.world_id.clone(),
            slug: "aurora".into(),
            title: "Aurora".into(),
            description: "remote city".into(),
            scene: Some(GatewayRuntime::default_city_scene("aurora", "Aurora")),
            resident_portable: true,
            approval_required: true,
            public_room_discovery_enabled: true,
            federation_policy: FederationPolicy::Open,
            relay_budget_hint: RelayBudgetHint::Balanced,
            retention_policy: GatewayRuntime::default_city_retention_policy(),
        },
        features: GatewayRuntime::default_city_features(),
    });
    secondary.memberships.push(CityMembership {
        city_id: upstream_city_id.clone(),
        resident_id: IdentityId("remote-lord".into()),
        role: CityRole::Lord,
        state: MembershipState::Active,
        joined_at_ms: 1_763_560_000_001,
        added_by: None,
    });
    secondary.public_rooms.push(PublicRoomRecord {
        room_id: ConversationId("room:city:aurora:lobby".into()),
        city_id: upstream_city_id,
        slug: "lobby".into(),
        title: "Aurora Lobby".into(),
        description: "remote room".into(),
        scene: Some(GatewayRuntime::default_public_room_scene(
            "aurora",
            "lobby",
            "Aurora Lobby",
        )),
        created_by: IdentityId("remote-lord".into()),
        created_at_ms: 1_763_560_000_002,
        frozen: false,
    });

    let merged = GatewayRuntime::merge_governance_snapshots(primary, secondary);
    assert!(
        merged
            .cities
            .iter()
            .any(|city| city.profile.slug == "aurora")
    );
    assert!(
        merged
            .memberships
            .iter()
            .any(|membership| membership.resident_id.0 == "remote-lord")
    );
    assert!(
        merged
            .public_rooms
            .iter()
            .any(|room| room.room_id.0 == "room:city:aurora:lobby")
    );
}

#[test]
fn disconnect_provider_returns_local_mode() {
    let temp = tempdir().expect("temp dir");
    let storage_root = temp.path().join("disconnect-provider");
    let mut runtime = GatewayRuntime::open(&storage_root, 8, None).expect("open runtime");
    runtime
        .set_upstream_provider_url(Some("http://127.0.0.1:9999".into()))
        .expect("persist provider");

    let provider = runtime.disconnect_provider().expect("disconnect provider");
    assert_eq!(provider.mode, "local-memory");
    assert!(provider.base_url.is_none());
}

#[test]
fn provider_url_contract_allows_loopback_dev_http_but_rejects_remote_http() {
    let temp = tempdir().expect("temp dir");
    let mut runtime =
        GatewayRuntime::open(temp.path().join("gateway"), 8, None).expect("open runtime");

    runtime
        .set_upstream_provider_url(Some("http://127.0.0.1:9999/".into()))
        .expect("loopback HTTP should be allowed in dev/test mode");
    assert_eq!(
        runtime.upstream_base_url.as_deref(),
        Some("http://127.0.0.1:9999")
    );

    let error = runtime
        .set_upstream_provider_url(Some("http://provider.example".into()))
        .expect_err("remote HTTP should be rejected");
    assert!(error.contains("must use HTTPS"));
}

#[test]
fn provider_url_contract_rejects_loopback_http_in_production_mode() {
    let temp = tempdir().expect("temp dir");
    let mut runtime =
        GatewayRuntime::open(temp.path().join("gateway"), 8, None).expect("open runtime");
    runtime.set_dev_auth_bypass_for_tests(false);

    let error = runtime
        .set_upstream_provider_url(Some("http://127.0.0.1:9999".into()))
        .expect_err("production mode should reject loopback HTTP");
    assert!(error.contains("must use HTTPS"));
}

#[test]
fn provider_config_load_fails_closed_for_remote_http() {
    let temp = tempdir().expect("temp dir");
    let storage_root = temp.path().join("gateway");
    std::fs::create_dir_all(&storage_root).expect("create gateway state dir");
    std::fs::write(
        storage_root.join("provider-config.json"),
        r#"{"upstream_gateway_url":"http://provider.example","mirror_sources":[]}"#,
    )
    .expect("write invalid provider config");

    let error = GatewayRuntime::open(&storage_root, 8, None)
        .expect_err("invalid persisted provider URL should fail closed");
    assert!(error.contains("must use HTTPS"));
}

#[test]
fn provider_direct_and_mirror_http_routes_roundtrip_gateway_contract() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);
    let (upstream_base_url, upstream_state, upstream_running, upstream_handle) =
        start_mock_upstream_gateway();
    {
        let remote_runtime =
            GatewayRuntime::open(temp.path().join("remote-gateway"), 64, None).expect("remote");
        let remote_bundle = remote_runtime
            .federation_read_plan()
            .world_snapshot_bundle();
        let mut shared = upstream_state.lock().expect("lock upstream state");
        shared.world_snapshot_bundle = Some(remote_bundle.clone());
        shared.governance_snapshot = Some(remote_bundle.payload.governance.clone());
    }

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (initial_status, initial_provider) =
            http_json("GET", &server.base_url, "/v1/provider", None);
        assert_eq!(initial_status, 200);
        assert_eq!(initial_provider["mode"], "local-memory");
        assert_eq!(initial_provider["base_url"], serde_json::Value::Null);

        let (connect_status, connected_provider) = http_json(
            "POST",
            &server.base_url,
            "/v1/provider/connect",
            Some(&serde_json::json!({
                "provider_url": upstream_base_url
            })),
        );
        assert_eq!(connect_status, 200);
        assert_eq!(connected_provider["mode"], "remote-gateway");
        assert_eq!(connected_provider["base_url"], upstream_base_url);
        assert_eq!(connected_provider["reachable"], true);
        assert!(
            upstream_state
                .lock()
                .expect("lock upstream state")
                .healthcheck_count
                >= 1
        );

        let (direct_status, direct) = http_json(
            "POST",
            &server.base_url,
            "/v1/direct/open",
            Some(&serde_json::json!({
                "requester_id": "rsaga",
                "requester_device_id": "desktop-1",
                "peer_id": "builder",
                "peer_device_id": "browser"
            })),
        );
        assert_eq!(direct_status, 200);
        assert_eq!(direct["conversation_id"], "dm:builder:rsaga");
        assert_eq!(direct["kind"], "Direct");
        assert_eq!(
            direct["members"].as_array().expect("direct members").len(),
            2
        );

        let (mirror_post_status, mirror_sources) = http_json(
            "POST",
            &server.base_url,
            "/v1/world-mirror-sources",
            Some(&serde_json::json!({
                "base_url": "http://mirror.example.invalid/",
                "enabled": false
            })),
        );
        assert_eq!(mirror_post_status, 200);
        assert_eq!(
            mirror_sources
                .as_array()
                .expect("mirror source config")
                .len(),
            1
        );
        assert_eq!(
            mirror_sources[0]["base_url"],
            "http://mirror.example.invalid"
        );
        assert_eq!(mirror_sources[0]["enabled"], false);

        let (mirror_get_status, mirror_statuses) =
            http_json("GET", &server.base_url, "/v1/world-mirror-sources", None);
        assert_eq!(mirror_get_status, 200);
        assert!(
            mirror_statuses
                .as_array()
                .expect("mirror source statuses")
                .iter()
                .any(|item| item["base_url"] == "http://mirror.example.invalid"
                    && item["enabled"] == false)
        );

        let (disconnect_status, disconnected_provider) =
            http_json("POST", &server.base_url, "/v1/provider/disconnect", None);
        assert_eq!(disconnect_status, 200);
        assert_eq!(disconnected_provider["mode"], "local-memory");
        assert_eq!(disconnected_provider["base_url"], serde_json::Value::Null);
        assert_eq!(disconnected_provider["reachable"], true);
    }));

    upstream_running.store(false, Ordering::SeqCst);
    let _ = TcpStream::connect(upstream_base_url.trim_start_matches("http://"));
    upstream_handle.join().expect("join upstream gateway");
    outcome.expect("provider direct and mirror http route contract");
}

#[test]
fn provider_and_auth_state_roundtrip_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("provider-auth-roundtrip");
    let (base_url, _state, running, handle) = start_mock_upstream_gateway();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        {
            let mut runtime = GatewayRuntime::open(&root, 8, None).expect("open runtime");
            runtime
                .connect_provider(ConnectProviderRequest {
                    provider_url: base_url.clone(),
                })
                .expect("connect provider");
            runtime
                .add_world_mirror_source(AddWorldMirrorSourceRequest {
                    base_url: "http://mirror.example.invalid/".into(),
                    enabled: Some(true),
                })
                .expect("add mirror source");
            runtime
                .request_email_otp(RequestEmailOtpRequest {
                    email: "roundtrip@example.com".into(),
                    mobile: Some("+86 13800138009".into()),
                    device_physical_address: Some("AA:BB:CC:DD:EE:09".into()),
                    resident_id: Some("roundtrip-user".into()),
                    nickname: None,
                })
                .expect("request email otp");
        }

        let runtime = GatewayRuntime::open(&root, 8, None).expect("reopen runtime");
        assert_eq!(
            runtime.upstream_base_url.as_deref(),
            Some(base_url.as_str())
        );
        assert_eq!(runtime.mirror_sources.len(), 1);
        assert_eq!(
            runtime.mirror_sources[0].base_url,
            "http://mirror.example.invalid"
        );
        assert_eq!(runtime.registrations.len(), 0);
        assert_eq!(runtime.email_otp_challenges.len(), 1);
        assert_eq!(
            runtime.email_otp_challenges[0]
                .desired_resident_id
                .as_ref()
                .map(|id| id.0.as_str()),
            Some("roundtrip-user")
        );
    }));

    running.store(false, Ordering::SeqCst);
    let _ = TcpStream::connect(base_url.trim_start_matches("http://"));
    handle.join().expect("stop mock upstream gateway");
    outcome.expect("provider/auth state should survive restart");
}

#[test]
fn resident_directory_groups_memberships_by_identity() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .create_city(CreateCityRequest {
            slug: Some("aurora".into()),
            title: "Aurora".into(),
            description: "remote city".into(),
            lord_id: "alice".into(),
            approval_required: Some(true),
            public_room_discovery_enabled: Some(true),
            federation_policy: None,
        })
        .expect("create city");
    register_resident(&mut runtime, "guest-01");
    runtime
        .join_city(JoinCityRequest {
            city: "aurora".into(),
            resident_id: "guest-01".into(),
        })
        .expect("join city");

    let directory = GatewayRuntime::resident_directory(&runtime.governance_snapshot());
    let alice = directory
        .iter()
        .find(|entry| entry.resident_id == "alice")
        .expect("alice entry");
    assert!(alice.active_cities.contains(&"aurora".into()));
    assert!(alice.roles.contains(&"Lord".into()));

    let guest = directory
        .iter()
        .find(|entry| entry.resident_id == "guest-01")
        .expect("guest entry");
    assert!(guest.pending_cities.contains(&"aurora".into()));
}

#[test]
fn direct_session_bootstrap_creates_private_conversation() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let group = runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "rsaga".into(),
            requester_device_id: Some("desktop-1".into()),
            peer_id: "builder".into(),
            peer_device_id: Some("browser".into()),
        })
        .expect("direct session should open");
    assert_eq!(group.scope, ConversationScope::Private);
    assert_eq!(group.members.len(), 2);
    assert!(
        runtime
            .timeline_store
            .active_conversations()
            .iter()
            .any(|conversation| conversation.conversation_id.0 == "dm:builder:rsaga")
    );
}

#[test]
fn direct_session_persists_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime
            .open_direct_session(OpenDirectSessionRequest {
                requester_id: "rsaga".into(),
                requester_device_id: Some("desktop-1".into()),
                peer_id: "builder".into(),
                peer_device_id: None,
            })
            .expect("direct session should open");
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
    let group = runtime
        .secure_sessions
        .group_state(&ConversationId("dm:builder:rsaga".into()))
        .expect("secure session should persist");
    assert_eq!(group.kind, crypto_mls::MlsGroupKind::Direct);
}

#[test]
fn direct_open_http_route_never_returns_or_persists_group_key() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");
    let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (status, response) = http_json(
        "POST",
        &server.base_url,
        "/v1/direct/open",
        Some(&serde_json::json!({
            "requester_id": "qa-a",
            "requester_device_id": "browser-a",
            "peer_id": "qa-b",
            "peer_device_id": "browser-b"
        })),
    );
    assert_eq!(status, 200);
    assert!(response.get("group_key").is_none());
    assert_eq!(response["conversation_id"], "dm:qa-a:qa-b");

    let snapshot = std::fs::read_to_string(root.join("secure-sessions.json"))
        .expect("sealed secure session snapshot");
    assert!(!snapshot.contains("group_key"));
    let snapshot_json: serde_json::Value =
        serde_json::from_str(&snapshot).expect("sealed snapshot json");
    assert_eq!(snapshot_json["schema_version"], 1);
    assert_eq!(snapshot_json["algorithm"], "AES-256-GCM-HKDF-SHA256");
    assert!(
        snapshot_json["ciphertext_hex"]
            .as_str()
            .is_some_and(|value| { !value.is_empty() && value.len() % 2 == 0 })
    );
}

#[test]
fn legacy_plaintext_secure_session_snapshot_is_migrated_to_sealed_storage() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");
    std::fs::create_dir_all(&root).expect("gateway state dir");
    let conversation = ConversationId("dm:legacy-a:legacy-b".into());
    let mut legacy_manager = SkeletonSecureSessionManager::new();
    legacy_manager
        .bootstrap_direct(
            &conversation,
            vec![
                MlsMember::device("legacy-a", "device-a"),
                MlsMember::device("legacy-b", "device-b"),
            ],
        )
        .expect("legacy session");
    std::fs::write(
        root.join("secure-sessions.json"),
        serde_json::to_vec(&legacy_manager.snapshot()).expect("legacy snapshot"),
    )
    .expect("write legacy snapshot");

    let runtime = GatewayRuntime::open(&root, 64, None).expect("migrate legacy snapshot");
    assert!(runtime.secure_sessions.group_state(&conversation).is_some());
    let migrated = std::fs::read_to_string(root.join("secure-sessions.json"))
        .expect("migrated secure session snapshot");
    assert!(!migrated.contains("group_key"));
    assert!(migrated.contains("AES-256-GCM-HKDF-SHA256"));
}

#[test]
fn direct_session_reuses_existing_legacy_conversation_id() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let group = runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "rsaga".into(),
            requester_device_id: Some("desktop-1".into()),
            peer_id: "builder".into(),
            peer_device_id: Some("browser".into()),
        })
        .expect("direct session should open");

    assert_eq!(group.conversation_id.0, "dm:builder:rsaga");
    let direct_conversations = runtime
        .timeline_store
        .active_conversations()
        .into_iter()
        .filter(|conversation| conversation.kind == ConversationKind::Direct)
        .count();
    assert_eq!(direct_conversations, 1);
}

#[test]
fn direct_session_reuses_existing_session_when_participants_are_reversed() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let first = runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "rsaga".into(),
            requester_device_id: Some("desktop-1".into()),
            peer_id: "builder".into(),
            peer_device_id: Some("browser".into()),
        })
        .expect("first direct session should open");

    let second = runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "builder".into(),
            requester_device_id: Some("browser".into()),
            peer_id: "rsaga".into(),
            peer_device_id: Some("desktop-1".into()),
        })
        .expect("second direct session should reuse first");

    assert_eq!(second.conversation_id, first.conversation_id);
    assert_eq!(second.epoch, first.epoch);
    let direct_conversations = runtime
        .timeline_store
        .active_conversations()
        .into_iter()
        .filter(|conversation| conversation.kind == ConversationKind::Direct)
        .count();
    assert_eq!(direct_conversations, 1);
}

#[test]
fn default_scene_image_layer_uses_canonical_16_9_ratio() {
    let layer = GatewayRuntime::scene_image_layer("private-room-loft", true);
    assert_eq!(layer.aspect_ratio_permyriad, 5_625);
}

#[test]
fn shell_scene_update_persists_gateway_owned_room_layers() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "rsaga".into(),
            requester_device_id: Some("desktop-1".into()),
            peer_id: "builder".into(),
            peer_device_id: Some("browser".into()),
        })
        .expect("direct session should open");

    let response = runtime
        .update_shell_scene(UpdateShellSceneRequest {
            room_id: "dm:builder:rsaga".into(),
            actor: "rsaga".into(),
            image_layer: Some(Some(SceneImageLayer {
                layer_id: "image-layer".into(),
                preset: "resident-custom-loft".into(),
                asset_hint: "resident-custom-loft-v1".into(),
                aspect_ratio_permyriad: 5_625,
                owner_editable: true,
                day_image_url: None,
                night_image_url: None,
            })),
            hotspot_layer: Some(Some(SceneHotspotLayer {
                layer_id: "resident-hotspots".into(),
                coordinate_system: "scene-permyriad".into(),
                owner_editable: true,
                hotspots: vec![SceneHotspot {
                    hotspot_id: "desk".into(),
                    label: "写作桌".into(),
                    sprite_hint: "desk".into(),
                    interaction_hint: "进入桌面私聊".into(),
                    x_permyriad: 2_000,
                    y_permyriad: 3_000,
                    width_permyriad: 900,
                    height_permyriad: 800,
                }],
            })),
        })
        .expect("participant should update editable scene");

    assert!(response.ok);
    assert_eq!(response.conversation_id, "dm:builder:rsaga");
    assert_eq!(
        response.image_layer.expect("image layer").preset,
        "resident-custom-loft"
    );

    let state = serde_json::to_value(runtime.shell_state()).expect("serialize shell state");
    let direct_scene = state["scene_render"]["scenes"]
        .as_array()
        .expect("scene render array")
        .iter()
        .find(|scene| scene["conversation_id"] == "dm:builder:rsaga")
        .expect("direct scene");
    assert_eq!(
        direct_scene["image_layer"]["preset"],
        "resident-custom-loft"
    );
    assert_eq!(
        direct_scene["hotspot_layer"]["hotspots"][0]["label"],
        "写作桌"
    );
}

#[test]
fn shell_scene_update_rejects_non_participant_actor() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "rsaga".into(),
            requester_device_id: Some("desktop-1".into()),
            peer_id: "builder".into(),
            peer_device_id: Some("browser".into()),
        })
        .expect("direct session should open");

    let result = runtime.update_shell_scene(UpdateShellSceneRequest {
        room_id: "dm:builder:rsaga".into(),
        actor: "stranger".into(),
        image_layer: Some(Some(SceneImageLayer {
            layer_id: "image-layer".into(),
            preset: "stranger-room".into(),
            asset_hint: "stranger-room".into(),
            aspect_ratio_permyriad: 5_625,
            owner_editable: true,
            day_image_url: None,
            night_image_url: None,
        })),
        hotspot_layer: None,
    });

    assert_eq!(
        result.expect_err("non participant should be rejected"),
        "scene update actor is not a room participant"
    );
}

#[test]
fn personal_room_scene_update_is_owner_only() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .ensure_personal_room(&IdentityId("alice".into()))
        .expect("personal room should exist");

    let request = |actor: &str| UpdateShellSceneRequest {
        room_id: "home:alice".into(),
        actor: actor.into(),
        image_layer: None,
        hotspot_layer: None,
    };

    assert_eq!(
        runtime
            .update_shell_scene(request("bob"))
            .expect_err("non-owner should be rejected"),
        "only the personal room owner can edit scene"
    );
    assert!(
        runtime
            .update_shell_scene(request("alice"))
            .expect("owner should edit personal room scene")
            .ok
    );
}

#[test]
fn admin_personal_room_scene_update_is_owner_only() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .ensure_personal_room(&IdentityId("alice".into()))
        .expect("personal room should exist");

    let request = |actor: &str| AdminUpdateSceneRequest {
        room_id: "home:alice".into(),
        actor_id: Some(actor.into()),
        image_layer: None,
        hotspot_layer: None,
    };

    assert_eq!(
        runtime
            .admin_update_scene(request("bob"))
            .expect_err("non-owner should be rejected"),
        "only the personal room owner can edit private room scenes"
    );
    assert!(
        runtime
            .admin_update_scene(request("alice"))
            .expect("owner should edit personal room scene")
            .ok
    );
}

#[test]
fn shell_scene_update_http_route_returns_updated_scene_contract() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "rsaga".into(),
            requester_device_id: Some("desktop-1".into()),
            peer_id: "builder".into(),
            peer_device_id: Some("browser".into()),
        })
        .expect("direct session should open");
    let server = start_local_gateway_http_server(runtime);

    let (status, payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/scene",
        Some(&serde_json::json!({
            "room_id": "dm:builder:rsaga",
            "actor": "builder",
            "image_layer": {
                "layer_id": "image-layer",
                "preset": "builder-study",
                "asset_hint": "builder-study-v1",
            "aspect_ratio_permyriad": 5625,
                "owner_editable": true
            },
            "hotspot_layer": {
                "layer_id": "builder-hotspots",
                "coordinate_system": "scene-permyriad",
                "owner_editable": true,
                "hotspots": [{
                    "hotspot_id": "window",
                    "label": "夜窗",
                    "sprite_hint": "window",
                    "interaction_hint": "查看窗外",
                    "x_permyriad": 1200,
                    "y_permyriad": 1800,
                    "width_permyriad": 700,
                    "height_permyriad": 700
                }]
            }
        })),
    );

    assert_eq!(status, 200);
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["image_layer"]["preset"], "builder-study");
    assert_eq!(payload["hotspot_layer"]["hotspots"][0]["label"], "夜窗");
}

#[test]
fn cli_address_parser_accepts_user_agent_and_room() {
    let user = parse_cli_address("user:rsaga").expect("user address should parse");
    assert_eq!(user, CliAddress::User(IdentityId("rsaga".into())));

    let agent = parse_cli_address("agent:codex").expect("agent address should parse");
    assert_eq!(agent, CliAddress::Agent(IdentityId("codex".into())));

    let room = parse_cli_address("room:city:core-harbor:lobby").expect("room address should parse");
    assert_eq!(
        room,
        CliAddress::Room(ConversationId("room:city:core-harbor:lobby".into()))
    );
}

#[test]
fn cli_direct_mapping_normalizes_dm_pair_order() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let forward = runtime.resolve_cli_direct_conversation_id(
        &IdentityId("openclaw".into()),
        &IdentityId("rsaga".into()),
    );
    let reverse = runtime.resolve_cli_direct_conversation_id(
        &IdentityId("rsaga".into()),
        &IdentityId("openclaw".into()),
    );

    assert_eq!(forward, reverse);
    assert_eq!(forward.0, "dm:openclaw:rsaga");
}

#[test]
fn cli_direct_mapping_reuses_reverse_legacy_conversation_id() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let reverse_legacy = ConversationId("dm:rsaga:openclaw".into());
    runtime
        .ensure_direct_conversation(
            &reverse_legacy,
            &[IdentityId("rsaga".into()), IdentityId("openclaw".into())],
        )
        .expect("seed reverse legacy direct conversation");

    let resolved = runtime.resolve_cli_direct_conversation_id(
        &IdentityId("openclaw".into()),
        &IdentityId("rsaga".into()),
    );

    assert_eq!(resolved, reverse_legacy);
}

#[test]
fn cli_address_parser_rejects_invalid_prefix() {
    let error = parse_cli_address("foo:bar").expect_err("invalid prefix should fail");
    assert!(error.contains("unsupported cli address"));
}

#[test]
fn cli_send_to_user_opens_direct_conversation_and_publishes() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let response = runtime
        .send_cli_message(CliSendRequest {
            from: "agent:openclaw".into(),
            to: "user:rsaga".into(),
            text: "晚上一起吃饭吗".into(),
            client_tag: Some("openclaw".into()),
        })
        .expect("cli direct send should succeed");

    assert_eq!(response.conversation_id, "dm:openclaw:rsaga");

    let messages = runtime
        .timeline_store
        .recent_messages(&ConversationId(response.conversation_id.clone()), 8);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].envelope.body.plain_text, "晚上一起吃饭吗");
}

#[test]
fn cli_send_trims_text_and_rejects_blank_or_too_long_text() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let response = runtime
        .send_cli_message(CliSendRequest {
            from: "agent:openclaw".into(),
            to: "user:rsaga".into(),
            text: "  去掉前后空白  ".into(),
            client_tag: Some("openclaw".into()),
        })
        .expect("trimmed cli send should succeed");
    let messages = runtime
        .timeline_store
        .recent_messages(&ConversationId(response.conversation_id.clone()), 8);
    assert_eq!(messages[0].envelope.body.plain_text, "去掉前后空白");

    let blank_error = runtime
        .send_cli_message(CliSendRequest {
            from: "agent:openclaw".into(),
            to: "user:rsaga".into(),
            text: "   \n\t ".into(),
            client_tag: Some("openclaw".into()),
        })
        .expect_err("blank CLI send should fail");
    assert!(blank_error.contains("message text required"));

    let too_long_error = runtime
        .send_cli_message(CliSendRequest {
            from: "agent:openclaw".into(),
            to: "user:rsaga".into(),
            text: "长".repeat(2_001),
            client_tag: Some("openclaw".into()),
        })
        .expect_err("too long CLI send should fail");
    assert!(too_long_error.contains("max 2000 chars"));
}

#[test]
fn cli_http_routes_roundtrip_send_inbox_rooms_and_tail_contract() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (missing_status, missing) = http_json("GET", &server.base_url, "/v1/cli/inbox", None);
    assert_eq!(missing_status, 400);
    assert_eq!(missing["message"], "missing for");

    let (send_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "agent:openclaw",
            "to": "user:rsaga",
            "text": "今晚一起吃饭吗",
            "client_tag": "openclaw"
        })),
    );
    assert_eq!(send_status, 200);
    assert_eq!(sent["ok"], true);
    assert_eq!(sent["conversation_id"], "dm:openclaw:rsaga");

    let (inbox_status, inbox) = http_json(
        "GET",
        &server.base_url,
        "/v1/cli/inbox?for=user%3Arsaga",
        None,
    );
    assert_eq!(inbox_status, 200);
    assert_eq!(inbox["identity"], "user:rsaga");
    assert!(
        inbox["conversations"]
            .as_array()
            .expect("inbox conversations")
            .iter()
            .any(|item| item["conversation_id"] == "dm:openclaw:rsaga"
                && item["last_message_preview"] == "今晚一起吃饭吗")
    );

    let (rooms_status, rooms) = http_json(
        "GET",
        &server.base_url,
        "/v1/cli/rooms?for=user%3Arsaga",
        None,
    );
    assert_eq!(rooms_status, 200);
    assert_eq!(rooms["identity"], "user:rsaga");
    assert!(
        rooms["entries"]
            .as_array()
            .expect("room entries")
            .iter()
            .any(|item| item["conversation_id"] == "dm:openclaw:rsaga")
    );

    let (tail_status, tail) = http_json(
        "GET",
        &server.base_url,
        "/v1/cli/tail?for=user%3Arsaga&conversation_id=dm%3Aopenclaw%3Arsaga",
        None,
    );
    assert_eq!(tail_status, 200);
    assert_eq!(tail["identity"], "user:rsaga");
    assert_eq!(tail["conversation_id"], "dm:openclaw:rsaga");
    assert!(
        tail["messages"]
            .as_array()
            .expect("tail messages")
            .iter()
            .any(|item| item["sender"] == "openclaw" && item["text"] == "今晚一起吃饭吗")
    );
}

#[test]
fn cli_search_filters_results_to_visible_conversations() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    for (recipient, text) in [
        ("user:rsaga", "private-rsaga-needle"),
        ("user:alice", "private-alice-needle"),
    ] {
        let (status, body) = http_json(
            "POST",
            &server.base_url,
            "/v1/cli/send",
            Some(&serde_json::json!({
                "from": "agent:openclaw",
                "to": recipient,
                "text": text,
                "client_tag": "search-test"
            })),
        );
        assert_eq!(status, 200, "seed send failed: {body}");
    }

    let (rsaga_status, rsaga_results) = http_json(
        "GET",
        &server.base_url,
        "/v1/cli/search?for=user%3Arsaga&q=private-rsaga-needle",
        None,
    );
    assert_eq!(rsaga_status, 200);
    assert_eq!(rsaga_results.as_array().unwrap().len(), 1);
    assert_eq!(rsaga_results[0]["text"], "private-rsaga-needle");

    let (alice_status, alice_results) = http_json(
        "GET",
        &server.base_url,
        "/v1/cli/search?for=user%3Aalice&q=private-rsaga-needle",
        None,
    );
    assert_eq!(alice_status, 200);
    assert!(alice_results.as_array().unwrap().is_empty());

    let (invisible_status, invisible_body) = http_json(
        "GET",
        &server.base_url,
        "/v1/cli/search?for=user%3Aalice&q=needle&room_id=dm%3Aopenclaw%3Arsaga",
        None,
    );
    assert_eq!(invisible_status, 400);
    assert!(
        invisible_body["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("not visible")
    );
}

#[test]
fn cli_send_http_requires_bound_agent_token_when_dev_bypass_disabled() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    runtime.set_agent_token_for_tests("agent:openclaw", "sidecar-secret");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({
        "from": "agent:openclaw",
        "to": "user:rsaga",
        "text": "authenticated sidecar send",
        "client_tag": "openclaw"
    });

    let (missing_status, _) = http_json("POST", &server.base_url, "/v1/cli/send", Some(&body));
    assert_eq!(missing_status, 401);

    let (invalid_status, _) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        &[("Authorization", "Bearer wrong-secret")],
        Some(&body),
    );
    assert_eq!(invalid_status, 401);

    let (accepted_status, accepted) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        &[("Authorization", "Bearer sidecar-secret")],
        Some(&body),
    );
    assert_eq!(accepted_status, 200);
    assert_eq!(accepted["ok"], true);
}

#[test]
fn cli_scoped_read_routes_require_bound_bearer_without_dev_bypass() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    runtime.set_agent_token_for_tests("agent:openclaw", "sidecar-secret");
    let server = start_local_gateway_http_server(runtime);

    for path in [
        "/v1/cli/inbox?for=agent%3Aopenclaw",
        "/v1/cli/rooms?for=agent%3Aopenclaw",
        "/v1/cli/tail?for=agent%3Aopenclaw",
        "/v1/cli/search?for=agent%3Aopenclaw&q=secret",
    ] {
        let (missing_status, _, missing_body) = http_raw("GET", &server.base_url, path, None);
        assert_eq!(
            missing_status, 401,
            "scoped CLI read {path} must reject missing Bearer auth: {missing_body}"
        );

        let (valid_status, _, valid_body) = http_raw_with_headers(
            "GET",
            &server.base_url,
            path,
            &[("Authorization", "Bearer sidecar-secret")],
            None,
        );
        assert_eq!(
            valid_status, 200,
            "bound sidecar token should read {path}: {valid_body}"
        );
    }
}

#[test]
fn cli_agent_edit_and_recall_require_and_accept_bound_sidecar_token() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    runtime.set_agent_token_for_tests("agent:openclaw", "sidecar-secret");
    let server = start_local_gateway_http_server(runtime);
    let send_body = serde_json::json!({
        "from": "agent:openclaw",
        "to": "user:rsaga",
        "text": "sidecar edit seed",
        "client_tag": "openclaw"
    });
    let (send_status, sent) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        &[("Authorization", "Bearer sidecar-secret")],
        Some(&send_body),
    );
    assert_eq!(send_status, 200);
    let message_id = sent["message_id"].as_str().expect("message id");
    let edit_body = serde_json::json!({
        "room_id": "dm:openclaw:rsaga",
        "message_id": message_id,
        "actor": "openclaw",
        "actor_address": "agent:openclaw",
        "text": "sidecar edit committed"
    });

    let (missing_edit_status, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        Some(&edit_body),
    );
    assert_eq!(missing_edit_status, 401);

    let (edit_status, edited) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        &[("Authorization", "Bearer sidecar-secret")],
        Some(&edit_body),
    );
    assert_eq!(edit_status, 200);
    assert_eq!(edited["edit_status"], "edited");

    let recall_body = serde_json::json!({
        "room_id": "dm:openclaw:rsaga",
        "message_id": message_id,
        "actor": "openclaw",
        "actor_address": "agent:openclaw"
    });
    let (recall_status, recalled) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/shell/message/recall",
        &[("Authorization", "Bearer sidecar-secret")],
        Some(&recall_body),
    );
    assert_eq!(recall_status, 200);
    assert_eq!(recalled["recall_status"], "recalled");
}

#[test]
fn cli_tail_and_inbox_project_recall_and_edit_metadata() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (edited_send_status, edited_send) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "user:rsaga",
            "to": "agent:openclaw",
            "text": "cli before edit",
            "client_tag": "cli"
        })),
    );
    assert_eq!(edited_send_status, 200);
    let conversation_id = edited_send["conversation_id"]
        .as_str()
        .expect("conversation id");
    let edited_message_id = edited_send["message_id"].as_str().expect("message id");

    let (edit_status, edit) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        Some(&serde_json::json!({
            "room_id": conversation_id,
            "message_id": edited_message_id,
            "actor": "rsaga",
            "text": "cli after edit"
        })),
    );
    assert_eq!(edit_status, 200);
    assert_eq!(edit["ok"], true);

    let (recalled_send_status, recalled_send) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "user:rsaga",
            "to": "agent:openclaw",
            "text": "cli before recall",
            "client_tag": "cli"
        })),
    );
    assert_eq!(recalled_send_status, 200);
    let recalled_message_id = recalled_send["message_id"].as_str().expect("message id");

    let (recall_status, recall) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/recall",
        Some(&serde_json::json!({
            "room_id": conversation_id,
            "message_id": recalled_message_id,
            "actor": "rsaga"
        })),
    );
    assert_eq!(recall_status, 200);
    assert_eq!(recall["ok"], true);

    let (tail_status, tail) = http_json(
        "GET",
        &server.base_url,
        "/v1/cli/tail?for=agent%3Aopenclaw&conversation_id=dm%3Aopenclaw%3Arsaga",
        None,
    );
    assert_eq!(tail_status, 200);
    let tail_messages = tail["messages"].as_array().expect("tail messages");
    let edited_message = tail_messages
        .iter()
        .find(|message| message["message_id"] == edited_message_id)
        .expect("edited message");
    assert_eq!(edited_message["text"], "cli after edit");
    assert_eq!(edited_message["is_edited"], true);
    assert_eq!(edited_message["edited_by"], "rsaga");
    assert_eq!(edited_message["is_recalled"], false);

    let recalled_message = tail_messages
        .iter()
        .find(|message| message["message_id"] == recalled_message_id)
        .expect("recalled message");
    assert_eq!(recalled_message["text"], "消息已撤回");
    assert_eq!(recalled_message["is_recalled"], true);
    assert_eq!(recalled_message["recalled_by"], "rsaga");
    assert_eq!(recalled_message["is_edited"], false);

    let (inbox_status, inbox) = http_json(
        "GET",
        &server.base_url,
        "/v1/cli/inbox?for=agent%3Aopenclaw",
        None,
    );
    assert_eq!(inbox_status, 200);
    let conversation = inbox["conversations"]
        .as_array()
        .expect("conversations")
        .iter()
        .find(|item| item["conversation_id"] == conversation_id)
        .expect("conversation");
    assert_eq!(conversation["last_message_preview"], "消息已撤回");
}

#[test]
fn cli_send_to_nonexistent_room_is_rejected() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (status, body) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "user:qa-a",
            "to": "room:nonexistent:fake",
            "text": "hello"
        })),
    );
    assert_eq!(status, 400);
    assert!(
        body["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("unknown public room")
    );
}

#[test]
fn cli_send_rejects_blank_text_at_http_level() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (status, body) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "user:qa-a",
            "to": "user:qa-b",
            "text": "   "
        })),
    );
    assert_eq!(status, 400);
    assert_eq!(body["Error"]["message"], "message text required");
}

#[test]
fn cli_edit_nonexistent_message_is_rejected() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (send_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "user:qa-a",
            "to": "user:qa-b",
            "text": "first message"
        })),
    );
    assert_eq!(send_status, 200);
    assert_eq!(sent["ok"], true);
    let conversation_id = sent["conversation_id"].as_str().expect("conversation id");

    let (edit_status, edit) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/edit",
        Some(&serde_json::json!({
            "room_id": conversation_id,
            "message_id": "msg:nonexistent:000",
            "actor": "qa-a",
            "text": "edited"
        })),
    );
    assert_eq!(edit_status, 400);
    assert!(
        edit["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("not found")
    );
}

#[test]
fn cli_recall_nonexistent_message_is_rejected() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (send_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "user:qa-a",
            "to": "user:qa-b",
            "text": "first message"
        })),
    );
    assert_eq!(send_status, 200);
    assert_eq!(sent["ok"], true);
    let conversation_id = sent["conversation_id"].as_str().expect("conversation id");

    let (recall_status, recall) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/recall",
        Some(&serde_json::json!({
            "room_id": conversation_id,
            "message_id": "msg:nonexistent:000",
            "actor": "qa-a"
        })),
    );
    assert_eq!(recall_status, 400);
    assert!(
        recall["Error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("not found")
    );
}

#[test]
fn cli_double_recall_is_idempotent() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (send_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "user:qa-a",
            "to": "user:qa-b",
            "text": "to be recalled"
        })),
    );
    assert_eq!(send_status, 200);
    assert_eq!(sent["ok"], true);
    let conversation_id = sent["conversation_id"].as_str().expect("conversation id");
    let message_id = sent["message_id"].as_str().expect("message id");

    let (first_recall_status, first_recall) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/recall",
        Some(&serde_json::json!({
            "room_id": conversation_id,
            "message_id": message_id,
            "actor": "qa-a"
        })),
    );
    assert_eq!(first_recall_status, 200);
    assert_eq!(first_recall["ok"], true);

    let (second_recall_status, _second_recall) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message/recall",
        Some(&serde_json::json!({
            "room_id": conversation_id,
            "message_id": message_id,
            "actor": "qa-a"
        })),
    );
    // Double recall should be idempotent — either 200 or 400.
    assert!(second_recall_status == 200 || second_recall_status == 400);

    let (tail_status, tail) = http_json(
        "GET",
        &server.base_url,
        "/v1/cli/tail?for=user%3Aqa-a&conversation_id=dm%3Aqa-a%3Aqa-b",
        None,
    );
    assert_eq!(tail_status, 200);
    let recalled_message = tail["messages"]
        .as_array()
        .expect("tail messages")
        .iter()
        .find(|message| message["message_id"] == message_id)
        .expect("recalled message");
    assert_eq!(recalled_message["is_recalled"], true);
}

#[test]
fn cli_send_to_room_appends_message_into_existing_room() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let room_id = ConversationId("room:world:lobby".into());
    let before = runtime.timeline_store.recent_messages(&room_id, 64).len();

    let response = runtime
        .send_cli_message(CliSendRequest {
            from: "user:rsaga".into(),
            to: "room:world:lobby".into(),
            text: "今晚八点开会".into(),
            client_tag: None,
        })
        .expect("cli room send should succeed");

    assert_eq!(response.conversation_id, "room:world:lobby");

    let after_messages = runtime.timeline_store.recent_messages(&room_id, 64);
    assert_eq!(after_messages.len(), before + 1);
    assert_eq!(
        after_messages
            .last()
            .expect("last room message")
            .envelope
            .body
            .plain_text,
        "今晚八点开会"
    );
}

#[test]
fn cli_send_to_default_city_room_accepts_user_surface_identity() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let room_id = ConversationId("room:city:core-harbor:lobby".into());
    let before = runtime.timeline_store.recent_messages(&room_id, 64).len();

    let response = runtime
        .send_cli_message(CliSendRequest {
            from: "user:tiyan".into(),
            to: "room:city:core-harbor:lobby".into(),
            text: "我也在大厅里".into(),
            client_tag: None,
        })
        .expect("default user surface identity should post into city lobby");

    assert_eq!(response.conversation_id, "room:city:core-harbor:lobby");

    let after_messages = runtime.timeline_store.recent_messages(&room_id, 64);
    assert_eq!(after_messages.len(), before + 1);
    assert_eq!(
        after_messages
            .last()
            .expect("last city room message")
            .envelope
            .sender
            .0,
        "tiyan"
    );
    assert_eq!(
        after_messages
            .last()
            .expect("last city room message")
            .envelope
            .body
            .plain_text,
        "我也在大厅里"
    );
}

#[test]
fn cli_send_to_seeded_governance_room_succeeds_for_admin_identity() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let room_id = ConversationId("room:city:aurora-hub:announcements".into());
    let before = runtime.timeline_store.recent_messages(&room_id, 64).len();

    let response = runtime
        .send_cli_message(CliSendRequest {
            from: "user:rsaga".into(),
            to: "room:city:aurora-hub:announcements".into(),
            text: "城务提醒已更新".into(),
            client_tag: None,
        })
        .expect("seeded governance room should accept admin send");

    assert_eq!(
        response.conversation_id,
        "room:city:aurora-hub:announcements"
    );

    let after_messages = runtime.timeline_store.recent_messages(&room_id, 64);
    assert_eq!(after_messages.len(), before + 1);
    assert_eq!(
        after_messages
            .last()
            .expect("last governance room message")
            .envelope
            .body
            .plain_text,
        "城务提醒已更新"
    );
}

#[test]
fn cli_send_rejects_unknown_room_targets() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let error = runtime
        .send_cli_message(CliSendRequest {
            from: "user:rsaga".into(),
            to: "room:city:phantom-city:ghost-room".into(),
            text: "今晚一起吃饭吗".into(),
            client_tag: None,
        })
        .expect_err("unknown room target should fail");

    assert!(error.contains("unknown public room"));
}

#[test]
fn cli_send_rejects_visitor_surface_identity_before_login() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let room_id = ConversationId("room:world:lobby".into());
    let before = runtime.timeline_store.recent_messages(&room_id, 64).len();

    let room_error = runtime
        .send_cli_message(CliSendRequest {
            from: "user:访客".into(),
            to: "room:world:lobby".into(),
            text: "visitor should not post".into(),
            client_tag: None,
        })
        .expect_err("visitor CLI room send should be rejected before login");

    assert!(room_error.contains("login"));
    assert_eq!(
        runtime.timeline_store.recent_messages(&room_id, 64).len(),
        before
    );

    let direct_error = runtime
        .send_cli_message(CliSendRequest {
            from: "user:访客".into(),
            to: "user:rsaga".into(),
            text: "visitor should not DM".into(),
            client_tag: None,
        })
        .expect_err("visitor CLI direct send should be rejected before login");

    assert!(direct_error.contains("login"));
}

#[test]
fn cli_inbox_returns_recent_conversation_summaries_for_identity() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .send_cli_message(CliSendRequest {
            from: "agent:openclaw".into(),
            to: "user:rsaga".into(),
            text: "今晚一起吃饭吗".into(),
            client_tag: Some("openclaw".into()),
        })
        .expect("seed direct message");

    let inbox = runtime
        .cli_inbox_for(&CliAddress::User(IdentityId("rsaga".into())))
        .expect("cli inbox should build");

    assert_eq!(inbox.identity, "user:rsaga");
    assert!(inbox.conversations.iter().any(|conversation| {
        conversation.conversation_id == "dm:openclaw:rsaga"
            && conversation.last_message_preview == "今晚一起吃饭吗"
            && conversation.title == "正在与 openclaw 聊天"
            && conversation.subtitle == "居所直达 · 你与 openclaw"
            && conversation.meta == "消息数：1"
            && conversation.scope == "private"
            && conversation.kind_hint.as_deref() == Some("居所")
            && conversation.list_summary.as_deref()
                == Some("正在与 openclaw 聊天 · 2 人 · 1 条消息")
            && conversation.status_line.as_deref() == Some("居所直达")
            && conversation.chat_status_summary.as_deref() == Some("可直接继续回复")
            && conversation.queue_summary.as_deref()
                == Some("1 条访客提醒待处理 · 1 条巡视提醒待看")
            && conversation.overview_summary.as_deref() == Some("正在与 openclaw 聊天")
            && conversation.context_summary.as_deref()
                == Some("旺财 会帮你记住与 openclaw 的留言和提醒，适合续聊、记任务和直接追问。")
            && conversation.preview_text.as_deref() == Some("今晚一起吃饭吗")
            && conversation
                .last_activity_label
                .as_deref()
                .is_some_and(|value| value.starts_with("openclaw · "))
            && conversation.activity_time_label.is_some()
            && conversation.self_label.as_deref() == Some("rsaga")
            && conversation.peer_label.as_deref() == Some("openclaw")
            && conversation.participant_label.as_deref() == Some("你与 openclaw")
            && conversation.route_label.as_deref() == Some("居所直达")
            && conversation.thread_headline.as_deref() == Some("正在与 openclaw 聊天")
            && conversation.member_count == Some(2)
            && conversation
                .search_terms
                .iter()
                .any(|term| term == "openclaw")
            && conversation.scene_banner.as_deref() == Some("个人房间")
            && conversation.room_variant.as_deref() == Some("private-room-loft")
            && conversation.room_motif.as_deref() == Some("木地板、工作台、沙发与像素人物")
    }));
}

#[test]
fn cli_rooms_lists_visible_room_and_direct_threads() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .send_cli_message(CliSendRequest {
            from: "agent:openclaw".into(),
            to: "user:rsaga".into(),
            text: "今晚一起吃饭吗".into(),
            client_tag: Some("openclaw".into()),
        })
        .expect("seed direct message");

    let rooms = runtime
        .cli_rooms_for(&CliAddress::User(IdentityId("rsaga".into())))
        .expect("cli rooms should build");

    assert!(rooms.entries.iter().any(|entry| {
        entry.conversation_id == "room:world:lobby"
            && entry.kind == "room"
            && entry.scope == "cross_city_shared"
            && entry.title == "世界广场"
            && entry.subtitle.starts_with("最近发言：")
            && entry
                .list_summary
                .as_deref()
                .is_some_and(|value| value.starts_with("世界广场 · "))
            && entry.status_line.as_deref() == Some("跨城共响线")
            && entry.chat_status_summary.as_deref() == Some("群聊当前比较安静")
            && entry.overview_summary.as_deref() == Some("跨城共响回廊 · 群聊")
            && entry.context_summary.as_deref()
                == Some("巡逻犬 会盯住公共提醒和巡视结果，适合看公告、围观和跨城讨论。")
            && entry
                .preview_text
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && entry
                .last_activity_label
                .as_deref()
                .is_some_and(|value| value.starts_with("builder · "))
            && entry.activity_time_label.as_deref() == Some("5m ago")
            && entry.participant_label.as_deref() == Some("跨城共响回廊")
            && entry.route_label.as_deref() == Some("跨城共响线")
            && entry.thread_headline.as_deref() == Some("跨城共响回廊 · 群聊")
            && entry
                .scene_banner
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && entry.scene_summary.as_deref() == Some("公共房间 · 公共频道、公告板与像素座位区")
            && entry
                .room_variant
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && entry.room_motif.as_deref() == Some("公共频道、公告板与像素座位区")
    }));
    assert!(rooms.entries.iter().any(|entry| {
        entry.conversation_id == "dm:openclaw:rsaga"
            && entry.kind == "direct"
            && entry.scope == "private"
            && entry.title == "正在与 openclaw 聊天"
            && entry.subtitle == "居所直达 · 你与 openclaw"
            && entry.list_summary.as_deref() == Some("正在与 openclaw 聊天 · 2 人 · 1 条消息")
            && entry.status_line.as_deref() == Some("居所直达")
            && entry.chat_status_summary.as_deref() == Some("可直接继续回复")
            && entry.queue_summary.as_deref() == Some("1 条访客提醒待处理 · 1 条巡视提醒待看")
            && entry.overview_summary.as_deref() == Some("正在与 openclaw 聊天")
            && entry.context_summary.as_deref()
                == Some("旺财 会帮你记住与 openclaw 的留言和提醒，适合续聊、记任务和直接追问。")
            && entry.preview_text.as_deref() == Some("今晚一起吃饭吗")
            && entry
                .last_activity_label
                .as_deref()
                .is_some_and(|value| value.starts_with("openclaw · "))
            && entry.activity_time_label.is_some()
            && entry.self_label.as_deref() == Some("rsaga")
            && entry.peer_label.as_deref() == Some("openclaw")
            && entry.participant_label.as_deref() == Some("你与 openclaw")
            && entry.route_label.as_deref() == Some("居所直达")
            && entry.thread_headline.as_deref() == Some("正在与 openclaw 聊天")
            && entry.scene_banner.as_deref() == Some("个人房间")
            && entry.room_variant.as_deref() == Some("private-room-loft")
            && entry.room_motif.as_deref() == Some("木地板、工作台、沙发与像素人物")
    }));
}

#[test]
fn cli_rooms_for_admin_identity_include_seeded_governance_room() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let rooms = runtime
        .cli_rooms_for(&CliAddress::User(IdentityId("rsaga".into())))
        .expect("cli rooms should build");

    assert!(rooms.entries.iter().any(|entry| {
        entry.conversation_id == "room:city:aurora-hub:announcements"
            && entry.kind == "room"
            && entry.title == "城主告示"
    }));
}

#[test]
fn cli_rooms_include_participant_visible_private_room_threads() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .timeline_store
        .upsert_conversation(Conversation {
            conversation_id: ConversationId("room:city:delta-hub:war-room".into()),
            kind: ConversationKind::Room,
            scope: ConversationScope::CityPrivate,
            scene: Some(GatewayRuntime::default_public_room_scene(
                "shared",
                "channel",
                "城邦门牌 · city:delta-hub:war-room",
            )),
            content_topic: transport_waku::WakuFrameCodec::content_topic_for(&ConversationId(
                "room:city:delta-hub:war-room".into(),
            )),
            participants: vec![IdentityId("rsaga".into()), IdentityId("builder".into())],
            created_at_ms: GatewayRuntime::now_ms(),
            last_active_at_ms: GatewayRuntime::now_ms(),
        })
        .expect("insert participant-visible private room");

    let rooms = runtime
        .cli_rooms_for(&CliAddress::User(IdentityId("rsaga".into())))
        .expect("cli rooms should build");

    assert!(rooms.entries.iter().any(|entry| {
        entry.conversation_id == "room:city:delta-hub:war-room"
            && entry.kind == "room"
            && entry.title == "城邦门牌 · city:delta-hub:war-room"
    }));
}

#[test]
fn cli_inbox_uses_last_message_preview_instead_of_full_body() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .send_cli_message(CliSendRequest {
            from: "agent:openclaw".into(),
            to: "user:rsaga".into(),
            text: "第一行\n第二行".into(),
            client_tag: Some("openclaw".into()),
        })
        .expect("seed direct message");

    let inbox = runtime
        .cli_inbox_for(&CliAddress::User(IdentityId("rsaga".into())))
        .expect("cli inbox should build");
    let json = serde_json::to_string(&inbox).expect("serialize cli inbox");

    assert!(json.contains("last_message_preview"));
    assert!(!json.contains("plain_text"));
}

#[test]
fn cli_tail_returns_recent_messages_for_explicit_conversation() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let response = runtime
        .send_cli_message(CliSendRequest {
            from: "agent:openclaw".into(),
            to: "user:rsaga".into(),
            text: "今晚一起吃饭吗".into(),
            client_tag: Some("openclaw".into()),
        })
        .expect("seed direct message");

    let tail = runtime
        .cli_tail_for(
            &CliAddress::User(IdentityId("rsaga".into())),
            Some(&ConversationId(response.conversation_id.clone())),
        )
        .expect("cli tail should build");

    assert_eq!(tail.conversation_id, "dm:openclaw:rsaga");
    assert_eq!(tail.title, "正在与 openclaw 聊天");
    assert_eq!(tail.subtitle, "居所直达 · 你与 openclaw");
    assert_eq!(tail.meta, "消息数：1");
    assert_eq!(tail.kind, "direct");
    assert_eq!(tail.scope, "private");
    assert_eq!(tail.kind_hint.as_deref(), Some("居所"));
    assert_eq!(
        tail.list_summary.as_deref(),
        Some("正在与 openclaw 聊天 · 2 人 · 1 条消息")
    );
    assert_eq!(tail.status_line.as_deref(), Some("居所直达"));
    assert_eq!(tail.chat_status_summary.as_deref(), Some("可直接继续回复"));
    assert_eq!(
        tail.queue_summary.as_deref(),
        Some("1 条访客提醒待处理 · 1 条巡视提醒待看")
    );
    assert_eq!(
        tail.overview_summary.as_deref(),
        Some("正在与 openclaw 聊天")
    );
    assert_eq!(
        tail.context_summary.as_deref(),
        Some("旺财 会帮你记住与 openclaw 的留言和提醒，适合续聊、记任务和直接追问。")
    );
    assert_eq!(tail.preview_text.as_deref(), Some("今晚一起吃饭吗"));
    assert!(
        tail.last_activity_label
            .as_deref()
            .is_some_and(|value| value.starts_with("openclaw · "))
    );
    assert!(tail.activity_time_label.is_some());
    assert_eq!(tail.self_label.as_deref(), Some("rsaga"));
    assert_eq!(tail.peer_label.as_deref(), Some("openclaw"));
    assert_eq!(tail.participant_label.as_deref(), Some("你与 openclaw"));
    assert_eq!(tail.route_label.as_deref(), Some("居所直达"));
    assert_eq!(
        tail.thread_headline.as_deref(),
        Some("正在与 openclaw 聊天")
    );
    assert_eq!(tail.member_count, Some(2));
    assert!(tail.search_terms.iter().any(|term| term == "openclaw"));
    assert_eq!(tail.scene_banner.as_deref(), Some("个人房间"));
    assert_eq!(tail.room_variant.as_deref(), Some("private-room-loft"));
    assert_eq!(
        tail.room_motif.as_deref(),
        Some("木地板、工作台、沙发与像素人物")
    );
    assert_eq!(
        tail.caretaker
            .as_ref()
            .map(|caretaker| caretaker.name.as_str()),
        Some("旺财")
    );
    assert_eq!(
        tail.detail_card
            .as_ref()
            .map(|detail_card| detail_card.summary_title.as_str()),
        Some("住宅私聊 / 房内状态")
    );
    assert!(tail.workflow.is_none());
    assert!(tail.inline_actions.is_empty());
    assert_eq!(tail.messages.len(), 1);
    assert_eq!(tail.messages[0].text, "今晚一起吃饭吗");
}

#[test]
fn cli_tail_defaults_to_identity_inbox_when_conversation_missing() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .send_cli_message(CliSendRequest {
            from: "agent:openclaw".into(),
            to: "user:rsaga".into(),
            text: "今晚一起吃饭吗".into(),
            client_tag: Some("openclaw".into()),
        })
        .expect("seed direct message");

    let tail = runtime
        .cli_tail_for(&CliAddress::User(IdentityId("rsaga".into())), None)
        .expect("cli tail should default");

    assert_eq!(tail.conversation_id, "dm:openclaw:rsaga");
    assert_eq!(tail.title, "正在与 openclaw 聊天");
    assert_eq!(tail.subtitle, "居所直达 · 你与 openclaw");
    assert_eq!(tail.kind, "direct");
    assert_eq!(tail.scope, "private");
    assert_eq!(
        tail.list_summary.as_deref(),
        Some("正在与 openclaw 聊天 · 2 人 · 1 条消息")
    );
    assert_eq!(tail.status_line.as_deref(), Some("居所直达"));
    assert_eq!(tail.chat_status_summary.as_deref(), Some("可直接继续回复"));
    assert_eq!(
        tail.queue_summary.as_deref(),
        Some("1 条访客提醒待处理 · 1 条巡视提醒待看")
    );
    assert_eq!(
        tail.overview_summary.as_deref(),
        Some("正在与 openclaw 聊天")
    );
    assert_eq!(
        tail.context_summary.as_deref(),
        Some("旺财 会帮你记住与 openclaw 的留言和提醒，适合续聊、记任务和直接追问。")
    );
    assert_eq!(tail.preview_text.as_deref(), Some("今晚一起吃饭吗"));
    assert!(
        tail.last_activity_label
            .as_deref()
            .is_some_and(|value| value.starts_with("openclaw · "))
    );
    assert!(tail.activity_time_label.is_some());
    assert_eq!(tail.self_label.as_deref(), Some("rsaga"));
    assert_eq!(tail.peer_label.as_deref(), Some("openclaw"));
    assert_eq!(tail.participant_label.as_deref(), Some("你与 openclaw"));
    assert_eq!(tail.route_label.as_deref(), Some("居所直达"));
    assert_eq!(
        tail.thread_headline.as_deref(),
        Some("正在与 openclaw 聊天")
    );
    assert_eq!(tail.scene_banner.as_deref(), Some("个人房间"));
    assert_eq!(tail.room_variant.as_deref(), Some("private-room-loft"));
    assert_eq!(
        tail.room_motif.as_deref(),
        Some("木地板、工作台、沙发与像素人物")
    );
    assert!(
        tail.messages
            .iter()
            .any(|message| message.text == "今晚一起吃饭吗")
    );
}

#[test]
fn cli_tail_rejects_explicit_conversation_not_visible_to_identity() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let response = runtime
        .send_cli_message(CliSendRequest {
            from: "agent:openclaw".into(),
            to: "user:rsaga".into(),
            text: "今晚一起吃饭吗".into(),
            client_tag: Some("openclaw".into()),
        })
        .expect("seed direct message");

    let error = runtime
        .cli_tail_for(
            &CliAddress::User(IdentityId("lisi".into())),
            Some(&ConversationId(response.conversation_id)),
        )
        .expect_err("explicit invisible conversation should fail");

    assert!(error.contains("is not visible"));
}

#[test]
fn cli_rooms_hide_non_discoverable_city_room_from_outsiders() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .create_city(CreateCityRequest {
            slug: Some("hidden-harbor".into()),
            title: "Hidden Harbor".into(),
            description: "hidden city for cli visibility test".into(),
            lord_id: "warden".into(),
            approval_required: Some(false),
            public_room_discovery_enabled: Some(false),
            federation_policy: None,
        })
        .expect("create city");

    let room = runtime
        .create_public_room(CreatePublicRoomRequest {
            city: "hidden-harbor".into(),
            title: "Hidden Room".into(),
            slug: Some("hidden-room".into()),
            description: "not discoverable".into(),
            creator_id: "warden".into(),
        })
        .expect("create hidden room");

    let outsider_rooms = runtime
        .cli_rooms_for(&CliAddress::User(IdentityId("outsider".into())))
        .expect("build outsider rooms");
    assert!(
        outsider_rooms
            .entries
            .iter()
            .all(|entry| entry.conversation_id != room.room_id.0)
    );

    let lord_rooms = runtime
        .cli_rooms_for(&CliAddress::User(IdentityId("warden".into())))
        .expect("build lord rooms");
    assert!(
        lord_rooms
            .entries
            .iter()
            .any(|entry| entry.conversation_id == room.room_id.0)
    );
}

#[test]
fn split_path_and_query_decodes_percent_escaped_components() {
    let (_, params) = crate::http_support::split_path_and_query(
        "/v1/cli/tail?for=agent%3Acodex&conversation_id=dm%3Aopenclaw%3Arsaga",
    );

    assert_eq!(params.get("for").map(String::as_str), Some("agent:codex"));
    assert_eq!(
        params.get("conversation_id").map(String::as_str),
        Some("dm:openclaw:rsaga")
    );
}

#[test]
fn split_path_and_query_keeps_unescaped_query_components_intact() {
    let (path, params) = crate::http_support::split_path_and_query(
        "/v1/export?for=user:rsaga&format=jsonl&include_public=true",
    );

    assert_eq!(path, "/v1/export");
    assert_eq!(params.get("for").map(String::as_str), Some("user:rsaga"));
    assert_eq!(params.get("format").map(String::as_str), Some("jsonl"));
    assert_eq!(
        params.get("include_public").map(String::as_str),
        Some("true")
    );
}

#[test]
fn cli_missing_for_body_uses_message_shape() {
    assert_eq!(
        crate::http_support::cli_missing_for_body(),
        "{\"message\":\"missing for\"}"
    );
}

#[test]
fn world_steward_can_publish_notice_and_quarantine_city() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let notice = runtime
        .publish_world_notice(PublishWorldNoticeRequest {
            actor_id: "rsaga".into(),
            title: "Mirror sync window".into(),
            body: "World Square mirrors will roll over at dusk.".into(),
            severity: Some("warning".into()),
            tags: Some(vec!["world".into(), "mirror".into()]),
        })
        .expect("publish notice");
    assert_eq!(notice.severity, "warning");

    let trust = runtime
        .update_city_trust(UpdateCityTrustRequest {
            actor_id: "rsaga".into(),
            city: "core-harbor".into(),
            state: CityTrustState::UnderReview,
            reason: Some("federation anomaly".into()),
        })
        .expect("update city trust");
    assert_eq!(trust.state, CityTrustState::UnderReview);
    assert!(!runtime.safety_advisories.is_empty());

    let safety = runtime.federation_read_plan().world_safety_snapshot();
    assert!(safety.stewards.contains(&"rsaga".into()));
    assert!(
        safety
            .advisories
            .iter()
            .any(|item| item.subject_ref == "city:core-harbor")
    );
}

#[test]
fn governance_http_routes_roundtrip_world_notice_report_review_advisory_and_sanction() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (notice_status, notice) = http_json(
        "POST",
        &server.base_url,
        "/v1/world-square/notices",
        Some(&serde_json::json!({
            "actor_id": "rsaga",
            "title": "维护窗口",
            "body": "今晚世界广场镜像短暂停机。",
            "severity": "warning",
            "tags": ["world", "maintenance"]
        })),
    );
    assert_eq!(notice_status, 200);
    assert_eq!(notice["title"], "维护窗口");
    assert_eq!(notice["severity"], "warning");

    let (report_status, report) = http_json(
        "POST",
        &server.base_url,
        "/v1/world-safety/reports",
        Some(&serde_json::json!({
            "reporter_id": "builder",
            "city": "core-harbor",
            "target_kind": "room",
            "target_ref": "room:city:core-harbor:lobby",
            "summary": "公共房间出现诈骗链接。",
            "evidence": ["https://example.invalid/evidence"]
        })),
    );
    assert_eq!(report_status, 200);
    assert_eq!(report["status"], "Submitted");
    let report_id = report["report_id"].as_str().expect("report id").to_string();

    let (review_status, reviewed) = http_json(
        "POST",
        &server.base_url,
        "/v1/world-safety/reports/review",
        Some(&serde_json::json!({
            "actor_id": "rsaga",
            "report_id": report_id,
            "status": "Resolved",
            "resolution": "已核实，隔离复查。",
            "city_state": "Quarantined",
            "cascade_resident_sanctions": false,
            "blacklist_registered_handles": false
        })),
    );
    assert_eq!(review_status, 200);
    assert_eq!(reviewed["status"], "Resolved");
    assert_eq!(reviewed["resolution"], "已核实，隔离复查。");

    let (advisory_status, advisory) = http_json(
        "POST",
        &server.base_url,
        "/v1/world-safety/advisories",
        Some(&serde_json::json!({
            "actor_id": "rsaga",
            "subject_kind": "city",
            "subject_ref": "city:core-harbor",
            "action": "watch",
            "reason": "举报已进入复查。"
        })),
    );
    assert_eq!(advisory_status, 200);
    assert_eq!(advisory["subject_ref"], "city:core-harbor");
    assert_eq!(advisory["action"], "watch");

    let (sanction_status, sanction) = http_json(
        "POST",
        &server.base_url,
        "/v1/world-safety/residents/sanction",
        Some(&serde_json::json!({
            "actor_id": "rsaga",
            "resident_id": "bad-actor",
            "city": "core-harbor",
            "report_id": reviewed["report_id"],
            "reason": "确认重复滥用。",
            "email": "bad.actor@example.com",
            "mobile": "+86 13800138000",
            "device_physical_addresses": ["00:11:22:33:44:55"],
            "portability_revoked": true
        })),
    );
    assert_eq!(sanction_status, 200);
    assert_eq!(sanction["resident_id"], "bad-actor");
    assert_eq!(sanction["status"], "Active");
    assert_eq!(sanction["portability_revoked"], true);

    let (safety_status, safety) = http_json("GET", &server.base_url, "/v1/world-safety", None);
    assert_eq!(safety_status, 200);
    assert!(
        safety["reports"]
            .as_array()
            .expect("reports array")
            .iter()
            .any(|item| item["report_id"] == reviewed["report_id"] && item["status"] == "Resolved")
    );
    assert!(
        safety["advisories"]
            .as_array()
            .expect("advisories array")
            .iter()
            .any(|item| item["subject_ref"] == "city:core-harbor")
    );
    assert!(
        safety["resident_sanctions"]
            .as_array()
            .expect("resident sanctions array")
            .iter()
            .any(|item| item["resident_id"] == "bad-actor" && item["portability_revoked"] == true)
    );
}

#[test]
fn unsanction_resident_endpoint_records_actor_audit_event() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (sanction_status, sanction) = http_json(
        "POST",
        &server.base_url,
        "/v1/world-safety/residents/sanction",
        Some(&serde_json::json!({
            "actor_id": "rsaga",
            "resident_id": "bad-actor",
            "city": "core-harbor",
            "reason": "temporary enforcement",
            "portability_revoked": true
        })),
    );
    assert_eq!(sanction_status, 200);
    let sanction_id = sanction["sanction_id"].as_str().expect("sanction id");

    let (unsanction_status, unsanctioned) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/residents/unsanction",
        Some(&serde_json::json!({
            "actor_id": "rsaga",
            "sanction_id": sanction_id
        })),
    );
    assert_eq!(unsanction_status, 200);
    assert_eq!(unsanctioned["sanction_id"], sanction_id);

    let (audit_status, _, audit_body) =
        http_raw("GET", &server.base_url, "/v1/admin/audit-log?limit=5", None);
    assert_eq!(audit_status, 200);
    let audit: serde_json::Value = serde_json::from_str(&audit_body).expect("parse audit log");
    assert!(
        audit["events"]
            .as_array()
            .expect("audit events")
            .iter()
            .any(|event| event["actor_id"] == "rsaga"
                && event["action"] == "admin:unsanction_resident"
                && event["target"] == sanction_id)
    );
}

#[test]
fn unsanction_resident_endpoint_rejects_oversized_body() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);
    let oversized_actor = "a".repeat(1_050_000);

    let (status, _, body) = http_raw(
        "POST",
        &server.base_url,
        "/v1/admin/residents/unsanction",
        Some(&serde_json::json!({
            "actor_id": oversized_actor,
            "sanction_id": "resident-sanction:test"
        })),
    );

    assert_eq!(status, 400);
    assert!(body.contains("exceeds 1 MiB"), "body was: {body}");
}

#[test]
fn world_directory_snapshot_hides_isolated_cities() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .create_city(CreateCityRequest {
            slug: Some("bad-harbor".into()),
            title: "Bad Harbor".into(),
            description: "temporary city for isolation test".into(),
            lord_id: "warden".into(),
            approval_required: Some(false),
            public_room_discovery_enabled: Some(true),
            federation_policy: None,
        })
        .expect("create city");

    runtime
        .update_city_trust(UpdateCityTrustRequest {
            actor_id: "rsaga".into(),
            city: "bad-harbor".into(),
            state: CityTrustState::Isolated,
            reason: Some("malware distribution".into()),
        })
        .expect("isolate city");

    let directory = runtime.federation_read_plan().world_directory_snapshot();
    assert!(
        directory
            .cities
            .iter()
            .all(|city| city.slug != "bad-harbor")
    );
    assert!(
        directory
            .mirrors
            .iter()
            .any(|mirror| mirror.slug == "bad-harbor" && !mirror.mirror_enabled)
    );
}

#[test]
fn resident_report_can_trigger_quarantine_and_hide_city() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .create_city(CreateCityRequest {
            slug: Some("bad-harbor".into()),
            title: "Bad Harbor".into(),
            description: "temporary city for safety report test".into(),
            lord_id: "warden".into(),
            approval_required: Some(false),
            public_room_discovery_enabled: Some(true),
            federation_policy: None,
        })
        .expect("create city");

    let report = runtime
        .submit_safety_report(SubmitSafetyReportRequest {
            reporter_id: "guest-01".into(),
            city: Some("bad-harbor".into()),
            target_kind: "room".into(),
            target_ref: "room:city:bad-harbor:lobby".into(),
            summary: "public room is broadcasting illegal scam links".into(),
            evidence: Some(vec!["https://example.invalid/evidence".into()]),
        })
        .expect("submit safety report");
    assert_eq!(report.status, WorldSafetyReportStatus::Submitted);

    let reviewed = runtime
        .review_safety_report(ReviewSafetyReportRequest {
            actor_id: "rsaga".into(),
            report_id: report.report_id.clone(),
            status: WorldSafetyReportStatus::Resolved,
            resolution: Some("confirmed abuse; quarantine city".into()),
            city_state: Some(CityTrustState::Quarantined),
            cascade_resident_sanctions: None,
            blacklist_registered_handles: None,
        })
        .expect("review safety report");
    assert_eq!(reviewed.status, WorldSafetyReportStatus::Resolved);

    let safety = runtime.federation_read_plan().world_safety_snapshot();
    assert!(
        safety
            .reports
            .iter()
            .any(|item| item.report_id == report.report_id && item.reviewed_by.is_some())
    );
    assert!(safety.city_trust.iter().any(
        |item| item.city_id.0 == "city:bad-harbor" && item.state == CityTrustState::Quarantined
    ));

    let directory = runtime.federation_read_plan().world_directory_snapshot();
    assert!(
        directory
            .cities
            .iter()
            .all(|city| city.slug != "bad-harbor")
    );
    assert!(
        directory
            .mirrors
            .iter()
            .any(|mirror| mirror.slug == "bad-harbor" && !mirror.mirror_enabled)
    );
}

#[test]
fn isolated_city_can_cascade_resident_ban_and_blacklist_handles() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .request_email_otp(RequestEmailOtpRequest {
            email: "tiyan@example.com".into(),
            mobile: Some("+86 13800138001".into()),
            device_physical_address: Some("AA:BB:CC:DD:EE:01".into()),
            resident_id: Some("tiyan".into()),
            nickname: None,
        })
        .and_then(|response| {
            runtime.verify_email_otp(VerifyEmailOtpRequest {
                challenge_id: response.challenge_id,
                code: response.dev_code.expect("dev otp"),
                resident_id: Some("tiyan".into()),
            })
        })
        .expect("register tiyann");

    runtime
        .create_city(CreateCityRequest {
            slug: Some("evil-harbor".into()),
            title: "Evil Harbor".into(),
            description: "temporary city for isolation cascade test".into(),
            lord_id: "warden".into(),
            approval_required: Some(false),
            public_room_discovery_enabled: Some(true),
            federation_policy: None,
        })
        .expect("create city");

    runtime
        .join_city(JoinCityRequest {
            city: "evil-harbor".into(),
            resident_id: "tiyan".into(),
        })
        .expect("join city");

    let report = runtime
        .submit_safety_report(SubmitSafetyReportRequest {
            reporter_id: "builder".into(),
            city: Some("evil-harbor".into()),
            target_kind: "city".into(),
            target_ref: "city:evil-harbor".into(),
            summary: "city is coordinating serious illegal abuse".into(),
            evidence: Some(vec!["https://example.invalid/abuse".into()]),
        })
        .expect("submit report");

    runtime
        .review_safety_report(ReviewSafetyReportRequest {
            actor_id: "rsaga".into(),
            report_id: report.report_id.clone(),
            status: WorldSafetyReportStatus::Resolved,
            resolution: Some("confirmed severe abuse; isolate city and burn handles".into()),
            city_state: Some(CityTrustState::Isolated),
            cascade_resident_sanctions: Some(true),
            blacklist_registered_handles: Some(true),
        })
        .expect("review report");

    let safety = runtime.federation_read_plan().world_safety_snapshot();
    assert!(
        safety
            .resident_sanctions
            .iter()
            .any(|item| item.resident_id.0 == "tiyan" && item.portability_revoked)
    );
    assert!(
        safety
            .registration_blacklist
            .iter()
            .any(|item| item.resident_id.0 == "tiyan" && item.handle_kind == "email")
    );
    assert!(
        safety
            .registration_blacklist
            .iter()
            .any(|item| item.resident_id.0 == "tiyan" && item.handle_kind == "mobile")
    );
    assert!(
        runtime
            .registrations
            .iter()
            .any(|item| item.resident_id.0 == "tiyan"
                && item.state == ResidentRegistrationState::Suspended)
    );

    let preflight = runtime
        .auth_preflight(AuthPreflightRequest {
            email: "tiyan@example.com".into(),
            mobile: Some("+86 13800138001".into()),
            device_physical_address: Some("AA-BB-CC-DD-EE-01".into()),
        })
        .expect("preflight");
    assert!(!preflight.allowed);
    assert!(
        preflight
            .blocked_reasons
            .iter()
            .any(|item| item.contains("device physical address"))
    );

    let join_err = runtime
        .join_city(JoinCityRequest {
            city: "core-harbor".into(),
            resident_id: "tiyan".into(),
        })
        .expect_err("isolated city resident should lose cross-city portability");
    assert!(join_err.contains("world-banned"));
}

#[test]
fn world_banned_resident_is_blocked_from_cross_city_join_and_handles_are_hashed() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .create_city(CreateCityRequest {
            slug: Some("south-harbor".into()),
            title: "South Harbor".into(),
            description: "temporary city for sanction test".into(),
            lord_id: "harbormaster".into(),
            approval_required: Some(false),
            public_room_discovery_enabled: Some(true),
            federation_policy: None,
        })
        .expect("create city");

    let sanction = runtime
        .sanction_resident(SanctionResidentRequest {
            actor_id: "rsaga".into(),
            resident_id: "bad-actor".into(),
            city: Some("south-harbor".into()),
            report_id: Some("report:test".into()),
            reason: "confirmed organized scam operation".into(),
            email: Some("Bad.Actor@example.com".into()),
            mobile: Some("+86 138-0013-8000".into()),
            device_physical_addresses: Some(vec!["00:11:22:33:44:55".into()]),
            portability_revoked: Some(true),
        })
        .expect("sanction resident");
    assert_eq!(sanction.status, WorldResidentSanctionStatus::Active);
    assert!(sanction.portability_revoked);

    let safety = runtime.federation_read_plan().world_safety_snapshot();
    assert!(
        safety
            .resident_sanctions
            .iter()
            .any(|item| item.resident_id.0 == "bad-actor")
    );
    assert_eq!(safety.registration_blacklist.len(), 3);
    assert!(
        safety
            .registration_blacklist
            .iter()
            .all(|item| !item.hash_sha256.is_empty())
    );
    assert!(
        safety
            .registration_blacklist
            .iter()
            .all(|item| item.hash_sha256 != "Bad.Actor@example.com")
    );

    let err = runtime
        .join_city(JoinCityRequest {
            city: "core-harbor".into(),
            resident_id: "bad-actor".into(),
        })
        .expect_err("world-banned resident should be blocked");
    assert!(err.contains("world-banned"));
}

#[test]
fn sanctioned_resident_cannot_send_messages() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    register_resident(&mut runtime, "bad-actor");

    // Sanction the resident
    runtime
        .admin_ban_resident("bad-actor", "violation of world safety rules")
        .expect("ban resident");

    // Verify the sanction is active
    let is_revoked = runtime.resident_portability_revoked(&IdentityId("bad-actor".into()));
    assert!(is_revoked, "resident should have revoked portability");

    // Sanctioned resident should be blocked from sending messages
    let result = runtime.append_shell_message(ShellMessageRequest {
        room_id: "room:world:lobby".into(),
        sender: "bad-actor".into(),
        text: "attempting to send after sanction".into(),
        reply_to_message_id: None,
        device_id: None,
        language_tag: None,
    });
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("sanctioned"),
        "sanctioned resident must not be able to send messages"
    );
}

#[test]
fn admin_resident_sanction_and_unban_persist_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime
            .admin_ban_resident("persisted-resident", "test sanction")
            .expect("ban resident");
    }

    {
        let runtime = GatewayRuntime::open(&root, 64, None).expect("reopened runtime");
        assert!(runtime.resident_portability_revoked(&IdentityId("persisted-resident".into())));
    }

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        assert_eq!(
            runtime
                .admin_unban_resident("persisted-resident")
                .expect("unban resident"),
            1
        );
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("reopened runtime");
    assert!(!runtime.resident_portability_revoked(&IdentityId("persisted-resident".into())));
}

#[test]
fn governance_profile_and_device_binding_persist_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");
    let sanction_id;

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime
            .admin_create_resident("profile-resident", "profile@example.com")
            .expect("create resident");
        runtime
            .admin_set_nickname("profile-resident", Some("Profile Resident"))
            .expect("persist nickname");
        runtime
            .admin_ban_resident("profile-resident", "profile test sanction")
            .expect("ban resident");
        sanction_id = runtime
            .resident_sanctions
            .last()
            .expect("sanction")
            .sanction_id
            .clone();
        runtime
            .admin_add_device(
                "11:22:33:44:55:66".into(),
                "Profile device".into(),
                "admin-1".into(),
            )
            .expect("add device");
        runtime
            .bind_device_to_resident("112233445566", "profile-resident")
            .expect("persist device binding");
    }

    {
        let runtime = GatewayRuntime::open(&root, 64, None).expect("reopened runtime");
        let registration = runtime
            .registrations
            .iter()
            .find(|registration| registration.resident_id.0 == "profile-resident")
            .expect("reopened resident");
        assert_eq!(registration.nickname.as_deref(), Some("Profile Resident"));
        let device = runtime
            .admin_list_devices()
            .into_iter()
            .find(|device| device.address == "112233445566")
            .expect("reopened device");
        assert_eq!(
            device.bound_resident_id.as_deref(),
            Some("profile-resident")
        );
    }

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime
            .revoke_sanction(&sanction_id)
            .expect("persist revoked sanction");
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("reopened runtime");
    let sanction = runtime
        .resident_sanctions
        .iter()
        .find(|sanction| sanction.sanction_id == sanction_id)
        .expect("reopened sanction");
    assert_eq!(sanction.status, WorldResidentSanctionStatus::Lifted);
}

#[test]
fn email_otp_registration_roundtrip_creates_persisted_resident() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");

    let response = {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime
            .request_email_otp(RequestEmailOtpRequest {
                email: "novel.reader@example.com".into(),
                mobile: Some("+86 13800138000".into()),
                device_physical_address: Some("66:55:44:33:22:11".into()),
                resident_id: Some("novel-reader".into()),
                nickname: None,
            })
            .expect("request email otp")
    };

    let dev_code = response.dev_code.expect("test mode should expose dev otp");
    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        let verified = runtime
            .verify_email_otp(VerifyEmailOtpRequest {
                challenge_id: response.challenge_id,
                code: dev_code,
                resident_id: Some("novel-reader".into()),
            })
            .expect("verify email otp");
        assert_eq!(verified.resident_id, "novel-reader");
        assert_eq!(verified.email, "novel.reader@example.com");
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
    assert!(runtime.registrations.iter().any(
        |item| item.resident_id.0 == "novel-reader" && item.email == "novel.reader@example.com"
    ));
}

#[test]
fn nickname_flows_through_otp_registration_directory_and_admin() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");

    let response = {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime
            .request_email_otp(RequestEmailOtpRequest {
                email: "nickname.test@example.com".into(),
                mobile: Some("+86 13800138005".into()),
                device_physical_address: Some("AA:BB:CC:DD:EE:05".into()),
                resident_id: Some("nickname-user".into()),
                nickname: Some("昵称测试".into()),
            })
            .expect("request email otp")
    };

    let dev_code = response.dev_code.expect("test mode should expose dev otp");
    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        let verified = runtime
            .verify_email_otp(VerifyEmailOtpRequest {
                challenge_id: response.challenge_id,
                code: dev_code,
                resident_id: Some("nickname-user".into()),
            })
            .expect("verify email otp");
        assert_eq!(verified.resident_id, "nickname-user");
        assert_eq!(verified.nickname.as_deref(), Some("昵称测试"));
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
    let reg = runtime
        .registrations
        .iter()
        .find(|r| r.resident_id.0 == "nickname-user")
        .expect("registration should persist");
    assert_eq!(reg.nickname.as_deref(), Some("昵称测试"));

    // Also verify nickname flows through when no nickname is provided (None case)
    let response2 = {
        let mut runtime2 = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime2
            .request_email_otp(RequestEmailOtpRequest {
                email: "nonick@example.com".into(),
                mobile: Some("+86 13800138006".into()),
                device_physical_address: Some("AA:BB:CC:DD:EE:06".into()),
                resident_id: Some("nonick-user".into()),
                nickname: None,
            })
            .expect("request email otp")
    };
    let dev_code2 = response2.dev_code.expect("test mode should expose dev otp");
    {
        let mut runtime2 = GatewayRuntime::open(&root, 64, None).expect("runtime");
        let verified = runtime2
            .verify_email_otp(VerifyEmailOtpRequest {
                challenge_id: response2.challenge_id,
                code: dev_code2,
                resident_id: Some("nonick-user".into()),
            })
            .expect("verify email otp");
        assert_eq!(verified.resident_id, "nonick-user");
        assert_eq!(verified.nickname, None);
    }
    let runtime2 = GatewayRuntime::open(&root, 64, None).expect("runtime");
    let reg2 = runtime2
        .registrations
        .iter()
        .find(|r| r.resident_id.0 == "nonick-user")
        .expect("registration without nickname should persist");
    assert_eq!(reg2.nickname, None);
}

#[test]
fn request_email_otp_replaces_prior_active_challenge_for_same_email() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let first = runtime
        .request_email_otp(RequestEmailOtpRequest {
            email: "swap@example.com".into(),
            mobile: Some("+86 13800138000".into()),
            device_physical_address: Some("66:55:44:33:22:11".into()),
            resident_id: Some("swap-reader".into()),
            nickname: None,
        })
        .expect("first email otp request");

    // Expire the rate limit window so the second request is not blocked
    if let Some(window) = runtime.rate_limits.get_mut("otp-req:swap@example.com") {
        window.window_start_ms -= 120_000;
    }

    let second = runtime
        .request_email_otp(RequestEmailOtpRequest {
            email: "swap@example.com".into(),
            mobile: Some("+86 13800138000".into()),
            device_physical_address: Some("66:55:44:33:22:11".into()),
            resident_id: Some("swap-reader".into()),
            nickname: None,
        })
        .expect("second email otp request");

    assert_eq!(runtime.email_otp_challenges.len(), 1);
    assert_eq!(
        runtime.email_otp_challenges[0].challenge_id,
        second.challenge_id
    );
    assert_ne!(first.challenge_id, second.challenge_id);
}

#[test]
fn email_otp_verification_seeds_canonical_guide_direct_conversation() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");

    let challenge = {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime
            .request_email_otp(RequestEmailOtpRequest {
                email: "tiyan@example.com".into(),
                mobile: Some("+86 13800138000".into()),
                device_physical_address: Some("66:55:44:33:22:11".into()),
                resident_id: Some("tiyan".into()),
                nickname: None,
            })
            .expect("request email otp")
    };

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime
            .verify_email_otp(VerifyEmailOtpRequest {
                challenge_id: challenge.challenge_id,
                code: challenge.dev_code.expect("dev otp"),
                resident_id: Some("tiyan".into()),
            })
            .expect("verify email otp");
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
    let expected_id =
        canonical_direct_conversation_id(&IdentityId("guide".into()), &IdentityId("tiyan".into()));
    let conversation = runtime
        .timeline_store
        .active_conversations()
        .into_iter()
        .find(|item| item.conversation_id == expected_id)
        .expect("guide direct conversation should exist");

    assert_eq!(conversation.kind, ConversationKind::Direct);
    assert_eq!(conversation.scope, ConversationScope::Private);
    assert_eq!(conversation.conversation_id, expected_id);
    assert_eq!(conversation.participants.len(), 2);
    assert!(
        conversation
            .participants
            .iter()
            .any(|participant| participant.0 == "tiyan")
    );
    assert!(
        conversation
            .participants
            .iter()
            .any(|participant| participant.0 == "guide")
    );
    let scene = conversation.scene.as_ref().expect("direct scene");
    assert_eq!(scene.scope, SceneScope::DirectRoom);
    assert_eq!(scene.title_banner.as_deref(), Some("个人房间"));
}

#[test]
fn join_city_rejects_unregistered_resident() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let err = runtime
        .join_city(JoinCityRequest {
            city: "core-harbor".into(),
            resident_id: "guest-01".into(),
        })
        .expect_err("unregistered resident should not join");
    assert!(err.contains("not registered"));
}

#[test]
fn blacklisted_handles_are_rejected_during_auth_preflight_and_otp_issue() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .sanction_resident(SanctionResidentRequest {
            actor_id: "rsaga".into(),
            resident_id: "repeat-offender".into(),
            city: Some("core-harbor".into()),
            report_id: Some("report:blacklist".into()),
            reason: "repeat harassment".into(),
            email: Some("blocked@example.com".into()),
            mobile: Some("+86 13900000000".into()),
            device_physical_addresses: Some(vec!["00-22-44-66-88-AA".into()]),
            portability_revoked: Some(true),
        })
        .expect("sanction resident");

    let preflight = runtime
        .auth_preflight(AuthPreflightRequest {
            email: "blocked@example.com".into(),
            mobile: Some("+86 13900000000".into()),
            device_physical_address: Some("00:22:44:66:88:aa".into()),
        })
        .expect("auth preflight");
    assert!(!preflight.allowed);
    assert_eq!(preflight.blocked_reasons.len(), 3);

    let err = runtime
        .request_email_otp(RequestEmailOtpRequest {
            email: "blocked@example.com".into(),
            mobile: Some("+86 13900000000".into()),
            device_physical_address: Some("00:22:44:66:88:AA".into()),
            resident_id: Some("new-handle".into()),
            nickname: None,
        })
        .expect_err("blacklisted handles should not receive otp");
    assert!(err.contains("blacklisted"));
}

#[test]
fn world_snapshot_bundle_exposes_checksum_and_payload() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let before = runtime.federation_read_plan().world_snapshot_bundle();
    assert_eq!(before.meta.world_id, "world:lobster");
    assert!(!before.meta.checksum_sha256.is_empty());
    assert!(before.payload.directory.city_count >= 1);
    assert!(!before.payload.square.is_empty());

    runtime
        .publish_world_notice(PublishWorldNoticeRequest {
            actor_id: "rsaga".into(),
            title: "Fresh notice".into(),
            body: "Mirror bundle changed.".into(),
            severity: Some("info".into()),
            tags: Some(vec!["snapshot".into()]),
        })
        .expect("publish notice");

    let after = runtime.federation_read_plan().world_snapshot_bundle();
    assert_ne!(before.meta.checksum_sha256, after.meta.checksum_sha256);
    assert!(
        after
            .payload
            .square
            .iter()
            .any(|notice| notice.title == "Fresh notice")
    );
}

#[test]
fn world_square_http_handler_returns_readonly_notice_feed() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .publish_world_notice(PublishWorldNoticeRequest {
            actor_id: "rsaga".into(),
            title: "Public square notice".into(),
            body: "Readonly cards can project this notice without becoming chat state.".into(),
            severity: Some("info".into()),
            tags: Some(vec!["world-square".into(), "readonly".into()]),
        })
        .expect("publish notice");

    let runtime = Arc::new(Mutex::new(runtime));
    let response = handle_get_world_square(&runtime);
    assert_eq!(response.status_code(), StatusCode(200));

    let mut body = String::new();
    response
        .into_reader()
        .read_to_string(&mut body)
        .expect("read response body");
    let payload: Vec<WorldSquareNotice> =
        serde_json::from_str(&body).expect("world-square notice json");

    assert!(
        payload
            .iter()
            .any(|notice| notice.title == "Public square notice"
                && notice.body.contains("Readonly cards")
                && notice.tags.iter().any(|tag| tag == "readonly")),
        "world-square should expose published notices as a readonly feed"
    );
}

#[test]
fn world_entry_state_projects_directory_into_route_cards() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .create_city(CreateCityRequest {
            slug: Some("signal-bay".into()),
            title: "Signal Bay".into(),
            description: "A visible city route for the world entry station".into(),
            lord_id: "alice".into(),
            approval_required: Some(false),
            public_room_discovery_enabled: Some(true),
            federation_policy: None,
        })
        .expect("create city");

    let entry = runtime.world_entry_state();

    assert_eq!(entry.title, "世界入口");
    assert_eq!(entry.station_label, "地铁候车站");
    assert_eq!(entry.current_city_slug, "core-harbor");
    assert!(entry.route_count >= 2);

    let signal = entry
        .routes
        .iter()
        .find(|route| route.slug == "signal-bay")
        .expect("signal bay route");
    assert_eq!(signal.title, "Signal Bay");
    assert_eq!(signal.href, "./index.html?city=signal-bay");
    assert_eq!(signal.status_label, "健康 · 可镜像");
    assert!(!signal.is_current);

    let current = entry
        .routes
        .iter()
        .find(|route| route.slug == "core-harbor")
        .expect("current city route");
    assert_eq!(current.href, "./index.html");
    assert!(current.is_current);
}

#[test]
fn world_entry_http_handler_returns_route_projection() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime
        .create_city(CreateCityRequest {
            slug: Some("signal-bay".into()),
            title: "Signal Bay".into(),
            description: "A visible city route for the world entry station".into(),
            lord_id: "alice".into(),
            approval_required: Some(false),
            public_room_discovery_enabled: Some(true),
            federation_policy: None,
        })
        .expect("create city");

    let runtime = Arc::new(Mutex::new(runtime));
    let response = handle_get_world_entry(&runtime);
    assert_eq!(response.status_code(), StatusCode(200));

    let mut body = String::new();
    response
        .into_reader()
        .read_to_string(&mut body)
        .expect("read response body");
    let payload: serde_json::Value = serde_json::from_str(&body).expect("world-entry json");

    assert_eq!(payload["title"], "世界入口");
    assert_eq!(payload["station_label"], "地铁候车站");
    assert_eq!(payload["current_city_slug"], "core-harbor");
    assert!(
        payload["routes"]
            .as_array()
            .expect("route array")
            .iter()
            .any(|route| route["slug"] == "signal-bay"
                && route["href"] == "./index.html?city=signal-bay"
                && route["status_label"] == "健康 · 可镜像"
                && route["is_current"] == false)
    );
    assert!(
        payload["routes"]
            .as_array()
            .expect("route array")
            .iter()
            .any(|route| route["slug"] == "core-harbor"
                && route["href"] == "./index.html"
                && route["is_current"] == true)
    );
}

#[test]
fn world_snapshot_bundle_fetches_each_upstream_bundle_only_once() {
    let temp = tempdir().expect("temp dir");
    let (base_url, state, running, handle) = start_mock_upstream_gateway();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut local =
            GatewayRuntime::open(temp.path().join("local-gateway"), 64, None).expect("local");
        local
            .publish_world_notice(PublishWorldNoticeRequest {
                actor_id: "rsaga".into(),
                title: "Local notice".into(),
                body: "local".into(),
                severity: Some("info".into()),
                tags: Some(vec!["local".into()]),
            })
            .expect("publish local notice");
        let remote_bundle = local.federation_read_plan().world_snapshot_bundle();
        {
            let mut shared = state.lock().expect("lock mock upstream state");
            shared.world_snapshot_bundle = Some(remote_bundle.clone());
            shared.governance_snapshot = Some(remote_bundle.payload.governance.clone());
        }

        let mut runtime =
            GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
        runtime
            .set_upstream_provider_url(Some(base_url.clone()))
            .expect("set upstream provider");

        let _ = runtime.federation_read_plan().world_snapshot_bundle();

        let shared = state.lock().expect("lock mock upstream state");
        assert_eq!(
            shared.world_snapshot_request_count, 1,
            "world_snapshot_bundle should reuse a single upstream world snapshot fetch",
        );
        assert_eq!(
            shared.world_request_count, 0,
            "world_snapshot_bundle should not fall back to /v1/world when /v1/world-snapshot succeeds",
        );
    }));

    running.store(false, Ordering::SeqCst);
    let _ = TcpStream::connect(base_url.trim_start_matches("http://"));
    handle.join().expect("stop mock upstream gateway");
    outcome.expect("world snapshot bundle should fetch upstream once");
}

#[test]
fn federation_read_plan_world_snapshot_bundle_fetches_upstream_once() {
    let temp = tempdir().expect("temp dir");
    let (base_url, state, running, handle) = start_mock_upstream_gateway();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut local =
            GatewayRuntime::open(temp.path().join("local-gateway"), 64, None).expect("local");
        local
            .publish_world_notice(PublishWorldNoticeRequest {
                actor_id: "rsaga".into(),
                title: "Local notice".into(),
                body: "local".into(),
                severity: Some("info".into()),
                tags: Some(vec!["local".into()]),
            })
            .expect("publish local notice");
        let remote_bundle = local.federation_read_plan().world_snapshot_bundle();
        {
            let mut shared = state.lock().expect("lock mock upstream state");
            shared.world_snapshot_bundle = Some(remote_bundle.clone());
            shared.governance_snapshot = Some(remote_bundle.payload.governance.clone());
        }

        let mut runtime =
            GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
        runtime
            .set_upstream_provider_url(Some(base_url.clone()))
            .expect("set upstream provider");

        let read_plan = runtime.federation_read_plan();
        let _ = read_plan.world_snapshot_bundle();

        let shared = state.lock().expect("lock mock upstream state");
        assert_eq!(
            shared.world_snapshot_request_count, 1,
            "federation read plan should fetch upstream world snapshot once",
        );
    }));

    running.store(false, Ordering::SeqCst);
    let _ = TcpStream::connect(base_url.trim_start_matches("http://"));
    handle.join().expect("stop mock upstream gateway");
    outcome.expect("federation read plan world snapshot should fetch upstream once");
}

#[test]
fn lord_can_mark_city_as_self_isolated_without_world_quarantine() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let city = runtime
        .update_federation_policy(UpdateFederationPolicyRequest {
            city: "core-harbor".into(),
            actor_id: "rsaga".into(),
            policy: FederationPolicy::Isolated,
        })
        .expect("update federation policy");

    assert_eq!(city.profile.federation_policy, FederationPolicy::Isolated);

    let directory = runtime.federation_read_plan().world_directory_snapshot();
    let mirror = directory
        .mirrors
        .iter()
        .find(|mirror| mirror.slug == "core-harbor")
        .expect("core harbor mirror");
    assert!(!mirror.mirror_enabled);

    let trusted = runtime
        .city_trust
        .iter()
        .find(|record| record.city_id.0 == "city:core-harbor")
        .expect("trust record");
    assert_eq!(trusted.state, CityTrustState::Healthy);
}

#[test]
fn resident_can_export_private_and_public_history() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    register_resident(&mut runtime, "guest-03");
    runtime
        .join_city(JoinCityRequest {
            city: "core-harbor".into(),
            resident_id: "guest-03".into(),
        })
        .expect("join core harbor");

    runtime
        .append_shell_message(ShellMessageRequest {
            room_id: "room:city:core-harbor:lobby".into(),
            sender: "guest-03".into(),
            text: "public hello".into(),
            reply_to_message_id: None,
            device_id: Some("browser".into()),
            language_tag: Some("en".into()),
        })
        .expect("append public message");

    runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "guest-03".into(),
            requester_device_id: Some("browser".into()),
            peer_id: "rsaga".into(),
            peer_device_id: Some("desktop-1".into()),
        })
        .expect("open direct session");

    runtime
        .append_shell_message(ShellMessageRequest {
            room_id: "dm:guest-03:rsaga".into(),
            sender: "guest-03".into(),
            text: "private hello".into(),
            reply_to_message_id: None,
            device_id: Some("browser".into()),
            language_tag: Some("en".into()),
        })
        .expect("append private message");

    let export = runtime
        .export_history(
            IdentityId("guest-03".into()),
            None,
            ExportFormat::Markdown,
            true,
        )
        .expect("export history");

    let guide_conversation_id = canonical_direct_conversation_id(
        &IdentityId("guest-03".into()),
        &IdentityId("guide".into()),
    );
    assert_eq!(export.conversation_count, 3);
    assert!(
        export
            .conversations
            .iter()
            .any(|conversation| conversation.conversation_id == guide_conversation_id.0)
    );
    assert!(export.conversations.iter().any(|conversation| {
        conversation.conversation_id == "room:city:core-harbor:lobby"
            && conversation.title == "第一城大厅"
            && conversation.kind == "public"
            && conversation.scope == "city_public"
            && conversation.meta == "消息数：1"
            && conversation.kind_hint.as_deref() == Some("城邦大厅")
            && conversation
                .list_summary
                .as_deref()
                .is_some_and(|value| value.starts_with("第一城大厅 · "))
            && conversation.status_line.as_deref() == Some("城内回响线")
            && conversation.chat_status_summary.as_deref() == Some("群聊当前比较安静")
            && conversation.overview_summary.as_deref() == Some("核心港回声大厅 · 群聊")
            && conversation.context_summary.as_deref()
                == Some("巡逻犬 会盯住公共提醒和巡视结果，适合看公告、围观和跨城讨论。")
            && conversation.preview_text.as_deref() == Some("public hello")
            && conversation
                .last_activity_label
                .as_deref()
                .is_some_and(|value| value.starts_with("guest-03 · "))
            && conversation.activity_time_label.is_some()
            && conversation.participant_label.as_deref() == Some("核心港回声大厅")
            && conversation
                .scene_banner
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && conversation.scene_summary.as_deref()
                == Some("公共房间 · 公共频道、公告板与像素座位区")
            && conversation
                .room_variant
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && conversation.room_motif.as_deref() == Some("公共频道、公告板与像素座位区")
            && conversation.member_count == Some(1)
            && conversation
                .search_terms
                .iter()
                .any(|term| term == "核心港回声大厅")
    }));
    assert!(export.conversations.iter().any(|conversation| {
        conversation.conversation_id == "dm:guest-03:rsaga"
            && conversation.title == "正在与 rsaga 聊天"
            && conversation.kind == "direct"
            && conversation.scope == "private"
            && conversation.meta == "消息数：1"
            && conversation.kind_hint.as_deref() == Some("居所")
            && conversation.list_summary.as_deref() == Some("正在与 rsaga 聊天 · 2 人 · 1 条消息")
            && conversation.status_line.as_deref() == Some("居所直达")
            && conversation.chat_status_summary.as_deref() == Some("可直接继续回复")
            && conversation.queue_summary.as_deref()
                == Some("1 条访客提醒待处理 · 1 条巡视提醒待看")
            && conversation.overview_summary.as_deref() == Some("正在与 rsaga 聊天")
            && conversation.context_summary.as_deref()
                == Some("旺财 会帮你记住与 rsaga 的留言和提醒，适合续聊、记任务和直接追问。")
            && conversation.preview_text.as_deref() == Some("private hello")
            && conversation
                .last_activity_label
                .as_deref()
                .is_some_and(|value| value.starts_with("guest-03 · "))
            && conversation.activity_time_label.is_some()
            && conversation.self_label.as_deref() == Some("guest-03")
            && conversation.peer_label.as_deref() == Some("rsaga")
            && conversation.participant_label.as_deref() == Some("你与 rsaga")
            && conversation.member_count == Some(2)
            && conversation.search_terms.iter().any(|term| term == "rsaga")
            && conversation.scene_banner.as_deref() == Some("个人房间")
            && conversation.room_variant.as_deref() == Some("private-room-loft")
            && conversation.room_motif.as_deref() == Some("木地板、工作台、沙发与像素人物")
            && conversation
                .caretaker
                .as_ref()
                .is_some_and(|caretaker| caretaker.name == "旺财")
            && conversation
                .detail_card
                .as_ref()
                .is_some_and(|detail_card| detail_card.summary_title == "住宅私聊 / 房内状态")
            && conversation.workflow.is_none()
            && conversation.inline_actions.is_empty()
    }));
    assert!(export.content.contains("public hello"));
    assert!(export.content.contains("private hello"));
    assert!(export.rights.may_export_private_conversations);
    assert!(export.rights.may_export_city_public_rooms);
}

#[test]
fn presence_records_online_status() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let resident_id = "alice";

    assert!(!runtime.is_online(resident_id, 120_000));
    runtime.record_presence(resident_id);
    assert!(runtime.is_online(resident_id, 120_000));
}

#[test]
fn presence_appears_in_enriched_resident_directory() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    runtime.record_presence("rsaga");

    let residents = runtime.enrich_resident_directory();
    let rsaga = residents
        .iter()
        .find(|entry| entry.resident_id == "rsaga")
        .expect("rsaga should be in directory");
    assert_eq!(rsaga.online, Some(true));
    assert!(rsaga.last_seen_at_ms.is_some());
    assert_eq!(rsaga.avatar_id.as_deref(), Some("avatar:rsaga"));
}

#[test]
fn resident_search_filters_by_query() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let all = runtime.enrich_resident_directory();
    assert!(all.len() >= 3);

    let filtered: Vec<_> = all
        .into_iter()
        .filter(|entry| entry.resident_id.to_lowercase().contains("rsaga"))
        .collect();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].resident_id, "rsaga");
}

#[test]
fn unread_increments_after_message_publish() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");

    let alice = IdentityId("alice".into());
    let bob = IdentityId("bob".into());
    let dm = runtime.resolve_direct_conversation_id(&alice, &bob);
    runtime
        .ensure_direct_conversation(&dm, &[alice.clone(), bob.clone()])
        .expect("ensure dm");

    let initial = runtime.unread_count(&bob, &dm);
    runtime.increment_unread(&dm, &alice);
    let after = runtime.unread_count(&bob, &dm);
    assert!(after > initial);
}

#[test]
fn mark_read_resets_unread_to_zero() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");

    let alice = IdentityId("alice".into());
    let bob = IdentityId("bob".into());
    let dm = runtime.resolve_direct_conversation_id(&alice, &bob);
    runtime
        .ensure_direct_conversation(&dm, &[alice.clone(), bob.clone()])
        .expect("ensure dm");

    runtime.increment_unread(&dm, &alice);
    assert!(runtime.unread_count(&bob, &dm) > 0);

    runtime.mark_read(&bob, &dm);
    assert_eq!(runtime.unread_count(&bob, &dm), 0);
}

#[test]
fn shell_state_includes_unread_count() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");

    let alice = IdentityId("alice".into());
    let bob = IdentityId("bob".into());
    let dm = runtime.resolve_direct_conversation_id(&alice, &bob);
    runtime
        .ensure_direct_conversation(&dm, &[alice.clone(), bob.clone()])
        .expect("ensure dm");

    runtime.increment_unread(&dm, &alice);
    runtime.increment_unread(&dm, &alice);

    let state = runtime.shell_state_for_viewer(Some(&bob));
    let dm_room = state
        .rooms
        .iter()
        .find(|room| room.id == dm.0)
        .expect("dm should be visible");
    assert_eq!(dm_room.unread_count, 2);
}

#[test]
fn presence_http_endpoint_roundtrips() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"resident_id": "rsaga"});
    let (status, _payload) = http_json("POST", &server.base_url, "/v1/shell/presence", Some(&body));
    assert_eq!(status, 200);
}

#[test]
fn presence_endpoint_rejects_empty_body() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json("POST", &server.base_url, "/v1/shell/presence", None);
    assert_eq!(status, 400);
    assert!(payload["error"].as_str().unwrap().contains("decode"));
}

#[test]
fn presence_endpoint_rejects_missing_resident_id() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"other": "value"});
    let (status, payload) = http_json("POST", &server.base_url, "/v1/shell/presence", Some(&body));
    assert_eq!(status, 400);
    assert!(payload["error"].as_str().unwrap().contains("resident_id"));
}

#[test]
fn presence_endpoint_rejects_empty_resident_id() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"resident_id": "  "});
    let (status, payload) = http_json("POST", &server.base_url, "/v1/shell/presence", Some(&body));
    assert_eq!(status, 400);
    assert!(payload["error"].as_str().unwrap().contains("resident_id"));
}

#[test]
fn mark_read_http_endpoint_resets_unread() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({
        "resident_id": "builder",
        "conversation_id": "room:world:lobby"
    });
    let (status, _payload) = http_json("POST", &server.base_url, "/v1/shell/read", Some(&body));
    assert_eq!(status, 200);
}

#[test]
fn mark_read_endpoint_rejects_empty_body() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json("POST", &server.base_url, "/v1/shell/read", None);
    assert_eq!(status, 400);
    assert!(payload["error"].as_str().unwrap().contains("decode"));
}

#[test]
fn mark_read_endpoint_rejects_missing_resident_id() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"conversation_id": "room:world:lobby"});
    let (status, payload) = http_json("POST", &server.base_url, "/v1/shell/read", Some(&body));
    assert_eq!(status, 400);
    assert!(payload["error"].as_str().unwrap().contains("decode"));
}

#[test]
fn mark_read_endpoint_rejects_missing_conversation_id() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"resident_id": "builder", "actor_id": "rsaga"});
    let (status, payload) = http_json("POST", &server.base_url, "/v1/shell/read", Some(&body));
    assert_eq!(status, 400);
    assert!(payload["error"].as_str().unwrap().contains("decode"));
}

#[test]
fn mark_read_http_endpoint_resets_unread_and_reflects_in_shell_state() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    // Open a direct conversation between qa-a and qa-b
    let (direct_status, direct) = http_json(
        "POST",
        &server.base_url,
        "/v1/direct/open",
        Some(&serde_json::json!({
            "requester_id": "qa-a",
            "requester_device_id": "browser-a",
            "peer_id": "qa-b",
            "peer_device_id": "browser-b"
        })),
    );
    assert_eq!(direct_status, 200);
    let dm_id = direct["conversation_id"].as_str().expect("conversation id");

    // qa-a sends a message
    let (send_status, _sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": dm_id,
            "sender": "qa-a",
            "text": "unread test message",
            "device_id": "browser",
            "language_tag": "zh-CN"
        })),
    );
    assert_eq!(send_status, 200);

    // Check unread count for qa-b before mark read
    let (state_before_status, state_before) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-b",
        None,
    );
    assert_eq!(state_before_status, 200);
    let dm_before = state_before["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == dm_id)
        .expect("dm room");
    let unread_before = dm_before["unread_count"].as_u64().unwrap_or(0);
    assert!(
        unread_before > 0,
        "unread should be > 0 after message, got {unread_before}"
    );

    // Mark read
    let (mark_status, _mark_payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/read",
        Some(&serde_json::json!({
            "resident_id": "qa-b",
            "conversation_id": dm_id
        })),
    );
    assert_eq!(mark_status, 200);

    // Check unread count after mark read
    let (state_after_status, state_after) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-b",
        None,
    );
    assert_eq!(state_after_status, 200);
    let dm_after = state_after["rooms"]
        .as_array()
        .expect("rooms")
        .iter()
        .find(|room| room["id"] == dm_id)
        .expect("dm room");
    let unread_after = dm_after["unread_count"].as_u64().unwrap_or(0);
    assert_eq!(
        unread_after, 0,
        "unread should be 0 after mark read, got {unread_after}"
    );
}

#[test]
fn resident_endpoint_supports_search_query() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (all_status, all_payload) = http_json("GET", &server.base_url, "/v1/residents", None);
    assert_eq!(all_status, 200);
    let all = all_payload.as_array().expect("should be array");
    assert!(!all.is_empty());

    let (filtered_status, filtered_payload) =
        http_json("GET", &server.base_url, "/v1/residents?q=rsaga", None);
    assert_eq!(filtered_status, 200);
    let filtered = filtered_payload.as_array().expect("should be array");
    assert_eq!(filtered.len(), 1);
    assert_eq!(
        filtered[0].get("resident_id").and_then(|v| v.as_str()),
        Some("rsaga")
    );
}

#[test]
fn admin_summary_endpoint_returns_counts_and_uptime() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json("GET", &server.base_url, "/v1/admin/summary", None);
    assert_eq!(status, 200);
    assert!(
        payload
            .get("resident_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            > 0
    );
    assert!(payload.get("room_count").and_then(|v| v.as_u64()).is_some());
    assert!(
        payload
            .get("message_count")
            .and_then(|v| v.as_u64())
            .is_some()
    );
    assert!(
        payload
            .get("online_count")
            .and_then(|v| v.as_u64())
            .is_some()
    );
    assert!(
        payload
            .get("gateway_uptime_ms")
            .and_then(|v| v.as_i64())
            .is_some()
    );
    assert!(
        payload
            .get("state_version")
            .and_then(|v| v.as_str())
            .is_some()
    );
}

#[test]
fn admin_conversations_endpoint_lists_all_conversations() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json("GET", &server.base_url, "/v1/admin/conversations", None);
    assert_eq!(status, 200);
    let conversations = payload.as_array().expect("should be array");
    assert!(
        !conversations.is_empty(),
        "should have seeded conversations"
    );
    let first = &conversations[0];
    assert!(
        first
            .get("conversation_id")
            .and_then(|v| v.as_str())
            .is_some()
    );
    assert!(first.get("kind").and_then(|v| v.as_str()).is_some());
    assert!(first.get("title").and_then(|v| v.as_str()).is_some());
    assert!(
        first
            .get("participant_count")
            .and_then(|v| v.as_u64())
            .is_some()
    );
    assert!(
        first
            .get("message_count")
            .and_then(|v| v.as_u64())
            .is_some()
    );
}

#[test]
fn admin_messages_endpoint_returns_audit_for_known_conversation() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let url = "/v1/admin/messages?conversation_id=room:world:lobby&limit=10".to_string();
    let (status, payload) = http_json("GET", &server.base_url, &url, None);
    assert_eq!(status, 200);
    assert!(
        payload
            .get("conversation_id")
            .and_then(|v| v.as_str())
            .is_some()
    );
    assert!(
        payload
            .get("total_count")
            .and_then(|v| v.as_u64())
            .is_some()
    );
    assert!(
        payload
            .get("returned_count")
            .and_then(|v| v.as_u64())
            .is_some()
    );
    let messages = payload.get("messages").and_then(|v| v.as_array());
    assert!(messages.is_some());
}

#[test]
fn admin_messages_endpoint_rejects_missing_conversation_id() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, _payload) = http_json("GET", &server.base_url, "/v1/admin/messages", None);
    assert_eq!(status, 400);
}

#[test]
fn admin_residents_endpoint_returns_resident_list() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json("GET", &server.base_url, "/v1/admin/residents", None);
    assert_eq!(status, 200);
    let residents = payload.as_array().expect("should be array");
    assert!(!residents.is_empty(), "should have seeded residents");
    let first = &residents[0];
    assert!(first.get("resident_id").and_then(|v| v.as_str()).is_some());
    assert!(first.get("roles").and_then(|v| v.as_array()).is_some());
    assert!(
        first
            .get("active_cities")
            .and_then(|v| v.as_array())
            .is_some()
    );
    assert!(
        first
            .get("pending_cities")
            .and_then(|v| v.as_array())
            .is_some()
    );
    assert!(first.get("sanctions").and_then(|v| v.as_array()).is_some());
    assert!(first.get("is_banned").and_then(|v| v.as_bool()).is_some());
    assert!(first.get("online").and_then(|v| v.as_bool()).is_some());
}

#[test]
fn admin_residents_endpoint_includes_registered_resident_without_city_membership() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({
        "resident_id": "registered-only",
        "email": "registered-only@example.com"
    });
    let (create_status, _) =
        http_json("POST", &server.base_url, "/v1/admin/residents", Some(&body));
    assert_eq!(create_status, 200);

    let (status, payload) = http_json("GET", &server.base_url, "/v1/admin/residents", None);
    assert_eq!(status, 200);
    let resident = payload
        .as_array()
        .expect("should be array")
        .iter()
        .find(|item| item.get("resident_id").and_then(|v| v.as_str()) == Some("registered-only"))
        .expect("registered resident must be visible before joining a city");
    assert_eq!(
        resident.get("email_masked").and_then(|v| v.as_str()),
        Some("r***@example.com")
    );
    assert_eq!(
        resident.get("registration_state").and_then(|v| v.as_str()),
        Some("active")
    );
    assert!(
        resident
            .get("created_at_ms")
            .and_then(|v| v.as_i64())
            .is_some()
    );
    assert_eq!(
        resident.get("verified_at_ms").and_then(|v| v.as_i64()),
        Some(0)
    );
    assert_eq!(
        resident.get("last_login_at_ms").and_then(|v| v.as_i64()),
        Some(0)
    );
    assert_eq!(
        resident.get("active_cities").and_then(|v| v.as_array()),
        Some(&Vec::new())
    );
}

#[test]
fn admin_ban_and_unban_resident_endpoints_roundtrip() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);

    // Use a seeded resident that already has memberships (e.g. builder)
    let ban_body = serde_json::json!({"resident_id": "builder", "reason": "testing ban", "actor_id": "builder", "actor_id": "rsaga"});
    let (ban_status, ban_payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/residents/ban",
        Some(&ban_body),
    );
    assert_eq!(ban_status, 200);
    assert_eq!(ban_payload.get("ok").and_then(|v| v.as_bool()), Some(true));

    let (list_status, list_payload) =
        http_json("GET", &server.base_url, "/v1/admin/residents", None);
    assert_eq!(list_status, 200);
    let banned = list_payload
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r.get("resident_id").and_then(|v| v.as_str()) == Some("builder"))
        .expect("should find builder in residents");
    assert_eq!(
        banned.get("is_banned").and_then(|v| v.as_bool()),
        Some(true)
    );

    let unban_body = serde_json::json!({"resident_id": "builder", "actor_id": "rsaga"});
    let (unban_status, unban_payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/residents/unban",
        Some(&unban_body),
    );
    assert_eq!(unban_status, 200);
    assert_eq!(
        unban_payload.get("ok").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert!(
        unban_payload
            .get("lifted_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            > 0
    );
}

#[test]
fn admin_set_nickname_endpoint_roundtrip() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);

    // Ensure builder has a registration (seeded only as message sender)
    let create_body = serde_json::json!({"resident_id": "builder", "email": "builder@localhost"});
    let (create_status, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/residents",
        Some(&create_body),
    );
    assert_eq!(create_status, 200);

    let set_body = serde_json::json!({"resident_id": "builder", "nickname": "建筑大师"});
    let (set_status, set_payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/residents/nickname",
        Some(&set_body),
    );
    assert_eq!(set_status, 200);
    assert_eq!(set_payload.get("ok").and_then(|v| v.as_bool()), Some(true));

    // Verify nickname appears in admin residents list
    let (list_status, list_payload) =
        http_json("GET", &server.base_url, "/v1/admin/residents", None);
    assert_eq!(list_status, 200);
    let entry = list_payload
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r.get("resident_id").and_then(|v| v.as_str()) == Some("builder"))
        .expect("should find builder");
    assert_eq!(
        entry.get("nickname").and_then(|v| v.as_str()),
        Some("建筑大师")
    );

    // Clear nickname
    let clear_body = serde_json::json!({"resident_id": "builder", "nickname": null});
    let (clear_status, clear_payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/residents/nickname",
        Some(&clear_body),
    );
    assert_eq!(clear_status, 200);
    assert_eq!(
        clear_payload.get("ok").and_then(|v| v.as_bool()),
        Some(true)
    );

    // Verify nickname is now null
    let (list2_status, list2_payload) =
        http_json("GET", &server.base_url, "/v1/admin/residents", None);
    assert_eq!(list2_status, 200);
    let entry2 = list2_payload
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r.get("resident_id").and_then(|v| v.as_str()) == Some("builder"))
        .expect("should find builder");
    assert_eq!(entry2.get("nickname").and_then(|v| v.as_str()), None);

    // Nonexistent resident
    let bad_body = serde_json::json!({"resident_id": "no-such-resident", "nickname": "test"});
    let (bad_status, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/residents/nickname",
        Some(&bad_body),
    );
    assert_eq!(bad_status, 404);
}

#[test]
fn admin_ban_endpoint_requires_reason() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"resident_id": "rsaga", "reason": "", "actor_id": "rsaga"});
    let (status, _payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/residents/ban",
        Some(&body),
    );
    assert_eq!(status, 400);
}

#[test]
fn admin_rooms_endpoint_returns_room_list() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json("GET", &server.base_url, "/v1/admin/rooms", None);
    assert_eq!(status, 200);
    let rooms = payload.as_array().expect("should be array");
    assert!(!rooms.is_empty(), "should have seeded rooms");
    let first = &rooms[0];
    assert!(first.get("id").and_then(|v| v.as_str()).is_some());
    assert!(first.get("kind").and_then(|v| v.as_str()).is_some());
    assert!(first.get("title").and_then(|v| v.as_str()).is_some());
    assert!(
        first
            .get("participant_count")
            .and_then(|v| v.as_u64())
            .is_some()
    );
    assert!(
        first
            .get("message_count")
            .and_then(|v| v.as_u64())
            .is_some()
    );
    assert!(first.get("is_frozen").and_then(|v| v.as_bool()).is_some());
    assert!(first.get("has_scene").and_then(|v| v.as_bool()).is_some());
}

#[test]
fn admin_freeze_and_unfreeze_room_endpoints_roundtrip() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);

    let room_id = "room:city:core-harbor:lobby";
    let freeze_body = serde_json::json!({"room_id": room_id, "actor_id": "rsaga"});
    let (freeze_status, freeze_payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/rooms/freeze",
        Some(&freeze_body),
    );
    assert_eq!(freeze_status, 200);
    assert_eq!(
        freeze_payload.get("ok").and_then(|v| v.as_bool()),
        Some(true)
    );

    let (list_status, list_payload) = http_json("GET", &server.base_url, "/v1/admin/rooms", None);
    assert_eq!(list_status, 200);
    let room = list_payload
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(room_id))
        .expect("should find lobby room");
    assert_eq!(room.get("is_frozen").and_then(|v| v.as_bool()), Some(true));

    let unfreeze_body = serde_json::json!({"room_id": room_id, "actor_id": "rsaga"});
    let (unfreeze_status, unfreeze_payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/rooms/unfreeze",
        Some(&unfreeze_body),
    );
    assert_eq!(unfreeze_status, 200);
    assert_eq!(
        unfreeze_payload.get("ok").and_then(|v| v.as_bool()),
        Some(true)
    );
}

#[test]
fn admin_freeze_endpoint_rejects_unknown_room() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"room_id": "room:nonexistent", "actor_id": "rsaga"});
    let (status, _payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/rooms/freeze",
        Some(&body),
    );
    assert_eq!(status, 400);
}

#[test]
fn admin_config_endpoints_get_and_set_roundtrip() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);

    let (get_status, _get_payload) = http_json("GET", &server.base_url, "/v1/admin/config", None);
    assert_eq!(get_status, 200);

    let set_body = serde_json::json!({"config": {"max_users": "1000", "maintenance_mode": "false"}, "actor_id": "rsaga"});
    let (set_status, set_payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/config",
        Some(&set_body),
    );
    assert_eq!(set_status, 200);
    assert_eq!(set_payload.get("ok").and_then(|v| v.as_bool()), Some(true));

    let (get_status2, get_payload2) = http_json("GET", &server.base_url, "/v1/admin/config", None);
    assert_eq!(get_status2, 200);
    assert_eq!(
        get_payload2.get("max_users").and_then(|v| v.as_str()),
        Some("1000")
    );
    assert_eq!(
        get_payload2
            .get("maintenance_mode")
            .and_then(|v| v.as_str()),
        Some("false")
    );
}

#[test]
fn admin_config_persists_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime
            .admin_set_config(HashMap::from([
                ("max_users".to_string(), "1000".to_string()),
                ("maintenance_mode".to_string(), "false".to_string()),
            ]))
            .expect("persist config");
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("reopened runtime");
    let config = runtime.admin_get_config();
    assert_eq!(config.get("max_users").map(String::as_str), Some("1000"));
    assert_eq!(
        config.get("maintenance_mode").map(String::as_str),
        Some("false")
    );
}

#[test]
fn admin_ban_endpoint_rejects_missing_resident_id() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"reason": "testing"});
    let (status, _payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/residents/ban",
        Some(&body),
    );
    assert_eq!(status, 400);
}

#[test]
fn admin_unban_endpoint_returns_zero_lifted_for_nonexistent_resident() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"resident_id": "nonexistent-user", "actor_id": "rsaga"});
    let (status, payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/residents/unban",
        Some(&body),
    );
    assert_eq!(status, 200);
    assert_eq!(payload.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        payload.get("lifted_count").and_then(|v| v.as_u64()),
        Some(0)
    );
}

#[test]
fn admin_unban_endpoint_rejects_missing_resident_id() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({});
    let (status, _payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/residents/unban",
        Some(&body),
    );
    assert_eq!(status, 400);
}

#[test]
fn admin_freeze_endpoint_rejects_missing_room_id() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({});
    let (status, _payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/rooms/freeze",
        Some(&body),
    );
    assert_eq!(status, 400);
}

#[test]
fn admin_config_endpoint_rejects_empty_body() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({});
    let (status, _payload) = http_json("POST", &server.base_url, "/v1/admin/config", Some(&body));
    assert_eq!(status, 400);
}

#[test]
fn admin_config_endpoint_rejects_missing_config_field() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"other": "value"});
    let (status, _payload) = http_json("POST", &server.base_url, "/v1/admin/config", Some(&body));
    assert_eq!(status, 400);
}

#[test]
fn admin_messages_endpoint_returns_empty_for_invalid_conversation_id() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json(
        "GET",
        &server.base_url,
        "/v1/admin/messages?conversation_id=room:invalid:nonexistent",
        None,
    );
    assert_eq!(status, 200);
    assert_eq!(payload["messages"].as_array().unwrap().len(), 0);
    assert_eq!(payload["total_count"], 0);
    assert_eq!(payload["returned_count"], 0);
}

#[test]
fn admin_rooms_endpoint_returns_seeded_rooms_for_fresh_runtime() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json("GET", &server.base_url, "/v1/admin/rooms", None);
    assert_eq!(status, 200);
    let rooms = payload.as_array().expect("rooms array");
    assert!(
        !rooms.is_empty(),
        "fresh runtime should have seeded public rooms"
    );
}

#[test]
fn admin_config_endpoint_returns_empty_for_fresh_runtime() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json("GET", &server.base_url, "/v1/admin/config", None);
    assert_eq!(status, 200);
    let config = payload.as_object().expect("config object");
    assert_eq!(config.len(), 0);
}

#[test]
fn scene_validate_endpoint_rejects_invalid_config() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);

    let body = serde_json::json!({
        "image_layer": {
            "preset": "custom",
            "layer_id": "bg-1",
            "asset_hint": "",
            "aspect_ratio_permyriad": 5625,
            "owner_editable": false
        }
    });
    let (status, payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/scene/validate",
        Some(&body),
    );
    assert_eq!(status, 200);
    assert_eq!(payload.get("valid").and_then(|v| v.as_bool()), Some(false));
    let errors = payload.get("errors").and_then(|v| v.as_array());
    assert!(errors.is_some());
    assert!(!errors.unwrap().is_empty());
}

#[test]
fn scene_validate_endpoint_accepts_valid_config() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);

    let body = serde_json::json!({
        "image_layer": {
            "preset": "custom",
            "layer_id": "bg-1",
            "asset_hint": "bedroom_cozy",
            "aspect_ratio_permyriad": 5625,
            "owner_editable": false
        },
        "hotspot_layer": {
            "layer_id": "hs-1",
            "coordinate_system": "permyriad",
            "owner_editable": false,
            "hotspots": [
                {"hotspot_id": "h1", "label": "door", "sprite_hint": "", "interaction_hint": "", "x_permyriad": 5000, "y_permyriad": 8000, "width_permyriad": 200, "height_permyriad": 200},
                {"hotspot_id": "h2", "label": "window", "sprite_hint": "", "interaction_hint": "", "x_permyriad": 2000, "y_permyriad": 3000, "width_permyriad": 200, "height_permyriad": 200}
            ]
        }
    });
    let (status, payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/scene/validate",
        Some(&body),
    );
    assert_eq!(status, 200);
    assert_eq!(payload.get("valid").and_then(|v| v.as_bool()), Some(true));
    let errors = payload.get("errors").and_then(|v| v.as_array());
    assert!(errors.is_some());
    assert!(errors.unwrap().is_empty());
}

// --- IM 后端收尾测试：持久化 + 速率限制 + 缺口补齐 ---

#[test]
fn presence_state_persists_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime.record_presence("rsaga");
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
    assert!(runtime.presence.contains_key("rsaga"));
    let last_seen = runtime.presence.get("rsaga").copied().unwrap();
    assert!(last_seen > 0);
}

#[test]
fn unread_state_persists_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");
    let dm_id;

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        let alice = chat_core::IdentityId("alice".into());
        let bob = chat_core::IdentityId("bob".into());
        dm_id = runtime.resolve_direct_conversation_id(&alice, &bob);
        runtime
            .ensure_direct_conversation(&dm_id, &[alice.clone(), bob.clone()])
            .expect("ensure dm");
        runtime.increment_unread(&dm_id, &alice);
        assert_eq!(runtime.unread_count(&bob, &dm_id), 1);
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
    let bob = chat_core::IdentityId("bob".into());
    let count = runtime.unread_count(&bob, &dm_id);
    assert_eq!(count, 1);
}

#[test]
fn rate_limit_rejects_excessive_messages() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    for _ in 0..3 {
        let blocked = runtime.check_rate_limit("spammer", 3);
        assert!(blocked.is_none());
    }

    let blocked = runtime.check_rate_limit("spammer", 3);
    assert!(blocked.is_some());
}

#[test]
fn rate_limit_resets_after_window() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    for _ in 0..3 {
        runtime.check_rate_limit("sender", 3);
    }
    assert!(runtime.check_rate_limit("sender", 3).is_some());

    // Mutate the window start to simulate time passing
    if let Some(window) = runtime.rate_limits.get_mut("sender") {
        window.window_start_ms -= 120_000;
    }

    let blocked = runtime.check_rate_limit("sender", 3);
    assert!(blocked.is_none());
}

#[test]
fn rate_limit_blocks_sender_via_http() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    register_resident(&mut runtime, "rater");
    let server = start_local_gateway_http_server(runtime);

    // 31st message past 30/min cap should return 429
    let mut hit_429 = false;
    for i in 0..40 {
        let (status, _body) = http_json(
            "POST",
            &server.base_url,
            "/v1/shell/message",
            Some(&serde_json::json!({
                "room_id": "room:world:lobby",
                "sender": "rater",
                "text": format!("msg {}", i),
            })),
        );
        if status == 429 {
            hit_429 = true;
            break;
        }
    }
    assert!(hit_429, "HTTP 429 rate limit expected after 30+ messages");
}

#[test]
fn otp_request_rate_limited_per_email() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    // First request should succeed
    let (status1, challenge1) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/request",
        Some(&serde_json::json!({
            "email": "ratelimited@example.com",
            "resident_id": "rl-user"
        })),
    );
    assert_eq!(status1, 200);
    assert!(challenge1["challenge_id"].is_string());

    // Second request within the same minute should be rate-limited
    let (status2, body2) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/request",
        Some(&serde_json::json!({
            "email": "ratelimited@example.com",
            "resident_id": "rl-user-2"
        })),
    );
    assert_eq!(status2, 400);
    let err = body2["Error"]["message"].as_str().unwrap_or_default();
    assert!(
        err.contains("too many otp requests"),
        "expected rate limit error, got: {err}"
    );
}

#[test]
fn otp_verify_rate_limited_per_challenge() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (_s, challenge) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/request",
        Some(&serde_json::json!({
            "email": "verify-rl@example.com",
            "resident_id": "verify-rl"
        })),
    );
    let challenge_id = challenge["challenge_id"].as_str().expect("challenge_id");

    // Attempt 6 verifications with wrong code — first 5 should be valid attempts, 6th rate-limited
    let mut hit_rate_limit = false;
    for i in 0..6 {
        let (status, body) = http_json(
            "POST",
            &server.base_url,
            "/v1/auth/email-otp/verify",
            Some(&serde_json::json!({
                "challenge_id": challenge_id,
                "code": "000000",
                "resident_id": "verify-rl"
            })),
        );
        if status == 400 {
            let err = body["Error"]["message"].as_str().unwrap_or_default();
            if err.contains("too many otp verification") {
                hit_rate_limit = true;
                break;
            }
            // Otherwise it's "invalid otp code"
            assert!(
                err.contains("invalid otp code"),
                "unexpected error at attempt {}: {err}",
                i + 1
            );
        }
    }
    assert!(
        hit_rate_limit,
        "expected rate limit after 5 failed verify attempts"
    );
}

#[test]
fn cors_origin_reads_from_env() {
    // Default is wildcard
    let default = crate::http_support::cors_origin_header().expect("default cors header");
    assert_eq!(
        default.value.as_str(),
        "*",
        "default CORS origin should be wildcard"
    );

    // Set env var and verify
    unsafe {
        std::env::set_var("LOBSTER_CORS_ORIGIN", "https://example.com");
    }
    let custom = crate::http_support::cors_origin_header().expect("custom cors header");
    assert_eq!(
        custom.value.as_str(),
        "https://example.com",
        "CORS origin should match env var"
    );

    // Empty string should fall back to wildcard
    unsafe {
        std::env::set_var("LOBSTER_CORS_ORIGIN", "");
    }
    let empty = crate::http_support::cors_origin_header().expect("empty cors header");
    assert_eq!(
        empty.value.as_str(),
        "*",
        "empty CORS origin should be wildcard"
    );

    unsafe {
        std::env::set_var(
            "LOBSTER_CORS_ORIGIN",
            "https://bad.example\nX-Bad: injected",
        );
    }
    let invalid = crate::http_support::cors_origin_header().expect("invalid cors header fallback");
    assert_eq!(
        invalid.value.as_str(),
        "*",
        "invalid CORS origin should fall back to wildcard"
    );

    unsafe {
        std::env::remove_var("LOBSTER_CORS_ORIGIN");
    }
}

#[test]
fn recall_rejects_non_sender() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let response = runtime
        .append_shell_message(ShellMessageRequest {
            room_id: "room:world:lobby".into(),
            sender: "rsaga".into(),
            text: "my message".into(),
            reply_to_message_id: None,
            device_id: None,
            language_tag: None,
        })
        .expect("send message");

    let result = runtime.recall_shell_message(RecallShellMessageRequest {
        room_id: "room:world:lobby".into(),
        message_id: response.message_id,
        actor: "intruder".into(),
        actor_address: None,
    });
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("only the original sender"));
}

#[test]
fn edit_rejects_invalid_message_id() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let result = runtime.edit_shell_message(EditShellMessageRequest {
        room_id: "room:world:lobby".into(),
        message_id: "nonexistent-msg-id".into(),
        actor: "rsaga".into(),
        text: "trying to edit".into(),
        actor_address: None,
    });
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn recall_rejects_invalid_message_id() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let result = runtime.recall_shell_message(RecallShellMessageRequest {
        room_id: "room:world:lobby".into(),
        message_id: "nonexistent-msg-id".into(),
        actor: "rsaga".into(),
        actor_address: None,
    });
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn edit_and_recall_state_persists_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");
    let message_id;

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        let response = runtime
            .append_shell_message(ShellMessageRequest {
                room_id: "room:world:lobby".into(),
                sender: "rsaga".into(),
                text: "original text".into(),
                reply_to_message_id: None,
                device_id: None,
                language_tag: None,
            })
            .expect("send message");
        message_id = response.message_id;

        runtime
            .edit_shell_message(EditShellMessageRequest {
                room_id: "room:world:lobby".into(),
                message_id: message_id.clone(),
                actor: "rsaga".into(),
                text: "edited text".into(),
                actor_address: None,
            })
            .expect("edit message");
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
    let state = runtime.shell_state();
    let lobby = state
        .rooms
        .into_iter()
        .find(|room| room.id == "room:world:lobby")
        .expect("lobby room");
    let edited = lobby
        .messages
        .iter()
        .find(|m| m.message_id == message_id)
        .expect("edited message");
    assert_eq!(edited.text, "edited text");
    assert!(edited.is_edited);
}

#[test]
fn presence_heartbeat_triggers_sse_notify_on_first_heartbeat() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    // First heartbeat: not online yet, should trigger notify
    let became_online = runtime.record_presence("new-resident");
    assert!(became_online);
}

#[test]
fn shell_state_caps_recent_messages_at_32() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    // Seed 40 messages to world lobby
    for i in 0..40 {
        let text = format!("message {}", i);
        runtime
            .append_shell_message(ShellMessageRequest {
                room_id: "room:world:lobby".into(),
                sender: "rsaga".into(),
                text,
                device_id: Some("test".into()),
                language_tag: Some("zh-CN".into()),
                reply_to_message_id: None,
            })
            .expect("publish message");
    }

    let state = runtime.shell_state();
    let lobby = state
        .rooms
        .into_iter()
        .find(|room| room.id == "room:world:lobby")
        .expect("lobby room");
    assert!(
        lobby.messages.len() <= 32,
        "shell state should cap messages at 32, got {}",
        lobby.messages.len()
    );
}

#[test]
fn shell_message_edit_does_not_increment_unread() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    let alice = IdentityId("alice".into());
    let bob = IdentityId("bob".into());
    let dm = runtime.resolve_direct_conversation_id(&alice, &bob);
    runtime
        .ensure_direct_conversation(&dm, &[alice.clone(), bob.clone()])
        .expect("ensure dm");

    // Alice sends a message
    runtime
        .append_shell_message(ShellMessageRequest {
            room_id: dm.0.clone(),
            sender: "alice".into(),
            text: "original".into(),
            device_id: Some("test".into()),
            language_tag: Some("zh-CN".into()),
            reply_to_message_id: None,
        })
        .expect("publish");

    let unread_after_send = runtime.unread_count(&bob, &dm);
    assert!(
        unread_after_send > 0,
        "unread should increase after new message"
    );

    // Bob marks read
    runtime.mark_read(&bob, &dm);
    assert_eq!(
        runtime.unread_count(&bob, &dm),
        0,
        "unread should be 0 after mark read"
    );

    // Alice edits the message
    let message_id = runtime
        .timeline_store
        .recent_messages(&dm, 1)
        .first()
        .expect("one message")
        .envelope
        .message_id
        .0
        .clone();
    runtime
        .edit_shell_message(EditShellMessageRequest {
            room_id: dm.0.clone(),
            message_id,
            actor: "alice".into(),
            text: "edited".into(),
            actor_address: None,
        })
        .expect("edit");

    let unread_after_edit = runtime.unread_count(&bob, &dm);
    assert_eq!(
        unread_after_edit, 0,
        "edit should not increment unread, got {unread_after_edit}"
    );
}

#[test]
fn admin_moderate_message_approve_block_handled_roundtrip() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);

    // First send a message to have a real message_id
    let convo_id = "room:world:lobby";
    let send_body = serde_json::json!({
        "room_id": convo_id,
        "sender": "qa-a",
        "text": "审核测试消息",
        "device_id": "test",
        "language_tag": "zh"
    });
    let (send_status, send_payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&send_body),
    );
    assert_eq!(send_status, 200);
    let message_id = send_payload["message_id"]
        .as_str()
        .expect("should have message_id");

    // Moderate: approve
    let approve_body = serde_json::json!({
        "message_id": message_id,
        "conversation_id": convo_id,
        "action": "approved",
        "actor_id": "rsaga"
    });
    let (status, payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/messages/moderate",
        Some(&approve_body),
    );
    assert_eq!(status, 200);
    assert_eq!(payload["ok"].as_bool(), Some(true));
    assert_eq!(payload["action"].as_str(), Some("approved"));

    // Moderate: block
    let block_body = serde_json::json!({
        "message_id": message_id,
        "conversation_id": convo_id,
        "action": "blocked",
        "actor_id": "rsaga"
    });
    let (status, payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/messages/moderate",
        Some(&block_body),
    );
    assert_eq!(status, 200);
    assert_eq!(payload["action"].as_str(), Some("blocked"));

    // Moderate: handled
    let handled_body = serde_json::json!({
        "message_id": message_id,
        "conversation_id": convo_id,
        "action": "handled",
        "actor_id": "rsaga"
    });
    let (status, payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/messages/moderate",
        Some(&handled_body),
    );
    assert_eq!(status, 200);
    assert_eq!(payload["action"].as_str(), Some("handled"));
}

#[test]
fn admin_moderate_message_rejects_invalid_action() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);

    let body = serde_json::json!({
        "message_id": "msg:nonexistent",
        "conversation_id": "room:world:lobby",
        "action": "invalid_action",
        "actor_id": "rsaga"
    });
    let (status, _payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/messages/moderate",
        Some(&body),
    );
    assert_eq!(status, 400);
}

#[test]
fn admin_moderate_message_rejects_nonexistent_message() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);

    let body = serde_json::json!({
        "message_id": "msg:nonexistent-99999",
        "conversation_id": "room:world:lobby",
        "action": "approved",
        "actor_id": "rsaga"
    });
    let (status, _payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/messages/moderate",
        Some(&body),
    );
    assert_eq!(status, 400);
}

#[test]
fn admin_invites_create_and_revoke_endpoints_roundtrip() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    // Create invite
    let create_body = serde_json::json!({"actor_id": "qa-a"});
    let (status, payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/invites",
        Some(&create_body),
    );
    assert_eq!(status, 200);
    let code = payload["code"].as_str().unwrap().to_string();
    assert!(code.starts_with("AJW-"));
    // Revoke it
    let revoke_body = serde_json::json!({"code": code, "actor_id": "qa-a"});
    let (status2, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/invites/revoke",
        Some(&revoke_body),
    );
    assert_eq!(status2, 200);
    // Revoke non-existent
    let bad_body = serde_json::json!({"code": "AJW-000000", "actor_id": "qa-a"});
    let (status3, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/invites/revoke",
        Some(&bad_body),
    );
    assert_eq!(status3, 404);
}

#[test]
fn admin_logs_handle_endpoint_returns_ok() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"log_id": "log-test-1", "actor_id": "qa-a"});
    let (status, payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/logs/handle",
        Some(&body),
    );
    assert_eq!(status, 200);
    assert_eq!(payload.get("ok").and_then(|v| v.as_bool()), Some(true));
}

#[test]
fn admin_rooms_members_add_and_remove_endpoints_roundtrip() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let room_id = "room:city:core-harbor:lobby";
    // Add
    let add_body = serde_json::json!({"room_id": room_id, "resident_id": "qa-b", "actor_id": "qa-a", "action": "add"});
    let (status, payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/rooms/members",
        Some(&add_body),
    );
    assert_eq!(status, 200);
    assert_eq!(payload.get("ok").and_then(|v| v.as_bool()), Some(true));
    // Remove
    let rm_body = serde_json::json!({"room_id": room_id, "resident_id": "qa-b", "actor_id": "qa-a", "action": "remove"});
    let (status2, payload2) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/rooms/members",
        Some(&rm_body),
    );
    assert_eq!(status2, 200);
    assert_eq!(payload2.get("ok").and_then(|v| v.as_bool()), Some(true));
    // Bad room
    let bad_body = serde_json::json!({"room_id": "room:nonexistent", "resident_id": "qa-b", "actor_id": "qa-a", "action": "add"});
    let (status3, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/rooms/members",
        Some(&bad_body),
    );
    assert_eq!(status3, 404);
}

#[test]
fn admin_logs_clear_returns_ok() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"log_id": "log-test-1", "actor_id": "qa-a"});
    http_json(
        "POST",
        &server.base_url,
        "/v1/admin/logs/handle",
        Some(&body),
    );
    let clear_body = serde_json::json!({});
    let (status, payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/logs/clear",
        Some(&clear_body),
    );
    assert_eq!(status, 200);
    assert_eq!(payload.get("ok").and_then(|v| v.as_bool()), Some(true));
}

#[test]
fn admin_create_resident_endpoint_succeeds() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"resident_id": "new-resident-test", "email": "test@example.com"});
    let (status, payload) = http_json("POST", &server.base_url, "/v1/admin/residents", Some(&body));
    assert_eq!(status, 200);
    assert_eq!(payload.get("ok").and_then(|v| v.as_bool()), Some(true));
}

#[test]
fn admin_create_resident_duplicate_returns_409() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"resident_id": "dup-test", "email": "dup@example.com"});
    let (status1, _) = http_json("POST", &server.base_url, "/v1/admin/residents", Some(&body));
    assert_eq!(status1, 200);
    let (status2, payload2) =
        http_json("POST", &server.base_url, "/v1/admin/residents", Some(&body));
    assert_eq!(status2, 409);
    assert_eq!(payload2.get("ok").and_then(|v| v.as_bool()), Some(false));
}

#[test]
fn admin_created_resident_persists_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        assert!(
            runtime
                .admin_create_resident("persisted-resident", "persisted@example.com")
                .expect("create resident")
        );
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("reopened runtime");
    assert!(
        runtime
            .registrations
            .iter()
            .any(|registration| registration.resident_id.0 == "persisted-resident")
    );
}

#[test]
fn world_directory_returns_ok() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json("GET", &server.base_url, "/v1/world-directory", None);
    assert_eq!(status, 200);
    assert!(payload.is_object());
}

#[test]
fn world_safety_returns_ok() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json("GET", &server.base_url, "/v1/world-safety", None);
    assert_eq!(status, 200);
    assert!(payload.is_object());
}

#[test]
fn shell_bootstrap_returns_ok() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, _payload) = http_json("GET", &server.base_url, "/v1/shell/bootstrap", None);
    assert_eq!(status, 200);
}

#[test]
fn admin_config_get_returns_ok() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json("GET", &server.base_url, "/v1/admin/config", None);
    assert_eq!(status, 200);
    assert!(payload.is_object());
}

#[test]
fn world_entry_returns_ok() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, payload) = http_json("GET", &server.base_url, "/v1/world-entry", None);
    assert_eq!(status, 200);
    assert!(payload.is_object());
}

#[test]
fn world_entry_endpoint_smoke() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, _) = http_json("GET", &server.base_url, "/v1/world-entry", None);
    assert_eq!(status, 200);
}

#[test]
fn residents_endpoint_returns_ok() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, _) = http_json("GET", &server.base_url, "/v1/residents", None);
    assert_eq!(status, 200);
}

#[test]
fn world_snapshot_returns_ok() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let (status, _) = http_json("GET", &server.base_url, "/v1/world-snapshot", None);
    assert_eq!(status, 200);
}

#[test]
fn moderation_state_persists_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");
    let msg_id = "gw-test-msg-1";

    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        runtime
            .message_moderation
            .insert(msg_id.to_string(), "approved".to_string());
        let _ = runtime.persist_moderation_state();
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
    let status = runtime.admin_message_moderation_status(msg_id);
    assert_eq!(status, Some("approved"));
}

#[test]
fn send_to_nonexistent_room_is_rejected() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    let send_body = serde_json::json!({
        "room_id": "room:nonexistent:fake",
        "sender": "qa-a",
        "text": "should fail"
    });
    let (_status, body) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&send_body),
    );
    let ok = body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    assert!(!ok, "send to nonexistent room must not succeed");
}

#[test]
fn admin_moderation_status_readback() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    runtime
        .message_moderation
        .insert("msg-001".to_string(), "blocked".to_string());
    assert_eq!(
        runtime.admin_message_moderation_status("msg-001"),
        Some("blocked")
    );
    assert_eq!(runtime.admin_message_moderation_status("msg-099"), None);
}

#[test]
fn concurrent_presence_heartbeats_via_http() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    let server = start_local_gateway_http_server(runtime);
    // send presence for two residents
    let a_body = serde_json::json!({"resident_id": "builder", "actor_id": "rsaga"});
    let (s1, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/presence",
        Some(&a_body),
    );
    assert_eq!(s1, 200);
    let b_body = serde_json::json!({"resident_id": "rsaga"});
    let (s2, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/presence",
        Some(&b_body),
    );
    assert_eq!(s2, 200);
    // verify residents endpoint returns them (enriched)
    let (s3, residents) = http_json("GET", &server.base_url, "/v1/residents", None);
    assert_eq!(s3, 200);
    assert!(
        !residents.as_array().map(|a| a.is_empty()).unwrap_or(true),
        "residents list should not be empty after heartbeats"
    );
}

#[test]
fn admin_logs_endpoint_returns_entries() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    runtime
        .admin_handle_log("audit-001")
        .expect("persist handled log");
    runtime
        .admin_handle_log("audit-002")
        .expect("persist handled log");
    let server = start_local_gateway_http_server(runtime);
    let (status, body) = http_json("GET", &server.base_url, "/v1/admin/logs", None);
    assert_eq!(status, 200);
    let arr = body.as_array().expect("logs should be array");
    assert_eq!(arr.len(), 2);
}

#[test]
fn admin_invites_list_endpoint_returns_created_invites() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    runtime
        .admin_create_invite("qa-a", 0)
        .expect("persist invite");
    let server = start_local_gateway_http_server(runtime);
    let (status, body) = http_json("GET", &server.base_url, "/v1/admin/invites", None);
    assert_eq!(status, 200);
    let arr = body.as_array().expect("invites should be array");
    assert!(!arr.is_empty(), "should have at least one invite");
}

#[test]
fn admin_messages_moderation_endpoint_returns_status() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path(), 64, None).expect("open gateway");
    runtime
        .message_moderation
        .insert("msg-xyz".into(), "approved".into());
    let server = start_local_gateway_http_server(runtime);
    let (status, body) = http_json(
        "GET",
        &server.base_url,
        "/v1/admin/messages/moderation?message_id=msg-xyz",
        None,
    );
    assert_eq!(status, 200);
    assert_eq!(body["status"].as_str(), Some("approved"));
    assert_eq!(body["message_id"].as_str(), Some("msg-xyz"));
}

#[test]
fn create_and_list_permission_groups_via_http() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let body = serde_json::json!({
        "actor_id": "admin-1",
        "name": "测试组",
        "description": "测试用权限组",
        "capabilities": ["send:message", "read:public"]
    });
    let (status, created) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/permission-groups",
        Some(&body),
    );
    assert_eq!(status, 200);
    assert_eq!(created["ok"], serde_json::Value::Bool(true));
    assert!(created["group"]["id"].as_str().unwrap().starts_with("pg-"));
    assert_eq!(created["group"]["name"].as_str(), Some("测试组"));
    assert_eq!(
        created["group"]["capabilities"].as_array().unwrap().len(),
        2
    );

    let (status, list) = http_json("GET", &server.base_url, "/v1/admin/permission-groups", None);
    assert_eq!(status, 200);
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["name"].as_str(), Some("测试组"));
}

#[test]
fn assign_permission_group_via_http() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    register_resident(&mut runtime, "alice");
    let resp = runtime
        .admin_create_permission_group(
            "admin-1",
            "协管",
            "steward-like",
            vec!["freeze:room".into(), "moderate:message".into()],
        )
        .expect("persist permission group");
    let pg_id = resp.group.id;
    let server = start_local_gateway_http_server(runtime);

    let body = serde_json::json!({"actor_id": "admin-1", "resident_id": "alice", "permission_group_id": &pg_id});
    let (status, assigned) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/permission-groups/assign",
        Some(&body),
    );
    assert_eq!(status, 200);
    assert_eq!(assigned["ok"], serde_json::Value::Bool(true));
    assert_eq!(assigned["resident_id"].as_str(), Some("alice"));
    assert_eq!(
        assigned["permission_group_id"].as_str(),
        Some(pg_id.as_str())
    );
}

#[test]
fn permission_groups_persist_across_restart() {
    let temp = tempdir().expect("temp dir");
    let pg_id;
    {
        let mut runtime =
            GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
        let resp = runtime
            .admin_create_permission_group(
                "admin-1",
                "持久化组",
                "survive restart",
                vec!["admin:config".into()],
            )
            .expect("persist permission group");
        pg_id = resp.group.id;
    }
    {
        let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
        let groups = runtime.admin_list_permission_groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, pg_id);
        assert_eq!(groups[0].name, "持久化组");
        assert_eq!(groups[0].capabilities, vec!["admin:config"]);
    }
}

#[test]
fn admin_ops_persist_across_restart() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path().join("gateway");
    let invite_code;
    let permission_group_id;
    let moderated_message_id;
    {
        let mut runtime = GatewayRuntime::open(&root, 64, None).expect("runtime");
        invite_code = runtime
            .admin_create_invite("admin-1", 3)
            .expect("persist invite")
            .code;
        runtime
            .admin_revoke_invite(&invite_code)
            .expect("persist revoked invite");
        runtime
            .admin_handle_log("audit-persisted")
            .expect("persist handled log");
        let message = runtime
            .append_shell_message(ShellMessageRequest {
                room_id: "room:world:lobby".into(),
                sender: "rsaga".into(),
                text: "moderation persistence".into(),
                reply_to_message_id: None,
                device_id: Some("browser".into()),
                language_tag: Some("en".into()),
            })
            .expect("append moderation target");
        moderated_message_id = message.message_id.clone();
        runtime
            .admin_moderate_message(&message.message_id, "room:world:lobby", "blocked")
            .expect("persist moderation decision");
        let permission_group = runtime
            .admin_create_permission_group(
                "admin-1",
                "持久化协管",
                "survive restart",
                vec!["freeze:room".into()],
            )
            .expect("persist permission group");
        permission_group_id = permission_group.group.id;
        runtime
            .admin_assign_permission_group("alice", &permission_group_id)
            .expect("persist permission assignment");
        let room = runtime
            .create_public_room(CreatePublicRoomRequest {
                city: "core-harbor".into(),
                creator_id: "rsaga".into(),
                slug: Some("persisted-member-room".into()),
                title: "Persisted member room".into(),
                description: "room membership persistence".into(),
            })
            .expect("create room");
        assert!(
            runtime
                .admin_manage_room_member(&room.room_id.0, "alice", "add")
                .expect("persist room member")
        );
        runtime
            .admin_add_device(
                "AA:BB:CC:DD:EE:FF".into(),
                "测试设备".into(),
                "admin-1".into(),
            )
            .expect("persist device");
        runtime
            .admin_block_device("AA:BB:CC:DD:EE:FF")
            .expect("persist blocked device");
    }

    let runtime = GatewayRuntime::open(&root, 64, None).expect("reopened runtime");
    let invites = runtime.admin_list_invites();
    let invite = invites
        .iter()
        .find(|invite| invite.code == invite_code)
        .expect("reopened invite");
    assert!(invite.revoked);
    assert_eq!(runtime.admin_logs()[0].status, "handled");
    assert_eq!(
        runtime.admin_message_moderation_status(&moderated_message_id),
        Some("blocked")
    );
    assert_eq!(
        runtime.admin_list_permission_groups()[0].id,
        permission_group_id
    );
    assert!(runtime.resident_has_capability("alice", "freeze:room"));
    let room = runtime
        .timeline_store
        .active_conversations()
        .into_iter()
        .find(|conversation| {
            conversation.conversation_id.0 == "room:city:core-harbor:persisted-member-room"
        })
        .expect("reopened room");
    assert!(
        room.participants
            .iter()
            .any(|resident| resident.0 == "alice")
    );
    let devices = runtime.admin_list_devices();
    assert_eq!(devices.len(), 1);
    assert!(devices[0].blocked);
    assert_eq!(devices[0].label, "测试设备");
}

#[test]
fn capability_catalog_endpoint_returns_all_entries() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);
    let (status, catalog) = http_json("GET", &server.base_url, "/v1/admin/capabilities", None);
    assert_eq!(status, 200);
    let arr = catalog.as_array().expect("catalog should be array");
    assert!(arr.len() >= 14);
    let keys: Vec<&str> = arr.iter().filter_map(|v| v["key"].as_str()).collect();
    assert!(keys.contains(&"send:message"));
    assert!(keys.contains(&"manage:permissions"));
}

#[test]
fn create_permission_group_rejects_empty_name() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);
    let body =
        serde_json::json!({"actor_id": "admin-1", "name": "", "capabilities": ["send:message"]});
    let (status, resp) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/permission-groups",
        Some(&body),
    );
    assert_eq!(status, 400);
    assert!(resp["error"].as_str().unwrap().contains("name"));
}

#[test]
fn create_permission_group_rejects_empty_capabilities() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);
    let body = serde_json::json!({"actor_id": "admin-1", "name": "X", "capabilities": []});
    let (status, resp) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/permission-groups",
        Some(&body),
    );
    assert_eq!(status, 400);
    assert!(resp["error"].as_str().unwrap().contains("capability"));
}

#[test]
fn admin_read_endpoints_require_bearer_auth_without_dev_bypass() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    let (valid_token, _) = runtime.issue_auth_session(
        &IdentityId("admin_rsaga".into()),
        "test-admin-read",
        GatewayRuntime::now_ms(),
    );
    let server = start_local_gateway_http_server(runtime);

    for path in [
        "/v1/admin/summary",
        "/v1/admin/conversations",
        "/v1/admin/messages?conversation_id=room:test",
        "/v1/admin/residents",
        "/v1/admin/rooms",
        "/v1/admin/config",
        "/v1/admin/logs",
        "/v1/admin/messages/moderation?message_id=missing",
        "/v1/admin/invites",
        "/v1/admin/permission-groups",
        "/v1/admin/audit-log",
        "/v1/admin/devices",
    ] {
        let (status, _, body) = http_raw("GET", &server.base_url, path, None);
        assert_eq!(status, 401, "admin read {path} must require auth: {body}");
    }

    let (forged_status, _, forged_body) = http_raw_with_headers(
        "GET",
        &server.base_url,
        "/v1/admin/summary",
        &[("Authorization", "Bearer forged-admin-token")],
        None,
    );
    assert_eq!(
        forged_status, 401,
        "admin read must reject an unknown bearer token: {forged_body}"
    );

    let valid_auth = format!("Bearer {valid_token}");
    let (valid_status, _, valid_body) = http_raw_with_headers(
        "GET",
        &server.base_url,
        "/v1/admin/summary",
        &[("Authorization", valid_auth.as_str())],
        None,
    );
    assert_eq!(
        valid_status, 200,
        "valid admin session should be accepted: {valid_body}"
    );

    let (status, _, _) = http_raw("GET", &server.base_url, "/v1/admin/capabilities", None);
    assert_eq!(status, 200, "capability catalog remains public metadata");
}

#[test]
fn admin_log_mutations_require_bearer_auth_without_dev_bypass() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    let server = start_local_gateway_http_server(runtime);

    let (handle_status, _, _) = http_raw(
        "POST",
        &server.base_url,
        "/v1/admin/logs/handle",
        Some(&serde_json::json!({"log_id": "audit-001", "actor_id": "admin_rsaga"})),
    );
    assert_eq!(handle_status, 401);

    let (clear_status, _, _) = http_raw(
        "POST",
        &server.base_url,
        "/v1/admin/logs/clear",
        Some(&serde_json::json!({})),
    );
    assert_eq!(clear_status, 401);
}

#[test]
fn admin_device_mutations_require_valid_bearer_auth_without_dev_bypass() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    let server = start_local_gateway_http_server(runtime);

    for (path, body) in [
        (
            "/v1/admin/devices/add",
            serde_json::json!({"address":"aa:bb:cc:dd:ee:ff","label":"test"}),
        ),
        (
            "/v1/admin/devices/remove",
            serde_json::json!({"address":"aa:bb:cc:dd:ee:ff"}),
        ),
        (
            "/v1/admin/devices/block",
            serde_json::json!({"address":"aa:bb:cc:dd:ee:ff"}),
        ),
        (
            "/v1/admin/devices/unblock",
            serde_json::json!({"address":"aa:bb:cc:dd:ee:ff"}),
        ),
    ] {
        let (status, _, response) = http_raw("POST", &server.base_url, path, Some(&body));
        assert_eq!(
            status, 401,
            "device mutation {path} must require auth: {response}"
        );
    }
}

#[test]
fn high_risk_admin_mutations_require_valid_bearer_auth_without_dev_bypass() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    let server = start_local_gateway_http_server(runtime);

    for path in [
        "/v1/admin/residents",
        "/v1/admin/permission-groups",
        "/v1/admin/permission-groups/assign",
        "/v1/admin/residents/unsanction",
        "/v1/world-safety/residents/sanction",
    ] {
        let (status, _, response) =
            http_raw("POST", &server.base_url, path, Some(&serde_json::json!({})));
        assert_eq!(
            status, 401,
            "high-risk mutation {path} must require auth: {response}"
        );
    }
}

#[test]
fn core_admin_write_routes_require_bearer_auth_without_dev_bypass() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    let server = start_local_gateway_http_server(runtime);

    for path in [
        "/v1/admin/residents/ban",
        "/v1/admin/residents/unban",
        "/v1/admin/residents/nickname",
        "/v1/admin/rooms/freeze",
        "/v1/admin/rooms/unfreeze",
        "/v1/admin/config",
        "/v1/admin/messages/moderate",
        "/v1/admin/invites",
        "/v1/admin/invites/revoke",
        "/v1/admin/rooms/members",
        "/v1/admin/logs/handle",
        "/v1/admin/logs/clear",
        "/v1/admin/scene",
    ] {
        let (status, _, response) =
            http_raw("POST", &server.base_url, path, Some(&serde_json::json!({})));
        assert_eq!(
            status, 401,
            "core admin write {path} must reject missing Bearer auth: {response}"
        );
    }
}

#[test]
fn provider_and_mirror_write_routes_require_bearer_auth_without_dev_bypass() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    let server = start_local_gateway_http_server(runtime);

    for (path, body) in [
        (
            "/v1/provider/connect",
            serde_json::json!({"provider_url":"http://127.0.0.1:9"}),
        ),
        ("/v1/provider/disconnect", serde_json::json!({})),
        (
            "/v1/world-mirror-sources",
            serde_json::json!({"base_url":"http://mirror.example.invalid","enabled":false}),
        ),
    ] {
        let (status, _, response) = http_raw("POST", &server.base_url, path, Some(&body));
        assert_eq!(
            status, 401,
            "provider/mirror write {path} must reject missing Bearer auth: {response}"
        );
    }
}

#[test]
fn export_requires_matching_bearer_session_without_dev_bypass() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    let (valid_token, _) = runtime.issue_auth_session(
        &IdentityId("export-user".into()),
        "export-auth-boundary",
        GatewayRuntime::now_ms(),
    );
    let (mismatched_token, _) = runtime.issue_auth_session(
        &IdentityId("other-user".into()),
        "export-auth-mismatch",
        GatewayRuntime::now_ms(),
    );
    let server = start_local_gateway_http_server(runtime);

    let (missing_status, _, missing_body) = http_raw(
        "GET",
        &server.base_url,
        "/v1/export?resident_id=export-user&format=md",
        None,
    );
    assert_eq!(
        missing_status, 401,
        "export must reject missing Bearer auth: {missing_body}"
    );

    let (forged_status, _, forged_body) = http_raw_with_headers(
        "GET",
        &server.base_url,
        "/v1/export?resident_id=export-user&format=md",
        &[("Authorization", "Bearer forged-export-token")],
        None,
    );
    assert_eq!(
        forged_status, 401,
        "export must reject unknown Bearer auth: {forged_body}"
    );

    let mismatched_auth = format!("Bearer {mismatched_token}");
    let (mismatched_status, _, mismatched_body) = http_raw_with_headers(
        "GET",
        &server.base_url,
        "/v1/export?resident_id=export-user&format=md",
        &[("Authorization", mismatched_auth.as_str())],
        None,
    );
    assert_eq!(
        mismatched_status, 401,
        "export must bind resident_id to the session identity: {mismatched_body}"
    );

    let valid_auth = format!("Bearer {valid_token}");
    let (valid_status, _, valid_body) = http_raw_with_headers(
        "GET",
        &server.base_url,
        "/v1/export?resident_id=export-user&format=md",
        &[("Authorization", valid_auth.as_str())],
        None,
    );
    assert_eq!(
        valid_status, 200,
        "matching export session should be accepted: {valid_body}"
    );
}

#[test]
fn city_write_rejects_body_actor_not_matching_bearer_session() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    register_resident(&mut runtime, "bob");
    let (token, _) = runtime.issue_auth_session(
        &IdentityId("bob".into()),
        "city-actor-match",
        GatewayRuntime::now_ms(),
    );
    let server = start_local_gateway_http_server(runtime);
    let auth_header = format!("Bearer {token}");

    let (status, response) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/cities/rooms",
        &[("Authorization", auth_header.as_str())],
        Some(&serde_json::json!({
            "city": "core-harbor",
            "creator_id": "rsaga",
            "slug": "session-actor-match-room",
            "title": "Session actor match room",
            "description": "auth boundary test"
        })),
    );
    assert_eq!(status, 401, "unexpected response: {response}");
    assert!(
        response["Error"]["message"]
            .as_str()
            .expect("error message")
            .contains("does not match authenticated session")
    );
}

#[test]
fn world_governance_write_requires_bearer_session() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    let server = start_local_gateway_http_server(runtime);

    let (status, response) = http_json(
        "POST",
        &server.base_url,
        "/v1/world-square/notices",
        Some(&serde_json::json!({
            "actor_id": "rsaga",
            "title": "Unauthenticated notice",
            "body": "must be rejected",
            "severity": "info"
        })),
    );
    assert_eq!(status, 401, "unexpected response: {response}");
}

#[test]
fn direct_and_shell_write_routes_require_bearer_session_without_dev_bypass() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    let server = start_local_gateway_http_server(runtime);

    let cases = [
        (
            "/v1/direct/open",
            serde_json::json!({
                "requester_id": "qa-a",
                "requester_device_id": "browser-a",
                "peer_id": "qa-b",
                "peer_device_id": "browser-b"
            }),
        ),
        (
            "/v1/shell/message",
            serde_json::json!({
                "room_id": "room:world:lobby",
                "sender": "qa-a",
                "text": "unauthenticated shell write"
            }),
        ),
        (
            "/v1/shell/scene",
            serde_json::json!({
                "room_id": "room:world:lobby",
                "actor": "qa-a",
                "image_layer": null,
                "hotspot_layer": null
            }),
        ),
        (
            "/v1/shell/message/edit",
            serde_json::json!({
                "room_id": "room:world:lobby",
                "message_id": "missing-message",
                "actor": "qa-a",
                "text": "unauthenticated edit"
            }),
        ),
        (
            "/v1/shell/message/recall",
            serde_json::json!({
                "room_id": "room:world:lobby",
                "message_id": "missing-message",
                "actor": "qa-a"
            }),
        ),
        (
            "/v1/shell/presence",
            serde_json::json!({"resident_id": "qa-a"}),
        ),
        (
            "/v1/shell/read",
            serde_json::json!({
                "resident_id": "qa-a",
                "conversation_id": "room:world:lobby"
            }),
        ),
        (
            "/v1/shell/nickname",
            serde_json::json!({"nickname": "unauthenticated"}),
        ),
    ];

    for (path, body) in cases {
        let (status, response) = http_json("POST", &server.base_url, path, Some(&body));
        assert_eq!(
            status, 401,
            "write route {path} must reject missing Bearer auth: {response}"
        );
    }
}

#[test]
fn personal_room_and_relationship_writes_require_bearer_session_without_dev_bypass() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    let server = start_local_gateway_http_server(runtime);

    let cases = [
        (
            "/v1/personal-room",
            serde_json::json!({"resident_id": "qa-a"}),
        ),
        (
            "/v1/personal-room/access-policy",
            serde_json::json!({"resident_id": "qa-a", "policy": "friends_only"}),
        ),
        (
            "/v1/resident-relationships/request",
            serde_json::json!({"actor_id": "qa-a", "peer_id": "qa-b"}),
        ),
        (
            "/v1/resident-relationships/accept",
            serde_json::json!({"actor_id": "qa-a", "peer_id": "qa-b"}),
        ),
    ];

    for (path, body) in cases {
        let (status, response) = http_json("POST", &server.base_url, path, Some(&body));
        assert_eq!(
            status, 401,
            "personal/relationship write {path} must reject missing Bearer auth: {response}"
        );
    }
}

#[test]
fn city_and_governance_write_routes_require_bearer_session_without_dev_bypass() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    let server = start_local_gateway_http_server(runtime);

    for path in [
        "/v1/cities",
        "/v1/cities/join",
        "/v1/cities/approve",
        "/v1/cities/stewards",
        "/v1/cities/federation-policy",
        "/v1/cities/rooms",
        "/v1/cities/rooms/freeze",
        "/v1/world-square/notices",
        "/v1/world-safety/cities/trust",
        "/v1/world-safety/reports",
        "/v1/world-safety/reports/review",
        "/v1/world-safety/advisories",
        "/v1/world-safety/residents/sanction",
        "/v1/admin/residents/unsanction",
    ] {
        let (status, _, response) =
            http_raw("POST", &server.base_url, path, Some(&serde_json::json!({})));
        assert_eq!(
            status, 401,
            "city/governance write {path} must reject missing Bearer auth: {response}"
        );
    }
}

#[test]
fn admin_actor_must_match_authenticated_session() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    register_resident(&mut runtime, "bob");
    let (token, _) = runtime.issue_auth_session(
        &IdentityId("bob".into()),
        "admin-actor-match",
        GatewayRuntime::now_ms(),
    );
    let room = runtime
        .create_public_room(CreatePublicRoomRequest {
            city: "core-harbor".into(),
            creator_id: "rsaga".into(),
            slug: Some("actor-match-room".into()),
            title: "Actor match room".into(),
            description: "auth boundary test".into(),
        })
        .expect("create room");
    let server = start_local_gateway_http_server(runtime);
    let auth_header = format!("Bearer {token}");

    let body = serde_json::json!({
        "actor_id": "admin_rsaga",
        "room_id": room.room_id.0,
    });
    let (status, response) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/admin/rooms/freeze",
        &[("Authorization", auth_header.as_str())],
        Some(&body),
    );
    assert_eq!(status, 401, "unexpected response: {response}");
    assert!(
        response["Error"]["message"]
            .as_str()
            .expect("error message")
            .contains("does not match authenticated session")
    );
}

#[test]
fn matching_authenticated_actor_with_capability_can_mutate_admin_room() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    register_resident(&mut runtime, "bob");
    let permission_group = runtime
        .admin_create_permission_group(
            "admin_rsaga",
            "RoomOps",
            "room operations",
            vec![crate::CAP_FREEZE_ROOM.into()],
        )
        .expect("persist permission group");
    runtime
        .admin_assign_permission_group("bob", &permission_group.group.id)
        .expect("persist permission assignment");
    let (token, _) = runtime.issue_auth_session(
        &IdentityId("bob".into()),
        "admin-actor-capability",
        GatewayRuntime::now_ms(),
    );
    let room = runtime
        .create_public_room(CreatePublicRoomRequest {
            city: "core-harbor".into(),
            creator_id: "rsaga".into(),
            slug: Some("matching-actor-room".into()),
            title: "Matching actor room".into(),
            description: "auth boundary test".into(),
        })
        .expect("create room");
    let server = start_local_gateway_http_server(runtime);
    let auth_header = format!("Bearer {token}");

    let (status, response) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/admin/rooms/freeze",
        &[("Authorization", auth_header.as_str())],
        Some(&serde_json::json!({
            "actor_id": "bob",
            "room_id": room.room_id.0,
        })),
    );
    assert_eq!(
        status, 200,
        "matching session should be accepted: {response}"
    );
    assert_eq!(response["ok"], true);
}

#[test]
fn authenticated_admin_mutation_still_requires_capability() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    register_resident(&mut runtime, "bob");
    let (token, _) = runtime.issue_auth_session(
        &IdentityId("bob".into()),
        "admin-capability-boundary",
        GatewayRuntime::now_ms(),
    );
    let server = start_local_gateway_http_server(runtime);
    let auth_header = format!("Bearer {token}");

    let (status, response) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/admin/logs/clear",
        &[("Authorization", auth_header.as_str())],
        Some(&serde_json::json!({})),
    );
    assert_eq!(status, 401);
    assert!(
        response["Error"]["message"]
            .as_str()
            .expect("error message")
            .contains("lacks capability")
    );
}

#[test]
fn resident_without_capability_is_denied_admin_action() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    register_resident(&mut runtime, "bob");
    let (token, _) = runtime.issue_auth_session(
        &IdentityId("bob".into()),
        "test-admin-capability",
        GatewayRuntime::now_ms(),
    );
    // Bob has no permission group assigned - should be denied
    let server = start_local_gateway_http_server(runtime);
    let auth_header = format!("Bearer {token}");

    let body = serde_json::json!({"actor_id": "bob", "room_id": "room-1"});
    let (status, resp) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/admin/rooms/freeze",
        &[("Authorization", auth_header.as_str())],
        Some(&body),
    );
    assert_eq!(status, 401);
    assert!(
        resp["Error"]["message"]
            .as_str()
            .expect("error")
            .contains("lacks capability")
    );
}

#[test]
fn resident_with_capability_can_perform_admin_action() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    register_resident(&mut runtime, "alice");
    let created = runtime
        .create_public_room(CreatePublicRoomRequest {
            city: "core-harbor".into(),
            creator_id: "rsaga".into(),
            slug: Some("test-room".into()),
            title: "Test Room".into(),
            description: "capability test".into(),
        })
        .expect("create room");
    let pg = runtime
        .admin_create_permission_group(
            "admin-1",
            "RoomOps",
            "room operations",
            vec!["freeze:room".into()],
        )
        .expect("persist permission group");
    runtime
        .admin_assign_permission_group("alice", &pg.group.id)
        .expect("persist permission assignment");

    let server = start_local_gateway_http_server(runtime);

    let body = serde_json::json!({"actor_id": "alice", "room_id": created.room_id.0});
    let (status, _resp) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/rooms/freeze",
        Some(&body),
    );
    assert_eq!(status, 200);
}

#[test]
fn special_admin_has_all_capabilities() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    assert!(runtime.resident_has_capability("admin_rsaga", "freeze:room"));
    assert!(runtime.resident_has_capability("admin_rsaga", "ban:resident"));
    assert!(runtime.resident_has_capability("admin_rsaga", "manage:permissions"));
    assert!(runtime.resident_has_capability("admin_rsaga", "nonexistent:cap"));
}

#[test]
fn unknown_resident_has_no_capabilities() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    assert!(!runtime.resident_has_capability("stranger", "send:message"));
    assert!(!runtime.resident_has_capability("stranger", "admin:config"));
}

#[test]
fn capability_check_only_matches_exact_key() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let pg = runtime
        .admin_create_permission_group(
            "admin-1",
            "Limited",
            "only freeze",
            vec!["freeze:room".into()],
        )
        .expect("persist permission group");
    runtime
        .admin_assign_permission_group("bob", &pg.group.id)
        .expect("persist permission assignment");

    assert!(runtime.resident_has_capability("bob", "freeze:room"));
    assert!(!runtime.resident_has_capability("bob", "ban:resident"));
    assert!(!runtime.resident_has_capability("bob", "freeze:room:extra"));
}

// ── Audit Log Tests ──

#[test]
fn audit_events_recorded_for_admin_actions() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    register_resident(&mut runtime, "alice");

    runtime.log_audit_event("admin-1", "admin:freeze_room", "room:test:1", Some("spam"));
    runtime.log_audit_event("admin-1", "admin:ban_resident", "bob", Some("harassment"));
    runtime.log_audit_event("admin-2", "admin:config", "app-config", None);

    let response = runtime.admin_list_audit_events(10);
    assert_eq!(response.total, 3);
    assert_eq!(response.events.len(), 3);

    // Events are returned newest first
    assert_eq!(response.events[0].action, "admin:config");
    assert_eq!(response.events[1].action, "admin:ban_resident");
    assert_eq!(response.events[2].action, "admin:freeze_room");
}

#[test]
fn audit_events_persist_across_restart() {
    let temp = tempdir().expect("temp dir");
    let storage = temp.path().join("gateway");

    {
        let mut runtime = GatewayRuntime::open(&storage, 64, None).expect("runtime");
        runtime.log_audit_event("admin-1", "admin:ban_resident", "eve", Some("bad behavior"));
        runtime.log_audit_event("admin-2", "admin:config", "app-config", None);
    }

    {
        let runtime = GatewayRuntime::open(&storage, 64, None).expect("runtime");
        let response = runtime.admin_list_audit_events(10);
        assert_eq!(response.total, 2);
        assert_eq!(response.events.len(), 2);
    }
}

#[test]
fn audit_log_endpoint_returns_events() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    register_resident(&mut runtime, "alice");

    runtime.log_audit_event("alice", "admin:freeze_room", "room:test:2", None);
    let server = start_local_gateway_http_server(runtime);

    let (status, _, body) = http_raw("GET", &server.base_url, "/v1/admin/audit-log", None);
    assert_eq!(status, 200);

    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse json");
    assert_eq!(parsed["total"], 1);
    assert_eq!(parsed["events"].as_array().unwrap().len(), 1);
    assert_eq!(parsed["events"][0]["action"], "admin:freeze_room");
}

#[test]
fn audit_log_endpoint_respects_limit() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    for i in 0..5 {
        runtime.log_audit_event("admin-1", "admin:config", &format!("item-{}", i), None);
    }

    let response = runtime.admin_list_audit_events(2);
    assert_eq!(response.total, 5);
    assert_eq!(response.events.len(), 2);
}

#[test]
fn audit_event_has_required_fields() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    runtime.log_audit_event(
        "admin-1",
        "admin:freeze_room",
        "room:test:3",
        Some("inappropriate"),
    );

    let response = runtime.admin_list_audit_events(1);
    let event = &response.events[0];

    assert!(event.event_id.starts_with("audit-"));
    assert_eq!(event.actor_id, "admin-1");
    assert_eq!(event.action, "admin:freeze_room");
    assert_eq!(event.target, "room:test:3");
    assert_eq!(event.reason, Some("inappropriate".to_string()));
    assert!(event.timestamp_ms > 0);
}

#[test]
fn audit_event_ids_include_a_unique_sequence_suffix() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    for index in 0..3 {
        runtime.log_audit_event("admin-1", "admin:config", &format!("item-{index}"), None);
    }

    let response = runtime.admin_list_audit_events(10);
    let mut sequence_numbers = response
        .events
        .iter()
        .rev()
        .map(|event| {
            let mut parts = event.event_id.split('-');
            assert_eq!(parts.next(), Some("audit"));
            assert!(parts.next().is_some(), "event id should include timestamp");
            parts
                .next()
                .expect("event id should include sequence")
                .parse::<u64>()
                .expect("event id sequence should be numeric")
        })
        .collect::<Vec<_>>();
    sequence_numbers.sort_unstable();
    assert_eq!(sequence_numbers, vec![0, 1, 2]);
}

#[test]
fn audit_event_sequence_continues_after_restart() {
    let temp = tempdir().expect("temp dir");
    let storage = temp.path().join("gateway");

    {
        let mut runtime = GatewayRuntime::open(&storage, 64, None).expect("runtime");
        runtime.log_audit_event("admin-1", "admin:config", "first", None);
        runtime.log_audit_event("admin-1", "admin:config", "second", None);
    }

    let mut runtime = GatewayRuntime::open(&storage, 64, None).expect("restored runtime");
    runtime.log_audit_event("admin-1", "admin:config", "third", None);
    let response = runtime.admin_list_audit_events(1);
    assert_eq!(
        response.events[0].event_id.split('-').next_back(),
        Some("2")
    );
}

#[test]
fn audit_log_http_endpoint_accepts_limit_param() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");

    for i in 0..10 {
        runtime.log_audit_event("admin-1", "admin:config", &format!("item-{}", i), None);
    }
    let server = start_local_gateway_http_server(runtime);

    let (status, _, body) = http_raw("GET", &server.base_url, "/v1/admin/audit-log?limit=3", None);
    assert_eq!(status, 200);

    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse json");
    assert_eq!(parsed["total"], 10);
    assert_eq!(parsed["events"].as_array().unwrap().len(), 3);
}

// ── Auth Logout Tests ──

#[test]
fn session_logout_revokes_token() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    // Register and get session token
    let (_s, challenge) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/request",
        Some(&serde_json::json!({
            "email": "tester@example.com",
            "resident_id": "tester"
        })),
    );
    let code = challenge["dev_code"].as_str().expect("dev otp");
    let (_s, verified) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/verify",
        Some(&serde_json::json!({
            "challenge_id": challenge["challenge_id"],
            "code": code,
            "resident_id": "tester"
        })),
    );
    let session_token = verified["session_token"].as_str().expect("session token");
    let auth_header: &[(&str, &str)] = &[("Authorization", &format!("Bearer {session_token}"))];

    // Verify session is valid before logout
    let (status, _body) = http_json_with_headers(
        "GET",
        &server.base_url,
        "/v1/auth/session",
        auth_header,
        None,
    );
    assert_eq!(status, 200);

    // Logout
    let (logout_status, logout_body) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/auth/logout",
        auth_header,
        None,
    );
    assert_eq!(logout_status, 200);
    assert_eq!(logout_body["ok"], true);

    // Verify session is now invalid
    let (status2, _body2) = http_json_with_headers(
        "GET",
        &server.base_url,
        "/v1/auth/session",
        auth_header,
        None,
    );
    assert_eq!(status2, 401);
}

#[test]
fn logout_requires_bearer_token() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (status, _body) = http_json("POST", &server.base_url, "/v1/auth/logout", None);
    assert_eq!(status, 401);
}

#[test]
fn double_logout_returns_error() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (_s, challenge) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/request",
        Some(&serde_json::json!({
            "email": "double@example.com",
            "resident_id": "double-tester"
        })),
    );
    let code = challenge["dev_code"].as_str().expect("dev otp");
    let (_s, verified) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/verify",
        Some(&serde_json::json!({
            "challenge_id": challenge["challenge_id"],
            "code": code,
            "resident_id": "double-tester"
        })),
    );
    let session_token = verified["session_token"].as_str().expect("session token");
    let auth_header: &[(&str, &str)] = &[("Authorization", &format!("Bearer {session_token}"))];

    // First logout
    let (status1, _) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/auth/logout",
        auth_header,
        None,
    );
    assert_eq!(status1, 200);

    // Second logout with same token
    let (status2, _) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/auth/logout",
        auth_header,
        None,
    );
    assert_eq!(status2, 401);
}

#[test]
fn admin_scene_endpoint_updates_any_room_regardless_of_participant() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    // Make admin a world steward so they can edit any room scene
    runtime.world_stewards.push(IdentityId("admin".into()));
    // Create a DM room where "admin" is NOT a participant
    runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "alice".into(),
            requester_device_id: Some("desktop".into()),
            peer_id: "bob".into(),
            peer_device_id: Some("browser".into()),
        })
        .expect("direct session should open");

    // admin actor is NOT a participant of dm:alice:bob
    let result = runtime.admin_update_scene(AdminUpdateSceneRequest {
        room_id: "dm:alice:bob".into(),
        actor_id: Some("admin".into()),
        image_layer: None,
        hotspot_layer: Some(Some(SceneHotspotLayer {
            layer_id: "admin-hotspots".into(),
            coordinate_system: "scene-permyriad".into(),
            owner_editable: true,
            hotspots: vec![SceneHotspot {
                hotspot_id: "admin-desk".into(),
                label: "管理台".into(),
                sprite_hint: "default".into(),
                interaction_hint: "管理员操作区".into(),
                x_permyriad: 1000,
                y_permyriad: 1000,
                width_permyriad: 800,
                height_permyriad: 600,
            }],
        })),
    });
    assert!(result.is_ok(), "admin should bypass participant check");
    let response = result.unwrap();
    assert!(response.ok);
    assert_eq!(response.conversation_id, "dm:alice:bob");

    let state = serde_json::to_value(runtime.shell_state()).expect("serialize shell state");
    let scene = state["scene_render"]["scenes"]
        .as_array()
        .expect("scene render array")
        .iter()
        .find(|s| s["conversation_id"] == "dm:alice:bob")
        .expect("scene should exist");
    assert_eq!(scene["hotspot_layer"]["hotspots"][0]["label"], "管理台");
}

#[test]
fn admin_scene_endpoint_http_route_works_without_participant_check() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "alice".into(),
            requester_device_id: Some("desktop".into()),
            peer_id: "bob".into(),
            peer_device_id: Some("browser".into()),
        })
        .expect("direct session should open");
    let server = start_local_gateway_http_server(runtime);

    // POST /v1/admin/scene with actor who is not a room participant
    let (status, payload) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/scene",
        Some(&serde_json::json!({
            "room_id": "dm:alice:bob",
            "hotspot_layer": {
                "layer_id": "http-admin-hotspots",
                "coordinate_system": "scene-permyriad",
                "owner_editable": true,
                "hotspots": [{
                    "hotspot_id": "http-desk",
                    "label": "HTTP管理台",
                    "sprite_hint": "default",
                    "interaction_hint": "HTTP端点测试",
                    "x_permyriad": 2000,
                    "y_permyriad": 2000,
                    "width_permyriad": 800,
                    "height_permyriad": 600
                }]
            }
        })),
    );

    assert_eq!(status, 200);
    assert_eq!(payload["ok"], true);
    assert_eq!(
        payload["hotspot_layer"]["hotspots"][0]["label"],
        "HTTP管理台"
    );
}

#[test]
fn admin_scene_http_null_layers_clear_existing_custom_layers() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime
        .open_direct_session(OpenDirectSessionRequest {
            requester_id: "alice".into(),
            requester_device_id: Some("desktop".into()),
            peer_id: "bob".into(),
            peer_device_id: Some("browser".into()),
        })
        .expect("direct session should open");
    let server = start_local_gateway_http_server(runtime);

    let (custom_status, custom) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/scene",
        Some(&serde_json::json!({
            "room_id": "dm:alice:bob",
            "image_layer": {
                "layer_id": "custom-image",
                "preset": "custom-clear-test",
                "asset_hint": "custom-clear-test",
                "aspect_ratio_permyriad": 5625,
                "owner_editable": true
            },
            "hotspot_layer": {
                "layer_id": "custom-hotspots",
                "coordinate_system": "scene-permyriad",
                "owner_editable": true,
                "hotspots": [{
                    "hotspot_id": "custom-only",
                    "label": "仅自定义热点",
                    "sprite_hint": "default",
                    "interaction_hint": "clear me",
                    "x_permyriad": 2000,
                    "y_permyriad": 2000,
                    "width_permyriad": 800,
                    "height_permyriad": 600
                }]
            }
        })),
    );
    assert_eq!(custom_status, 200);
    assert_eq!(custom["image_layer"]["preset"], "custom-clear-test");
    assert_eq!(
        custom["hotspot_layer"]["hotspots"][0]["hotspot_id"],
        "custom-only"
    );

    let (clear_status, cleared) = http_json(
        "POST",
        &server.base_url,
        "/v1/admin/scene",
        Some(&serde_json::json!({
            "room_id": "dm:alice:bob",
            "image_layer": null,
            "hotspot_layer": null
        })),
    );
    assert_eq!(clear_status, 200);
    assert_ne!(cleared["image_layer"]["preset"], "custom-clear-test");
    assert_ne!(
        cleared["hotspot_layer"]["hotspots"][0]["hotspot_id"],
        "custom-only"
    );
}

#[test]
fn revoked_session_cannot_access_protected_endpoints() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (_s, challenge) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/request",
        Some(&serde_json::json!({
            "email": "revoked@example.com",
            "resident_id": "revoked-user"
        })),
    );
    let code = challenge["dev_code"].as_str().expect("dev otp");
    let (_s, verified) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/verify",
        Some(&serde_json::json!({
            "challenge_id": challenge["challenge_id"],
            "code": code,
            "resident_id": "revoked-user"
        })),
    );
    let session_token = verified["session_token"].as_str().expect("session token");
    let auth: &[(&str, &str)] = &[("Authorization", &format!("Bearer {session_token}"))];

    // Verify session is valid
    let (status, _) =
        http_json_with_headers("GET", &server.base_url, "/v1/auth/session", auth, None);
    assert_eq!(status, 200);

    // Logout
    let (lo_status, _) =
        http_json_with_headers("POST", &server.base_url, "/v1/auth/logout", auth, None);
    assert_eq!(lo_status, 200);

    // Session check should now be rejected
    let (status2, _) =
        http_json_with_headers("GET", &server.base_url, "/v1/auth/session", auth, None);
    assert_eq!(status2, 401);
}

#[test]
fn shell_set_nickname_endpoint_roundtrip() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    // Register and login to get a session token
    let (req_status, challenge) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/request",
        Some(&serde_json::json!({
            "email": "nick-self@example.com",
            "resident_id": "nick-self"
        })),
    );
    assert_eq!(req_status, 200);
    let code = challenge["dev_code"].as_str().expect("dev otp");

    let (verify_status, verified) = http_json(
        "POST",
        &server.base_url,
        "/v1/auth/email-otp/verify",
        Some(&serde_json::json!({
            "challenge_id": challenge["challenge_id"],
            "code": code,
            "resident_id": "nick-self"
        })),
    );
    assert_eq!(verify_status, 200);
    let session_token = verified["session_token"].as_str().expect("session token");
    let auth_value = format!("Bearer {session_token}");
    let auth = &[("Authorization", auth_value.as_str())];

    // Set nickname via shell endpoint
    let (set_status, set_payload) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/shell/nickname",
        auth,
        Some(&serde_json::json!({"nickname": "我的昵称"})),
    );
    assert_eq!(set_status, 200);
    assert_eq!(set_payload.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        set_payload.get("nickname").and_then(|v| v.as_str()),
        Some("我的昵称")
    );

    // Clear nickname (omit field to set to None via serde(default))
    let (clear_status, clear_payload) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/shell/nickname",
        auth,
        Some(&serde_json::json!({})),
    );
    assert_eq!(clear_status, 200);
    assert_eq!(
        clear_payload.get("ok").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(clear_payload.get("nickname").and_then(|v| v.as_str()), None);

    // Without auth should fail
    let (noauth_status, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/nickname",
        Some(&serde_json::json!({"nickname": "no-auth"})),
    );
    assert_eq!(noauth_status, 401);

    // With invalid auth should fail
    let invalid_auth = &[("Authorization", "Bearer invalid-token-12345")];
    let (invalid_auth_status, _) = http_json_with_headers(
        "POST",
        &server.base_url,
        "/v1/shell/nickname",
        invalid_auth,
        Some(&serde_json::json!({"nickname": "bad-auth"})),
    );
    assert_eq!(invalid_auth_status, 401);
}

#[test]
fn concurrent_send_to_same_room_preserves_both_messages() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (send_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "agent:concurrency",
            "to": "user:qa",
            "text": "establish room",
            "client_tag": "concurrent"
        })),
    );
    assert_eq!(send_status, 200);
    assert_eq!(sent["ok"], true);
    let conversation_id = sent["conversation_id"]
        .as_str()
        .expect("conversation id")
        .to_string();

    let url1 = server.base_url.clone();
    let t1 = thread::spawn(move || {
        http_json(
            "POST",
            &url1,
            "/v1/cli/send",
            Some(&serde_json::json!({
                "from": "agent:concurrency",
                "to": "user:qa",
                "text": "concurrent A",
                "client_tag": "concurrent"
            })),
        )
    });

    let url2 = server.base_url.clone();
    let t2 = thread::spawn(move || {
        http_json(
            "POST",
            &url2,
            "/v1/cli/send",
            Some(&serde_json::json!({
                "from": "agent:concurrency",
                "to": "user:qa",
                "text": "concurrent B",
                "client_tag": "concurrent"
            })),
        )
    });

    let (s1, b1) = t1.join().expect("thread 1");
    let (s2, b2) = t2.join().expect("thread 2");
    assert_eq!(s1, 200);
    assert_eq!(s2, 200);
    assert_eq!(b1["ok"], true);
    assert_eq!(b2["ok"], true);

    let tail_for = "user%3Aqa";
    let tail_conv = conversation_id.replace(':', "%3A");
    let (tail_status, tail) = http_json(
        "GET",
        &server.base_url,
        &format!(
            "/v1/cli/tail?for={}&conversation_id={}",
            tail_for, tail_conv
        ),
        None,
    );
    assert_eq!(tail_status, 200);
    let messages = tail["messages"].as_array().expect("tail messages");
    assert!(
        messages.len() >= 3,
        "expected >= 3 messages, got {}",
        messages.len()
    );
    let texts: Vec<&str> = messages.iter().filter_map(|m| m["text"].as_str()).collect();
    assert!(texts.contains(&"concurrent A"), "missing concurrent A");
    assert!(texts.contains(&"concurrent B"), "missing concurrent B");
}

#[test]
fn concurrent_edit_same_message_last_write_wins() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    let (send_status, sent) = http_json(
        "POST",
        &server.base_url,
        "/v1/cli/send",
        Some(&serde_json::json!({
            "from": "user:qa-a",
            "to": "agent:qa-b",
            "text": "original text",
            "client_tag": "concurrent-edit"
        })),
    );
    assert_eq!(send_status, 200);
    let conversation_id = sent["conversation_id"]
        .as_str()
        .expect("conversation id")
        .to_string();
    let message_id = sent["message_id"].as_str().expect("message id").to_string();

    let url1 = server.base_url.clone();
    let room1 = conversation_id.clone();
    let msg1 = message_id.clone();
    let t1 = thread::spawn(move || {
        http_json(
            "POST",
            &url1,
            "/v1/shell/message/edit",
            Some(&serde_json::json!({
                "room_id": room1,
                "message_id": msg1,
                "actor": "qa-a",
                "text": "edit version GREEN"
            })),
        )
    });

    let url2 = server.base_url.clone();
    let room2 = conversation_id.clone();
    let msg2 = message_id.clone();
    let t2 = thread::spawn(move || {
        http_json(
            "POST",
            &url2,
            "/v1/shell/message/edit",
            Some(&serde_json::json!({
                "room_id": room2,
                "message_id": msg2,
                "actor": "qa-a",
                "text": "edit version BLUE"
            })),
        )
    });

    let (s1, b1) = t1.join().expect("thread 1");
    let (s2, b2) = t2.join().expect("thread 2");
    assert_eq!(s1, 200, "edit 1 failed: {:?}", b1);
    assert_eq!(s2, 200, "edit 2 failed: {:?}", b2);

    let tail_for = "user%3Aqa-a";
    let tail_conv = conversation_id.replace(':', "%3A");
    let (tail_status, tail) = http_json(
        "GET",
        &server.base_url,
        &format!(
            "/v1/cli/tail?for={}&conversation_id={}",
            tail_for, tail_conv
        ),
        None,
    );
    assert_eq!(tail_status, 200);
    let messages = tail["messages"].as_array().expect("tail messages");
    let edited_msg = messages
        .iter()
        .find(|m| m["message_id"] == message_id)
        .expect("edited message");
    let final_text = edited_msg["text"].as_str().expect("text");
    assert!(
        final_text == "edit version GREEN" || final_text == "edit version BLUE",
        "final text '{}' is neither edit version",
        final_text
    );
}

#[test]
fn presence_heartbeat_updates_last_seen() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    register_resident(&mut runtime, "qa-a");
    let server = start_local_gateway_http_server(runtime);

    let before_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as i64;

    let (status, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/presence",
        Some(&serde_json::json!({"resident_id": "qa-a"})),
    );
    assert_eq!(status, 200);

    // Verify presence recorded in runtime (不用 /v1/residents, 该端点需要 city
    // membership 而 register_resident 不自动入城).
    {
        let rt = server.runtime.lock().expect("runtime lock");
        let last_seen = rt.presence.get("qa-a").copied();
        assert!(last_seen.is_some(), "qa-a should have a presence timestamp");
        assert!(
            last_seen.unwrap() >= before_ms,
            "presence timestamp {last_seen:?} should be >= {before_ms}"
        );
    }
}

#[test]
fn shell_state_read_consistent_during_concurrent_writes() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    for _ in 0..3 {
        let (s, _) = http_json(
            "POST",
            &server.base_url,
            "/v1/cli/send",
            Some(&serde_json::json!({
                "from": "user:qa-a",
                "to": "agent:qa-b",
                "text": "seed message",
                "client_tag": "stress"
            })),
        );
        assert_eq!(s, 200);
    }

    let running = Arc::new(AtomicBool::new(true));
    let mut handles: Vec<thread::JoinHandle<()>> = vec![];

    for _ in 0..3 {
        let url = server.base_url.clone();
        let flag = running.clone();
        handles.push(thread::spawn(move || {
            while flag.load(Ordering::Acquire) {
                let (status, body) =
                    http_json("GET", &url, "/v1/shell/state?resident_id=qa-a", None);
                assert_eq!(status, 200, "reader got status {}", status);
                assert!(body.is_object(), "reader got non-object body");
                assert!(body.get("rooms").is_some(), "reader state missing rooms");
            }
        }));
    }

    for _ in 0..3 {
        let url = server.base_url.clone();
        let flag = running.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..10 {
                if !flag.load(Ordering::Acquire) {
                    break;
                }
                let (status, _) = http_json(
                    "POST",
                    &url,
                    "/v1/cli/send",
                    Some(&serde_json::json!({
                        "from": "user:qa-a",
                        "to": "agent:qa-b",
                        "text": "concurrent stress write",
                        "client_tag": "stress"
                    })),
                );
                assert_eq!(status, 200, "writer failed");
            }
        }));
    }

    thread::sleep(Duration::from_millis(500));
    running.store(false, Ordering::Release);

    for handle in handles {
        handle
            .join()
            .expect("thread panicked during concurrent stress");
    }

    let (final_status, final_state) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/state?resident_id=qa-a",
        None,
    );
    assert_eq!(final_status, 200);
    assert!(final_state["rooms"].is_array());
    assert!(final_state["conversation_shell"]["conversations"].is_array());
}

#[test]
fn demo_messages_seed_only_for_tests_or_explicit_dev_bypass() {
    assert!(!GatewayRuntime::should_seed_demo_messages(false, false));
    assert!(GatewayRuntime::should_seed_demo_messages(true, false));
    assert!(GatewayRuntime::should_seed_demo_messages(false, true));
}

#[test]
fn message_search_finds_text_in_conversation() {
    let temp = tempdir().expect("temp dir");
    let runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    let server = start_local_gateway_http_server(runtime);

    // Send 3 messages with different text into the world lobby
    let (status1, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "Hello, this contains keyword_xyz",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(status1, 200);

    let (status2, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-a",
            "text": "Completely unrelated message",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(status2, 200);

    let (status3, _) = http_json(
        "POST",
        &server.base_url,
        "/v1/shell/message",
        Some(&serde_json::json!({
            "room_id": "room:world:lobby",
            "sender": "qa-b",
            "text": "Another message with keyword_xyz inside",
            "device_id": "browser",
            "language_tag": "en"
        })),
    );
    assert_eq!(status3, 200);

    // Search for keyword_xyz — should find 2 matching messages
    let (search_status, search_results) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/messages/search?q=keyword_xyz&resident_id=qa-a",
        None,
    );
    assert_eq!(search_status, 200);
    let results = search_results.as_array().expect("search results array");
    assert_eq!(results.len(), 2, "should find exactly 2 matching messages");
    for msg in results {
        let text = msg["text"].as_str().expect("message text");
        assert!(
            text.contains("keyword_xyz"),
            "matching message should contain keyword_xyz: got {text}"
        );
    }

    // Search for nonexistent keyword — should find 0 results
    let (notfound_status, notfound_results) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/messages/search?q=nonexistent&resident_id=qa-a",
        None,
    );
    assert_eq!(notfound_status, 200);
    assert_eq!(
        notfound_results
            .as_array()
            .expect("search results array")
            .len(),
        0,
        "should find 0 messages for nonexistent keyword"
    );

    // Search with room_id filter — should still work
    let (room_filtered_status, room_filtered_results) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/messages/search?q=keyword_xyz&room_id=room:world:lobby&resident_id=qa-a",
        None,
    );
    assert_eq!(room_filtered_status, 200);
    let filtered = room_filtered_results
        .as_array()
        .expect("room filtered results");
    assert_eq!(filtered.len(), 2, "room filter should also find 2 matches");

    // Search with missing q — should return 400
    let (no_q_status, _no_q_body) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/messages/search?resident_id=qa-a",
        None,
    );
    assert_eq!(no_q_status, 400);

    // Search with empty q — should return 400
    let (empty_q_status, _empty_q_body) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/messages/search?q=&resident_id=qa-a",
        None,
    );
    assert_eq!(empty_q_status, 400);
}

#[test]
fn message_search_requires_auth_and_only_returns_viewer_visible_messages() {
    let temp = tempdir().expect("temp dir");
    let mut runtime = GatewayRuntime::open(temp.path().join("gateway"), 64, None).expect("runtime");
    runtime.set_dev_auth_bypass_for_tests(false);
    register_resident(&mut runtime, "alice");
    register_resident(&mut runtime, "bob");
    register_resident(&mut runtime, "carol");
    let alice = IdentityId("alice".into());
    let bob = IdentityId("bob".into());
    let (alice_token, _) =
        runtime.issue_auth_session(&alice, "message-search-alice", GatewayRuntime::now_ms());
    let (bob_token, _) =
        runtime.issue_auth_session(&bob, "message-search-bob", GatewayRuntime::now_ms());
    let carol = IdentityId("carol".into());
    let (carol_token, _) =
        runtime.issue_auth_session(&carol, "message-search-carol", GatewayRuntime::now_ms());
    runtime
        .ensure_direct_conversation(
            &GatewayRuntime::direct_conversation_id(&alice, &bob),
            &[alice.clone(), bob.clone()],
        )
        .expect("create private conversation");
    runtime
        .append_shell_message(ShellMessageRequest {
            room_id: GatewayRuntime::direct_conversation_id(&alice, &bob).0,
            sender: "bob".into(),
            text: "private-search-secret".into(),
            reply_to_message_id: None,
            device_id: Some("browser".into()),
            language_tag: Some("en".into()),
        })
        .expect("append private message");
    let server = start_local_gateway_http_server(runtime);
    let alice_auth = format!("Bearer {alice_token}");
    let bob_auth = format!("Bearer {bob_token}");
    let carol_auth = format!("Bearer {carol_token}");

    let (missing_status, _) = http_json(
        "GET",
        &server.base_url,
        "/v1/shell/messages/search?q=private-search-secret&resident_id=alice",
        None,
    );
    assert_eq!(missing_status, 401);

    let (alice_status, alice_results) = http_json_with_headers(
        "GET",
        &server.base_url,
        "/v1/shell/messages/search?q=private-search-secret&resident_id=alice",
        &[("Authorization", alice_auth.as_str())],
        None,
    );
    assert_eq!(alice_status, 200);
    assert_eq!(alice_results.as_array().expect("alice results").len(), 1);

    let (bob_status, bob_results) = http_json_with_headers(
        "GET",
        &server.base_url,
        "/v1/shell/messages/search?q=private-search-secret&resident_id=bob",
        &[("Authorization", bob_auth.as_str())],
        None,
    );
    assert_eq!(bob_status, 200);
    assert_eq!(bob_results.as_array().expect("bob results").len(), 1);

    let (carol_status, carol_results) = http_json_with_headers(
        "GET",
        &server.base_url,
        "/v1/shell/messages/search?q=private-search-secret&resident_id=carol",
        &[("Authorization", carol_auth.as_str())],
        None,
    );
    assert_eq!(carol_status, 200);
    assert!(carol_results.as_array().expect("carol results").is_empty());
}
