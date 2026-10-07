"""Independent page-object contract in original PNG pixel coordinates."""

from .pdf_artifacts import read_json, page_files
from .pdf_schemas import object_schema, STRING, BOOL

TYPES = ["cover", "header", "footer", "toc_heading", "toc", "thumb_index", "chapter", "heading",
         "callout", "callout_text", "prose", "code_block", "table", "graphic", "caption",
         "footnote", "table_legend", "index_heading", "index",
         "list_of_tables_heading", "list_of_tables", "list_of_figures_heading", "list_of_figures"]
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


def read_conversion(workspace, page_object_directory="02_page_conversion"):
    """Read the specified page objects with Stage 01 data in physical-page order."""
    for path in page_files(workspace / page_object_directory, "page_*_segments.json"):
        page_id = path.stem.removesuffix("_segments")
        entry = read_json(workspace / "01_preprocess" / f"{page_id}.json")
        entry["png_file"] = f"01_preprocess/{page_id}.png"
        yield entry, read_json(path)
