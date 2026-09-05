#!/usr/bin/env python3
"""
convert_guide.py - Comprehensive AmigaGuide to Markdown Converter

Converts Commodore AmigaGuide (.guide) files into modern, clean Markdown (.md)
optimized for Obsidian and GitHub. Supports:
- Node parsing & multi-chapter splitting or single-document consolidation.
- Removal of running headers, footers, page numbers, and repeated boilerplate.
- Removal of Index nodes, List of Tables, and List of Figures/Images.
- Removal of partial / chapter-level TOCs while preserving chapter prose.
- Bidirectional navigation links (Prev | TOC | Next) at both begin and end of files/sections.
- Full Obsidian TD() and decodeURI() heading anchor compatibility.
"""

import re
import sys
import argparse
import urllib.parse
from collections import Counter
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple

# Regex patterns for AmigaGuide markup
NODE_PATTERN = re.compile(r'@NODE\s+([^\s"]+)(?:\s+"((?:[^"\\]|\\.)*)")?', re.IGNORECASE)
ENDNODE_PATTERN = re.compile(r'@ENDNODE\b', re.IGNORECASE)
LINK_PATTERN = re.compile(r'@\{\s*"((?:[^"\\]|\\.)*)"\s+link\s+([^\}\s]+)(?:\s+\d+)?\s*\}', re.IGNORECASE)
GLOBAL_CMD_PATTERN = re.compile(r'^@([a-zA-Z0-9_\$]+)(?:\s+(.*))?$', re.MULTILINE)

def clean_title(title: str) -> str:
    """Removes invalid filename characters and normalizes whitespace."""
    t = title.replace(r'\"', '"').replace('"', '')
    t = t.replace(':', ' -').replace('/', '-').replace('\\', '-')
    t = re.sub(r'[<>\|\?\*]', '', t)
    t = re.sub(r'\s+', ' ', t).strip()
    return t

def obsidian_encode_anchor(heading_title: str) -> str:
    """
    Obsidian's JavaScript engine passes link subpath anchors through decodeURI().
    decodeURI() does NOT decode %2C (comma) or %3A (colon).
    Therefore, punctuation must remain literal, and only spaces are encoded as %20.
    """
    return urllib.parse.quote(heading_title, safe=";,/?:@&=+$-_.!~*'()#")

class GuideNode:
    def __init__(self, node_id: str, raw_title: str, content: str, index: int):
        self.node_id = node_id.strip()
        self.raw_title = raw_title.replace(r'\"', '"').strip() if raw_title else ""
        self.content = content
        self.index = index

        # If raw_title is missing or identical to node_id, inspect first line of content
        if not self.raw_title or self.raw_title.lower() == self.node_id.lower():
            self._infer_title_from_content()

        if not self.raw_title:
            self.raw_title = self.node_id

        self.clean_name = clean_title(self.raw_title)

        # Heading level & numbers detection
        self.heading_title = self.raw_title
        m_num = re.match(r'^\s*(\d+(?:\.\d+)*)(?:\.|\:)\s*(.*)', self.raw_title)
        if m_num:
            self.sec_num = m_num.group(1)
            self.base_title = m_num.group(2).strip()
            self.heading_title = f"{self.sec_num}. {self.base_title}"
        else:
            self.sec_num = None
            self.base_title = self.raw_title

        # Routing information assigned during planning
        self.target_file = ""
        self.heading_level = 2
        self.is_file_root = False

    def _infer_title_from_content(self):
        """Extracts a prominent first-line heading if node title was omitted."""
        lines = self.content.splitlines()
        for idx, line in enumerate(lines[:5]):
            line_s = line.strip()
            if not line_s:
                continue
            # Match @{u}Heading@{uu} or <u>Heading</u> or @{b}Heading@{ub}
            m = re.match(r'^(?:@\{[uUbB]\}|<u>|\*\*)\s*(.*?)\s*(?:@\{(?:uu|UU|ub|UB)\}|</u>|\*\*)$', line_s)
            if m:
                extracted = m.group(1).strip()
                if len(extracted) > 1 and len(extracted) < 120:
                    self.raw_title = extracted
                    del lines[idx]
                    self.content = '\n'.join(lines)
                    break
            break

    def is_excluded(self) -> bool:
        """
        Returns True if node represents an Index, List of Tables, or List of Figures/Images.
        These are removed as per documentation rules.
        """
        nid = self.node_id.lower().strip()
        raw = self.raw_title.lower().strip()

        # 1. Index node patterns
        index_ids = {'index', 'alphabetic', 'alphabetical', 'alphabetical_index', 'idx', 'function_index'}
        if nid in index_ids:
            return True
        if re.search(r'\b(?:alphabetical|function|subject|master)?\s*index\b', raw):
            # Avoid false positives like "register index" or "hunk index" inside a chapter
            if not re.search(r'\b(?:register|memory|hunk|address|array)\s+index\b', raw):
                return True

        # 2. List of Tables patterns
        if nid in {'tables', 'list_of_tables', 'table_of_tables', 'listoftables'}:
            return True
        if re.search(r'\blist\s+of\s+tables\b', raw) or re.search(r'\btable\s+of\s+tables\b', raw):
            return True

        # 3. List of Figures / Images / Illustrations
        if nid in {'figures', 'images', 'illustrations', 'list_of_figures', 'list_of_images'}:
            return True
        if re.search(r'\blist\s+of\s+(?:figures|images|illustrations)\b', raw):
            return True

        return False

class GuideDocument:
    def __init__(self, raw_text: str):
        self.raw_text = raw_text
        self.metadata = {}
        self.nodes: List[GuideNode] = []
        self.nodes_by_id: Dict[str, GuideNode] = {}
        self.repeated_boilerplate_lines: Set[str] = set()
        self._parse()
        self._find_repeated_boilerplate()

    def _parse(self):
        # 1. Parse global commands before first @NODE
        first_node_match = NODE_PATTERN.search(self.raw_text)
        header_text = self.raw_text[:first_node_match.start()] if first_node_match else self.raw_text

        for line in header_text.splitlines():
            line_s = line.strip()
            if line_s.startswith('@'):
                parts = line_s[1:].split(None, 1)
                cmd = parts[0].upper()
                arg = parts[1].strip('"\'; ') if len(parts) > 1 else ""
                self.metadata[cmd] = arg

        if not first_node_match:
            return

        # 2. Extract nodes
        node_matches = list(NODE_PATTERN.finditer(self.raw_text))
        for idx, match in enumerate(node_matches):
            node_id = match.group(1)
            raw_title = match.group(2) or ""
            start_pos = match.end()

            next_start = node_matches[idx + 1].start() if idx + 1 < len(node_matches) else len(self.raw_text)
            chunk = self.raw_text[start_pos:next_start]

            endnode_m = ENDNODE_PATTERN.search(chunk)
            if endnode_m:
                node_body = chunk[:endnode_m.start()]
            else:
                node_body = chunk

            node = GuideNode(node_id=node_id, raw_title=raw_title, content=node_body, index=idx)
            self.nodes.append(node)
            self.nodes_by_id[node_id.lower()] = node

        # 3. If any nodes still lack descriptive titles, resolve from TOC link labels
        toc_node = self.nodes_by_id.get('main') or (self.nodes[0] if self.nodes else None)
        if toc_node:
            for m in LINK_PATTERN.finditer(toc_node.content):
                label = m.group(1).replace(r'\"', '"').strip()
                target_id = m.group(2).strip().strip('"\'').lower()
                if target_id in self.nodes_by_id:
                    target_node = self.nodes_by_id[target_id]
                    if target_node.raw_title.lower() == target_node.node_id.lower() and label:
                        target_node.raw_title = label
                        target_node.clean_name = clean_title(label)
                        m_num = re.match(r'^\s*(\d+(?:\.\d+)*)(?:\.|\:)\s*(.*)', label)
                        if m_num:
                            target_node.sec_num = m_num.group(1)
                            target_node.base_title = m_num.group(2).strip()
                            target_node.heading_title = f"{target_node.sec_num}. {target_node.base_title}"
                        else:
                            target_node.sec_num = None
                            target_node.base_title = label
                            target_node.heading_title = label

    def _find_repeated_boilerplate(self):
        """
        Identifies lines repeated across many nodes at top or bottom (headers/footers).
        """
        if len(self.nodes) < 3:
            return

        top_lines = Counter()
        bottom_lines = Counter()

        for node in self.nodes:
            lines = [l.strip() for l in node.content.splitlines() if l.strip()]
            for l in lines[:3]:
                if len(l) > 3:
                    top_lines[l] += 1
            for l in lines[-3:]:
                if len(l) > 3:
                    bottom_lines[l] += 1

        threshold = max(3, int(len(self.nodes) * 0.35))
        for line, count in top_lines.items():
            if count >= threshold:
                self.repeated_boilerplate_lines.add(line)
        for line, count in bottom_lines.items():
            if count >= threshold:
                self.repeated_boilerplate_lines.add(line)

def strip_headers_and_footers(text: str, doc_title: str, boilerplate: Set[str]) -> str:
    """
    Removes running headers, footers, page numbers, divider bars, and repeated UI elements.
    """
    lines = text.splitlines()

    # 1. Strip leading header lines
    start_idx = 0
    while start_idx < len(lines):
        line = lines[start_idx].strip()
        if not line:
            start_idx += 1
            continue

        # Border separator line (e.g. ======== or --------)
        if re.match(r'^[\=\-\*_]{4,}\s*$', line) or re.match(r'^\+{4,}\s*$', line):
            start_idx += 1
            continue

        # Page numbering (e.g. Page 12, - 12 -, [Page 12])
        if re.match(r'^(?:page\s+\d+|-\s*\d+\s*-|\[\s*page\s+\d+\s*\])\s*$', line, re.IGNORECASE):
            start_idx += 1
            continue

        # Repeated navigation bar (e.g. [Contents] [Index] [Help] [Prev] [Next])
        if re.search(r'\[\s*(?:Contents|Index|Help|Prev|Next|Main)\s*\]', line, re.IGNORECASE):
            start_idx += 1
            continue
        if re.match(r'^@\{[bB]\}\s*(?:Contents|Main|Index|Help|Prev|Next).*@\{[uU][bB]\}\s*$', line, re.IGNORECASE):
            start_idx += 1
            continue

        # Document title repeated at top of node
        if doc_title and line.lower() == doc_title.lower():
            start_idx += 1
            continue

        # Global boilerplate line detected across multiple nodes
        if line in boilerplate:
            start_idx += 1
            continue

        break

    # 2. Strip trailing footer lines
    end_idx = len(lines)
    while end_idx > start_idx:
        line = lines[end_idx - 1].strip()
        if not line:
            end_idx -= 1
            continue

        # Border separator line
        if re.match(r'^[\=\-\*_]{4,}\s*$', line) or re.match(r'^\+{4,}\s*$', line):
            end_idx -= 1
            continue

        # Page numbering
        if re.match(r'^(?:page\s+\d+|-\s*\d+\s*-|\[\s*page\s+\d+\s*\])\s*$', line, re.IGNORECASE):
            end_idx -= 1
            continue

        # Repeating copyright notice on every page
        if re.match(r'^(?:\(c\)|copyright|\&copy;)\s+.*$', line, re.IGNORECASE):
            end_idx -= 1
            continue

        # Trailing navigation chrome
        if re.search(r'\[\s*(?:Contents|Index|Help|Prev|Next|Main|Back to top)\s*\]', line, re.IGNORECASE):
            end_idx -= 1
            continue
        if re.search(r'@\{\s*"(?:Contents|Index|Help|Prev|Next|Main|Back to top)"\s+link\s+.*\}', line, re.IGNORECASE):
            end_idx -= 1
            continue

        # Global boilerplate line
        if line in boilerplate:
            end_idx -= 1
            continue

        break

    return '\n'.join(lines[start_idx:end_idx])

def strip_partial_chapter_toc(text: str) -> str:
    """
    Removes per-chapter mini-TOCs (lists of links/sections inside a chapter node),
    while preserving introductory quotes, epigraphs, and prose.
    """
    lines = text.splitlines()
    if len(lines) < 3:
        return text

    # Identify lines that look like TOC entries:
    # 1. Contains a link: @{"..." link ...}
    # 2. Or is an indented section numbering: e.g. "    1.1.1. Title"
    def is_toc_line(line: str) -> bool:
        ls = line.strip()
        if not ls:
            return False
        if LINK_PATTERN.search(line):
            return True
        if re.match(r'^\s+\d+\.\d+(?:\.\d+)*\b', line):
            return True
        return False

    # Scan for a contiguous cluster of TOC lines
    cleaned_lines = []
    i = 0
    while i < len(lines):
        if is_toc_line(lines[i]):
            # Check how many TOC lines follow
            j = i
            link_count = 0
            while j < len(lines):
                if is_toc_line(lines[j]):
                    if LINK_PATTERN.search(lines[j]):
                        link_count += 1
                    j += 1
                elif not lines[j].strip():
                    # Allow blank line inside TOC block
                    j += 1
                else:
                    break

            # If block has at least 2 link lines or 3 outline lines, treat as partial TOC
            if link_count >= 2 or (j - i) >= 4:
                # Skip this sub-TOC block
                i = j
                continue

        cleaned_lines.append(lines[i])
        i += 1

    return '\n'.join(cleaned_lines)

def unwrap_paragraphs(text: str) -> str:
    """
    Unwraps hard-wrapped prose paragraphs while preserving code blocks,
    tables, headings, list items, definition lines, and blockquotes.
    """
    lines = text.splitlines()
    new_lines = []
    current_para = []
    in_code = False

    def flush():
        if not current_para:
            return
        para_str = ""
        for line_str in current_para:
            ls = line_str.strip()
            if not para_str:
                para_str = ls
            else:
                if para_str.endswith('-') and len(para_str) > 1 and para_str[-2].isalpha() and ls and ls[0].islower():
                    para_str += ls
                else:
                    para_str += ' ' + ls
        new_lines.append(para_str)
        current_para.clear()

    definition_prefixes = (
        '**Inputs:**', '**Result:**', '**Prototype:**', '**Purpose:**',
        '**Description:**', '**See also:**', '**Note:**', '**Warning:**',
        '**Example:**', '**Bugs:**', '**Arguments:**'
    )

    for line in lines:
        stripped = line.strip()

        # Code block fence toggle
        if stripped.startswith('```'):
            flush()
            in_code = not in_code
            new_lines.append(line)
            continue

        if in_code:
            new_lines.append(line)
            continue

        # Blank line
        if not stripped:
            flush()
            new_lines.append('')
            continue

        # Markdown headings
        if stripped.startswith('#'):
            flush()
            new_lines.append(line)
            continue

        # Tables (Markdown or HTML)
        if stripped.startswith('|') or stripped.startswith('<table') or stripped.startswith('</table') or stripped.startswith('<tr') or stripped.startswith('<td') or stripped.startswith('<th'):
            flush()
            new_lines.append(line)
            continue

        # Lists (- , * , + , 1. )
        is_list = (re.match(r'^\s*[-*+]\s+', line) or re.match(r'^\s*\d+\.\s+', line))
        if is_list:
            if current_para and re.match(r'^\s*-\s+[a-z]', line):
                current_para.append(line)
                continue
            flush()
            new_lines.append(line)
            continue

        # Blockquotes
        if stripped.startswith('>'):
            flush()
            new_lines.append(line)
            continue

        # Definition headings
        if any(stripped.startswith(dp) for dp in definition_prefixes):
            flush()
            new_lines.append(line)
            continue

        # Item entry lines (e.g. filename/path - description)
        if re.match(r'^\S+\s+-\s+\S+', stripped):
            flush()
            new_lines.append(line)
            continue

        # Normal paragraph line
        current_para.append(line)

    flush()
    return '\n'.join(new_lines)

def convert_tags(text: str, current_file: str, nodes_by_id: Dict[str, GuideNode]) -> str:
    """Translates inline styling tags and hypertext links."""
    def resolve_target_node(target: str, label: str) -> Optional[GuideNode]:
        target_lower = target.lower().strip()
        label_lower = label.lower().strip()

        if target_lower in nodes_by_id:
            return nodes_by_id[target_lower]
        if label_lower in nodes_by_id:
            return nodes_by_id[label_lower]

        for node in nodes_by_id.values():
            if target_lower in (node.clean_name.lower(), node.base_title.lower()):
                return node
            if label_lower in (node.clean_name.lower(), node.base_title.lower()):
                return node

        candidates = [node for node in nodes_by_id.values() if target_lower.startswith(node.node_id.lower()) and len(node.node_id) >= 3]
        if len(candidates) == 1:
            return candidates[0]

        return None

    def replace_link(m):
        label = m.group(1).replace(r'\"', '"').strip()
        target = m.group(2).strip().strip('"\'')
        target_lower = target.lower()

        # Check for image or external media file
        image_exts = ('.iff', '.ilbm', '.gif', '.png', '.jpg', '.jpeg', '.bmp', '.svg')
        if any(target_lower.endswith(ext) for ext in image_exts):
            if target_lower.endswith('.iff') or target_lower.endswith('.ilbm'):
                img_name = Path(target).with_suffix('.png').name
            else:
                img_name = Path(target).name
            return f"![{label}](assets/{img_name})"

        node = resolve_target_node(target, label)
        if node:
            # If target node was excluded (e.g. Index), redirect gracefully to Table of Contents
            if node.is_excluded():
                return f"[{label}](00%20-%20Table%20of%20Contents.md)"

            anchor = obsidian_encode_anchor(node.heading_title)
            if node.target_file == current_file:
                return f"[{label}](#{anchor})"
            else:
                enc_file = urllib.parse.quote(node.target_file)
                if node.is_file_root:
                    return f"[{label}]({enc_file})"
                return f"[{label}]({enc_file}#{anchor})"

        # Fallback to Table of Contents
        return f"[{label}](00%20-%20Table%20of%20Contents.md)"

    # Convert standalone underline / bold lines to Markdown subheadings (###)
    converted = re.sub(
        r'^(?:@\{[uUbB]\}|<u>|\*\*)\s*([^@<\n\r]{2,100}?)\s*(?:@\{(?:uu|UU|ub|UB)\}|</u>|\*\*)\s*$',
        r'### \1',
        text,
        flags=re.MULTILINE
    )

    converted = LINK_PATTERN.sub(replace_link, converted)

    # Font styling with strict non-capturing groups to avoid stray trailing braces
    converted = re.sub(r'@\{[bB]\}', '**', converted)
    converted = re.sub(r'@\{(?:ub|UB)\}', '**', converted)
    converted = re.sub(r'@\{[iI]\}', '*', converted)
    converted = re.sub(r'@\{(?:ui|UI)\}', '*', converted)
    converted = re.sub(r'@\{[uU]\}', '<u>', converted)
    converted = re.sub(r'@\{(?:uu|UU)\}', '</u>', converted)

    # Strip layout, color, and formatting tags
    converted = re.sub(r'@\{(?:jleft|jcenter|jright|code|plain|line|fg[^\}]*|bg[^\}]*)\}', '', converted, flags=re.IGNORECASE)

    # Convert bullet lines (* or **) to -
    converted = re.sub(r'^(\s*)\*\s+', r'\1- ', converted, flags=re.MULTILINE)

    # Escapes
    converted = converted.replace(r'\@', '@')
    converted = converted.replace(r'\"', '"')

    return converted

def convert_single_mode(doc: GuideDocument, out_file: Path, unwrap: bool) -> None:
    """Converts entire AmigaGuide into a single Markdown file with section navigation."""
    doc_title = doc.metadata.get('DATABASE') or out_file.stem
    lines = [f"# {doc_title}\n"]

    # Add metadata callout
    meta_items = []
    if 'AUTHOR' in doc.metadata:
        meta_items.append(f"**Author:** {doc.metadata['AUTHOR']}")
    if '$VER:' in doc.metadata:
        meta_items.append(f"**Version:** {doc.metadata['$VER:']}")
    if 'COPYRIGHT' in doc.metadata:
        meta_items.append(f"**Copyright:** {doc.metadata['COPYRIGHT']}")

    if meta_items:
        lines.append("> [!NOTE]")
        for m in meta_items:
            lines.append(f"> {m}")
        lines.append("")

    toc_anchor = obsidian_encode_anchor("Table of Contents")
    lines.append(f"## Table of Contents\n")

    # Filter excluded nodes (Index, List of Tables, List of Figures)
    active_nodes = [n for n in doc.nodes if not n.is_excluded()]

    for n in active_nodes:
        n.target_file = out_file.name
        n.heading_level = 2
        n.is_file_root = False
        anchor = obsidian_encode_anchor(n.heading_title)
        lines.append(f"- [{n.heading_title}](#{anchor})")

    # Append sections with bidirectional navigation
    num_nodes = len(active_nodes)
    for idx, node in enumerate(active_nodes):
        # Section navigation links
        if idx == 0:
            prev_link = f"[⬅ Previous](#{toc_anchor})"
        else:
            prev_anchor = obsidian_encode_anchor(active_nodes[idx - 1].heading_title)
            prev_link = f"[⬅ Previous](#{prev_anchor})"

        toc_link = f"[📋 Table of Contents](#{toc_anchor})"

        if idx == num_nodes - 1:
            next_link = f"[Next ➡](#{toc_anchor})"
        else:
            next_anchor = obsidian_encode_anchor(active_nodes[idx + 1].heading_title)
            next_link = f"[Next ➡](#{next_anchor})"

        nav_bar = f"{prev_link} | {toc_link} | {next_link}"

        # Clean content
        cleaned_content = strip_headers_and_footers(node.content, doc_title, doc.repeated_boilerplate_lines)
        cleaned_content = strip_partial_chapter_toc(cleaned_content)
        body = convert_tags(cleaned_content, current_file=out_file.name, nodes_by_id=doc.nodes_by_id)
        if unwrap:
            body = unwrap_paragraphs(body)

        lines.append(f"\n\n---\n\n## {node.heading_title}\n")
        lines.append(f"{nav_bar}\n\n---\n")
        lines.append(body.strip())
        lines.append(f"\n\n---\n\n{nav_bar}")

    full_md = '\n'.join(lines) + '\n'
    out_file.parent.mkdir(parents=True, exist_ok=True)
    out_file.write_text(full_md, encoding="utf-8")
    print(f"Generated single-file guide: {out_file} ({len(active_nodes)} sections, {len(doc.nodes) - len(active_nodes)} excluded)")

def convert_split_mode(doc: GuideDocument, out_dir: Path, unwrap: bool) -> None:
    """Splits an AmigaGuide into numbered chapter files and a Table of Contents with bidirectional navigation."""
    out_dir.mkdir(parents=True, exist_ok=True)
    doc_title = doc.metadata.get('DATABASE') or out_dir.name

    # Determine TOC node
    toc_node = doc.nodes_by_id.get('main') or doc.nodes[0]
    toc_node.target_file = "00 - Table of Contents.md"
    toc_node.heading_title = "Table of Contents"
    toc_node.is_file_root = True

    # Filter out excluded nodes (Index, List of Tables/Images) and TOC node
    content_nodes = [n for n in doc.nodes if n != toc_node and not n.is_excluded()]

    # Map content nodes to numbered files
    for idx, node in enumerate(content_nodes, start=1):
        filename = f"{idx:02d} - {node.clean_name}.md"
        node.target_file = filename
        node.is_file_root = True
        node.heading_level = 1

    enc_toc = urllib.parse.quote(toc_node.target_file)
    first_chapter_file = urllib.parse.quote(content_nodes[0].target_file) if content_nodes else enc_toc

    # 1. Write Table of Contents
    toc_lines = [f"# {doc_title}: Table of Contents\n"]

    meta_items = []
    if 'AUTHOR' in doc.metadata:
        meta_items.append(f"**Author:** {doc.metadata['AUTHOR']}")
    if '$VER:' in doc.metadata:
        meta_items.append(f"**Version:** {doc.metadata['$VER:']}")
    if 'COPYRIGHT' in doc.metadata:
        meta_items.append(f"**Copyright:** {doc.metadata['COPYRIGHT']}")

    if meta_items:
        toc_lines.append("> [!NOTE]")
        for m in meta_items:
            toc_lines.append(f"> {m}")
        toc_lines.append("")

    if content_nodes:
        toc_lines.append(f"[Next ➡]({first_chapter_file})\n\n---\n")

    toc_lines.append("## Navigation\n")
    for n in content_nodes:
        enc_target = urllib.parse.quote(n.target_file)
        toc_lines.append(f"- [{n.heading_title}]({enc_target})")

    # If TOC node had introductory prose, append it
    cleaned_toc_content = strip_headers_and_footers(toc_node.content, doc_title, doc.repeated_boilerplate_lines)
    cleaned_toc_content = strip_partial_chapter_toc(cleaned_toc_content)
    toc_body = convert_tags(cleaned_toc_content, current_file=toc_node.target_file, nodes_by_id=doc.nodes_by_id)
    if unwrap:
        toc_body = unwrap_paragraphs(toc_body)
    if toc_body.strip():
        toc_lines.append(f"\n\n---\n\n{toc_body.strip()}")

    if content_nodes:
        toc_lines.append(f"\n\n---\n\n[Next ➡]({first_chapter_file})")

    (out_dir / toc_node.target_file).write_text('\n'.join(toc_lines) + '\n', encoding="utf-8")

    # 2. Write individual chapter files with bidirectional navigation (begin & end)
    num_chapters = len(content_nodes)
    for idx, node in enumerate(content_nodes):
        # Determine navigation links
        if idx == 0:
            prev_link = f"[⬅ Previous]({enc_toc})"
        else:
            prev_node = content_nodes[idx - 1]
            enc_prev = urllib.parse.quote(prev_node.target_file)
            prev_link = f"[⬅ Previous]({enc_prev})"

        toc_link = f"[📋 Table of Contents]({enc_toc})"

        if idx == num_chapters - 1:
            next_link = f"[Next ➡]({enc_toc})"
        else:
            next_node = content_nodes[idx + 1]
            enc_next = urllib.parse.quote(next_node.target_file)
            next_link = f"[Next ➡]({enc_next})"

        nav_bar = f"{prev_link} | {toc_link} | {next_link}"

        # Clean content
        cleaned_content = strip_headers_and_footers(node.content, doc_title, doc.repeated_boilerplate_lines)
        cleaned_content = strip_partial_chapter_toc(cleaned_content)
        body = convert_tags(cleaned_content, current_file=node.target_file, nodes_by_id=doc.nodes_by_id)
        if unwrap:
            body = unwrap_paragraphs(body)

        ch_lines = [
            f"# {node.heading_title}\n",
            f"{nav_bar}\n\n---\n",
            body.strip(),
            f"\n\n---\n\n{nav_bar}\n"
        ]

        file_path = out_dir / node.target_file
        file_path.write_text('\n'.join(ch_lines), encoding="utf-8")

    num_excluded = len([n for n in doc.nodes if n != toc_node and n.is_excluded()])
    print(f"Generated split guide: {out_dir} (1 TOC + {num_chapters} chapters, {num_excluded} excluded)")

def main():
    parser = argparse.ArgumentParser(description="Convert AmigaGuide (.guide) to modern Markdown (.md).")
    parser.add_argument("input", type=str, help="Path to input .guide file")
    parser.add_argument("--output-dir", "-o", type=str, required=True, help="Output directory")
    parser.add_argument("--mode", choices=["split", "single"], default="split", help="Output mode (split chapters or single document)")
    parser.add_argument("--unwrap", action="store_true", default=True, help="Unwrap hard-wrapped prose paragraphs (default: True)")
    parser.add_argument("--no-unwrap", action="store_false", dest="unwrap", help="Disable paragraph unwrapping")
    args = parser.parse_args()

    input_path = Path(args.input)
    if not input_path.exists():
        print(f"Error: Input file '{input_path}' not found.")
        sys.exit(1)

    try:
        raw_text = input_path.read_text(encoding="latin-1")
    except Exception:
        raw_text = input_path.read_text(encoding="utf-8", errors="replace")

    doc = GuideDocument(raw_text)
    out_dir = Path(args.output_dir)

    if args.mode == "single":
        out_file = out_dir / f"{input_path.stem}.md"
        convert_single_mode(doc, out_file, unwrap=args.unwrap)
    else:
        convert_split_mode(doc, out_dir, unwrap=args.unwrap)

if __name__ == "__main__":
    main()
