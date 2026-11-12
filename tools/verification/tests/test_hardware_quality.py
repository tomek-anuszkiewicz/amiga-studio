"""Output-interface checks follow physical pins when event polling is retired."""

from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "checks"))
import audit_hardware_quality as audit


class HardwareOutputInterfaces(unittest.TestCase):
    def findings(self, source, chip="agnus"):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            src = root / f"crates/{chip}/src"
            src.mkdir(parents=True)
            methods = "\n".join(
                f"pub fn {method}() {{}}" for method in audit.MANDATORY_POLL_METHODS["agnus"]
            ) if chip == "agnus" else ""
            (src / f"{chip}.rs").write_text(methods, encoding="utf-8")
            (src / "pins.rs").write_text(source, encoding="utf-8")
            with patch.object(audit, "REPO_ROOT", root), patch.object(audit, "CRATES_DIR", root / "crates"):
                return audit.check_bus_topology()

    def test_explicit_completion_output_replaces_event_polling(self):
        self.assertEqual(self.findings("pub struct AgnusPins { pub int3_high: bool }"), [])

    def test_retired_poll_does_not_substitute_for_completion_pin(self):
        issues = self.findings("pub fn poll_blitter_irq() -> bool { false }")
        self.assertEqual([issue["type"] for issue in issues], ["missing_output_pin"])


    def test_shared_register_constants_are_not_peer_dependencies(self):
        for path in [
            "config::custom_reg::paula::REGISTER_AUD0DAT",
            "custom_reg::paula::REGISTER_DSKDAT",
        ]:
            with self.subTest(path=path):
                source = "pub struct AgnusPins { pub int3_high: bool }\n"
                self.assertEqual(self.findings(source + f"let address = {path};"), [])

    def test_peer_paths_remain_forbidden_beside_shared_constants(self):
        for statement in [
            "use paula::Paula;",
            "use crate::paula::Paula;",
            "let chip = paula::Paula::new();",
            "let pair = (custom_reg::paula::REGISTER_DSKDAT, crate::paula::Paula::new());",
            "use other::custom_reg::paula::REGISTER_DSKDAT;",
        ]:
            with self.subTest(statement=statement):
                source = "pub struct AgnusPins { pub int3_high: bool }\n" + statement
                self.assertEqual(
                    [issue["type"] for issue in self.findings(source)],
                    ["inter_chip_import"],
                )

    def test_direct_peer_calls_remain_forbidden(self):
        source = "pub struct AgnusPins { pub int3_high: bool }\nself.paula.step_cck();"
        self.assertEqual(
            [issue["type"] for issue in self.findings(source)],
            ["signal_smuggling"],
        )

    def test_floppy_does_not_require_paula_disk_interrupt_polling(self):
        self.assertEqual(self.findings("pub struct FloppyPins {}", chip="floppy"), [])

if __name__ == "__main__":
    unittest.main()
