"""Structured response contracts owned by the existing PDF inference stages."""


def object_schema(properties):
    return {"type": "object", "properties": properties, "required": list(properties), "additionalProperties": False}


STRING = {"type": "string"}
NULL_STRING = {"type": ["string", "null"]}
BOOL = {"type": "boolean"}
BBOX = {"type": "array", "items": {"type": "number"}, "minItems": 4, "maxItems": 4}
NULL_BBOX = {**BBOX, "type": ["array", "null"]}
EMPTY_PAGE = object_schema({"is_blank": BOOL, "type": NULL_STRING, "graphic_bbox_norm": NULL_BBOX, "caption": NULL_STRING})
SEGMENTATION = object_schema({
    "is_full_page_graphic": BOOL, "graphic_caption": NULL_STRING,
    "segments": {"type": "array", "items": object_schema({
        "idx": {"type": "integer", "minimum": 0},
        "type": {"type": "string", "enum": ["header", "footer", "toc_heading", "toc", "thumb_index", "chapter", "heading", "callout", "callout_text", "prose", "code_block", "table", "graphic", "caption", "caption_continuation"]},
        "heading_level": {"type": ["integer", "null"]}, "graphic_bbox_norm": NULL_BBOX,
    })},
})
SEAM = object_schema({"is_continuation": BOOL, "de_hyphenated_word": NULL_STRING, "explanation": STRING})
GRAPHIC_UNION = object_schema({"is_single_graphic": BOOL, "title": STRING, "rationale": STRING})
CONTINUATION = object_schema({"is_continuation": BOOL, "confidence": {"type": "number", "minimum": 0, "maximum": 1}, "relationship": STRING, "explanation": STRING})
GRAPHIC_TRIAGE = object_schema({"type": {"type": "string", "enum": ["mermaid", "ascii_art", "schematic"]}, "caption": STRING})
PROPERTIES = object_schema({"title": STRING, "book": STRING, "chapter": STRING, "tags": {"type": "array", "items": STRING}})
TITLE = object_schema({"title": STRING, "slug": STRING})
