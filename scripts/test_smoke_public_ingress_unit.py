#!/usr/bin/env python3
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "smoke-public-ingress.sh"


def main() -> int:
    assert SCRIPT.exists(), f"missing public ingress smoke script: {SCRIPT}"
    text = SCRIPT.read_text(encoding="utf-8")

    assert 'BASE_URL="${BASE_URL:-${1:-}}"' in text
    assert 'EXPECT_HOME_TEXT="${EXPECT_HOME_TEXT:-我和狗蛋儿的家 · 主城群聊}"' in text
    assert 'EXPECT_RESIDENT_TEXT="${EXPECT_RESIDENT_TEXT:-我和狗蛋儿的家 · 住宅}"' in text
    assert 'EXPECT_ADMIN_TEXT="${EXPECT_ADMIN_TEXT:-AJW聊天 · 管理后台}"' in text
    assert 'EXPECT_PROVIDER_FRAGMENT="${EXPECT_PROVIDER_FRAGMENT:-\\"reachable\\":true}"' in text
    assert 'EXPECT_CORS_ORIGIN="${EXPECT_CORS_ORIGIN:-}"' in text
    assert 'EXPECT_RELEASE_GIT_SHA="${EXPECT_RELEASE_GIT_SHA:-}"' in text
    assert 'EXPECT_MANIFEST_CONTENT_TYPE="${EXPECT_MANIFEST_CONTENT_TYPE:-application/json}"' in text
    assert 'CURL_BIN="${CURL_BIN:-curl}"' in text
    assert 'require_non_empty "BASE_URL" "$BASE_URL"' in text
    assert 'BASE_URL="${BASE_URL%/}"' in text
    assert 'mktemp "${TMPDIR:-/tmp}/lobster-public-smoke.XXXXXX"' in text
    assert 'trap \'rm -f "$BODY_FILE" "$HEADER_FILE"\' EXIT' in text
    assert 'fetch_body_with_headers()' in text
    assert 'extract_git_sha()' in text
    assert 'need_cmd sed' in text
    assert 'need_cmd tr' in text
    assert 'fetch_body "$BASE_URL/" "$BODY_FILE"' in text
    assert 'grep -F "$EXPECT_HOME_TEXT" "$BODY_FILE"' in text
    assert 'fetch_body "$BASE_URL/creative.html" "$BODY_FILE"' in text
    assert 'grep -F "$EXPECT_RESIDENT_TEXT" "$BODY_FILE"' in text
    assert 'fetch_body "$BASE_URL/admin-ds.html" "$BODY_FILE"' in text
    assert 'grep -F "$EXPECT_ADMIN_TEXT" "$BODY_FILE"' in text
    assert 'fetch_body "$BASE_URL/health" "$BODY_FILE"' in text
    assert 'if [[ "$(cat "$BODY_FILE")" != "ok" ]]; then' in text
    assert 'health_status="$(fetch_head_status "$BASE_URL/health")"' in text
    assert 'grep -F "200"' in text
    assert 'fetch_body "$BASE_URL/v1/provider" "$BODY_FILE"' in text
    assert 'grep -F "$EXPECT_PROVIDER_FRAGMENT" "$BODY_FILE"' in text
    assert 'fetch_body "$BASE_URL/v1/version" "$BODY_FILE"' in text
    assert 'version_git_sha="$(extract_git_sha "$BODY_FILE")"' in text
    assert 'fetch_body_with_headers "$BASE_URL/release-manifest.json" "$BODY_FILE" "$HEADER_FILE"' in text
    assert 'grep -Fi "content-type: $EXPECT_MANIFEST_CONTENT_TYPE" "$HEADER_FILE"' in text
    assert 'manifest_git_sha="$(extract_git_sha "$BODY_FILE")"' in text
    assert 'runtime git_sha does not match release manifest git_sha' in text
    assert 'EXPECT_RELEASE_GIT_SHA must be a 40-character hexadecimal Git SHA' in text
    assert 'fetch_status()' in text
    assert 'assert_status "401" "GET" "$BASE_URL/v1/admin/summary"' in text
    assert 'assert_status "401" "POST" "$BASE_URL/v1/auth/logout"' in text
    assert 'anonymous shell state exposed a direct conversation' in text
    assert '"id"[[:space:]]*:[[:space:]]*"dm:' in text
    assert 'Origin: ${EXPECT_CORS_ORIGIN}' in text
    assert 'public ingress smoke passed' in text
    return 0


if __name__ == "__main__":
    sys.exit(main())
