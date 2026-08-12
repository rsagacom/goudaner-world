#!/usr/bin/env python3
from pathlib import Path
import os
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "production-readiness.sh"


def main() -> int:
    assert SCRIPT.exists(), f"missing production readiness script: {SCRIPT}"
    text = SCRIPT.read_text(encoding="utf-8")
    assert "set -euo pipefail" in text
    assert 'ENV_FILE="${ENV_FILE:-/etc/lobster-chat/gateway.env}"' in text
    assert 'BASE_URL="${BASE_URL:-}"' in text
    assert 'CHECK_PUBLIC="${CHECK_PUBLIC:-0}"' in text
    assert 'LOBSTER_DEV_AUTH_BYPASS' in text
    assert 'LOBSTER_DEV_EMAIL_OTP_INLINE' in text
    assert 'LOBSTER_EMAIL_OTP_MAILER_URL' in text
    assert 'LOBSTER_EMAIL_OTP_MAILER_BEARER_TOKEN' in text
    assert 'LOBSTER_WAKU_UPSTREAM_URL' in text
    assert 'LOBSTER_WAKU_UPSTREAM_TOKEN' in text
    assert 'LOBSTER_CORS_ORIGIN' in text
    assert 'https://' in text
    assert '[[ "$cors_origin" =~ ^https://[^[:space:]]+$ ]]' in text
    assert '[[ "${LOBSTER_DEV_AUTH_BYPASS:-0}" == "0" ]]' in text
    assert '[[ "${LOBSTER_DEV_EMAIL_OTP_INLINE:-0}" == "0" ]]' in text
    assert 'curl -fsS' in text
    assert '/health' in text
    assert '/v1/provider' in text
    assert 'Access-Control-Allow-Origin' in text
    assert 'Bearer' not in text or 'Authorization' not in text
    assert 'echo "$LOBSTER_EMAIL_OTP_MAILER_BEARER_TOKEN"' not in text
    assert 'printf "%s" "$LOBSTER_EMAIL_OTP_MAILER_BEARER_TOKEN"' not in text

    with tempfile.TemporaryDirectory() as tmp:
        env_file = Path(tmp) / "gateway.env"
        env_file.write_text(
            "\n".join(
                [
                    "LOBSTER_CORS_ORIGIN=https://chat.example.com",
                    "LOBSTER_DEV_AUTH_BYPASS=0",
                    "LOBSTER_DEV_EMAIL_OTP_INLINE=0",
                    "LOBSTER_EMAIL_OTP_MAILER_URL=https://mailer.example.com/otp",
                    "LOBSTER_EMAIL_OTP_MAILER_BEARER_TOKEN=test-only-token",
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        base_env = os.environ.copy()
        base_env.update({"ENV_FILE": str(env_file), "CHECK_PUBLIC": "0"})
        valid = subprocess.run(["bash", str(SCRIPT)], env=base_env, capture_output=True, text=True)
        assert valid.returncode == 0, valid.stderr

        env_file.write_text(
            env_file.read_text(encoding="utf-8").replace(
                "LOBSTER_CORS_ORIGIN=https://chat.example.com",
                "LOBSTER_CORS_ORIGIN=http://chat.example.com",
            ),
            encoding="utf-8",
        )
        invalid_cors = subprocess.run(["bash", str(SCRIPT)], env=base_env, capture_output=True, text=True)
        assert invalid_cors.returncode != 0

        env_file.write_text(
            env_file.read_text(encoding="utf-8").replace(
                "LOBSTER_CORS_ORIGIN=http://chat.example.com",
                "LOBSTER_CORS_ORIGIN=https://chat.example.com",
            ).replace("LOBSTER_DEV_AUTH_BYPASS=0", "LOBSTER_DEV_AUTH_BYPASS=maybe"),
            encoding="utf-8",
        )
        invalid_dev_flag = subprocess.run(["bash", str(SCRIPT)], env=base_env, capture_output=True, text=True)
        assert invalid_dev_flag.returncode != 0

        # 同机 lobster-mailer 的 loopback http 放行(与 Gateway email_otp_mailer 例外一致)
        env_file.write_text(
            env_file.read_text(encoding="utf-8")
            .replace("LOBSTER_DEV_AUTH_BYPASS=maybe", "LOBSTER_DEV_AUTH_BYPASS=0")
            .replace(
                "LOBSTER_EMAIL_OTP_MAILER_URL=https://mailer.example.com/otp",
                "LOBSTER_EMAIL_OTP_MAILER_URL=http://127.0.0.1:8791/lobster/email-otp",
            ),
            encoding="utf-8",
        )
        loopback_mailer = subprocess.run(["bash", str(SCRIPT)], env=base_env, capture_output=True, text=True)
        assert loopback_mailer.returncode == 0, loopback_mailer.stderr

        with_upstream = env_file.read_text(encoding="utf-8") + (
            "LOBSTER_WAKU_UPSTREAM_URL=https://upstream.example.com\n"
        )
        env_file.write_text(with_upstream, encoding="utf-8")
        missing_upstream_token = subprocess.run(
            ["bash", str(SCRIPT)], env=base_env, capture_output=True, text=True
        )
        assert missing_upstream_token.returncode != 0

        env_file.write_text(
            with_upstream + "LOBSTER_WAKU_UPSTREAM_TOKEN=test-federation-token\n",
            encoding="utf-8",
        )
        authenticated_upstream = subprocess.run(
            ["bash", str(SCRIPT)], env=base_env, capture_output=True, text=True
        )
        assert authenticated_upstream.returncode == 0, authenticated_upstream.stderr

        # 非 loopback 的明文 http 仍然拒绝
        env_file.write_text(
            env_file.read_text(encoding="utf-8").replace(
                "LOBSTER_EMAIL_OTP_MAILER_URL=http://127.0.0.1:8791/lobster/email-otp",
                "LOBSTER_EMAIL_OTP_MAILER_URL=http://mailer.example.com/otp",
            ),
            encoding="utf-8",
        )
        plaintext_mailer = subprocess.run(["bash", str(SCRIPT)], env=base_env, capture_output=True, text=True)
        assert plaintext_mailer.returncode != 0
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
