# Multi-Page Table and Graphic Continuation Prompt

You are an expert technical editor analyzing two candidate adjacent content blocks in a technical manual across a page transition.

You are given:
- **Block A (Page N)**: Type, bounding box, header rows, and extracted raw text.
- **Block B (Page N+1)**: Type, bounding box, and extracted raw text.

## Evaluation Goal
Determine whether Block B is a direct continuation of Block A.

### Table Continuation Indicators:
1. **Identical Column Schema**: Block B has the exact same number of columns or continuation of field structures as Block A.
2. **Continued Data Rows**: The first rows of Block B continue data items (e.g. sequentially increasing memory addresses, register offsets, or alphabetical parameter lists) without a new standalone table caption.
3. **Repeated Header with Continuation Tag**: Block B repeats the header row from Block A, optionally appended with "(continued)".

### Graphic Continuation Indicators:
1. **Multi-Page Schematic**: A wide block diagram or schematic split across facing pages.

## Output Format
Return a strict JSON object:
```json
{
  "is_continuation": true,
  "confidence": 0.95,
  "relationship": "continued_data_rows",
  "explanation": "Block B continues register bit assignment rows from Block A under the same 4-column layout."
}
```
