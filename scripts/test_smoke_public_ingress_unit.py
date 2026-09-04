#!/usr/bin/env python3
"""smoke-public-ingress.sh 的合同单测。

脚本本身依赖可达的 BASE_URL，无法离线端到端运行；此处锁定其**检查合同**：
语法有效，且 WebPush / 隐私 / 版本追溯等关键断言不缺失（防止静默弱化）。
"""
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "smoke-public-ingress.sh"


class SmokePublicIngressContractTests(unittest.TestCase):
    def test_script_exists_and_passes_bash_syntax(self):
        self.assertTrue(SCRIPT.is_file())
        result = subprocess.run(
            ["bash", "-n", str(SCRIPT)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_webpush_endpoint_contract_is_present(self):
        text = SCRIPT.read_text(encoding="utf-8")
        # 公钥端点公开可读且校验载荷
        self.assertIn("/v1/push/vapid-public-key", text)
        self.assertIn('"public_key"', text)
        # 订阅/退订未授权必须 401（防匿名写订阅垃圾）
        self.assertIn('/v1/push/subscribe"', text)
        self.assertIn('/v1/push/unsubscribe"', text)
        self.assertIn(
            'assert_status "401" "POST" "$BASE_URL/v1/push/subscribe"',
            text,
        )

    def test_core_contract_assertions_are_present(self):
        text = SCRIPT.read_text(encoding="utf-8")
        for required in (
            "/v1/version",
            "release-manifest.json",
            "EXPECT_RELEASE_GIT_SHA",
            "/v1/admin/summary",
            "anonymous shell state exposed a direct conversation",
            "EXPECT_MANIFEST_CONTENT_TYPE",
        ):
            self.assertIn(required, text)


if __name__ == "__main__":
    unittest.main()
