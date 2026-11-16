"""Bootstrap explicitly builds or refreshes both independent graph datasets."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class GraphifyBootstrapTests(unittest.TestCase):
    def test_both_selections_run_for_missing_and_existing_graphs_and_propagate_failure(self):
        tool = Path(__file__).resolve().parents[1] / "bootstrap_graphify.ps1"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            wrapper = root / "bootstrap-probe.ps1"
            wrapper.write_text("""param([string]$ToolPath)
function graphify {}
function python {
    param([string]$ScriptPath, [string]$Selector, [string]$Graph, [string]$RootFlag, [string]$Checkout)
    @{ graph = $Graph; selector = $Selector; rootFlag = $RootFlag; root = $Checkout } |
        ConvertTo-Json -Compress | Add-Content -LiteralPath $env:GRAPHIFY_PROBE_LOG -Encoding UTF8
    $graphPath = Join-Path $env:GRAPHIFY_PROBE_OUTPUT "$Graph.json"
    if (Test-Path -LiteralPath $graphPath) { Add-Content -LiteralPath $graphPath 'refreshed' }
    else { Set-Content -LiteralPath $graphPath 'created' }
    $global:LASTEXITCODE = 0
    if ($env:GRAPHIFY_PROBE_FAIL -eq $Graph) { $global:LASTEXITCODE = 7 }
}
& $ToolPath
exit $LASTEXITCODE
""", encoding="utf-8")
            log = root / "calls.jsonl"
            for fail in ("", "", "project", "reference"):
                with self.subTest(fail=fail):
                    environment = dict(os.environ, GRAPHIFY_PROBE_LOG=str(log),
                                       GRAPHIFY_PROBE_OUTPUT=str(root), GRAPHIFY_PROBE_FAIL=fail)
                    log.unlink(missing_ok=True)
                    result = subprocess.run(["powershell", "-NoProfile", "-File", str(wrapper), str(tool)],
                                            env=environment, capture_output=True, text=True)
                    self.assertEqual(result.returncode, 1 if fail else 0, result.stdout + result.stderr)
                    calls = [json.loads(line) for line in log.read_text(encoding="utf-8-sig").splitlines()]
                    self.assertEqual([c["graph"] for c in calls], ["project", "reference"])
                    self.assertTrue(all(c["selector"] == "--graph" and c["rootFlag"] == "--root" for c in calls))
                    self.assertTrue(all(Path(c["root"]) == tool.parents[2] for c in calls))
            for graph in ("project", "reference"):
                self.assertEqual((root / f"{graph}.json").read_text(encoding="utf-8-sig").splitlines(),
                                 ["created", "refreshed", "refreshed", "refreshed"])


if __name__ == "__main__":
    unittest.main()
