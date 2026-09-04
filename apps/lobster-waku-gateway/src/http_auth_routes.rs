use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};

use tiny_http::{Request, Response, StatusCode};
use transport_waku::WakuGatewayResponse;

use crate::{
    AuthPreflightRequest, GatewayRuntime, GatewayStateNotifier, RequestEmailOtpRequest,
    RequestEmailOtpResponse, RequestMobileOtpRequest, VerifyEmailOtpRequest,
    VerifyMobileOtpRequest,
    email_otp_mailer::{EmailOtpDelivery, deliver_email_otp_from_env},
    http_support::{ResponseHeaderExt, authorization_bearer_token, json_header},
};

pub(crate) type HttpResponse = Response<Cursor<Vec<u8>>>;

fn ok_json() -> HttpResponse {
    Response::from_string("{\"ok\":true}")
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

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

pub(crate) fn request_email_otp_with_delivery(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    payload: RequestEmailOtpRequest,
    inline_delivery: bool,
    deliver: impl FnOnce(&EmailOtpDelivery) -> Result<(), String>,
) -> Result<Result<RequestEmailOtpResponse, String>, HttpResponse> {
    let prepared = match with_runtime(runtime, |runtime| {
        runtime.prepare_email_otp_request(payload, inline_delivery)
    })? {
        Ok(prepared) => prepared,
        Err(error) => return Ok(Err(error)),
    };

    if let Some(delivery) = prepared.delivery.as_ref()
        && let Err(delivery_error) = deliver(delivery)
    {
        // 投递失败细节(内部 mailer 地址、连接错误)只进服务端日志,
        // 对居民端只暴露通用文案,避免泄露内部拓扑。
        eprintln!("email otp delivery failed: {delivery_error}");
        let rollback = with_runtime(runtime, |runtime| {
            runtime.rollback_email_otp_request(
                &prepared.response.challenge_id,
                &prepared.normalized_email,
            )
        })?;
        if let Err(rollback_error) = rollback {
            eprintln!("email otp rollback failed: {rollback_error}");
            return Ok(Err("email otp delivery failed".to_string()));
        }
        return Ok(Err("email otp delivery failed".to_string()));
    }

    Ok(Ok(prepared.response))
}

pub(crate) fn handle_get_auth_session(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &Request,
) -> HttpResponse {
    let Some(token) = authorization_bearer_token(request) else {
        return unauthorized("authorization bearer token required".into());
    };
    let result = match with_runtime(runtime, |runtime| runtime.auth_session_projection(&token)) {
        Ok(result) => result,
        Err(response) => return response,
    };
    match result {
        Ok(session) => {
            Response::from_string(serde_json::to_string(&session).unwrap_or_else(|_| "{}".into()))
                .with_status_code(StatusCode(200))
                .with_optional_header(json_header())
        }
        Err(message) => unauthorized(message),
    }
}

pub(crate) fn handle_post_auth_preflight(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }

    match serde_json::from_slice::<AuthPreflightRequest>(&body) {
        Ok(payload) => {
            let result = match with_runtime(runtime, |runtime| runtime.auth_preflight(payload)) {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(preflight) => Response::from_string(
                    serde_json::to_string(&preflight).unwrap_or_else(|_| "{}".into()),
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
                message: format!("decode auth preflight failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_request_email_otp(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }

    match serde_json::from_slice::<RequestEmailOtpRequest>(&body) {
        Ok(payload) => {
            let result = match request_email_otp_with_delivery(
                runtime,
                payload,
                GatewayRuntime::dev_email_otp_inline_enabled(),
                deliver_email_otp_from_env,
            ) {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(response_body) => Response::from_string(
                    serde_json::to_string(&response_body).unwrap_or_else(|_| "{}".into()),
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
                message: format!("decode email otp request failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_verify_email_otp(
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

    match serde_json::from_slice::<VerifyEmailOtpRequest>(&body) {
        Ok(payload) => {
            let result =
                match with_runtime(runtime, |runtime| match runtime.verify_email_otp(payload) {
                    Ok(response_body) => {
                        runtime.log_audit_event(
                            &response_body.resident_id,
                            "auth:login",
                            &response_body.session.session_id,
                            None,
                        );
                        Ok(response_body)
                    }
                    Err(message) => Err(message),
                }) {
                    Ok(result) => result,
                    Err(response) => return response,
                };
            match result {
                Ok(response_body) => {
                    notifier.notify_changed();
                    Response::from_string(
                        serde_json::to_string(&response_body).unwrap_or_else(|_| "{}".into()),
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
                message: format!("decode email otp verify failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_request_mobile_otp(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &mut Request,
) -> HttpResponse {
    let mut body = Vec::new();
    if let Err(error) = request.as_reader().read_to_end(&mut body) {
        return Response::from_string(format!("{{\"error\":\"{error}\"}}"))
            .with_status_code(StatusCode(400))
            .with_optional_header(json_header());
    }

    match serde_json::from_slice::<RequestMobileOtpRequest>(&body) {
        Ok(payload) => {
            let result = match with_runtime(runtime, |runtime| runtime.request_mobile_otp(payload))
            {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(response_body) => Response::from_string(
                    serde_json::to_string(&response_body).unwrap_or_else(|_| "{}".into()),
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
                message: format!("decode mobile otp request failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_verify_mobile_otp(
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

    match serde_json::from_slice::<VerifyMobileOtpRequest>(&body) {
        Ok(payload) => {
            let result = match with_runtime(runtime, |runtime| {
                match runtime.verify_mobile_otp(payload) {
                    Ok(response_body) => {
                        runtime.log_audit_event(
                            &response_body.resident_id,
                            "auth:login",
                            &response_body.session.session_id,
                            None,
                        );
                        Ok(response_body)
                    }
                    Err(message) => Err(message),
                }
            }) {
                Ok(result) => result,
                Err(response) => return response,
            };
            match result {
                Ok(response_body) => {
                    notifier.notify_changed();
                    Response::from_string(
                        serde_json::to_string(&response_body).unwrap_or_else(|_| "{}".into()),
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
                message: format!("decode mobile otp verify failed: {error}"),
            })
            .unwrap_or_else(|_| "{\"error\":true}".into()),
        )
        .with_status_code(StatusCode(400))
        .with_optional_header(json_header()),
    }
}

pub(crate) fn handle_post_auth_logout(
    runtime: &Arc<Mutex<GatewayRuntime>>,
    request: &Request,
) -> HttpResponse {
    let Some(token) = authorization_bearer_token(request) else {
        return unauthorized("authorization bearer token required".into());
    };
    let result = match with_runtime(runtime, |runtime| {
        let session = runtime.resolve_bearer_session(&token)?;
        runtime.revoke_auth_session(&token)?;
        runtime.log_audit_event(
            &session.resident_id.0,
            "auth:logout",
            &session.session_id,
            None,
        );
        Ok(())
    }) {
        Ok(result) => result,
        Err(response) => return response,
    };
    match result {
        Ok(()) => ok_json(),
        Err(message) => unauthorized(message),
    }
}
