# Obsidian Markdown Linking Mechanics & Heading Anchor Quirks

When converting documents to Markdown for Obsidian, standard GitHub/CommonMark slugification algorithms (such as lowercase kebab-case `#heading-title-with-dashes`) **fail completely** in Obsidian.

This document details the exact, reverse-engineered link resolution algorithms used by the Obsidian desktop engine, explains why percent-encoded punctuation like `%2C` breaks navigation, and provides the canonical rules for generating links that work 100% reliably in Obsidian.

---

## 1. How Obsidian Resolves Heading Anchors

Unlike GitHub or static site generators that convert `## 2.1.3. System Addresses!` into `#213-system-addresses`, Obsidian resolves heading anchors against the **exact plain-text title of the heading**, normalized through an internal stripping function:

```javascript
// Obsidian Core Internal Functions:
var SD = /[!"#$%&()*+,.:;<=>?@^`{|}~\/\[\]\\\r\n]/g;

function TD(e) {
    return e.replace(SD, " ").replace(/\s+/g, " ").trim().toLowerCase();
}
```

When a user clicks an internal link `[Link Text](file.md#target_anchor)`, Obsidian:
1. Opens `file.md`.
2. Extracts the subpath anchor `target_anchor`.
3. Runs JavaScript's standard `decodeURI(target_anchor)`.
4. Compares `TD(decodeURI(target_anchor))` against `TD(heading.heading)` for every heading in the file.
5. Jumps to the first heading where `TD(heading) === TD(decoded_anchor)`.

---

## 2. The `decodeURI` Trap: Why `%2C` and `%3A` Fail

In JavaScript:
- `encodeURI()` encodes spaces as `%20`, but leaves URI-reserved characters like `,`, `:`, `/`, `?`, `@`, `&`, `=`, `+`, `$`, `#` **as literal characters**.
- `decodeURI()` **only reverses percent sequences that encode non-reserved characters**. It intentionally **does NOT decode** `%2C` (comma), `%3A` (colon), `%2F` (slash), etc. Only `decodeURIComponent()` would decode them, but Obsidian calls `decodeURI()` on the target URI!

### What happens when an anchor contains `%2C`?
Suppose the heading is:
```markdown
### 2.1.3. System addresses, jumps into ROM, and private data structures
```

1. Heading stripped:
   `TD("2.1.3. System addresses, jumps into ROM, and private data structures")`
   $\rightarrow$ `"2 1 3 system addresses jumps into rom and private data structures"`
2. If link anchor is URL-encoded with standard Python `urllib.parse.quote()`:
   `#2.1.3.%20System%20addresses%2C%20jumps%20into%20ROM%2C%20and%20private%20data%20structures`
3. Obsidian executes `decodeURI()`:
   `%20` becomes a space `" "`.
   `%2C` **remains literal `"%2C"`**!
4. Obsidian executes `TD("...addresses%2C jumps...")`:
   The `%` character is matched by `SD` and replaced with space:
   $\rightarrow$ `"...addresses 2c jumps..."`
5. **Comparison:** `"addresses 2c jumps"` $\neq$ `"addresses jumps"` $\implies$ **LOOKUP FAILS!** Obsidian scrolls to the top of the file instead of jumping to the heading.

---

## 3. The Obsidian Anchor Encoding Rules

To ensure 100% reliable heading jumps in Obsidian:

### Rule 1: Use Literal Punctuation
Do **NOT** percent-encode punctuation characters in the anchor. Leave punctuation literal:
- Comma: `,`
- Colon: `:`
- Period: `.`
- Question mark: `?`
- Hyphen: `-`
- Slash: `/`
- Parentheses: `(` and `)` (or `%28` / `%29` — both resolve cleanly in `TD()`)

### Rule 2: Percent-Encode Spaces
Spaces must be encoded as `%20` (in standard Markdown links) so Markdown parsers recognize the entire string as a single URI:
```markdown
[cf. Section 2.1.3](02%20-%20Programming%20Guidelines.md#2.1.3.%20System%20addresses,%20jumps%20into%20ROM,%20and%20private%20data%20structures)
```

### Rule 3: Same-Document Links Must Omit Filename
If the link targets a heading in the **same document**, you **must omit the filename completely**:
- ✅ **CORRECT (works in Obsidian):**
  ```markdown
  [Back to Overview](#1.1.%20Data%20types%20of%20the%20M68000%20family)
  ```
- ❌ **INCORRECT (breaks in Obsidian):**
  ```markdown
  [Back to Overview](01%20-%20Data%20Types.md#1.1.%20Data%20types%20of%20the%20M68000%20family)
  ```
  *(If the current document is `01 - Data Types.md`, including its filename causes Obsidian to reload or ignore the anchor jump).*

### Rule 4: Cross-Document Links Must Encode Filename
When linking across files, URL-encode spaces in the relative filename path:
```markdown
[See DOS Functions](17%20-%20Dos%20Functions.md#17.1.71.%20InternalLoadSeg%28%29%20%282.0%29)
```

---

## 4. Python Implementation of Obsidian-Safe Link Generation

```python
import urllib.parse
import re

def obsidian_encode_anchor(heading_title: str) -> str:
    """
    Encodes a heading string for use as an Obsidian markdown anchor.
    Encodes spaces as %20 while preserving literal punctuation characters
    that JavaScript's decodeURI() would otherwise fail to decode.
    """
    # Safe chars: alphanumeric, spaces, and punctuation allowed in URIs
    # encode spaces to %20, keep punctuation literal
    # safe=";,/?:@&=+$-_.!~*'()#"
    return urllib.parse.quote(heading_title, safe=";,/?:@&=+$-_.!~*'()#")

def build_markdown_link(label: str, target_file: str, current_file: str, heading_title: str = None) -> str:
    anchor_part = ""
    if heading_title:
        anchor_part = f"#{obsidian_encode_anchor(heading_title)}"
        
    if target_file == current_file:
        # Same-document link: omit filename
        return f"[{label}]({anchor_part})"
    else:
        enc_file = urllib.parse.quote(target_file)
        return f"[{label}]({enc_file}{anchor_part})"
```
