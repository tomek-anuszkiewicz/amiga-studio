"""Regression coverage for the RAG CLI bootstrap tier."""

import os
import subprocess
import tempfile
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
BOOTSTRAP_SCRIPT = REPOSITORY_ROOT / "tools" / "bootstrap.ps1"
COMPATIBILITY_BOOTSTRAP_SCRIPT = REPOSITORY_ROOT / "tools" / "bootstrap" / "bootstrap_rag.ps1"
QDRANT_BOOTSTRAP_SCRIPT = REPOSITORY_ROOT / "tools" / "bootstrap" / "ensure_qdrant.ps1"


def write_docker_command(temporary_root: Path) -> None:
    command_path = temporary_root / "docker.cmd"
    command_path.write_text(
        "@echo off\r\n"
        "echo %*>> \"%AMIGA_QDRANT_LOG%\"\r\n"
        "if \"%1 %2\"==\"container inspect\" (\r\n"
        "  if \"%AMIGA_QDRANT_INSPECT_EXIT%\"==\"1\" exit /b 1\r\n"
        "  echo false\r\n"
        ")\r\n"
        "exit /b 0\r\n",
        encoding="utf-8",
    )


class BootstrapRagContractTests(unittest.TestCase):
    def test_rag_switch_indexes_the_amiga_scopes_through_path_cli(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            temporary_root = Path(temporary_directory)
            command_log = temporary_root / "rag_qdrant.log"
            command_path = temporary_root / "rag_qdrant.cmd"
            command_path.write_text(
                "@echo off\r\n"
                "echo %*>> \"%RAG_QDRANT_LOG%\"\r\n"
                "exit /b 0\r\n",
                encoding="utf-8",
            )
            environment = os.environ.copy()
            environment["PATH"] = f"{temporary_directory}{os.pathsep}{environment['PATH']}"
            environment["RAG_QDRANT_LOG"] = str(command_log)
            environment["AMIGA_QDRANT_LOG"] = str(temporary_root / "docker.log")
            environment["RAG_INDEX_JSON"] = str(temporary_root / "rag-index.json")
            write_docker_command(temporary_root)

            completed = subprocess.run(
                [
                    "powershell",
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    str(BOOTSTRAP_SCRIPT),
                    "-Rag",
                ],
                cwd=REPOSITORY_ROOT,
                capture_output=True,
                text=True,
                env=environment,
                check=False,
            )

            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertEqual(
                command_log.read_text(encoding="utf-8").splitlines(),
                [
                    f'{REPOSITORY_ROOT / "docs"} --source amiga --index-json {temporary_root / "rag-index.json"}',
                    f'{REPOSITORY_ROOT / "Obsidian" / "Amiga" / "Design"} --source amiga --index-json {temporary_root / "rag-index.json"}',
                    f'{REPOSITORY_ROOT / "Obsidian" / "Amiga" / "Reference"} --source amiga --index-json {temporary_root / "rag-index.json"}',
                ],
            )
            self.assertEqual(
                Path(environment["AMIGA_QDRANT_LOG"]).read_text(encoding="utf-8").splitlines(),
                [
                    "container inspect amiga-rag-qdrant --format {{.State.Running}}",
                    "start amiga-rag-qdrant",
                ],
            )

    def test_all_includes_the_rag_bootstrap_tier(self):
        bootstrap_source = BOOTSTRAP_SCRIPT.read_text(encoding="utf-8")

        self.assertIn("if ($Rag -or $All)", bootstrap_source)
        self.assertIn("tests -> Graphify -> RAG", bootstrap_source)

    def test_compatibility_bootstrap_delegates_to_path_cli(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            temporary_root = Path(temporary_directory)
            command_log = temporary_root / "rag_qdrant.log"
            command_path = temporary_root / "rag_qdrant.cmd"
            command_path.write_text(
                "@echo off\r\n"
                "echo %*>> \"%RAG_QDRANT_LOG%\"\r\n"
                "exit /b 0\r\n",
                encoding="utf-8",
            )
            environment = os.environ.copy()
            environment["PATH"] = f"{temporary_directory}{os.pathsep}{environment['PATH']}"
            environment["RAG_QDRANT_LOG"] = str(command_log)
            environment["AMIGA_QDRANT_LOG"] = str(temporary_root / "docker.log")
            environment["RAG_INDEX_JSON"] = str(temporary_root / "rag-index.json")
            write_docker_command(temporary_root)

            completed = subprocess.run(
                [
                    "powershell",
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    str(COMPATIBILITY_BOOTSTRAP_SCRIPT),
                ],
                cwd=REPOSITORY_ROOT,
                capture_output=True,
                text=True,
                env=environment,
                check=False,
            )

            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertEqual(
                command_log.read_text(encoding="utf-8").splitlines(),
                [
                    f'{REPOSITORY_ROOT / "docs"} --source amiga --index-json {temporary_root / "rag-index.json"}',
                    f'{REPOSITORY_ROOT / "Obsidian" / "Amiga" / "Design"} --source amiga --index-json {temporary_root / "rag-index.json"}',
                    f'{REPOSITORY_ROOT / "Obsidian" / "Amiga" / "Reference"} --source amiga --index-json {temporary_root / "rag-index.json"}',
                    f'--status --index-json {temporary_root / "rag-index.json"}',
                ],
            )

    def test_qdrant_bootstrap_creates_a_persistent_docker_container_when_absent(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            temporary_root = Path(temporary_directory)
            command_log = temporary_root / "docker.log"
            environment = os.environ.copy()
            environment["PATH"] = f"{temporary_directory}{os.pathsep}{environment['PATH']}"
            environment["AMIGA_QDRANT_LOG"] = str(command_log)
            environment["AMIGA_QDRANT_INSPECT_EXIT"] = "1"
            write_docker_command(temporary_root)

            completed = subprocess.run(
                [
                    "powershell",
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    f". '{QDRANT_BOOTSTRAP_SCRIPT}'; if (-not (Ensure-AmigaQdrantContainer)) {{ exit 1 }}",
                ],
                cwd=REPOSITORY_ROOT,
                capture_output=True,
                text=True,
                env=environment,
                check=False,
            )

            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertEqual(
                command_log.read_text(encoding="utf-8").splitlines(),
                [
                    "container inspect amiga-rag-qdrant --format {{.State.Running}}",
                    "run --detach --name amiga-rag-qdrant --restart unless-stopped --publish 6333:6333 --publish 6334:6334 --volume amiga-rag-qdrant-storage:/qdrant/storage qdrant/qdrant:latest",
                ],
            )


if __name__ == "__main__":
    unittest.main()
