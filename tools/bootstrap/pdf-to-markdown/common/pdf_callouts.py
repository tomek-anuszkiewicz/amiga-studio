"""Advisory vocabulary, frame mapping and explicit frozen-page edit interpretation."""

import re
from .pdf_schemas import object_schema, STRING, NULL_STRING
from .pdf_page_conversion import CALLOUT_STAGE

CANDIDATE_TYPES = {"prose", "code_block", "chapter", "heading", "toc_heading",
                   "index_heading", "list_of_tables_heading", "list_of_figures_heading",
                   "footnote", "table_legend", "caption"}
TEXT_TYPES = CANDIDATE_TYPES | {"callout", "callout_text"}
INITIAL_KEYWORDS = {"NOTE", "CAUTION", "WARNING"}
FRAME_IDS = {"type": "array", "items": STRING, "minItems": 1}
DECISION = object_schema({
    "candidate_frame_id": STRING,
    "decision": {"type": "string", "enum": ["keep", "replace", "uncertain"]},
    "keyword": STRING,
    "attachment": {"type": "string", "enum": ["independent_advisory", "table_note",
                   "figure_note", "ordinary_text", "uncertain"]},
    "anchor_frame_id": NULL_STRING, "reason": STRING,
})
REPLACEMENT = object_schema({
    "type": {"type": "string", "enum": sorted(TEXT_TYPES)},
    "md_text": STRING, "source_frame_ids": FRAME_IDS,
})
RESPONSE = object_schema({
    "decisions": {"type": "array", "items": DECISION},
    "replacements": {"type": "array", "items": object_schema({
        "source_frame_ids": FRAME_IDS,
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


def request_frames(page, keywords):
    matcher = re.compile(r"\b(?:" + "|".join(map(re.escape, keywords)) + r")\b", re.I)
    frames = []
    for index, segment in enumerate(page["segments"], 1):
        hits = sorted({match.group().upper() for match in matcher.finditer(segment["md_text"])})
        frames.append({**segment, "frame_id": f"B{index}",
                       "read_only": segment["type"] not in TEXT_TYPES,
                       "candidate_keywords": hits if segment["type"] in CANDIDATE_TYPES else []})
    return frames


def apply_replacements(page, frames, response):
    """Splice disjoint, executable proposals once; retain all ambiguous ranges."""
    index_by_frame = {frame["frame_id"]: index for index, frame in enumerate(frames)}
    candidates = {frame["frame_id"] for frame in frames if frame["candidate_keywords"]}
    decisions = {}
    for decision in response.get("decisions", []):
        identity = decision.get("candidate_frame_id")
        decisions.setdefault(identity, []).append(decision)
    reports = []
    for proposal in response.get("replacements", []):
        indices = []
        report = {"proposal": proposal, "status": "retained", "reason": "", "indices": indices}
        reports.append(report)
        try:
            source = proposal["source_frame_ids"]
            indices.extend(index_by_frame[identity] for identity in source if identity in index_by_frame)
            if len(indices) != len(source):
                raise ValueError("Source frame ID does not resolve in this page")
            if not indices or indices != list(range(indices[0], indices[-1]+1)):
                raise ValueError("Source frames are not an ordered contiguous range")
            if any(frames[index]["read_only"] for index in indices):
                raise ValueError("Range crosses a read-only structural anchor")
            affected = candidates.intersection(source)
            if not affected:
                raise ValueError("Range has no candidate")
            for identity in affected:
                entries = decisions.get(identity, [])
                if len(entries) != 1 or entries[0].get("decision") != "replace" or entries[0].get("attachment") != "independent_advisory":
                    raise ValueError("Candidate decision is missing, conflicting or preserves the source")
            replacements = proposal["replacement_segments"]
            if not replacements or not any(item["type"] == "callout" for item in replacements):
                raise ValueError("Replacement must include an explicit advisory label")
            covered = set()
            for item in replacements:
                contributors = item["source_frame_ids"]
                mapped = [index_by_frame[identity] for identity in contributors]
                if not mapped or mapped != sorted(set(mapped)) or not set(contributors) <= set(source):
                    raise ValueError("Replacement contributors do not resolve within the consumed range")
                if item["type"] not in TEXT_TYPES or not isinstance(item["md_text"], str) or not item["md_text"].strip():
                    raise ValueError("Replacement cannot be interpreted as a nonempty text object")
                covered.update(contributors)
            if covered != set(source):
                raise ValueError("Consumed frame has no replacement provenance")
            report["status"] = "ready"
        except (KeyError, TypeError, ValueError) as error:
            report["reason"] = str(error)
    # Reject both sides of a conflict, including a malformed proposal whose
    # resolvable frames overlap an otherwise executable proposal.
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
                contributors = [page["segments"][index_by_frame[identity]] for identity in replacement["source_frame_ids"]]
                first = contributors[0]
                identity = first["segment_id"]
                if identity in used_ids:
                    suffix = 2
                    while f"{identity}_callout_{suffix}" in used_ids | reserved_ids:
                        suffix += 1
                    identity = f"{identity}_callout_{suffix}"
                used_ids.add(identity)
                boxes = [item["bbox"] for item in contributors]
                kind = replacement["type"]
                produced.append({**first, "segment_id": identity, "type": kind,
                    "md_text": replacement["md_text"],
                    "bbox": [min(b[0] for b in boxes), min(b[1] for b in boxes),
                             max(b[2] for b in boxes), max(b[3] for b in boxes)],
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
