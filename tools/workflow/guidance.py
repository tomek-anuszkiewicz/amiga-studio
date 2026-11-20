"""Validate and render caller-authored choices; never authorize or dispatch them."""
import hashlib
import json
import subprocess

from jsonschema import Draft202012Validator

from result import portable, render, validate
from skill_matrix import MATRIX_PATH, MATRIX_REFERENCE, load_matrix


def read_reference(ref, root, errors):
    if not portable(ref["path"]) or ref["sha256"] is None:
        errors.append(f"reference needs a portable path and content identity: {ref['path']}")
        return None
    try:
        content = (root / ref["path"]).read_bytes()
    except OSError as error:
        errors.append(f"unavailable reference {ref['path']}: {error}")
        return None
    if hashlib.sha256(content).hexdigest() != ref["sha256"]:
        errors.append(f"changed reference: {ref['path']}")
        return None
    return content


def check_candidate(candidate, root, errors):
    """Compare only the declared scope using read-only Git and content hashes."""
    scope = candidate["scope"]
    if not scope or any(not portable(path) for path in scope):
        errors.append("current candidate requires a nonempty portable scope")
        return
    base = candidate["base_commit"]
    if len(base) not in (40, 64) or any(char not in "0123456789abcdef" for char in base):
        errors.append("current candidate requires an immutable base commit")
        return
    try:
        patch = subprocess.check_output(
            ["git", "diff", "--binary", "--no-ext-diff", "--no-textconv", base, "--", *scope],
            cwd=root, stderr=subprocess.PIPE)
        names = subprocess.check_output(
            ["git", "ls-files", "--others", "--exclude-standard", "-z", "--", *scope],
            cwd=root, stderr=subprocess.PIPE).decode("utf-8").split("\0")
    except (OSError, subprocess.CalledProcessError, UnicodeError) as error:
        errors.append(f"cannot compare current candidate: {error}")
        return
    if hashlib.sha256(patch).hexdigest() != candidate["diff_sha256"]:
        errors.append("current candidate diff changed; refresh guidance before selection")
    actual = {name for name in names if name}
    declared = [ref["path"] for ref in candidate["untracked"]]
    if actual != set(declared) or len(declared) != len(set(declared)):
        errors.append("current candidate untracked paths changed; refresh guidance")
    for ref in candidate["untracked"]:
        read_reference(ref, root, errors)


def selected_review_findings(assignment, qualified_findings):
    """Resolve legacy local IDs only when they identify a single finding."""
    choices = {}
    for key in qualified_findings:
        choices.setdefault(key.partition("/")[2], set()).add(key)
    selected = []
    for identifier in assignment["selected_findings"]:
        if identifier in qualified_findings:
            selected.append(identifier)
        else:
            matches = choices.get(identifier, ())
            if len(matches) != 1:
                raise ValueError(f"select a known lane-qualified finding for {identifier}")
            selected.extend(matches)
    if len(selected) != len(set(selected)):
        raise ValueError("select each lane-qualified finding once")
    return set(selected)


def route_finding_id(finding):
    return f"{finding['lane']}/{finding['id']}"


def check_review_route(guidance, source, root, errors):
    ref = guidance.get("review_route_ref")
    operations = [item for item in guidance["options"] if item["kind"] == "operation"]
    if not ref and not operations:
        return

    from review_inputs import Artifacts, REFERENCE, check_shape, current_route_path, normalize_candidate
    from review_routing import recommend

    root = root.resolve()
    artifacts = Artifacts(root)
    authority = current_route_path(guidance["task_id"], guidance["work_item_id"])
    try:
        current = artifacts.path(authority)
        if not (ref or (source and source["result"]["kind"] in ("review", "finding_challenge"))
                or current.parent.exists()):
            return
        published = check_shape(json.loads(current.read_bytes()), REFERENCE, authority)
        if ref != published:
            errors.append("review guidance requires the current authoritative routing request")
            return
    except (ValueError, OSError) as error:
        errors.append(f"current review routing request is unavailable: {error}")
        return

    try:
        request = artifacts.read_json(ref)
        source_identity = None
        if operations:
            source_identity = (guidance.get("source_result"),
                               source["inputs"]["candidate"] if source else None,
                               source["result"]["kind"] if source else None)
        advice = recommend(request, root, source=source_identity)
    except (ValueError, OSError) as error:
        errors.append(f"invalid review routing request: {error}")
        return
    if any(advice.get(name) != guidance[name] for name in ("task_id", "work_item_id")):
        errors.append("review sequence belongs to another task or work item")
    candidate = guidance["current_candidate"]
    if candidate:
        try:
            if normalize_candidate(candidate) != advice.get("candidate"):
                errors.append("review guidance does not identify the current frozen candidate")
        except (ValueError, KeyError, TypeError) as error:
            errors.append(f"invalid frozen review candidate: {error}")
    stopped = advice["phase"] in (
        "human_decision_required", "paused", "one_round_authorized", "continuation_already_selected", "needs_input",
    )
    expected = {row["skill"]: row for row in advice["recommended_lanes"]}
    challenge = advice.get("challenge", {})
    state = challenge.get("state")
    collection = challenge.get("manifest", {}).get("findings", [])
    supported = {route_finding_id(row["finding"]): row for row in challenge.get("eligible_repairs", [])}
    sources = {route_finding_id(row["finding"]): row["source"] for row in request["assessments"]}
    for item in operations:
        assignment = item["assignment"] or {}
        skill = assignment.get("skill")
        lane = expected.get(skill, {})
        action = lane.get("action")
        released = advice["phase"] == "continuation_already_selected" and (
            action == "collect_primary"
            or (skill == "TA-challenge-findings" and state == "ready_to_claim")
            or (skill == "TA-fix-findings" and state == "returned" and supported)
        )
        if stopped and not released:
            errors.append("the review checkpoint or missing history suppresses operation options")
            if advice["phase"] == "needs_input":
                errors.extend(f"review routing: {gap}" for gap in advice["gaps"])
            continue
        if skill in ("TA-fix-findings", "TA-challenge-findings"):
            try:
                selected = selected_review_findings(assignment, sources.keys())
            except ValueError as error:
                errors.append(f"{item['id']}: {error}")
                continue
        if skill == "TA-fix-findings":
            if not selected or not selected <= supported.keys():
                errors.append("repair choices require an upheld lane-qualified finding from the current challenge")
            elif any(sources[key] not in assignment["input_refs"] for key in selected):
                errors.append("repair choices require the selected findings' original lane reports")
        if skill == "TA-challenge-findings":
            ids = {route_finding_id(row["finding"]) for row in collection}
            if state != "ready_to_claim" or assignment["round_limit"] != 1 or selected != ids:
                errors.append("the challenge requires the exact current collection and a single pass")
        if skill == "TA-commit-batch" and advice["phase"] != "done":
            errors.append("commit guidance requires completed independent reviews and passing mandatory gates")
        if action == "collect_primary" and assignment["selected_findings"]:
            errors.append("the outstanding primary review does not determine selected findings")
        if skill and skill.startswith("TA-review-") and skill not in expected:
            errors.append(f"{item['id']}: review lane is outside the current recommendation")


def validate_guidance(guidance, schema, root):
    errors = [f"{'/'.join(map(str, error.path))}: {error.message}" for error in
              Draft202012Validator(schema["$defs"]["guidance"]).iter_errors(guidance)]
    if errors:
        return errors
    matrix = load_matrix(schema)
    matrix_ref = guidance["matrix_ref"]
    if matrix_ref["path"] != MATRIX_REFERENCE:
        errors.append("guidance must identify the authoritative skill matrix")
    matrix_bytes = read_reference(matrix_ref, root, errors)
    if matrix_bytes is not None and matrix_bytes != MATRIX_PATH.read_bytes():
        errors.append("guidance matrix differs from the validating tool's matrix; refresh the context")
    source = None
    if guidance["source_result"] is not None:
        content = read_reference(guidance["source_result"], root, errors)
        if content is not None:
            try:
                source = json.loads(content)
            except ValueError as error:
                errors.append(f"invalid source result: {error}")
            else:
                # Its original input bytes may have changed. Keep the source claim
                # immutable; only the offered operation's current inputs are live.
                problems = validate(source, schema, root, check_references=False)
                errors.extend(f"source result: {error}" for error in problems)
                if problems:
                    source = None
                else:
                    report = (root / guidance["source_result"]["path"]).with_suffix(".md")
                    try:
                        if report.read_text(encoding="utf-8") != render(source):
                            errors.append("source text and JSON differ")
                    except OSError as error:
                        errors.append(f"source text is unavailable: {error}")
    if source is not None:
        for field in ("task_id", "work_item_id"):
            if source[field] != guidance[field]:
                errors.append(f"source result has a different {field}")
        if source["run_id"] != guidance["source_run_id"]:
            errors.append("source result has a different run ID")
        if source["inputs"]["candidate"] != guidance["source_candidate"]:
            errors.append("source candidate differs from the preserved worker input")
    elif not guidance["identity_limits"]:
        errors.append("missing source result requires an identity limitation")
    check_review_route(guidance, source, root, errors)
    if guidance["supersedes"] is not None:
        content = read_reference(guidance["supersedes"], root, errors)
        if content is not None:
            try:
                previous = json.loads(content)
                if (previous["kind"] != "guidance" or previous["task_id"] != guidance["task_id"]
                        or previous["work_item_id"] != guidance["work_item_id"]
                        or previous["guidance_id"] == guidance["guidance_id"]):
                    errors.append("supersedes must identify a prior guidance set for this work item")
            except (ValueError, KeyError, TypeError):
                errors.append("supersedes is not a guidance record")
    ids = [option["id"] for option in guidance["options"]]
    if len(ids) != len(set(ids)):
        errors.append("option IDs must be unique within one guidance set")
    if guidance["recommended_option_id"] is not None and guidance["recommended_option_id"] not in ids:
        errors.append("recommended option is absent from this guidance set")
    ready = False
    for option in guidance["options"]:
        assignment = option["assignment"]
        transition = option["transition"]
        if (option["kind"] == "operation") != (assignment is not None):
            errors.append(f"{option['id']}: only an operation has an executor assignment")
            continue
        if (option["kind"] == "operation") != (transition is not None):
            errors.append(f"{option['id']}: only an operation has a matrix transition")
            continue
        if option["availability"] == "ready" and option["missing_inputs"]:
            errors.append(f"{option['id']}: a ready choice cannot have missing inputs")
        if option["availability"] == "needs_input" and not option["missing_inputs"]:
            errors.append(f"{option['id']}: name the missing inputs")
        if assignment is None:
            continue
        contract = schema["x-skill-contracts"][assignment["skill"]]
        if assignment["executor"] not in contract.get("allowed_agents", [contract["agent"]]):
            errors.append(f"{option['id']}: executor does not own selected skill")
        scopes = assignment["read_scope"] + assignment["write_scope"]
        if not scopes or any(not portable(path) for path in scopes):
            errors.append(f"{option['id']}: assignment requires a portable read/write scope")
        if assignment["previous_run_id"] is not None:
            if not assignment["cumulative_history"] or not assignment["new_approach"]:
                errors.append(f"{option['id']}: continuation requires history and a new approach")
        elif assignment["cumulative_history"] or assignment["new_approach"] is not None:
            errors.append(f"{option['id']}: continuation history requires a predecessor")
        if source is not None:
            codes = matrix["transitions"][source["skill"]].get(assignment["skill"], [])
            if transition["condition"] not in codes:
                errors.append(f"{option['id']}: not an official matrix recommendation for this source skill")
            known = {finding["id"] for finding in source["findings"]}
            routed_selection = guidance.get("review_route_ref") and assignment["skill"] in (
                "TA-fix-findings", "TA-challenge-findings",
            )
            if routed_selection:
                # The current route validates qualified findings across all applicable reports.
                known.update(assignment["selected_findings"])
            if not set(assignment["selected_findings"]).issubset(known):
                errors.append(f"{option['id']}: selected findings are absent from the source result")
            if (source["status"] in ("limit_reached", "interrupted")
                    and assignment["skill"] == source["skill"]
                    and assignment["previous_run_id"] != source["run_id"]):
                errors.append(f"{option['id']}: resumed operation must retain its predecessor")
        else:
            errors.append(f"{option['id']}: an official operation proposal needs an identified source result")
        if option["availability"] == "ready":
            ready = True
            if source is None or guidance["current_candidate"] is None or not assignment["input_refs"]:
                errors.append(f"{option['id']}: ready operation needs source, current candidate and inputs")
            for ref in assignment["input_refs"]:
                read_reference(ref, root, errors)
    if ready and guidance["current_candidate"] is not None:
        check_candidate(guidance["current_candidate"], root, errors)
    return errors


def operation_instruction(guidance, option):
    """Construct the copyable assignment from the same data that is validated."""
    assignment = option["assignment"]
    inputs = "; ".join(f"{ref['path']} (SHA-256 {ref['sha256']})" for ref in assignment["input_refs"])
    candidate = json.dumps(guidance["current_candidate"], ensure_ascii=False, sort_keys=True)
    source = guidance["source_result"]
    lines = [f"Select option {option['id']} from guidance {guidance['guidance_id']} for "
             f"{guidance['task_id']}/{guidance['work_item_id']}; recheck applicability before execution.",
             f"Use executor {assignment['executor']} with skill {assignment['skill']}.",
             f"Matrix: {guidance['matrix_ref']['path']} (SHA-256 {guidance['matrix_ref']['sha256']}).",
             f"Matrix condition: {option['transition']['condition']}. Basis: {option['transition']['basis']}",
             option["instruction"],
             f"Source result: {source['path']} (SHA-256 {source['sha256']}); run {guidance['source_run_id']}.",
             f"Current candidate: {candidate}",
             f"Inputs: {inputs}",
             f"Selected findings: {', '.join(assignment['selected_findings']) or 'none'}.",
             f"Read scope: {', '.join(assignment['read_scope']) or 'none'}.",
             f"Write scope: {', '.join(assignment['write_scope']) or 'none'}.",
             f"Round limit: {assignment['round_limit']}; count the first pass and stop at success or the limit.",
             f"Authorization: {option['authorization_impact']}"]
    if assignment["previous_run_id"] is not None:
        lines.extend([f"Previous run: {assignment['previous_run_id']}.",
                      f"Cumulative history: {'; '.join(assignment['cumulative_history'])}",
                      f"New approach: {assignment['new_approach']}"])
    return "\n".join(lines)


def render_guidance(guidance):
    lines = [f"# Guidance {guidance['guidance_id']}", "", guidance["outcome"], "",
             guidance["consequence"], "",
             f"Source: {guidance['task_id']}/{guidance['work_item_id']}/{guidance['source_run_id']}.",
             f"Official recommendation matrix: {guidance['matrix_ref']['path']} (SHA-256 {guidance['matrix_ref']['sha256']}).",
             "The receiving session names the current guidance set. An older set grants no authority.", ""]
    for field in ("blockers", "uncertainty", "identity_limits"):
        lines.extend(f"- {field.replace('_', ' ').capitalize()}: {item}" for item in guidance[field])
    lines.extend(["", f"Recommendation: {guidance['recommended_option_id'] or 'no further operation'}. "
                  f"{guidance['recommendation_reason']}", ""])
    for option in guidance["options"]:
        lines.extend([f"## {option['id']}. {option['label']} ({option['availability']})", "",
                      option["effect"], "", f"Authorization: {option['authorization_impact']}", ""])
        lines.extend(f"- Missing input: {item}" for item in option["missing_inputs"])
        if option["transition"] is not None:
            lines.extend([f"Matrix condition: {option['transition']['condition']}. "
                          f"Basis: {option['transition']['basis']}", ""])
        if option["kind"] == "operation" and option["availability"] == "ready":
            lines.extend(["Copyable instruction:", "", "```text", operation_instruction(guidance, option), "```", ""])
        elif option["kind"] == "operation":
            assignment = option["assignment"]
            lines.extend([f"Proposed executor: {assignment['executor']}; skill: {assignment['skill']}.",
                          "This proposal is not ready to execute.", option["instruction"], ""])
        else:
            lines.extend([option["instruction"], ""])
    lines.extend(["No operation starts from this report. Select a current option or give a different instruction.", ""])
    return "\n".join(lines)
