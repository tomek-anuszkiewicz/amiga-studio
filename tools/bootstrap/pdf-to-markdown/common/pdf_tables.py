"""Source-ordered table groups and shared page/stream companion assembly."""

import re
from html.parser import HTMLParser
from bs4 import BeautifulSoup

STAGE = "02.81_transform_page_tables"
ASSOCIATED = {"caption", "footnote", "table_legend"}
FIELDS = ("table_format", "table_rag_text", "table_group_segment_ids", "table_source_asset")


def converted(segment):
    return segment.get("table_format") in {"markdown", "html"}


def collect_groups(segments, page):
    groups = {}
    owners = {}
    for index, table in enumerate(segments):
        if table["type"] != "table":
            continue
        start, end = index, index + 1
        while start and segments[start - 1]["type"] == "caption":
            start -= 1
        while end < len(segments) and segments[end]["type"] in ASSOCIATED:
            end += 1
        group = segments[start:end]
        identity = table["segment_id"]
        groups[identity] = group
        table["table_group_segment_ids"] = [s["segment_id"] for s in group]

        def warn(reason, blocks):
            print(f"WARNING: {identity} (physical page {page}): {reason}: "
                  + ", ".join(s["segment_id"] for s in blocks))

        for kind in sorted(ASSOCIATED):
            blocks = [s for s in group if s["type"] == kind]
            if len(blocks) > 1:
                warn(f"multiple {kind} blocks", blocks)
        if start < index and any(s["type"] == "caption" for s in segments[index + 1:end]):
            warn("captions above and below table", [s for s in group if s["type"] == "caption"])
        for block in group:
            if block is table:
                continue
            block_id = block["segment_id"]
            if block_id in owners:
                warn(f"ambiguous ownership with {owners[block_id]}", [block])
            owners[block_id] = identity
    return groups


def simple_html(markup):
    """Conservative warning heuristic, never a conversion or fidelity check."""
    class Structure(HTMLParser):
        def __init__(self):
            super().__init__()
            self.stack = []
            self.regular = True

        def handle_starttag(self, tag, attrs):
            self.stack.append(tag)

        def handle_endtag(self, tag):
            if not self.stack or self.stack[-1] != tag:
                self.regular = False
            else:
                self.stack.pop()

        def handle_startendtag(self, tag, attrs):
            self.regular = False

        def handle_data(self, data):
            if data.strip() and "table" in self.stack and not any(tag in self.stack for tag in ("td", "th")):
                self.regular = False

    structure = Structure()
    structure.feed(markup)
    if not structure.regular or structure.stack:
        return False
    soup = BeautifulSoup(markup, "html.parser")
    tables = soup.find_all("table")
    if len(tables) != 1:
        return False
    table = tables[0]
    if any(tag.name not in {"thead", "tbody", "tfoot", "tr", "td", "th"}
           for tag in table.find_all(True) if tag.find_parent(["td", "th"]) is None):
        return False
    allowed = {"b", "strong", "i", "em", "s", "del", "code", "a", "span", "sup", "sub"}
    rows = table.find_all("tr")
    widths = []
    for row in rows:
        cells = row.find_all(["td", "th"], recursive=False)
        if not cells or any(getattr(c, "name", None) not in {"td", "th"}
                            for c in row.children if str(c).strip()):
            return False
        widths.append(len(cells))
        for cell in cells:
            if any(str(cell.get(span, "1")) != "1" for span in ("rowspan", "colspan")):
                return False
            if any(tag.name not in allowed for tag in cell.find_all(True)):
                return False
    return bool(widths) and len(set(widths)) == 1 and len(table.find_all(["td", "th"])) == sum(widths)


def companions(table, asset_reference):
    text = table["table_rag_text"]
    fence = "`" * max(3, max((len(m[0]) + 1 for m in re.finditer(r"`+", text)), default=3))
    return ("<details>\n<summary>Table text for RAG</summary>\n\n"
            f"{fence}text\n{text}\n{fence}\n\n</details>\n\n"
            "<details>\n<summary>Original table image</summary>\n\n"
            f"![Original table {table['segment_id']}]({asset_reference})\n\n</details>")


def ordered_markdown(segments, render, asset_reference):
    """Emit source blocks once, then each saved HTML companion after its group."""
    after = {}
    positions = {s["segment_id"]: i for i, s in enumerate(segments) if s.get("segment_id")}
    for table in segments:
        if table.get("table_format") != "html":
            continue
        ids = table["table_group_segment_ids"]
        last = max(positions[s] for s in ids)
        after.setdefault(last, []).append(table)
    parts = []
    for index, segment in enumerate(segments):
        content = render(segment)
        if content:
            parts.append(content)
        for table in after.get(index, []):
            parts.append(companions(table, asset_reference(table)))
    return "\n\n".join(parts)
