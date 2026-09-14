# Non-Text & Formatted Content Conversion Hierarchy

This reference outlines the decision ladder for representing complex figures, circuit diagrams, and data tables converted from HTML into Markdown.

---

## 1. Decision Matrix

| Source HTML Element | Characteristics | Target Markdown Representation |
| :--- | :--- | :--- |
| `<pre>`, `<code>`, monospace `<p>` | M68000 assembly instructions, memory addresses, register traces | Fenced Code Block: ```` ```assembly ```` |
| Monospace `<pre>` / `<p>` | Raw hex dumps, CLI sessions, binary packet layouts | Fenced Code Block: ```` ```text ```` |
| `<table>` with >1 row/col | Pinouts, register bits, instruction timing, cycle breakdowns | GFM Table: `\| Col 1 \| Col 2 \|` |
| `<table>` with 1 cell | Centering container for image, caption, or single note | Unwrapped content (strip table wrapper) |
| Flowchart or pipeline image (`<img>`) | Conceptual data flow between internal registers (e.g. IRC -> IR -> IRD) | Mermaid flowchart (`flowchart TD`) + ASCII fallback |
| Circuit / timing schematic image (`<img>`) | Complex hardware schematic or oscillogram | Embedded image (`![caption](path)`) + Git-tracked `<path>.txt` sidecar |

---

## 2. Table Conversion Guidelines

1. **Header Row Identification:**
   - If the table contains `<th>` tags, use them as the header.
   - If the first row contains `<b>` or styled cells, treat row 1 as the header.
   - If no header exists, generate descriptive headers (e.g., `| Step | Action | Description |`).
2. **Column Alignment:**
   - Left-align text columns (`:---`).
   - Right-align or center numerical and cycle counts (`---:` or `:---:`).
3. **Multiline Cell Unwrapping:**
   - Convert internal line breaks inside table cells to `<br/>` or unwrap onto a single line.

---

## 3. Diagram & Visual Architecture Representation

For architecture flow diagrams (like the 68000 prefetch queue):
- **Mermaid Block:** Provides interactive, responsive rendering in Obsidian and GitHub.
- **ASCII Art Fallback:** Enclosed in a `<details>` block for offline terminals or plain text viewers.
- **Asset Description Sidecar:** A mandatory `.txt` sidecar file describing register operations, clock phases, and signal lines per `.agents/rules/asset-descriptions.md`.
