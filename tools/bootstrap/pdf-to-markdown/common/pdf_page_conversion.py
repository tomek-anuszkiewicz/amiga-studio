"""Independent page-object contract in original PNG pixel coordinates."""

from jsonschema import validate
from .pdf_artifacts import read_json, require, validate_preprocess
from .pdf_schemas import object_schema, STRING, BOOL

TYPES = ["cover", "header", "footer", "toc_heading", "toc", "thumb_index", "chapter", "heading",
         "callout", "callout_text", "prose", "code_block", "table", "graphic", "caption",
         "footnote", "table_legend", "index_heading", "index",
         "list_of_tables_heading", "list_of_tables", "list_of_figures_heading", "list_of_figures"]
HEADING_TYPES = {"chapter", "heading", "toc_heading", "index_heading",
                 "list_of_tables_heading", "list_of_figures_heading"}
DEFERRED_TEXT_TYPES = {"graphic", "table", "cover"}
SEGMENT = object_schema({
    "type": {"type": "string", "enum": TYPES},
    "continuation": BOOL,
    "heading_level": {"type": ["integer", "null"], "minimum": 1, "maximum": 6},
    "md_text": STRING,
    "bbox": {"type": "array", "items": {"type": "integer"}, "minItems": 4, "maxItems": 4},
})
RESPONSE = object_schema({
    "page": {"type": "integer", "minimum": 1},
    "image_width": {"type": "integer", "minimum": 1},
    "image_height": {"type": "integer", "minimum": 1},
    "segments": {"type": "array", "items": SEGMENT},
})
STORED_SEGMENT = object_schema({"segment_id": STRING, **SEGMENT["properties"]})
ARTIFACT = object_schema({**RESPONSE["properties"],
                          "segments": {"type": "array", "items": STORED_SEGMENT}})


def validate_page(value, entry, *, stored=False):
    validate(value, ARTIFACT if stored else RESPONSE)
    raster = entry["raster"]
    require(value["page"] == entry["page"] and value["image_width"] == raster["pixel_width"]
            and value["image_height"] == raster["pixel_height"], "Stage 02 page/raster identity mismatch")
    for ordinal, segment in enumerate(value["segments"], 1):
        if stored:
            require(segment["segment_id"] == f"{entry['page_id']}_seg_{ordinal:03d}",
                    "Stage 02 segment identity/order mismatch")
        kind, box, level = segment["type"], segment["bbox"], segment["heading_level"]
        require(kind not in DEFERRED_TEXT_TYPES or segment["md_text"] == "",
                "Stage 02 graphic/table/cover md_text must be empty")
        require((level is not None) == (kind in HEADING_TYPES), "Stage 02 heading level/type mismatch")
        require(all(type(v) is int for v in box), "Stage 02 bbox coordinates must be integer pixels")
        x0, y0, x1, y1 = box
        require(0 <= x0 < x1 <= value["image_width"] and 0 <= y0 < y1 <= value["image_height"],
                "Stage 02 bbox lies outside the original PNG")
    return value


def validate_conversion(workspace, source=None):
    manifest = validate_preprocess(workspace, source,
                                   manifest_path=workspace / "01_preprocess/.manifests/pages_manifest.json")
    directory = workspace / "02_page_conversion"
    expected = {f"{entry['page_id']}_segments.json" for entry in manifest["pages"]}
    require({p.name for p in directory.glob("page_*_segments.json")} == expected,
            "Stage 02 objects do not cover the selected page set")
    pages = []
    for entry in manifest["pages"]:
        value = read_json(directory / f"{entry['page_id']}_segments.json")
        pages.append((entry, validate_page(value, entry, stored=True)))
    return pages
