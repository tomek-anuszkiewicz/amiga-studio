"""Validate a TA result and render its matching human-readable handoff."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import sys

from jsonschema import Draft202012Validator


SCHEMA_PATH = Path(__file__).with_name("result.schema.json")


def portable(path):
    return ("\\" not in path and ":" not in path
            and not PurePosixPath(path).is_absolute()
            and ".." not in PurePosixPath(path).parts)


def references(value):
    if isinstance(value, dict):
        if "path" in value and "sha256" in value:
            yield value
        for child in value.values():
            yield from references(child)
    elif isinstance(value, list):
        for child in value:
            yield from references(child)


def validate(result, schema, root, *, check_references=True):
    errors = [f"{'/'.join(map(str, error.path))}: {error.message}"
              for error in Draft202012Validator(schema).iter_errors(result)]
    if errors:
        return errors
    rounds = result["rounds"]
    if rounds["used"] > rounds["limit"]:
        errors.append("rounds.used exceeds rounds.limit")
    if result["status"] == "limit_reached" and rounds["used"] != rounds["limit"]:
        errors.append("limit_reached requires an exhausted round budget")
    if result["status"] == "completed" and rounds["used"] == 0:
        errors.append("completed requires an actual pass")
    contract = schema["x-skill-contracts"][result["skill"]]
    if result["result"]["kind"] != contract["kind"]:
        errors.append("result kind does not match selected skill")
    # Supporting operations may run inside an assigned specialist context.
    allowed_agents = contract.get("allowed_agents", [contract["agent"]])
    if result["agent"] not in allowed_agents:
        errors.append("agent does not own selected skill")
    data = result["result"]
    for field in ("lane", "profile", "source_family"):
        if field in contract and data.get(field) != contract[field]:
            errors.append(f"{field} does not match selected skill")
    if result["previous_run_id"] == result["run_id"]:
        errors.append("a continuation must have a new run ID")
    if data["kind"] == "review":
        if result["status"] != "completed" and data["verdict"] != "not_assessed":
            errors.append("unfinished review cannot issue a verdict")
        if data["verdict"] != "not_assessed" and not result["inputs"]["candidate"]:
            errors.append("review verdict requires an exact candidate")
        if data["verdict"] == "approved":
            inputs = result["inputs"]
            required_refs = inputs["plans"] + inputs["prior_results"] + [
                ref for ref in inputs["instructions"] if ref["exposure"] == "loaded"]
            if (not required_refs or any(ref["sha256"] is None for ref in required_refs)
                    or not result["evidence"]):
                errors.append("approval requires identified inputs and evidence")
            if any(f["severity"] in ("blocking", "major") and
                   f["disposition"] not in ("verified", "refuted") for f in result["findings"]):
                errors.append("approval contains unresolved required findings")
    if data["kind"] == "test_audit" and result["status"] != "completed" and data["verdict"] != "not_assessed":
        errors.append("unfinished audit cannot issue a verdict")
    if data["kind"] == "verification" and data["verdict"] == "passed":
        if result["status"] != "completed" or not data["report_refs"]:
            errors.append("passed verification requires completed status and actual reports")
    if data["kind"] == "publication" and data["published"]:
        if result["status"] != "completed" or not data["record_ref"]:
            errors.append("publication requires completed status and an operation record")
    ids = [finding["id"] for finding in result["findings"]]
    if len(ids) != len(set(ids)):
        errors.append("finding IDs must be unique within one result")
    for path in result["changed_paths"]:
        if not portable(path):
            errors.append(f"nonportable changed path: {path}")
    for ref in references(result) if check_references else ():
        if not portable(ref["path"]):
            errors.append(f"nonportable reference: {ref['path']}")
            continue
        if ref["sha256"] is None:
            if not result["inputs"]["identity_limits"]:
                errors.append(f"missing identity without a stated limit: {ref['path']}")
            continue
        path = root / ref["path"]
        try:
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
        except OSError as error:
            errors.append(f"unavailable reference {ref['path']}: {error}")
        else:
            if digest != ref["sha256"]:
                errors.append(f"changed reference: {ref['path']}")
    return errors


def render(result):
    """Render a concise handoff from the validated object, without new claims."""
    lines = [f"# {result['skill']}: {result['status']}", "",
             result["summary"], "",
             f"Invocation: {result['task_id']} / {result['work_item_id']} / {result['run_id']}",
             f"Agent: {result['agent']}; previous run: {result['previous_run_id']}",
             f"Rounds: {result['rounds']['used']}/{result['rounds']['limit']}; {result['rounds']['stop_reason']}",
             "", "Full typed result, input identities and observation details: [result.json](result.json).", ""]
    data = result["result"]
    if "verdict" in data:
        lines.extend([f"Verdict: {data['verdict']}; scope: {data.get('lane', data['kind'])}.", ""])
    if "routing" in data:
        lines.extend(["## Scope and finding comparisons", ""])
        lines.extend(f"- Inspected {item}" for item in data["inspected_scope"])
        for comparison in data["routing"]["comparisons"]:
            finding = comparison["finding"]
            lines.append(f"- {finding['lane']}/{finding['id']} checked {comparison['trigger']}. "
                         f"Parent {comparison['parent']['outcome']}. Current {comparison['current']['outcome']}. "
                         f"Parent scope {comparison['parent_scope']}. Changed input {comparison['change']}. "
                         f"Disposition {comparison.get('disposition', 'open')}.")
        lines.append("")
    lines.extend(["## Findings", ""])
    for finding in result["findings"]:
        lines.append(f"- {finding['id']} ({finding['severity']}, {finding['disposition']}): "
                     f"{finding['scope']}. Expected: {finding['expected']} Observed: {finding['observed']}")
    if not result["findings"]:
        lines.append("No findings reported; this alone is not approval.")
    lines.extend(["", "## Checks and evidence", ""])
    for evidence in result["evidence"]:
        command = evidence["command"] or "inspection"
        artifact = evidence["artifact_ref"]
        location = f"; artifact: {artifact['path']}" if artifact else ""
        lines.append(f"- {evidence['id']} ({evidence['origin']}): {command}; "
                     f"exit: {evidence['exit_code']}{location}. {evidence['validity_basis']}")
    if not result["evidence"]:
        lines.append("No execution or inspection evidence supplied.")
    lines.extend(["", "## Changes", ""])
    lines.extend(f"- {path}" for path in result["changed_paths"])
    if not result["changed_paths"]:
        lines.append("No changed paths reported.")
    remaining = [item for key in ("remaining_work", "missing_prerequisites", "unresolved", "gaps", "limits",
                                  "measurement_limits", "freshness_limits") for item in data.get(key, [])]
    lines.extend(["", "## Limits and handoff", ""])
    lines.extend(f"- {item}" for item in remaining)
    if result["inputs"]["identity_limits"]:
        lines.append("Input identity limitations are recorded in result.json; claims remain bounded by them.")
    observation = result["observations"]
    lines.append(f"Observed interval: {observation['started_at']} to {observation['finished_at']}; "
                 f"duration seconds: {observation['duration_seconds']}; model: {observation['model']}.")
    lines.extend(["", "Worker recommendation (input to the receiving session):",
                  result["recommendation"], ""])
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result", type=Path, nargs="?", help="Existing result.json to validate")
    parser.add_argument("--describe", metavar="TA-SKILL", help="Print only this skill's input/result schema")
    parser.add_argument("--guidance", action="store_true", help="Validate a receiving session's guidance instead of a worker result")
    parser.add_argument("--describe-guidance", action="store_true", help="Print the receiving session's guidance schema")
    matrix_view = parser.add_mutually_exclusive_group()
    matrix_view.add_argument("--matrix", action="store_true", help="Print the complete official skill recommendation matrix as Markdown")
    matrix_view.add_argument("--next-skills", metavar="TA-SKILL", help="Print one source row and its recommendation conditions; execute nothing")
    parser.add_argument("--root", type=Path, default=Path.cwd(), help="Repository containing relative references")
    parser.add_argument("--render", action="store_true", help="Create matching result.md; never overwrite a different report")
    args = parser.parse_args()
    try:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        Draft202012Validator.check_schema(schema)
        if args.matrix or args.next_skills:
            if args.result or args.render or args.describe or args.describe_guidance or args.guidance:
                parser.error("matrix views cannot validate or render a result or guidance")
            from skill_matrix import load_matrix, render_matrix, render_row
            matrix = load_matrix(schema)
            print(render_row(matrix, schema, args.next_skills) if args.next_skills
                  else render_matrix(matrix, schema), end="")
            return 0
        if args.describe_guidance:
            if args.result or args.render or args.describe or args.guidance:
                parser.error("--describe-guidance cannot validate or render an artifact")
            print(json.dumps(schema["$defs"]["guidance"], indent=2))
            return 0
        if args.describe:
            if args.result or args.render or args.guidance:
                parser.error("--describe cannot validate or render a result")
            contract = schema["x-skill-contracts"].get(args.describe)
            if contract is None:
                parser.error("unknown skill")
            selected = dict(schema)
            selected["properties"] = dict(schema["properties"])
            selected["properties"]["result"] = next(
                variant for variant in schema["properties"]["result"]["oneOf"]
                if variant["properties"]["kind"]["const"] == contract["kind"])
            selected["x-skill-contracts"] = {args.describe: contract}
            selected.pop("$defs", None)
            print(json.dumps(selected, indent=2))
            return 0
        if args.result is None:
            parser.error("supply a result file or --describe TA-SKILL")
        result = json.loads(args.result.read_text(encoding="utf-8"))
        if args.guidance:
            from guidance import render_guidance, validate_guidance
            errors = validate_guidance(result, schema, args.root.resolve())
            renderer = render_guidance
        else:
            errors = validate(result, schema, args.root.resolve())
            renderer = render
        report = args.result.with_suffix(".md")
        if not errors:
            expected = renderer(result)
            if report.exists() and report.read_text(encoding="utf-8") != expected:
                errors.append("text and JSON differ; reconcile the existing report explicitly")
            elif not report.exists() and args.render:
                report.write_text(expected, encoding="utf-8")
            elif not report.exists():
                errors.append("missing text result; use --render to create it")
    except (OSError, ValueError) as error:
        errors = [str(error)]
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    identity = result["guidance_id"] if args.guidance else result["run_id"]
    kind = "guidance" if args.guidance else "result"
    print(f"Valid dual {kind}: {result['task_id']}/{result['work_item_id']}/{identity}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
