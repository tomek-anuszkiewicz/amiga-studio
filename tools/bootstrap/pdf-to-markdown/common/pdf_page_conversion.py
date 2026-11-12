"""Read prompt-defined page objects in physical-page order."""

from .pdf_artifacts import read_json, page_files

PAGE_OBJECT_STAGES = ("03_page_conversion", "04_filter_page_content",
                      "05_transform_page_tables", "07_repair_callouts", "08_proofreading")


def read_conversion(workspace, page_object_directory="03_page_conversion", *, pages=None):
    """Overlay pages through the input stage, omitting empty replacements."""
    paths = {}
    for stage in PAGE_OBJECT_STAGES[:PAGE_OBJECT_STAGES.index(page_object_directory) + 1]:
        directory = workspace / stage
        if directory.is_dir():
            for path in page_files(directory, "page_*_segments.json", pages):
                paths[path.name] = path
    for path in sorted(paths.values(), key=lambda path: int(path.stem.split("_")[1])):
        page = read_json(path)
        if not page["segments"]:
            continue
        page_id = path.stem.removesuffix("_segments")
        entry = read_json(workspace / "02_preprocess" / f"{page_id}.json")
        entry["png_file"] = f"02_preprocess/{page_id}.png"
        yield entry, page
