"""Persist stage subprocess call metrics without discarding usage or errors."""

import json
import os
from pathlib import Path


def record(metric):
    target = os.environ.get("LLM_STAGE_METRICS_FILE")
    if not target:
        return
    path = Path(target)
    data = json.loads(path.read_text(encoding="utf-8")) if path.exists() else {}
    data.setdefault("requests", []).append(metric)
    cached = metric.get("cached", False)
    field = "llm_cached_calls" if cached else "llm_calls"
    data[field] = data.get(field, 0) + 1
    if not cached:
        data["llm_time_seconds"] = data.get("llm_time_seconds", 0.0) + (metric.get("duration_seconds") or 0.0)
    temporary = path.with_suffix(".metrics.tmp")
    temporary.write_text(json.dumps(data, indent=2), encoding="utf-8")
    temporary.replace(path)
