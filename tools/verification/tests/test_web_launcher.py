"""The browser build rejects lint warnings and restores the caller's environment."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[3]
SHELL = "powershell" if os.name == "nt" else "pwsh"


class WebLauncherContracts(unittest.TestCase):
    def test_build_and_serve_enforce_warnings_and_restore_environment_after_tool_failure(self):
        initial_environment = {
            "RUSTUP_TOOLCHAIN": "stable",
            "CARGO_UNSTABLE_BUILD_STD": "std",
            "CARGO_BUILD_WARNINGS": "allow",
            "RUSTFLAGS": "-C debuginfo=1",
            "NO_COLOR": "false",
        }
        script = """
        $ErrorActionPreference = 'Stop'
        function trunk {
            if ($env:CARGO_BUILD_WARNINGS -ne 'deny') {
                throw 'Cargo lint warnings were not denied'
            }
            if ($env:RUSTUP_TOOLCHAIN -ne 'nightly') {
                throw 'The production browser toolchain was not selected'
            }
            $global:trunkArguments = @($args)
            $global:trunkLocation = (Get-Location).Path
            $global:LASTEXITCODE = $global:trunkStatus
        }
        $cases = @(
            @{ action = 'build'; status = 0 }
            @{ action = 'build'; status = 1 }
            @{ action = 'serve'; status = 1 }
        )
        $results = foreach ($case in $cases) {
            $global:trunkStatus = $case.status
            $global:trunkArguments = $null
            $global:trunkLocation = $null
            & $env:WEB_TEST_LAUNCHER $case.action
            [ordered]@{
                status = $LASTEXITCODE
                location = (Get-Location).Path
                trunkLocation = $global:trunkLocation
                arguments = $global:trunkArguments
                environment = @{
                    RUSTUP_TOOLCHAIN = $env:RUSTUP_TOOLCHAIN
                    CARGO_UNSTABLE_BUILD_STD = $env:CARGO_UNSTABLE_BUILD_STD
                    CARGO_BUILD_WARNINGS = $env:CARGO_BUILD_WARNINGS
                    RUSTFLAGS = $env:RUSTFLAGS
                    NO_COLOR = $env:NO_COLOR
                }
            }
        }
        ConvertTo-Json -InputObject @($results) -Compress
        """
        with tempfile.TemporaryDirectory() as directory:
            caller = Path(directory).resolve()
            environment = {**os.environ, **initial_environment, "WEB_TEST_LAUNCHER": str(ROOT / "tools/web.ps1")}
            result = subprocess.run([SHELL, "-NoProfile", "-Command", script], cwd=caller, env=environment,
                                    capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(result.returncode, 0, result.stderr)
        observations = json.loads(result.stdout)
        for (action, status), observed in zip((("build", 0), ("build", 1), ("serve", 1)),
                                              observations, strict=True):
            with self.subTest(action=action, status=status):
                self.assertEqual(observed, {
                    "status": status, "location": str(caller), "trunkLocation": str(ROOT),
                    "arguments": [action, "--release", "--locked", "--no-sri"],
                    "environment": initial_environment,
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
