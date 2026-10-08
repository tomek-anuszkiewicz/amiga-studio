"""Stable identities and source-field preservation for split page objects."""

from .pdf_page_conversion import SEGMENT


def merge_segment(source, changes):
    """Restore omitted field positions when replacements introduce real values."""
    result = dict(source)
    fields = list(SEGMENT["properties"])
    for key, value in changes.items():
        if key not in result and key in fields:
            following = fields[fields.index(key) + 1:]
            anchor = next((name for name in result if name in following), None)
            if anchor is not None:
                ordered = {}
                for name, item in result.items():
                    if name == anchor:
                        ordered[key] = value
                    ordered[name] = item
                result = ordered
                continue
        result[key] = value
    return result


def segment_replacements(source, response, reserved_ids):
    """Preserve retained objects and assign stable child identities in source order."""
    if response["action"] == "retain":
        return [source]
    children = response["replacements"]
    if not children:
        raise ValueError("Segment reclassification replacement contains no objects")
    replacements = []
    for index, child in enumerate(children):
        identity = source["segment_id"]
        if index:
            suffix = 2
            while f"{identity}_part_{suffix}" in reserved_ids:
                suffix += 1
            identity = f"{identity}_part_{suffix}"
        reserved_ids.add(identity)
        replacement = merge_segment(source, {**child, "segment_id": identity})
        if len(children) == 1:
            replacement.update(bbox=list(source["bbox"]), continuation=source.get("continuation", False))
        replacements.append(replacement)
    return replacements
