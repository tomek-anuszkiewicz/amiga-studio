"""Table conversion identity and format recognition."""

from .pdf_artifacts import read_json
from .pdf_page_conversion import read_conversion
from .pdf_selection import selected_pages

STAGE = "02.81_transform_page_tables"


def converted(segment):
    return segment.get("table_format") in {"markdown", "html"}


def read_table_conversion(workspace):
    """Overlay changed table pages on the selected filtered predecessor dataset."""
    status = read_json(workspace / "stage_status.json")
    if "page_ranges" not in status["02.81"]:
        # Historical runs saved the selected dataset in full in this directory.
        yield from read_conversion(workspace, STAGE)
        return
    pages = selected_pages(status["02.81"].get("page_ranges"))
    for entry, page in read_conversion(workspace, "02.80_filter_page_content", pages=pages):
        replacement = workspace / STAGE / f"{entry['page_id']}_segments.json"
        yield entry, read_json(replacement) if replacement.exists() else page
