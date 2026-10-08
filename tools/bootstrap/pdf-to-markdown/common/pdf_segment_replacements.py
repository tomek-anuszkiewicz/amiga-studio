"""Stable identities and source-field preservation for split page objects."""

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
        replacement = {**source, **child, "segment_id": identity}
        if len(children) == 1:
            replacement.update(bbox=list(source["bbox"]), continuation=source.get("continuation", False))
        replacements.append(replacement)
    return replacements
