"""Table conversion identity and format recognition."""

STAGE = "02.81_transform_page_tables"


def converted(segment):
    return segment.get("table_format") in {"markdown", "html"}
