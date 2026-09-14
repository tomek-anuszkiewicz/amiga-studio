# HTML Sanitization & Normalization Heuristics

This reference details the heuristics used by `convert_html.py` to sanitize legacy web exports, Microsoft Word HTML, and vintage retrocomputing documentation.

---

## 1. Character Encodings & Typographic Entities

Many vintage technical articles were exported from Microsoft Word 97/2000 or authored on Windows systems using the `windows-1252` code page.

| Raw Byte / Entity | Windows-1252 Character | Standard Unicode / GFM Replacement |
| :--- | :--- | :--- |
| `0x91` / `&lsquo;` | `‘` (Left single quotation mark) | `'` or `‘` |
| `0x92` / `&rsquo;` | `’` (Right single quotation mark / apostrophe) | `'` or `’` |
| `0x93` / `&ldquo;` | `“` (Left double quotation mark) | `"` |
| `0x94` / `&rdquo;` | `”` (Right double quotation mark) | `"` |
| `0x96` / `&ndash;` | `–` (En dash) | `–` or `--` |
| `0x97` / `&mdash;` | `—` (Em dash) | `—` or `---` |
| `0xA0` / `&nbsp;`  | Non-breaking space | Standard space ` ` |
| `0x85` / `&hellip;`| `…` (Horizontal ellipsis) | `...` |

When parsing with BeautifulSoup, always decode using the declared charset (`windows-1252` if indicated in `<meta>` or auto-detected), replacing invalid byte sequences gracefully.

---

## 2. Microsoft Word (MSO) XML & Style Cruft

Word HTML exports embed extensive proprietary styling and XML islands:
- `<html xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office" ...>`
- `<head>` contains `<link rel=File-List>`, `<meta name=ProgId content=Word.Document>`, `<o:DocumentProperties>` XML islands, and massive `<style>` sheets with `@font-face` and `p.MsoNormal`.
- `<!--[if !mso]>` and `<!--[if gte mso 9]>` conditional comments.
- Inline styles: `style="mso-bidi-font-size:12.0pt;font-family:Courier New"`.

### Sanitization Action:
1. Strip all `<xml>`, `<style>`, `<link>`, and conditional comment blocks.
2. Strip attributes matching `mso-*`, `v:*`, `o:*`, `w:*`.
3. Drop purely stylistic wrapper `<span>` elements while preserving inner text.
4. Normalize `p.MsoNormal` to regular Markdown paragraphs.

---

## 3. Heading Hierarchy Recovery

In vintage Word HTML:
- Document titles are frequently tagged with `<h2>` or `<p class="MsoNormal"><b><span style="font-size:18pt">...</span></b></p>`.
- Sections may be tagged with `<h1>` despite being subordinate to the title, or formatted as paragraphs with all-caps text (`READ MODIFY INSTRUCTIONS`).

### Normalization Strategy:
1. Identify the single top-level Document Title and emit as `# Title`.
2. Normalize subsequent major sections to `## Section Title`.
3. Normalize subsections to `### Subsection Title` and `#### Sub-subsection Title`.
4. Ensure every heading generates a standard GitHub/Obsidian compatible slug for anchor linking (`#heading-name`).

---

## 4. Assembly & Code Block Identification

Technical 68000 documentation contains instruction sequences, register traces, and code snippets that are often formatted as simple paragraphs or `<pre>` blocks.

### Detection Heuristics:
A block should be wrapped into ```` ```assembly ```` if:
1. Its text contains valid M68000 mnemonics: `NOP`, `MOVE`, `MOVE.L`, `MOVE.W`, `MOVE.B`, `MOVEQ`, `LEA`, `ADD`, `ADDQ`, `SUB`, `SUBQ`, `AND`, `OR`, `EOR`, `BSR`, `JSR`, `RTS`, `BRA`, `BEQ`, `BNE`, `PEA`, `LINK`, `UNLK`, `TAS`, `MOVEM`, etc.
2. Lines begin with hexadecimal address prefixes: `$001000:`, `001000:`, `$FF8000:`.
3. Lines contain register operands: `D0-D7`, `A0-A7`, `SP`, `PC`, `SR`, `CCR`.
4. Indented lines following a label or semicolon comment (`; comment`).
