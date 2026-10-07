"""Independent page-object contract in original PNG pixel coordinates."""

from .pdf_artifacts import read_json, page_files
from .pdf_schemas import object_schema, STRING, BOOL

TYPES = ["cover", "header", "footer", "toc_heading", "toc", "thumb_index", "chapter", "heading",
         "callout", "callout_text", "prose", "code_block", "table", "graphic", "caption",
         "footnote", "table_legend", "index_heading", "index",
         "list_of_tables_heading", "list_of_tables", "list_of_figures_heading", "list_of_figures"]
DEFERRED_TEXT_TYPES = {"graphic", "table", "cover"}
CALLOUT_STAGE = "02.4_reclassify_callouts"


def callout_layer_completed(workspace):
    """Absent optional stage means skipped; partial execution never falls back."""
    status_path = workspace / "stage_status.json"
    status = read_json(status_path) if status_path.exists() else {}
    entry = status.get("02.4")
    if entry is None:
        return False
    if entry.get("status") != "success":
        raise ValueError("Incomplete Stage 02.4 blocks direct Stage 02 consumers")
    return True


def resolved_page_files(workspace):
    """Enumerate Stage 02 identities and select completed sparse replacements."""
    completed = callout_layer_completed(workspace)
    for path in page_files(workspace / "02_page_conversion", "page_*_segments.json"):
        override = workspace / CALLOUT_STAGE / path.name
        yield override if completed and override.exists() else path


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
    paths = (resolved_page_files(workspace) if page_object_directory == "02_page_conversion"
             else page_files(workspace / page_object_directory, "page_*_segments.json"))
    for path in paths:
        page_id = path.stem.removesuffix("_segments")
        entry = read_json(workspace / "01_preprocess" / f"{page_id}.json")
        entry["png_file"] = f"01_preprocess/{page_id}.png"
        yield entry, read_json(path)
