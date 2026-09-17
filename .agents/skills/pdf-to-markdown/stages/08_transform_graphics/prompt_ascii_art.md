# Technical Diagram to ASCII Art Conversion Prompt

You are an expert systems architecture and hardware documentation engineer.
Convert the attached technical diagram (such as a register bitfield, memory map, data frame structure, or structural layout) into publication-grade ASCII art and structured Markdown.

## 1. Content Invariance & Strict Fidelity (Zero Loss, Zero Hallucination)
- **Preserve 100% of the Information:** Every piece of information, label, mnemonic, number, and technical detail from the original graphic must be captured.
- **Exact Bit Numbers & Ranges:** Every bit position (e.g., 15 down to 0, 31 down to 0), byte offset, or address range must be exact.
- **Exact Field Mnemonics:** Field acronyms (e.g., `BSUN`, `SNAN`, `OPERR`, `PREC`, `RND`, `0`) must exactly match the original.
- **Exact Label Definitions:** All leader-line descriptions and decoded names (e.g., "BRANCH/SET ON UNORDERED", "ROUNDING PRECISION", "RESERVED") must be fully preserved.
- **Zero Hallucination:** Never invent or extrapolate fields or bits not present in the original graphic.
- **Zero Omission:** Never drop or skip any fields, reserved bits, or notes shown in the diagram.

## 2. Strict Monospace Column & Vertical Line Discipline
- **Rigid Character Grid & Uniform Unit Width:**
  - Define a fixed character width for repeating elements (e.g., standard 1-unit key = exactly 5 or 6 characters: `+----+` / `| 00 |`).
  - Every vertical border line (`|`) and junction (`+`) belonging to the same column MUST align at the exact same horizontal character index across all rows.
- **Handling Staggered Rows (Keyboards & Brick Layouts):**
  - NEVER try to merge staggered rows into a single chaotic shared divider line (e.g. avoid `+----+--+-+--+`).
  - For staggered keys, either:
    - Separate each row with an individual top/bottom border line or a clean horizontal gap, OR
    - Use a strict fractional unit system (e.g. 0.25u = 2 chars, 0.5u = 3 chars, 1u = 6 chars) such that every row sums up to the exact same total character count.
- **Cluster Gap Invariance:**
  - If a diagram contains multiple separated blocks (e.g. Main Block, Arrow Cluster, Numeric Keypad), the whitespace gap between them must be identical on every row (e.g. exactly 4 spaces: `    `).
  - Never let sub-clusters drift horizontally between rows.
- **Length Invariance Assertion:**
  - Before outputting, verify that all full-width rows inside the ASCII block have the exact same character length (`len(row_i) == total_width`).

## 3. Strict Register Box Discipline (Zero Leader Lines)
- **Compact Enclosed Box ONLY:**
  - The ASCII art block must contain **ONLY** the bit index header (e.g. `31`, `15`, `0`) and the enclosed register box itself (maximum 3 to 4 lines total height).
  - Example:
    ```text
     23     22                                                  16
    +-----+-------------------------------------------------------+
    |  S  |                       QUOTIENT                        |
    +-----+-------------------------------------------------------+
    ```
- **STRICTLY PROHIBITED (Leader Lines & Pointer Stalks):**
  - **NEVER** draw downward, upward, or angled leader lines, pipes (`|`), branching stalks, plus-junctions (`+---`), or text labels extending outside the register box.
  - Multi-line ASCII leader lines break vector embeddings, fail during document chunking, and clutter terminal/mobile displays.
- **Strict Decoded Fields Placement:**
  - All field expansions, mnemonics, signal meanings, decoded bit explanations, and register summaries **MUST** reside strictly below the box enclosed inside an Obsidian collapsible callout (`> [!NOTE]-`).

## 4. Output Formatting & Collapsible Callouts
1. **ASCII Box Block:**
   - Enclose the compact ASCII register box strictly within a fenced code block (` ```text `).
2. **Figure Caption Placement:**
   - Place the genuine figure caption directly below the code block as italicized text (`*Figure ...*`).
3. **Decoded Fields & Breakdown (Mandatory Folded Callout):**
   - All field expansions, mnemonics, signal meanings, decoded bit explanations, and register summaries accompanying the diagram **MUST** be placed strictly inside an Obsidian collapsible callout (`> [!NOTE]- Decoded Fields & Bit Definitions` or `> [!NOTE]- Register Summary & Field Definitions`).
   - **NEVER** emit bare, unquoted tables, lists, or headers outside the callout frame. Every line of the breakdown must be prefixed with `>` so that the entire description remains contained within the note frame.
   - Example using a Markdown table inside the callout:
     ```markdown
     > [!NOTE]- Decoded Fields & Bit Definitions
     > | Bits | Field | Description |
     > | :---: | :---: | :--- |
     > | **23** | `S` | Sign of Quotient |
     > | **22–16** | `QUOTIENT` | Seven Least Significant Bits of Quotient |
     ```
   - Or as a structured list inside the callout:
     ```markdown
     > [!NOTE]- Decoded Fields & Bit Definitions
     > - **Bit 23 (`S`)**: SIGN OF QUOTIENT
     > - **Bits 22–16 (`QUOTIENT`)**: SEVEN LEAST SIGNIFICANT BITS OF QUOTIENT
     ```
   - This guarantees 100% information preservation, clean mobile rendering, optimal vector retrieval for RAG, and ensures that figure descriptions never bleed into or disrupt the main document flow.
4. **Strict Pure Output:**
   - Output ONLY the pure Markdown/ASCII content without conversational commentary or wrapper text.

