"""Build the Cargo benchmark and profile the executable reported by that build."""
import json
import subprocess
import sys

from repository import REPO_ROOT


def main(argv=None):
    options = sys.argv[1:] if argv is None else argv
    command = ["cargo", "bench", "-p", "test_runner", "--bench", "bench_instructions",
               "--no-run", "--message-format=json-render-diagnostics"]
    executable = None
    with subprocess.Popen(command, cwd=REPO_ROOT, stdout=subprocess.PIPE, text=True,
                          encoding="utf-8", errors="replace") as process:
        for line in process.stdout:
            message = json.loads(line)
            if message.get("reason") == "compiler-message":
                rendered = message["message"].get("rendered")
                if rendered:
                    print(rendered, end="", file=sys.stderr)
            elif message.get("reason") == "compiler-artifact":
                target = message.get("target", {})
                if target.get("name") == "bench_instructions" and "bench" in target.get("kind", []):
                    executable = message.get("executable")
        code = process.wait()
    if code:
        return code
    if not executable:
        print("Error: Cargo did not report a benchmark executable", file=sys.stderr)
        return 1
    return subprocess.call(["samply", "record", executable, *options], cwd=REPO_ROOT)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError) as error:
        print(f"Error: {error}", file=sys.stderr)
        sys.exit(1)
