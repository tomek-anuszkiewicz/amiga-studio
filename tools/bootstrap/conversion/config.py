"""Stage selections; HTML validation and direct PDF configuration parsing."""

from dataclasses import dataclass
from pathlib import Path
import math
import yaml

PDF_STAGES = frozenset((
    "02_page_conversion", "02.4_reclassify_callouts", "02.41_split_tables", "02.42_reclassify_tables",
    "02.81_transform_page_tables", "04_stream_reduction",
    "06_detect_continuations", "07_transform_tables", "08_transform_graphics",
    "09_transform_prose", "10_proofread_stream", "12_generate_properties",
    "13_refine_first_chapter_name",
))
HTML_STAGES = frozenset(("html_to_markdown",))


class UniqueLoader(yaml.SafeLoader):
    """Reject duplicate mapping keys instead of silently overwriting them."""


def _mapping(loader, node):
    pairs = loader.construct_pairs(node, deep=True)
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate configuration key: {key}")
        result[key] = value
    return result


UniqueLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _mapping)


@dataclass(frozen=True)
class StageSelection:
    stage: str
    model: str
    reasoning_effort: str


def validate_config(config, known_stages, required_stages):
    if not isinstance(config, dict) or not isinstance(config.get("llm"), dict):
        raise ValueError("Configuration requires an llm mapping")
    llm = config["llm"]
    if set(llm) - {"stages", "timeout_seconds", "concurrency"}:
        raise ValueError("Unknown llm configuration fields")
    stages = llm.get("stages")
    if not isinstance(stages, dict) or set(stages) - set(known_stages):
        raise ValueError("Invalid or unknown inference stages")
    if set(required_stages) - set(stages):
        raise ValueError(f"Missing inference stages: {sorted(set(required_stages) - set(stages))}")
    for stage, pair in stages.items():
        if not isinstance(pair, dict) or set(pair) != {"model", "reasoning_effort"}:
            raise ValueError(f"{stage}: requires only model and reasoning_effort")
        if any(not isinstance(v, str) or not v.strip() or v != v.strip() for v in pair.values()):
            raise ValueError(f"{stage}: model and effort must be nonempty identifiers")
    timeout = llm.get("timeout_seconds", 180)
    if isinstance(timeout, bool) or not isinstance(timeout, (int, float)) or not math.isfinite(timeout) or timeout <= 0:
        raise ValueError("timeout_seconds must be finite and positive")
    if llm.get("concurrency", 1) != 1 or isinstance(llm.get("concurrency"), bool):
        raise ValueError("Conversion currently requires concurrency 1")
    return config


def load_config(path: Path, *, known_stages=HTML_STAGES, required_stages=HTML_STAGES):
    if known_stages == PDF_STAGES:
        return yaml.safe_load(Path(path).read_text(encoding="utf-8"))
    config = yaml.load(Path(path).read_text(encoding="utf-8"), Loader=UniqueLoader)
    return validate_config(config, known_stages, required_stages)


def selection(config, stage):
    pair = config["llm"]["stages"][stage]
    return StageSelection(stage, pair["model"], pair["reasoning_effort"])
