"""Read prompt-defined page objects in physical-page order."""

from .pdf_artifacts import read_json, page_files


def read_conversion(workspace, page_object_directory="03_page_conversion", *, pages=None):
    """Read the specified page objects with Stage 02 data in physical-page order."""
    paths = page_files(workspace / page_object_directory, "page_*_segments.json", pages)
    for path in paths:
        page_id = path.stem.removesuffix("_segments")
        entry = read_json(workspace / "02_preprocess" / f"{page_id}.json")
        entry["png_file"] = f"02_preprocess/{page_id}.png"
        yield entry, read_json(path)
