"""Output-interface checks follow physical pins when event polling is retired."""

from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "checks"))
import audit_hardware_quality as audit


class HardwareOutputInterfaces(unittest.TestCase):
    def findings(self, source):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            src = root / "crates/agnus/src"
            src.mkdir(parents=True)
            methods = "\n".join(
                f"pub fn {method}() {{}}" for method in audit.MANDATORY_POLL_METHODS["agnus"]
            )
            (src / "agnus.rs").write_text(methods, encoding="utf-8")
            (src / "pins.rs").write_text(source, encoding="utf-8")
            with patch.object(audit, "REPO_ROOT", root), patch.object(audit, "CRATES_DIR", root / "crates"):
                return audit.check_bus_topology()

    def test_explicit_completion_output_replaces_event_polling(self):
        self.assertEqual(self.findings("pub struct AgnusPins { pub int3_high: bool }"), [])

    def test_retired_poll_does_not_substitute_for_completion_pin(self):
        issues = self.findings("pub fn poll_blitter_irq() -> bool { false }")
        self.assertEqual([issue["type"] for issue in issues], ["missing_output_pin"])


if __name__ == "__main__":
    unittest.main()
