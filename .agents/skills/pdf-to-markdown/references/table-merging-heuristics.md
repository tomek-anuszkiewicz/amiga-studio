# Multi-Page Table Merging Heuristics

When PDF documents are transcribed page-by-page, large tables spanning multiple pages are split into separate fragments. This document describes the heuristic algorithm and rules for stitching them into a single, cohesive Markdown table.

---

## 1. Table Continuation Detection Heuristic

Two Markdown tables (Table $A$ at the end of Page $N$, and Table $B$ at the beginning of Page $N+1$) are considered parts of the same table if:

1. **Context Adjacency**: There is no intervening chapter heading (`#`, `##`) between Table $A$ and Table $B$.
2. **Column Count Identity**: Table $A$ and Table $B$ have the exact same number of columns:
   $$\text{len}(\text{cols}_A) == \text{len}(\text{cols}_B)$$
3. **Header Match (Pattern 1)**: Table $B$ repeats the exact header row of Table $A$ (or an abbreviated variant like "Table 6-1 (Continued)").
4. **Headerless Continuation (Pattern 2)**: Table $B$ starts immediately with a data row (`| val1 | val2 |`) without a header or separator line (`|---|---|`).

---

## 2. Merging Algorithm

```python
def merge_markdown_tables(table_a_lines: list[str], table_b_lines: list[str]) -> list[str]:
    """
    Merges table_b into table_a, removing duplicate headers and joining interrupted rows.
    """
    # 1. Identify header and separator in Table B
    b_start_idx = 0
    if len(table_b_lines) >= 2 and '|' in table_b_lines[0] and set(table_b_lines[1].replace('|', '').strip()) <= {'-', ':', ' '}:
        # Table B has a repeated header and separator row
        b_start_idx = 2
    elif len(table_b_lines) >= 1 and 'continued' in table_b_lines[0].lower():
        # Table B has a "(Continued)" title row
        b_start_idx = 1
        if len(table_b_lines) > 2 and set(table_b_lines[2].replace('|', '').strip()) <= {'-', ':', ' '}:
            b_start_idx = 3

    table_b_data_rows = table_b_lines[b_start_idx:]
    if not table_b_data_rows:
        return table_a_lines

    # 2. Check if the last row of Table A was split across page boundary
    # (e.g. unfinished sentence ending without punctuation, or empty final cell)
    last_row_a = table_a_lines[-1]
    first_row_b = table_b_data_rows[0]
    
    # Check if first_row_b has blank primary key/ID column while continuing text
    cols_a = [c.strip() for c in last_row_a.strip('|').split('|')]
    cols_b = [c.strip() for c in first_row_b.strip('|').split('|')]
    
    if len(cols_a) == len(cols_b) and cols_b[0] == "" and cols_b[-1] != "":
        # Row B continues Row A's description cell
        merged_last_cell = cols_a[-1] + " " + cols_b[-1]
        cols_a[-1] = merged_last_cell
        table_a_lines[-1] = "| " + " | ".join(cols_a) + " |"
        table_b_data_rows = table_b_data_rows[1:]

    # 3. Concatenate remaining rows
    return table_a_lines + table_b_data_rows
```

---

## 3. Formatting Guidelines for Merged Tables

1. **Consistent Alignment**: Ensure markdown table delimiters use standard `|---|---|` or alignment specifiers (`|:---|:---:|---:|`).
2. **Column Width Polish**: If columns look compressed or misaligned in raw markdown, pad cells with spaces so column dividers align vertically.
3. **Preserve Internal Markdown**: Retain all inline code formatting (`` `D0` ``, `` `$DFF000` ``) and bold/italic markers inside table cells.
