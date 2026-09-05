# AmigaGuide Specification & Syntax Reference

AmigaGuide is the native hypertext document format introduced in Commodore AmigaOS 2.0/3.0. Documents are plain text files (traditionally encoded in ISO-8859-1 / Latin-1 or Topaz font ASCII) containing markup commands prefixed with `@`.

---

## 1. Global Document Commands

Global commands typically appear at the beginning of the file before the first `@NODE` declaration.

| Command | Syntax | Description |
|---|---|---|
| `@DATABASE` | `@DATABASE <name>` | Sets the name/identifier of the hypertext database. |
| `@$VER:` | `@$VER: <name> <version>.<revision> (<date>)` | Standard Amiga version string (e.g. `@$VER: Kickstart 1.0 (15.02.94)`). |
| `@AUTHOR` | `@AUTHOR "<author_name>"` | Identifies the document's author. |
| `@COPYRIGHT` | `@COPYRIGHT "<notice>"` | Copyright notice for the document. |
| `@(DIR)` | `@(DIR "<path>")` | Default directory path for external links and files. |
| `@WORDWRAP` | `@WORDWRAP` | Enables automatic word-wrapping by default for all nodes in the guide. |
| `@SMARTWRAP` | `@SMARTWRAP` | Enables smart word-wrapping (collapsing superfluous whitespace). |
| `@TOC` | `@TOC <node_name>` | Specifies the default Table of Contents node (usually `MAIN`). |
| `@INDEX` | `@INDEX <node_name>` | Specifies the default Index node. |
| `@HELP` | `@HELP <node_name>` | Specifies the default Help node when user presses Help key. |
| `@FONT` | `@FONT <name> <size>` | Sets the default font (e.g. `@FONT topaz.font 8`). |

---

## 2. Node Commands & Hierarchy

Every document is composed of one or more **nodes**. Each node is a standalone page or section.

### Defining a Node
```text
@NODE <node_id> ["<window_title>"]
... text content ...
@ENDNODE
```
- `<node_id>`: A unique alphanumeric identifier for the node (case-insensitive in most Amiga viewers, but conventionally matched exactly). Standard convention uses `MAIN` for the entry point.
- `"<window_title>"`: An optional human-readable title displayed in the AmigaGuide viewer window's title bar.

### Node Navigation Overrides
Within a node, you may override standard navigation paths:
- `@PREV <node_id>`: Overrides the node visited when clicking the "Prev" button.
- `@NEXT <node_id>`: Overrides the node visited when clicking the "Next" button.
- `@TOC <node_id>`: Overrides the Table of Contents target for this specific node.
- `@HELP <node_id>`: Overrides the Help target for this specific node.
- `@TITLE "<string>"`: Alternative way to define the node title.

---

## 3. Hypertext Links

Hypertext links allow the user to click a text button or highlighted string to jump to another location.

### Basic Link Syntax
```text
@{"<link_text>" link <node_target> [<line_offset>]}
```
- `"<link_text>"`: The visible label presented to the user (e.g. `[Contents]`, `[Chapter 1]`).
- `<node_target>`: Target location:
  - Local node in the same file: `@{"Chapter 1" link CH01}`
  - Node in an external guide: `@{"DOS Manual" link "SYS:Locale/Help/dos.guide/MAIN"}`
  - External file or image: `@{"Architecture Diagram" link AmigaGuruBook.iff}`
- `[<line_offset>]`: Optional integer line number to scroll to within the target node.

### System & Script Execution Links
- `@{"<label>" system "<command>"}`: Runs an AmigaDOS CLI command when clicked.
- `@{"<label>" rxs "<script>"}`: Executes an ARexx script command.
- `@{"<label>" rx "<script_file>"}`: Executes an external ARexx script file.

---

## 4. In-Line Text Formatting (`@{...}`)

Inline formatting commands are delimited by `@\{` and `\}`.

### Font Styling
| Tag | Description | Markdown Equivalent |
|---|---|---|
| `@{B}` | Bold On | `**` |
| `@{UB}` | Bold Off | `**` |
| `@{I}` | Italic On | `*` |
| `@{UI}` | Italic Off | `*` |
| `@{U}` | Underline On | `<u>` (or stripped in table headers) |
| `@{UU}` | Underline Off | `</u>` |
| `@{CODE}` | Fixed-width / Monospace On | Inline code (`` ` ``) or code fence |
| `@{PLAIN}` | Reset all styles to normal | Closes active formatting |

### Justification & Alignment
| Tag | Description | Conversion Recommendation |
|---|---|---|
| `@{JLEFT}` | Left-justify following text | Standard paragraph / default |
| `@{JCENTER}` | Center-align following text | Centered HTML `<div align="center">` or plain text |
| `@{JRIGHT}` | Right-align following text | Right HTML `<div align="right">` or plain text |

### Color Control
| Tag | Description |
|---|---|
| `@{FG <pen>}` | Sets foreground text color (pen 0..7 or named like `text`, `shine`, `shadow`). |
| `@{BG <pen>}` | Sets background color. |

*(In Markdown conversion, color tags are typically stripped as Markdown themes govern presentation).*

---

## 5. Escaping & Special Characters

- `\@`: Literal `@` character (prevents the parser from interpreting it as a command).
- `\"`: Literal double-quote inside quoted arguments (e.g. `System \"constants\"`).
- `\\`: Literal backslash.
