"""Launch one planned command with the task and execution from its selected registration."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "verification"))
from case_report import CONTEXT, identifier, primary_checkout


def selected_environment(receipt, cwd, environment, work_item=None, round_number=None):
    receipt = receipt.resolve(strict=True)
    record = json.loads(receipt.read_bytes())
    task = identifier(record["task_id"])
    execution = identifier(record["execution_id"])
    expected = primary_checkout(cwd) / ".agent/task-archives" / task / execution / "registration.json"
    if receipt != expected.resolve(strict=True):
        raise ValueError("Select the authoritative registration in the primary checkout")
    if not record.get("root_thread_id") or not record.get("start_utc") or record.get("end_utc"):
        raise ValueError("The selected execution has no active registration interval")
    child = dict(environment)
    for key, value in (("task_id", task), ("execution_id", execution)):
        name = CONTEXT[key]
        if child.get(name) and child[name] != value:
            raise ValueError(f"{name} conflicts with the selected registration")
        child[name] = value
    if work_item:
        child[CONTEXT["work_item"]] = identifier(work_item)
    if round_number is not None:
        if round_number < 1:
            raise ValueError("The round must be positive")
        child[CONTEXT["round"]] = str(round_number)
    return child


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registration", required=True, type=Path)
    parser.add_argument("--work-item")
    parser.add_argument("--round", type=int)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("Supply the planned command")
    try:
        environment = selected_environment(args.registration, Path.cwd(), os.environ, args.work_item, args.round)
        return subprocess.run(command, env=environment).returncode
    except (KeyError, ValueError, OSError, RuntimeError) as error:
        parser.exit(2, f"Planned command could not start: {error}\n")


if __name__ == "__main__":
    sys.exit(main())
