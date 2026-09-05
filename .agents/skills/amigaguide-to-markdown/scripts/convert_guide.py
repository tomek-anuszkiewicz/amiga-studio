#!/usr/bin/env python3
"""
convert_guide.py - Comprehensive AmigaGuide to Markdown Converter

Converts Commodore AmigaGuide (.guide) files into modern, clean Markdown (.md)
optimized for Obsidian and GitHub. Supports both single-document consolidation
and multi-chapter file splitting, with full Obsidian TD() anchor compatibility.
"""

import re
import sys
import argparse
import urllib.parse
from pathlib import Path
from typing import Dict, List, Optional, Tuple

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
    # Safe chars include reserved punctuation characters that decodeURI will not unescape
    return urllib.parse.quote(heading_title, safe=";,/?:@&=+$-_.!~*'()#")

class GuideNode:
    def __init__(self, node_id: str, raw_title: str, content: str, index: int):
        self.node_id = node_id
        self.raw_title = raw_title.replace(r'\"', '"') if raw_title else node_id
        self.content = content
        self.index = index
        self.clean_name = clean_title(self.raw_title)
        
        # Heading level & numbers detection
        self.heading_title = self.raw_title
        # Detect if title starts with number like "1: ..." or "1.1: ..."
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

class GuideDocument:
    def __init__(self, raw_text: str):
        self.raw_text = raw_text
        self.metadata = {}
        self.nodes: List[GuideNode] = []
        self.nodes_by_id: Dict[str, GuideNode] = {}
        self._parse()

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
            
            # Find end of this node (either @ENDNODE or start of next @NODE)
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
                # Check if hyphenated word at line end
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

        # Tables
        if stripped.startswith('|'):
            flush()
            new_lines.append(line)
            continue

        # Lists (- , * , + , 1. )
        is_list = (re.match(r'^\s*[-*+]\s+', line) or re.match(r'^\s*\d+\.\s+', line))
        if is_list:
            # Check for em-dash within an ongoing prose sentence
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

        # Normal paragraph line
        current_para.append(line)

    flush()
    return '\n'.join(new_lines)

def convert_tags(text: str, current_file: str, nodes_by_id: Dict[str, GuideNode]) -> str:
    """Translates inline styling tags and hypertext links."""
        # 1. Hypertext links: @{"label" link target [offset]}
    def resolve_target_node(target: str, label: str) -> Optional[GuideNode]:
        target_lower = target.lower().strip()
        label_lower = label.lower().strip()

        # Direct node ID match
        if target_lower in nodes_by_id:
            return nodes_by_id[target_lower]

        # Match by label as node ID
        if label_lower in nodes_by_id:
            return nodes_by_id[label_lower]

        # Match by node clean_name or base_title
        for node in nodes_by_id.values():
            if target_lower in (node.clean_name.lower(), node.base_title.lower()):
                return node
            if label_lower in (node.clean_name.lower(), node.base_title.lower()):
                return node

        # Fuzzy prefix match (e.g. 'muls' -> 'mul', 'mulu' -> 'mul')
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
            anchor = obsidian_encode_anchor(node.heading_title)
            if node.target_file == current_file:
                # Same-document link: always use exact heading anchor
                return f"[{label}](#{anchor})"
            else:
                # Cross-document link
                enc_file = urllib.parse.quote(node.target_file)
                if node.is_file_root:
                    return f"[{label}]({enc_file})"
                return f"[{label}]({enc_file}#{anchor})"

        # If unresolved node: link to Table of Contents as fallback
        return f"[{label}](00%20-%20Table%20of%20Contents.md)"

    converted = LINK_PATTERN.sub(replace_link, text)

    # 2. Font styling
    converted = re.sub(r'@\{[bB]\}', '**', converted)
    converted = re.sub(r'@\{ub|UB\}', '**', converted)
    converted = re.sub(r'@\{[iI]\}', '*', converted)
    converted = re.sub(r'@\{ui|UI\}', '*', converted)
    converted = re.sub(r'@\{[uU]\}', '<u>', converted)
    converted = re.sub(r'@\{uu|UU\}', '</u>', converted)
    
    # 3. Strip layout and color tags
    converted = re.sub(r'@\{(?:jleft|jcenter|jright|code|plain|fg[^\}]*|bg[^\}]*)\}', '', converted, flags=re.IGNORECASE)

    # 4. Convert bullet lines (* or **) to -
    converted = re.sub(r'^(\s*)\*\s+', r'\1- ', converted, flags=re.MULTILINE)

    # 5. Escapes
    converted = converted.replace(r'\@', '@')
    converted = converted.replace(r'\"', '"')

    return converted

def convert_single_mode(doc: GuideDocument, out_file: Path, unwrap: bool) -> None:
    """Converts entire AmigaGuide into a single Markdown file."""
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

    # Assign single target file
    for n in doc.nodes:
        n.target_file = out_file.name
        n.heading_level = 2
        n.is_file_root = False

    # Find TOC or Main node
    toc_node = doc.nodes_by_id.get('main') or (doc.nodes[0] if doc.nodes else None)

    # Append nodes
    for idx, node in enumerate(doc.nodes):
        node_body = convert_tags(node.content, current_file=out_file.name, nodes_by_id=doc.nodes_by_id)
        if unwrap:
            node_body = unwrap_paragraphs(node_body)

        lines.append(f"\n\n---\n\n## {node.heading_title}\n")
        lines.append(node_body.strip())

    full_md = '\n'.join(lines) + '\n'
    out_file.parent.mkdir(parents=True, exist_ok=True)
    out_file.write_text(full_md, encoding="utf-8")
    print(f"Generated single-file guide: {out_file} ({len(doc.nodes)} nodes)")

def convert_split_mode(doc: GuideDocument, out_dir: Path, unwrap: bool) -> None:
    """Splits an AmigaGuide into numbered chapter/section files and a Table of Contents."""
    out_dir.mkdir(parents=True, exist_ok=True)
    doc_title = doc.metadata.get('DATABASE') or out_dir.name

    # Determine file mapping
    # Node 'MAIN' or index 0 is TOC
    toc_node = doc.nodes_by_id.get('main') or doc.nodes[0]
    toc_node.target_file = "00 - Table of Contents.md"
    toc_node.heading_title = "Table of Contents"
    toc_node.is_file_root = True

    content_nodes = [n for n in doc.nodes if n != toc_node]

    # Map content nodes to files
    for idx, node in enumerate(content_nodes, start=1):
        filename = f"{idx:02d} - {node.clean_name}.md"
        node.target_file = filename
        node.is_file_root = True
        node.heading_level = 1

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

    toc_lines.append("## Navigation\n")
    for n in content_nodes:
        enc_target = urllib.parse.quote(n.target_file)
        toc_lines.append(f"- [{n.heading_title}]({enc_target})")

    # If TOC node had introductory prose, append it
    toc_body = convert_tags(toc_node.content, current_file=toc_node.target_file, nodes_by_id=doc.nodes_by_id)
    if unwrap:
        toc_body = unwrap_paragraphs(toc_body)
    if toc_body.strip():
        toc_lines.append(f"\n\n---\n\n{toc_body.strip()}")

    (out_dir / toc_node.target_file).write_text('\n'.join(toc_lines) + '\n', encoding="utf-8")

    # 2. Write individual chapter files
    for node in content_nodes:
        ch_lines = [f"# {node.heading_title}\n"]
        body = convert_tags(node.content, current_file=node.target_file, nodes_by_id=doc.nodes_by_id)
        if unwrap:
            body = unwrap_paragraphs(body)
        ch_lines.append(body.strip())
        
        # Navigation footer
        enc_toc = urllib.parse.quote(toc_node.target_file)
        ch_lines.append(f"\n\n---\n\n[⬅ Return to Table of Contents]({enc_toc})\n")

        file_path = out_dir / node.target_file
        file_path.write_text('\n'.join(ch_lines), encoding="utf-8")

    print(f"Generated split guide: {out_dir} (1 TOC + {len(content_nodes)} chapters)")

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
    except Exception as e:
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
