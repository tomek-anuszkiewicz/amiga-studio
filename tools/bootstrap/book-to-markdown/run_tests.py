"""Install missing pinned review dependencies before running its tests."""

import importlib.metadata
import json
from pathlib import Path
import shutil
import subprocess
import sys


HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools/verification"))
from case_report import ReportArgumentParser, run_reported


def ensure_python_packages():
    requirements = HERE / "requirements.txt"
    for line in requirements.read_text(encoding="utf-8").splitlines():
        name, expected = line.split("==")
        try:
            installed = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError:
            installed = None
        if installed != expected:
            print(f"[SETUP] Installing pinned Python review packages ({name}: {installed or 'missing'}).",
                  flush=True)
            subprocess.run([sys.executable, "-m", "pip", "install", "-r", str(requirements)],
                           cwd=REPO_ROOT, check=True)
            return


def ensure_katex():
    expected = json.loads((HERE / "package.json").read_text(encoding="utf-8"))["dependencies"]["katex"]
    installed_package = HERE / "node_modules/katex/package.json"
    asset = HERE / "node_modules/katex/dist/katex.min.js"
    installed = (json.loads(installed_package.read_text(encoding="utf-8")).get("version")
                 if installed_package.exists() else None)
    if installed != expected or not asset.exists():
        print("[SETUP] Installing pinned local KaTeX review assets.", flush=True)
        npm = shutil.which("npm.cmd" if sys.platform == "win32" else "npm")
        if npm is None:
            raise OSError("npm is required to install local KaTeX assets")
        subprocess.run([npm, "ci", "--prefix", str(HERE), "--ignore-scripts", "--no-audit", "--no-fund"],
                       cwd=REPO_ROOT, check=True)
        if not asset.exists():
            raise OSError(f"KaTeX asset is still missing after npm ci: {asset}")


def execute(argv, report):
    report.phase("argument selection")
    parser = ReportArgumentParser(report, description=__doc__)
    parser.add_argument("-p", "--pattern")
    parser.add_argument("-k", "--case", action="append")
    args = parser.parse_args(argv)
    try:
        report.phase("dependency setup")
        ensure_python_packages()
        ensure_katex()
        command = [sys.executable, str(HERE / "tests/run_suite.py"), "-q"]
        if args.pattern:
            command += ["-p", args.pattern]
        for case in args.case or ():
            command += ["-k", case]
        report.phase("test execution", command=command)
        return subprocess.run(command, cwd=REPO_ROOT, check=False, env=report.child_environment()).returncode
    except (OSError, subprocess.CalledProcessError) as error:
        report.event("launcher_error", detail=str(error), native_exit_code=getattr(error, "returncode", None))
        print(f"[ERROR] Book Review test dependencies could not be installed: {error}", file=sys.stderr)
        return 1


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)

    def run(report):
        try:
            return execute(argv, report)
        finally:
            report.merge_children()

    return run_reported("Book Review test launcher", [sys.executable, str(Path(__file__).resolve()), *argv],
                        run, cwd=REPO_ROOT)


if __name__ == "__main__":
    raise SystemExit(main())
