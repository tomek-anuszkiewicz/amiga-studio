"""Prepare compact request aliases without changing inference or page artifacts."""

import json


def compact_cache_objects(value):
    """Copy page objects, omitting defaults and retired replacement metadata."""
    if isinstance(value, list):
        return [compact_cache_objects(item) for item in value]
    if not isinstance(value, dict):
        return value
    result = {key: compact_cache_objects(item) for key, item in value.items()}
    if isinstance(value.get("type"), str) and "bbox" in value:
        for key in ("source_segment_ids", "replacement_stage", "image_only"):
            result.pop(key, None)
        for key, defaults in (("continuation", (False,)), ("heading_level", (None,)),
                              ("md_text", ("", None)), ("rotation", (0,))):
            if key in result and result[key] in defaults:
                del result[key]
        if isinstance(result.get("metadata"), dict) and result["metadata"].get("rotation") == 0:
            del result["metadata"]["rotation"]
    return result


def page_json_prompts(prefix, value, suffix=""):
    """Keep JSON order/spacing identical to future compact artifact requests."""
    prompt = prefix + json.dumps(value, ensure_ascii=False) + suffix
    alias = prefix + json.dumps(compact_cache_objects(value), ensure_ascii=False) + suffix
    return prompt, alias
