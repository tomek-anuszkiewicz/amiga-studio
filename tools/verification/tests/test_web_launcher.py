"""The browser build rejects lint warnings and restores the caller's environment."""
import json
import os
from pathlib import Path
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[3]
SHELL = "powershell" if os.name == "nt" else "pwsh"


class WebLauncherContracts(unittest.TestCase):
    def test_build_and_serve_enforce_warnings_and_restore_environment_after_tool_failure(self):
        for action, status in (("build", 0), ("build", 1), ("serve", 1)):
            with self.subTest(action=action, status=status):
                script = """
                $env:CARGO_BUILD_WARNINGS = 'allow'
                $originalLocation = (Get-Location).Path
                function trunk {
                    if ($env:CARGO_BUILD_WARNINGS -ne 'deny') {
                        throw 'Cargo lint warnings were not denied'
                    }
                    if ($env:RUSTUP_TOOLCHAIN -ne 'nightly') {
                        throw 'The production browser toolchain was not selected'
                    }
                    $global:trunkArguments = @($args)
                    $global:LASTEXITCODE = STATUS
                }
                & ./tools/web.ps1 ACTION
                $buildStatus = $LASTEXITCODE
                [ordered]@{
                    status = $buildStatus
                    warnings = $env:CARGO_BUILD_WARNINGS
                    locationRestored = ((Get-Location).Path -eq $originalLocation)
                    arguments = $global:trunkArguments
                } | ConvertTo-Json -Compress
                """.replace("STATUS", str(status)).replace("ACTION", action)
                result = subprocess.run([SHELL, "-NoProfile", "-Command", script], cwd=ROOT,
                                        capture_output=True, text=True, encoding="utf-8")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(json.loads(result.stdout), {
                    "status": status, "warnings": "allow", "locationRestored": True,
                    "arguments": [action, "--release", "--locked", "--no-sri"],
                })

    def test_no_action_shows_usage_and_invalid_action_fails(self):
        for arguments, status in (([], 0), (["invalid"], 1)):
            with self.subTest(arguments=arguments):
                result = subprocess.run([SHELL, "-NoProfile", "-File", "tools/web.ps1", *arguments],
                                        cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
                self.assertEqual(result.returncode, status, result.stderr)
                self.assertIn("Usage: ./tools/web.ps1 <build|serve>", result.stdout)


if __name__ == "__main__":
    unittest.main()
