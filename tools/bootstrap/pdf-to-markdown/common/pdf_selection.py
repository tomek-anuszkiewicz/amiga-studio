"""Strict physical (1-based) page selection, independent of OCR workers."""

import re


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


def selected_pages(value, page_count):
    pages = parse_page_ranges(value) if value is not None else list(range(1, page_count + 1))
    if not pages or pages[-1] > page_count:
        raise ValueError("Physical PDF page selection is empty or outside the source document")
    return pages
