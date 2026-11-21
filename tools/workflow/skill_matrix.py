"""Read the official recommendation matrix; never dispatch or authorize work."""
import json
import re
from pathlib import Path


MATRIX_PATH = Path(__file__).with_name("skill-matrix.json")
MATRIX_REFERENCE = "tools/workflow/skill-matrix.json"
RUNTIME_REFERENCE = ".agents/references/workflow-runtime.md"
RUNTIME_PATH = MATRIX_PATH.parents[2] / RUNTIME_REFERENCE
RUNTIME_SECTION = RUNTIME_REFERENCE + "#resolve-one-invocation"


def load_matrix(schema):
    matrix = json.loads(MATRIX_PATH.read_text(encoding="utf-8"))
    if (not isinstance(matrix, dict) or set(matrix) != {"version", "conditions", "transitions"}
            or matrix["version"] != 1):
        raise ValueError("matrix requires version 1, conditions and transitions")
    conditions = matrix["conditions"]
    if (not isinstance(conditions, dict) or not conditions
            or any(not isinstance(code, str) or not code.strip()
                   or not isinstance(text, str) or not text.strip()
                   for code, text in conditions.items())):
        raise ValueError("matrix conditions require nonempty codes and explanations")
    skills = schema["x-skill-contracts"]
    rows = matrix["transitions"]
    if not isinstance(rows, dict) or set(rows) != set(skills):
        raise ValueError("matrix source rows must exactly match the existing skill mapping")
    for source, destinations in rows.items():
        if not isinstance(destinations, dict) or not set(destinations).issubset(skills):
            raise ValueError(f"{source}: matrix destinations must be known skills")
        for target, codes in destinations.items():
            if (not isinstance(codes, list) or not codes
                    or any(not isinstance(code, str) or code not in conditions for code in codes)
                    or len(codes) != len(set(codes))):
                raise ValueError(f"{source} -> {target}: use distinct known condition codes")
    return matrix


def runtime_settings(schema):
    runtime = RUNTIME_PATH.read_text(encoding="utf-8")
    role_section = runtime.partition("### Role defaults\n")[2]
    role_table, skill_heading, skill_section = role_section.partition("### Skill overrides\n")
    skill_table, condition_heading, condition_section = skill_section.partition("### Assignment conditions\n")
    condition_table, resolve_heading, _ = condition_section.partition("### Resolve one invocation\n")
    if not (skill_heading and condition_heading and resolve_heading):
        raise ValueError("runtime reference must contain defaults and assignment conditions")

    row = r"^\|[ \t]*`([^|`\n]+)`[ \t]*\|[ \t]*([^|\n]+?)[ \t]*\|[ \t]*([^|\n]+?)[ \t]*\|[ \t]*$"
    role_defaults = {name: (model, effort) for name, model, effort in re.findall(row, role_table, re.MULTILINE)}
    skill_defaults = {name: (model, effort) for name, model, effort in re.findall(row, skill_table, re.MULTILINE)}
    conditions = {}
    for names, condition in re.findall(r"^\|\s*(`[^|\n]+)\|([^|\n]+)\|[ \t]*$", condition_table, re.MULTILINE):
        for name in re.findall(r"`([^`]+)`", names):
            conditions[name] = condition.strip()

    settings = {}
    for skill, contract in schema["x-skill-contracts"].items():
        executor = contract["agent"]
        defaults = ("Session setting", "Session setting") if executor == "main-session" else role_defaults.get(executor)
        if defaults is None:
            raise ValueError(f"{executor}: missing default model and reasoning effort")
        override = skill_defaults.get(skill, ("Inherit role default", "Inherit role default"))
        model, effort = (default if choice == "Inherit role default" else choice
                         for default, choice in zip(defaults, override, strict=True))
        settings[skill] = (executor, model, effort, conditions.get(skill))
    return settings

def render_row(matrix, schema, source):
    if source not in schema["x-skill-contracts"]:
        raise ValueError("unknown source skill")
    settings = runtime_settings(schema)
    lines = [f"# Official next skills after {source}", "",
             "Evaluate the listed conditions against the actual result and current task.",
             "A cell is a possible recommendation, not permission or an automatic next step.",
             "The user can explicitly request another skill outside this matrix.",
             f"[Runtime procedure]({RUNTIME_SECTION}) owns assignment and session checks.", ""]
    for target, codes in matrix["transitions"][source].items():
        executor, model, effort, condition = settings[target]
        lines.extend([f"## {target}", "", f"Executor: {executor}.",
                      f"Default model: {model}. Default reasoning effort: {effort}."])
        if condition:
            lines.append(condition)
        lines.append("")
        lines.extend(f"- {code}: {matrix['conditions'][code]}" for code in codes)
        lines.append("")
    if not matrix["transitions"][source]:
        lines.append("No standard next operation is defined; do not invent one.")
    return "\n".join(lines).rstrip() + "\n"


def render_matrix(matrix, schema):
    skills = list(schema["x-skill-contracts"])
    settings = runtime_settings(schema)
    lines = ["# Official skill recommendation matrix", "",
             "Generated by `python tools/workflow/result.py --matrix`. Edit the",
             "[authoritative matrix](../tools/workflow/skill-matrix.json), then regenerate this view.",
             "The [receiving procedure](../.agents/references/workflow-guidance.md) owns its use.", "",
             "Rows are the skill that returned a result; columns are possible next skills.",
             "A cell names conditions to consider, not mandatory execution or permission.",
             "Multiple codes are alternatives. An empty cell (`-`) means no official recommendation",
             "for that pair. It does not prohibit the user from directly selecting that skill.",
             "Normal authorization, prerequisites, role boundaries and verification still apply.", "",
             "The receiver chooses normally one to three relevant options, explains the evidence",
             "for each selected condition, and waits for the user's choice. Completion, clarification,",
             "approval, stop and defer need no invented skill or matrix cell. Reuse valid results;",
             "do not repeat checks or continue completed work merely because a cell exists.", "",
             "## Skill axes", "",
             f"The [runtime procedure](../{RUNTIME_SECTION}) owns assignment and session checks.", "",
             "| ID | Skill | Executor | Default model | Default reasoning effort | Assignment condition |",
             "| --- | --- | --- | --- | --- | --- |"]
    for index, skill in enumerate(skills, 1):
        executor, model, effort, condition = settings[skill]
        lines.append(f"| {index:02} | `{skill}` | `{executor}` | {model} | {effort} | {condition or '-'} |")
    lines.extend(["", "## Cell conditions", "", "| Code | Required context |", "| --- | --- |"])
    lines.extend(f"| {code} | {text} |" for code, text in matrix["conditions"].items())
    for start in range(0, len(skills), 11):
        targets = skills[start:start + 11]
        lines.extend(["", f"## Destination columns {start + 1:02}-{start + len(targets):02}", "",
                      "| Source skill | " + " | ".join(f"{i:02}" for i in range(start + 1, start + len(targets) + 1)) + " |",
                      "| --- | " + " | ".join("---" for _ in targets) + " |"])
        for index, source in enumerate(skills, 1):
            cells = [", ".join(matrix["transitions"][source].get(target, [])) or "-" for target in targets]
            lines.append(f"| {index:02} `{source}` | " + " | ".join(cells) + " |")
    return "\n".join(lines) + "\n"
