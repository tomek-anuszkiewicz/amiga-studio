"""Advisory vocabulary and explicit frozen-page edit interpretation."""

import re
from .pdf_schemas import object_schema, STRING, BBOX
from .pdf_page_conversion import CALLOUT_STAGE

CANDIDATE_TYPES = {"prose", "code_block", "chapter", "heading", "toc_heading",
                   "index_heading", "list_of_tables_heading", "list_of_figures_heading",
                   "footnote", "table_legend", "caption"}
TEXT_TYPES = CANDIDATE_TYPES | {"callout", "callout_text"}
INITIAL_KEYWORDS = {"NOTE", "CAUTION", "WARNING"}
SEGMENT_IDS = {"type": "array", "items": STRING, "minItems": 1}
REPLACEMENT = object_schema({
    "type": {"type": "string", "enum": sorted(TEXT_TYPES)},
    "md_text": STRING, "bbox": BBOX, "source_segment_ids": SEGMENT_IDS,
})
RESPONSE = object_schema({
    "replacements": {"type": "array", "items": object_schema({
        "source_segment_ids": SEGMENT_IDS,
        "replacement_segments": {"type": "array", "items": REPLACEMENT, "minItems": 1},
    })},
})


def leading_label(text):
    """Remove Markdown decoration from a leading label, never from its body."""
    match = re.match(r"^\s*[#>*_`\s]*([A-Za-z]+)\b", text)
    return match.group(1).upper() if match else None


def advisory_prefix(text, words):
    """Recognize decorated leading labels while preserving body Markdown."""
    pattern = (r"^\s*[*_`]*(" + "|".join(map(re.escape, sorted(words))) +
               r")\b[*_`]*\s*(?:[:.-][*_`]*\s*|(?:\r?\n)+|$)")
    match = re.match(pattern, text, re.I)
    return (match.group(1).upper(), text[match.end():].strip()) if match else (None, text)


def advisory_keywords(pages):
    words = set(INITIAL_KEYWORDS)
    for page in pages:
        for segment in page["segments"]:
            if segment["type"] == "callout":
                label = leading_label(segment["md_text"])
                if label:
                    words.add(label)
    return sorted(words)


def request_objects(page, keywords):
    matcher = re.compile(r"\b(?:" + "|".join(map(re.escape, keywords)) + r")\b", re.I)
    objects = []
    for segment in page["segments"]:
        hits = sorted({match.group().upper() for match in matcher.finditer(segment["md_text"])})
        objects.append({**segment,
                        "read_only": segment["type"] not in TEXT_TYPES,
                        "candidate_keywords": hits if segment["type"] in CANDIDATE_TYPES else []})
    return objects


def apply_replacements(page, objects, response):
    """Splice disjoint, executable proposals once; retain all ambiguous ranges."""
    index_by_segment = {obj["segment_id"]: index for index, obj in enumerate(objects)}
    candidates = {obj["segment_id"] for obj in objects if obj["candidate_keywords"]}
    reports = []
    for proposal in response.get("replacements", []):
        indices = []
        report = {"proposal": proposal, "status": "retained", "reason": "", "indices": indices}
        reports.append(report)
        try:
            source = proposal["source_segment_ids"]
            indices.extend(index_by_segment[identity] for identity in source if identity in index_by_segment)
            if len(indices) != len(source):
                raise ValueError("Source segment ID does not resolve in this page")
            if not indices or indices != list(range(indices[0], indices[-1]+1)):
                raise ValueError("Source segments are not an ordered contiguous range")
            if any(objects[index]["read_only"] for index in indices):
                raise ValueError("Range crosses a read-only structural anchor")
            affected = candidates.intersection(source)
            if not affected:
                raise ValueError("Range has no candidate")
            replacements = proposal["replacement_segments"]
            if not replacements or not any(item["type"] == "callout" for item in replacements):
                raise ValueError("Replacement must include an explicit advisory label")
            covered = set()
            for item in replacements:
                contributors = item["source_segment_ids"]
                mapped = [index_by_segment[identity] for identity in contributors]
                if not mapped or mapped != sorted(set(mapped)) or not set(contributors) <= set(source):
                    raise ValueError("Replacement contributors do not resolve within the consumed range")
                if item["type"] not in TEXT_TYPES or not isinstance(item["md_text"], str) or not item["md_text"].strip():
                    raise ValueError("Replacement cannot be interpreted as a nonempty text object")
                covered.update(contributors)
            if covered != set(source):
                raise ValueError("Consumed segment has no replacement provenance")
            report["status"] = "ready"
        except (KeyError, TypeError, ValueError) as error:
            report["reason"] = str(error)
    # Reject both sides of a conflict, including a malformed proposal whose
    # resolvable segments overlap an otherwise executable proposal.
    for i, report in enumerate(reports):
        for other in reports[i+1:]:
            if set(report["indices"]) & set(other["indices"]):
                report.update(status="retained", reason="Overlapping replacement proposals")
                other.update(status="retained", reason="Overlapping replacement proposals")
    accepted = {report["indices"][0]: report for report in reports if report["status"] == "ready"}
    consumed = {index for report in accepted.values() for index in report["indices"]}
    used_ids = {segment["segment_id"] for index, segment in enumerate(page["segments"]) if index not in consumed}
    reserved_ids = {segment["segment_id"] for segment in page["segments"]}
    result = []
    for index, segment in enumerate(page["segments"]):
        if index in accepted:
            report = accepted[index]
            produced = []
            for replacement in report["proposal"]["replacement_segments"]:
                contributors = [page["segments"][index_by_segment[identity]] for identity in replacement["source_segment_ids"]]
                first = contributors[0]
                identity = first["segment_id"]
                if identity in used_ids:
                    suffix = 2
                    while f"{identity}_callout_{suffix}" in used_ids | reserved_ids:
                        suffix += 1
                    identity = f"{identity}_callout_{suffix}"
                used_ids.add(identity)
                kind = replacement["type"]
                produced.append({**first, "segment_id": identity, "type": kind,
                    "md_text": replacement["md_text"],
                    "bbox": replacement["bbox"],
                    "heading_level": first.get("heading_level") if kind == first["type"] and kind not in {"callout", "callout_text"} else None,
                    "continuation": any(item.get("continuation", False) for item in contributors),
                    "source_segment_ids": [item["segment_id"] for item in contributors],
                    "replacement_stage": CALLOUT_STAGE})
            result.extend(produced)
            report.update(status="applied", replacement_segments=produced,
                          consumed_segment_ids=[page["segments"][i]["segment_id"] for i in report["indices"]])
        elif index not in consumed:
            result.append(segment)
    return {**page, "segments": result}, reports
