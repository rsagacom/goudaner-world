use serde::Serialize;
use tiny_http::{Response, StatusCode};

use crate::{
    http_router::HttpResponse,
    http_support::{ResponseHeaderExt, json_header},
};

#[derive(Debug, Serialize)]
pub(crate) struct ReleaseVersion {
    schema_version: u8,
    package_version: &'static str,
    git_sha: &'static str,
}

pub(crate) fn release_version() -> ReleaseVersion {
    ReleaseVersion {
        schema_version: 1,
        package_version: env!("CARGO_PKG_VERSION"),
        git_sha: env!("LOBSTER_BUILD_GIT_SHA"),
    }
}

pub(crate) fn handle_get_version() -> HttpResponse {
    let body = serde_json::to_string(&release_version()).unwrap_or_else(|_| "{}".into());
    Response::from_string(body)
        .with_status_code(StatusCode(200))
        .with_optional_header(json_header())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_version_exposes_non_secret_build_identity() {
        let payload = serde_json::to_value(release_version()).expect("release version json");
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["package_version"], env!("CARGO_PKG_VERSION"));
        let git_sha = payload["git_sha"].as_str().expect("git sha string");
        assert!(git_sha == "unknown" || git_sha.len() == 40);
        assert_eq!(payload.as_object().expect("version object").len(), 3);
    }
}
