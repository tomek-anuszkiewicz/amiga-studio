"""Independent page-object contract in original PNG pixel coordinates."""

from .pdf_artifacts import read_json, page_files
from .pdf_schemas import object_schema, STRING, BOOL

TYPES = ["cover", "header", "footer", "toc_heading", "toc", "thumb_index", "chapter", "heading",
         "callout", "callout_text", "prose", "code_block", "table", "graphic", "caption",
         "footnote", "table_legend", "index_heading", "index",
         "list_of_tables_heading", "list_of_tables", "list_of_figures_heading", "list_of_figures"]
DEFERRED_TEXT_TYPES = {"graphic", "table", "cover"}
CALLOUT_STAGE = "02.4_reclassify_callouts"
TABLE_SPLIT_STAGE = "02.41_split_tables"
TABLE_RECLASSIFY_STAGE = "02.42_reclassify_tables"
CODE_RECLASSIFY_STAGE = "02.43_reclassify_code_blocks"
IMAGE_ROTATION_STAGE = "02.44_detect_image_rotation"
CODE_REFORMAT_STAGE = "02.45_reformat_code_block"
PAGE_OVERRIDE_LAYERS = (("02.4", CALLOUT_STAGE), ("02.41", TABLE_SPLIT_STAGE),
                        ("02.42", TABLE_RECLASSIFY_STAGE), ("02.43", CODE_RECLASSIFY_STAGE),
                        ("02.44", IMAGE_ROTATION_STAGE), ("02.45", CODE_REFORMAT_STAGE))


def page_override_layers(before_stage=None):
    """Return only predecessor layers when an override worker reads its input."""
    for stage_id, directory in PAGE_OVERRIDE_LAYERS:
        if stage_id == before_stage:
            break
        yield stage_id, directory


def completed_page_layers(workspace, before_stage=None):
    """Absent optional stages mean skipped; partial execution never falls back."""
    status_path = workspace / "stage_status.json"
    status = read_json(status_path) if status_path.exists() else {}
    completed = []
    for stage_id, directory in page_override_layers(before_stage):
        entry = status.get(stage_id)
        if entry is None:
            continue
        if entry.get("status") != "success":
            raise ValueError(f"Incomplete Stage {stage_id} blocks direct Stage 02 consumers")
        completed.append(directory)
    return completed


def resolved_page_files(workspace, pages=None, *, before_stage=None):
    """Enumerate Stage 02 identities and select completed sparse replacements."""
    completed = completed_page_layers(workspace, before_stage)
    for path in page_files(workspace / "02_page_conversion", "page_*_segments.json", pages):
        for directory in completed:
            override = workspace / directory / path.name
            if override.exists():
                path = override
        yield path


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


def read_conversion(workspace, page_object_directory="02_page_conversion", *, pages=None):
    """Read the specified page objects with Stage 01 data in physical-page order."""
    paths = (resolved_page_files(workspace, pages) if page_object_directory == "02_page_conversion"
             else page_files(workspace / page_object_directory, "page_*_segments.json", pages))
    for path in paths:
        page_id = path.stem.removesuffix("_segments")
        entry = read_json(workspace / "01_preprocess" / f"{page_id}.json")
        entry["png_file"] = f"01_preprocess/{page_id}.png"
        yield entry, read_json(path)
