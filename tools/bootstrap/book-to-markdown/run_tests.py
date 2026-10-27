"""Install missing pinned review dependencies before running its tests."""

import argparse
import importlib.metadata
import json
from pathlib import Path
import shutil
import subprocess
import sys


HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[2]


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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-p", "--pattern")
    parser.add_argument("-k", "--case")
    args = parser.parse_args()
    try:
        ensure_python_packages()
        ensure_katex()
        command = [sys.executable, str(HERE / "tests/run_suite.py"), "-q"]
        if args.pattern:
            command += ["-p", args.pattern]
        if args.case:
            command += ["-k", args.case]
        return subprocess.run(command, cwd=REPO_ROOT, check=False).returncode
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"[ERROR] Book Review test dependencies could not be installed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
