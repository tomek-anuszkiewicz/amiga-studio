"""PDF artifact IO and requested input resolution; no runtime certification."""

import json
from pathlib import Path


def write_json(path, value):
    path = Path(path)
    candidate = path.with_suffix(path.suffix + ".tmp")
    try:
        candidate.write_text(json.dumps(value, indent=2, ensure_ascii=False), encoding="utf-8")
        candidate.replace(path)
    finally:
        candidate.unlink(missing_ok=True)


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def prepared_pdf_path(source):
    source = Path(source)
    return source.with_stem(f"{source.stem}-ocr")


def page_files(directory, pattern, pages=None):
    selected = set(pages) if pages is not None else None
    return sorted((path for path in Path(directory).iterdir() if path.match(pattern)
                   and (selected is None or int(path.stem.split("_")[1]) in selected)),
                  key=lambda path: int(path.stem.split("_")[1]))


def prepared_text_metadata(blocks):
    return {"provenance": "prepared" if blocks else "none",
            "page_type": "text_page" if blocks else "pure_graphic",
            "classification_basis": "inferred_from_prepared_spans"}
