"""Strict physical (1-based) page selection, independent of OCR workers."""

import re


def selected_pages(page_ranges):
    """CLI selection for this run only; omission selects all available pages."""
    return parse_page_ranges(page_ranges) if page_ranges is not None else None


def parse_page_ranges(value):
    if not isinstance(value, str) or not value.strip():
        raise ValueError("Empty physical PDF page selection")
    # Permit whitespace-separated numbers, but reject empty comma/semicolon terms.
    parts = re.split(r"[,;]", value)
    if any(not part.strip() for part in parts):
        raise ValueError("Empty term in physical PDF page selection")
    pages = set()
    for part in parts:
        tokens = re.findall(r"\d+\s*(?:-|\.\.|:)\s*\d+|\d+|\S+", part.strip())
        for token in tokens:
            match = re.fullmatch(r"(\d+)\s*(?:-|\.\.|:)\s*(\d+)", token)
            if match:
                first, last = map(int, match.groups())
                if first < 1 or last < first:
                    raise ValueError(f"Invalid physical PDF page range: {token}")
                pages.update(range(first, last + 1))
            elif token.isdigit() and int(token) > 0:
                pages.add(int(token))
            else:
                raise ValueError(f"Malformed physical PDF page selection: {token}")
    return sorted(pages)
