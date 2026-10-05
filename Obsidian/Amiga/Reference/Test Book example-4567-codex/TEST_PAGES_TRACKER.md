# Test Book Page Tracker & Pipeline Objectives

**Document:** `Test Book example-4567-codex/Amiga TestBook example-4567.pdf`\
**Total Pages:** 160\
**Source Documents:**\
- `68000 User's Manual/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8.pdf` (Pages 1–2, 4–9, 12–19, 23–24, 26–29, 32–33, 35–36, 39, 41, 44, 47, 59, 62, 70, 96, 112, 115, 208) [37 pages]\
- `68000 Programmer's Reference Manual/M68000PRM.pdf` (Pages 13, 16, 22, 23, 29, 32, 42, 46, 65, 80, 97, 104, 105, 106–111, 112, 338, 597–598, 603, 632, 639) [26 pages]\
- `A500 A2000 Technical Reference Manual/Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore.pdf` (Pages 5, 7, 13–15, 20, 21, 29–31, 42–45, 109, 110, 113, 124, 126, 131, 135, 138, 139, 151–152, 155, 178, 184, 201, 202, 205, 207, 211, 228, 233, 235, 237, 247, 248) [39 pages]\
- `Hardware Reference Manual/Commodore_Amiga_Hardware_Reference_Manual_2nd.pdf` (Pages 26, 28, 35, 37, 42, 43, 57, 58, 71, 84, 88, 90, 109, 124, 136, 145, 151, 153, 155, 156, 159, 171, 176, 183, 186, 187, 191, 201, 203, 204, 206, 208, 210, 229, 234, 241, 256, 257, 267, 270, 271, 277, 279, 293, 301, 302, 303, 308, 312, 320, 321, 336, 346, 356, 362, 368, 369, 391) [58 pages]\
**Status:** Assembled, optimized, and ready for pipeline verification.

### Local Reconstruction and Source Identification

This tracker records page selection, technical facts, and test expectations. It does not contain the source pages or grant permission to use or redistribute the manuals. Reconstruct the test PDF locally from source copies you are authorized to process; keep the PDFs and converted book content outside Git.

Use the **Complete 160-Page Mapping Matrix** as the assembly order. Both `Test Page` and `Source PDF Page` are **1-based PDF page numbers**; `Printed Book Folio` is descriptive and must not be used as a PDF index. Match each source by filename and SHA-256 below, then append its selected pages in ascending `Test Page` order. Source directory names above identify the manuals; local storage directories may differ. The historical change log and older scenario page references do not override the current matrix.

| Source PDF Filename | Total PDF Pages | SHA-256 |
| --- | ---: | --- |
| `M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8.pdf` | 216 | `e41cbe7e14dc7cb853f1185adb1dcd7043d2a2a3f43cdc909e0f6c146007a8e5` |
| `M68000PRM.pdf` | 646 | `06e4864b78da0e815054cead9326b7ec9914661f240fd39a455f2061ff47c4e8` |
| `Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore.pdf` | 308 | `adc8d1eecb727f2cb7e524192722e7e2cfe90bc91c1139b0a7a2d7efe3af2379` |
| `Commodore_Amiga_Hardware_Reference_Manual_2nd.pdf` | 405 | `69bbdd2f8cfc3c0b337fa662445e75dbbae39c113654a666964c68a26234a86a` |

On 2026-10-05, all 160 matrix entries matched the local test PDF's decoded page content streams and media boxes against these sources. This verifies page selection and order, not byte-identical output files or complete rendering equivalence; PDF writer settings and metadata can differ.

Alert examples below use placeholders to illustrate Markdown syntax. Verify the actual source text locally during conversion tests.

---

## 1. Overview & Purpose

This composite test book isolates specific structural patterns and edge cases from technical retrocomputing documentation to test and validate pipeline stages without processing hundreds of pages:
1. **Front Cover Graphics:** Detection and handling of title/cover graphics and logos.
2. **Thumb/Tab Indexes:** Verifying edge bookmarks and index tabs are ignored or cleanly handled without spurious asset cropping.
3. **Table of Contents (TOC):** Multi-page TOC generation, continuation handling, and conversion of entries to active Markdown links.
4. **Multi-Page Lists with Repeated Headers:** Verifying the repetition and conclusion of list headers (`LIST OF ILLUSTRATIONS (Continued)`, `(Concluded)`, `LIST OF TABLES (Concluded)`) across page breaks and ensuring clean consolidation during chapter fusion (Stage 21).
5. **Technical Prose & Section Starts:** Baseline chapter detection and content transcription for Section 1.
6. **Anti-Table Regression Tests (Programmer Models, Pointer Trees, Spatial Memory Maps, Data Storage Encodings, IC Pinout Diagrams & Register Word Formats):** Guaranteeing that complex processor architecture models (Figures 2-1, 2-2, 2-3), status register pointer callout diagrams (Figure 2-4), spatial memory organization maps with jagged break lines (Figure 2-5), multi-tier data storage encoding diagrams (Figure 2-6), byte-serial memory packing diagrams (Figure 2-7), integrated circuit (IC) functional pinout / signal grouping diagrams (Figure 3-1), and register word format diagrams (Figure 6-9) are **NEVER converted into HTML or Markdown tables**, but are strictly classified as `image` for Stage 16.
7. **Table Reduction with Mathematical Notation:** Verifying that a structured technical table (`Table 2-1. Data Addressing Modes`) is converted to a semantic table in Stage 12/13 and cleanly reduced to a **pure GFM Markdown table** in Stage 15, preserving native LaTeX mathematical notation: displacement subscripts (`$d_8$`, `$d_{16}$`), replacement arrows (`$\leftarrow$`), and effective address calculation formulas (`$\text{EA} = (\text{PC}) + d_{16}$`).
8. **Multi-Page Spanning Table Fusion Test (Table 2-2 Instruction Set Summary):** Verifying that a large 3-page technical table spanning consecutive sheets (`Table 2-2. Instruction Set Summary (Sheets 1, 2, and 4)) is transcribed per-page in Stages 7–15, properly tagged across page boundaries with `<continuation-marker>` in Stage 20, and seamlessly merged into a single unified table in Stage 21 while stripping redundant sheet suffixes and repeated column header rows (`Opcode | Operation | Syntax`).
9. **Active-Low Bus Signals & Overbar Healing (Paragraph 3.3 Asynchronous Bus Control):** Verifying that printed overbars on active-low bus signals ($\overline{\text{AS}}$, $\text{R}/\overline{\text{W}}$, $\overline{\text{UDS}}$, $\overline{\text{LDS}}$) that were corrupted in the scanned PDF text layer (e.g. into stray tildes `Address Strobe (~)` or un-negated `R/W`) are healed by multimodal cross-referencing in Stage 7 and Stage 19 into standard retrocomputing notation (`` `_AS` ``, `` `R/_W` ``, `` `_UDS` ``, `` `_LDS` ``) or typographical LaTeX math (`$\overline{\text{AS}}$`, `$\text{R}/\overline{\text{W}}$`).
10. **Markdown Callout Alert Style for Advisory Notes (NOTE, WARNING, CAUTION):** Verifying that explicit advisory blocks (such as the centered `NOTE` at the bottom of Page 41 regarding MC68008 interrupt priority levels) are cleanly formatted in GitHub Flavored Markdown alert blockquote syntax (`> [!NOTE]`) with normalized hardware registers and signals, rather than left as plain unformatted body text.
11. **HTML Table Preservation with Colspan/Rowspan (Table 3-3 & Table 6-1):** Verifying that multi-span tables with complex column groupings (`colspan="3"`) and multi-row cells (`rowspan="2"`) are transcribed into semantic HTML in Stage 12/13 and explicitly protected from reduction by the Stage 15 gatekeeper.
12. **Multi-Figure Page Decomposition (Page 34):** Verifying that pages containing multiple distinct graphical entities (Figure 4-1 flowchart and Figure 4-2 timing waveform) produce separate `<crop>` tags and independent visual image assets rather than merging them into a single crop.
13. **Cross-Reference Internal Anchor Linking (Page 35):** Verifying that prose cross-references (e.g. `refer to Appendix B M6800 Peripheral Interface`) are converted into active GitHub Flavored Markdown internal anchor links (`[Appendix B M6800 Peripheral Interface](#appendix-b-m6800-peripheral-interface)`).
14. **Anti-Table Regression for Bus/CPU Space Encodings (Figure 5-10):** Verifying that CPU space address encoding diagrams with bit partitions, dotted column dividers, and callout brackets are strictly classified as `image` and never converted to HTML tables.
15. **Crop Perimeter Isolation vs. Native Legend & Notes Text (Figure 5-18):** Verifying that state transition diagrams are cropped tightly around the graphic state bubbles and arcs, while bottom abbreviation legends and numbered notes remain selectable native Markdown text.
16. **Blank Page Handling (Page 38):** Verifying that completely blank pages are gracefully handled without hallucinated text or errors.
17. **Anti-Table Regression for Register Word Formats (Figure 6-9):** Verifying that register bitfield formats captioned as "Figure" (such as `Figure 6-9. Special Status Word Format`) are strictly classified as `image` (`detected_type: "image"`), deferred to Stage 16 for architectural breakdown, and never converted into an HTML or Markdown table.
18. **Table Cell Text Formatting & HTML Entity Escaping (Tables 7-1 & 7-2):** Verifying that multi-span and parameter execution tables preserve exact parenthesized notation (`0(0/0)`, `8(2/0)`), addressing mode specifications (`(xxx).W`, `(d16, An)`), and escape HTML special characters (`&lt;data&gt;` for `#<data>`) inside cells, and are preserved as HTML tables by the Stage 15 gatekeeper.
19. **Back-of-the-Book Multi-Page Subject Index Processing & Final Omission Rule (Pages 42–46 / Source Pages 208–212):** *(Policy Updated 2026-10-01)* Verifying that the back-of-the-book printed index is transcribed cleanly through Stages 5–19 as single-column vertical text without image crops, but is systematically stripped and omitted from the final publication chapters in Stage 20/21, as static page-number-based print indices are obsolete in searchable digital Markdown documents.
20. **Multi-Manual Anti-Table Regression for PRM Programming Models & Pointer Trees (Pages 47 & 48 / PRM Pages 13 & 16):** Verifying multi-manual test book capability and ensuring that complex processor architectural models with overarching category brackets (`Figure 1-1. M68000 Family User Programming Model`) and register word diagrams with 2D branching pointer/callout trees connecting bit cells to condition/mode labels (`Figure 1-3. Floating-Point Control Register`) from the *68000 Programmer's Reference Manual* are **strictly classified as `image`** and never converted to HTML or Markdown tables.
21. **Composite Register & Table Disaggregation (Page 49 / PRM Page 22, Figure 1-8):** Verifying that a composite figure containing a status register diagram with pointer callouts alongside two lookup tables (`TRACE MODE` and `ACTIVE STACK`) is cleanly disaggregated into an `image` crop for the pointer graphic, and two native Markdown tables for the data lookup grids.
22. **Bounded 32-Bit Register Bitfield Conversion (Page 50 / PRM Page 23, Figure 1-9):** Verifying that a bounded 32-bit register grid (`Figure 1-9. MC68030 Transparent Translation/MC68EC030 Access Control Register Format`) showing bit boundaries (31..0) and subfields is converted into a semantic HTML table (`table_html` in Stage 14) with explicit `colspan` attributes.
23. **Floating-Point Real Format Diagram Bounding & Bailout (Page 51 / PRM Page 29, Figures 1-13 & 1-14):** Verifying that real format specification diagrams (`Normalized Number Format` and `Denormalized Number Format`) are classified strictly as `image` (`detected_type: "image"`), deferred to Stage 16.
24. **Nested HTML Table Embedding (Page 52 / PRM Page 32, Table 1-4):** Verifying that a high-density specification table (`Table 1-4. Single-Precision Real Format Summary Data Format`) is converted to an HTML table with an embedded child HTML sub-table (`<table>` inside `<td>`) representing the 32-bit `s | e | f` field breakdown inside the top cell.
25. **Instruction Word Format Table with Bit Boundaries (Page 53 / PRM Page 42, Figure 2-1):** Verifying that an instruction word general format diagram with bit numbers (`15` on the left, `0` on the right) spanning stacked instruction composition rows is converted to a semantic HTML table in Stage 14.
26. **Multi-Section Crop Isolation vs. Native Paragraph Prose (Page 54 / PRM Page 46):** Verifying that a page with three successive addressing mode sub-clauses (2.2.1, 2.2.2, 2.2.3) preserves the introductory text of each section as native Markdown paragraphs, while extracting three separate, isolated `image` crops for the underlying register/memory addressing diagrams.
27. **Split Lookup Table & Arithmetic Graph Decomposition (Page 55 / PRM Page 65, Figure 2-5):** Verifying that `Figure 2-5. No Memory Indirect Action` is decomposed into an upper 4-column Markdown table (`BR | Xn | bd | Addressing Mode`) and a lower `image` crop for the address calculation flowchart.
28. **Complex Graphical Operation Table Bailout to Image (Page 56 / PRM Page 80, Table 3-5):** Verifying that despite being titled `Table 3-5`, `Shift and Rotate Operation Format` is classified as `image` (deferred to Stage 16) due to graphical bit-shift schematics and flip-flop routing arrows in the `Operation` column, producing an in-depth architectural breakdown in Markdown and RAG text.
29. **Algorithmic Flowchart Classification (Page 57 / PRM Page 97, Figure 3-2):** Verifying that multi-branch decision flowcharts (`Figure 3-2. Rounding Algorithm Flowchart`) are strictly classified as `image` for Stage 16.
30. **Instruction Description Format Structural Template Bailout (Page 58 / PRM Page 104, Figure 3-3):** Verifying that documentation format layout templates and instruction description guides (`Figure 3-3. Instruction Description Format`) with bracket annotations are strictly classified as `image` (`detected_type: "image"`), deferred to Stage 16.
31. **Side-by-Side Addressing Mode Tables (Page 59 / PRM Page 105):** Verifying that two side-by-side addressing mode lookup tables at the bottom of the Section 4 start page (`(bd,An,Xn)` and `(bd,PC,Xn)`) are cleanly recognized and transcribed as separate, properly structured tables.
32. **Multi-Page Instruction Specification Fusion without Header Repetition (Pages 60–65 / PRM Pages 106–111):** Verifying that the instruction sequence in Section 4, specifically the `ADD` instruction spanning 3 consecutive printed pages (Pages 62–64 / PRM Pages 108–110: `ADD <ea>, Dn` and `ADD Dn, <ea>`), is transcribed cleanly without repeated running headers/footers, without repeated `ADD` banners on every page, and fused in Stage 21 into a single unified instruction documentation entry.
33. **Side-by-Side Effective Address Column Unrolling (Page 66 / PRM Page 112):** Verifying that print-space-saving layouts featuring 4 side-by-side table blocks (e.g. `ADDA` effective address modes split into two base mode columns and two 68020+ mode columns) are recognized and unrolled into **2 continuous single-column tables** (one for base architecture modes, one for 68020+ modes), preserving linear readability in Markdown.
34. **Dual Left/Right Text Alignment in Single Column HTML Tables (Page 67 / PRM Page 338, FDIV Operation Table):** Verifying that the `FDIV` instruction `Operation Table` is converted into a semantic HTML table (`table_html` in Stage 13/14) with cell formatting that accurately handles dual left/right alignment within the same column (such as operand symbols `+` / `-` aligned to the left and numeric/infinity values aligned to the right).
35. **Multi-Page Spanning Appendix Table Unification (Pages 68–74 / PRM Pages 597–603, Table A-1):** Verifying that Appendix A `Table A-1. M68000 Family Instruction Set And...`, spanning across Sheets 1, 2, and 7, is transcribed per-page and unified in Stage 20 & 21 into **ONE single continuous Markdown/HTML table**, suppressing redundant intermediate sheet titles, repeated running headers, and duplicate table header rows.
36. **Exception Stack Frame Diagram Bailouts (Page 75 / PRM Page 632, Figures B-7 & B-8):** Verifying that processor exception stack frame layouts (`Figure B-7. MC68EC040/LC040 Floating-Point Unimplemented Stack Frame` and `Figure B-8. MC68040 Access Error Stack Frame`) with stacked hexadecimal byte offset indicators (`SP`, `+$02`, `+$06`...) are strictly classified as `image` (`detected_type: "image"`), deferred to Stage 16.
37. **Idle & Unimplemented Stack Frame Diagram Bailouts (Page 76 / PRM Page 639, Figures B-21 & B-22):** Verifying that processor stack frame diagrams (`Figure B-21. MC68040 Idle Stack Frame` and `Figure B-22. MC68040 Unimplemented Instruction Stack Frame`) showing internal stack frame formats with byte offset callouts (`$00`, `$04`, `$08`...) are strictly classified as `image` (`detected_type: "image"`), deferred to Stage 16.
38. **Keyboard Layout & Scan Code Diagram Bailout to Image (Page 77 / A500 TRM Page 5, Figure 1.1):** Verifying that keyboard hardware diagrams showing physical keycaps mapped to raw scan codes (`Figure 1.1 Key Codes`) are strictly classified as `image` (`detected_type: "image"`), deferred to Stage 16.
39. **Side-by-Side Connector Graphic & Pinout Table Linearization (Page 78 / A500 TRM Page 7):** Verifying that pages placing a connector illustration (DB25 pin diagram) side-by-side with a comparison pinout table (A1000 vs. A500/A2000 Centronics port) are linearized vertically in Markdown: the image link is emitted first, followed immediately by the transcribed table underneath.
40. **Multi-Page Spanning Table Unification with Intermediate Footnote Relocation (Pages 79–81 / A500 TRM Pages 13–15, Table 1-1):** Verifying that a 3-page reference table (`Table 1-1 RAW KEY CODES`) is unified into **ONE single continuous Markdown/HTML table** in Stage 21, and that footnotes printed on an intermediate sheet (Page 14: `*In shifted Forward Arrow...` and `<CSI> stands for...`) are extracted and cleanly relocated to the very end of the completed unified table.
41. **Markdown Callout Warning Alert Formatting (Page 82 / A500 TRM Page 20):** Verifying that an explicit `WARNING` block regarding worst-case expansion bus hardware specifications is formatted cleanly as a GitHub Flavored Markdown warning alert callout (`> [!WARNING]`).
42. **Markdown Callout Note Alert Formatting (Page 83 / A500 TRM Page 21):** Verifying that an explicit `NOTE` block regarding signal radiation control and FCC Class B compliance is formatted cleanly as a GitHub Flavored Markdown note alert callout (`> [!NOTE]`).
43. **Auto-Config Register Bit Descriptions & Structures as Formatted Text (Pages 84–86 / A500 TRM Pages 29–31):** Verifying that auto-config address specifications and bit descriptions (`(00/02) 7654 3210` nibbles, board types, sizes, memory lists, chained config flags, and bit structures with horizontal bars) are transcribed as structured native text / bulleted lists / code blocks rather than cropped as visual assets.
44. **Reserved Address Register Block Table Conversion (Page 86 / A500 TRM Page 31):** Verifying that the repetitive register offset block at the bottom of Page 31 (`(50/52)` through `(7C/7E)` with bit fields `7654 3210` and `Reserved, must be 00`) is converted into a clean Markdown table (`| Offset | Bits | Description |`).
45. **Multi-Table Detection on Single Page (Page 87 / A500 TRM Page 42 & Page 92 / A500 TRM Page 110):** Verifying that pages containing multiple distinct data tables produce separate table entities: Page 87 (Standard Load/Drive Table + Signal Drive Table Start) and Page 92 (PC Memory & I/O Map + Amiga Memory Map).
46. **Multi-Page Bus Signal Drive Loading Table Fusion (Pages 87 & 88 / A500 TRM Pages 42 & 43):** Verifying that the `A2000 System Bus Loading / Signal Drive Table` beginning on Page 87 and concluding on Page 88 is fused into **ONE single continuous table** in Stage 21, suppressing the repeated column headers (`Named Signals | DIR | Expansion Slots (each) | ...`).
47. **HDL Logic Specifications & PAL Equations in Code Blocks (Pages 89 & 90 / A500 TRM Pages 44 & 45):** Verifying that PAL boolean equations and logic definitions (`TABLE 3-2 PAL16L8 STEERING1SOR17 REV3` and `TABLE 3-3 PAL16R6 ARBITRATE REV1`) are formatted as fenced code blocks (` ```text ` or ` ```pal `), never cropped as images and never forced into HTML tables.
48. **Emulator Memory Mapping Tables (Pages 91 & 92 / A500 TRM Pages 109 & 110):** Verifying conversion of the PC/XT emulator interface memory table (Page 91) and the dual memory maps (Page 92) into clean semantic tables.
49. **Bridgeboard PC/AT I/O Address Table Evaluation (Page 93 / A500 TRM Page 113):** Evaluating whether the bridgeboard PC/AT I/O port address mapping (`PC/AT I/O Address | Usage | Offset Address`) is transcribed as a clean monospaced text block or structured table.
50. **BIOS Tele-Type Output & Bottom Bitfield Graphics Evaluation (Page 94 / A500 TRM Page 124):** Verifying transcription of BIOS calling conventions and evaluating whether the bottom register bitfield drawings (`Bits of AL` and `Bits of AH`) are cropped as graphics or parsed as bitfield tables.
51. **Serial Port EIA DSR BIOS Parameter Specifications (Page 95 / A500 TRM Page 126):** Verifying clean transcription and formatting of serial port calling parameters (`AH = 00H`, baud rate / UART options) and return status bit definitions.
52. **`janus.library` API Function Signatures & Register Conventions (Page 96 / A500 TRM Page 131):** Verifying that library function signatures (e.g., `SetJanusHandler(jintnum, intserver)`) and CPU register parameter bindings (`D0`, `A1`) are formatted cleanly in Markdown with backtick code styling and structured parameter lists.
53. **Multi-Page Assembly Listings & File Demarcation (Pages 97–101 / A500 TRM Pages 135–139):** Verifying multi-page assembly code formatting (` ```assembly `) across 5 consecutive pages (`janus_i86block.i`), and verifying that the start of the next file (`janus.i` on Page 139) is cleanly demarcated with a closed code block, distinct file header, and newly opened code block.
54. **Multi-File C Code Block Listings & Demarcation (Pages 102 & 103 / A500 TRM Pages 151 & 152):** Verifying C header listings (`janus.h`), and ensuring that when `janus.h` concludes on Page 152 and `janus_memrw.h` begins, the two files are cleanly separated into distinct C code blocks (` ```c `).
55. **PC Janus Service BIOS Interrupt Reference (Page 104 / A500 TRM Page 155):** Verifying clean Markdown formatting for INT JANUS software interrupt service definitions (`J_GET_SERVICE`, `J_ALLOC_MEM`, `J_FREE_MEM`) with input and return register mappings (`AH`, `AL`, `ES:DI`, `DX`, `BX`).
56. **Hard Disk Controller Command Summary Table with Footnotes (Page 105 / A500 TRM Page 178):** Verifying conversion of `Table 5-6. Command Summary` into a semantic table, with bottom error code footnotes preserved as clean Markdown text directly beneath.
57. **Hardware Tables with Multi-Byte/Bit Column Spans & Custom Chip Register Formats (Pages 106–109 / A500 TRM Pages 184, 201, 202, 205):** Verifying that `Table 5-10. Change Command Block Address` (with multi-byte DMA address spans `A23..A16`), Blitter line draw mode bit tables (Pages 107 & 108), and Copper instruction register format tables (Page 109 with `MOVE/WAIT/SKIP` spans and `VE`/`HE` footnotes) are converted as semantic HTML tables with `colspan`/`rowspan`, explicitly protected from reduction by the Stage 15 gatekeeper.
58. **Display Data Fetch Timing Tables & Bit Assignment (Page 110 / A500 TRM Page 207):** Verifying conversion of `DDFSTRT` and `DDFSTOP` horizontal timing tables (`PURPOSE | H8 H7 H6 H5 H4`), register bit assignment fields (`BIT# | USE`), and `DMACON`/`DMACONR` control write/read bit descriptions.
59. **Surrounding Prose / Text Flow Linearization around DMA Time Slot Diagram (Page 111 / A500 TRM Page 211):** Verifying that text flanking or wrapping around a visual diagram (`DMA Time Slot Allocation / Horizontal Line (Cont'd)`) is linearized cleanly into narrative prose -> diagram `<crop>` -> subsequent continuation prose.
60. **PAL20L8 Logic Specification in Fenced Code Block (Page 112 / A500 TRM Page 228):** Verifying that PAL20L8 memory and DTACK decoder design specifications, pin declarations, and logic equations (`IF (OVR) /VPA = ...`) in Section 7.3 are formatted as a fenced code block (` ```text ` or ` ```pal `), never cropped as an image and never forced into an HTML table.
61. **Motherboard Jumper Pin Configuration Diagrams as Images (Page 113 / A500 TRM Page 233):** Verifying that Section 7.4 motherboard jumper block diagrams (`J101`, `J200`, `J301`, `J500`) showing pin shunt configurations (1-2 vs 2-3) are cropped as visual `image` crops, while surrounding text explaining address bit selection (A23 vs A19) remains selectable Markdown prose.
62. **Large-Format Hardware Schematic Foldout Extraction (Page 114 / A500 TRM Page 235):** Verifying that Appendix E oversized hardware schematics (1831x2160 pt foldout sheet) are cropped cleanly as high-resolution visual `image` assets for Stage 16 architectural breakdown and RAG text indexing.
63. **Backplane Bus Expansion Signal Interconnection Table / Routing Diagram (Page 115 / A500 TRM Page 237):** Verifying handling of expansion backplane slot interconnects (`J1`–`J5`, `74LS32`, `LOCAL_OWN`, `SLAVE`, `INT6`, `CONFIG_OUT`) and evaluation of tabular bus signal interconnect grids vs schematic diagram.
64. **Tri-Part Hardware Connector Linearization (Page 116 / A500 TRM Page 247):** Verifying that `A2000 Keyboard Connector` documentation is linearized cleanly into: introductory text -> connector mechanical/pin illustration `<crop>` -> `Pin Name Description` pinout table (Pins 1–6: KCLK, KDAT, NC, GND, +5V, SHIELD).
65. **Peripheral Dimension Drawing & Pin Connection Table (Page 117 / A500 TRM Page 248):** Verifying that the Amiga 500/2000 mouse specification is linearized into: physical dimension illustration `<crop>` -> `CONNECTION TABLE` (`PIN | FUNCTION`) transcribed as a clean GFM table.
66. **Multi-Language Fenced Code Blocks (Page 118 / HRM Page 26):** Verifying that dual assembly and C code examples documenting custom chip register offsets (`hardware/custom.i` and `hardware/custom.h`) are formatted in distinct fenced code blocks (`` ```assembly `` and `` ```c ``).
67. **Alert Blockquotes & Instruction Support Table (Page 119 / HRM Page 28):** Verifying formatting of upper advisory `> [!NOTE]` (TAS instruction warning), middle Markdown table (`| CPU | User Mode | Super Mode |`), and bottom advisory `> [!NOTE]` (MOVE.W vs CLR.W strobe).
68. **Independent Consecutive Code Blocks (Page 120 / HRM Page 35):** Verifying that consecutive assembly code fragments illustrating the Copper `WAIT` instruction (`DC.W $9601,$FF00` and `DC.W $FFFF,$FFFE`) are formatted as separate fenced code blocks with inline comments.
69. **Instruction Interpretation Table & Alternation Alert (Page 121 / HRM Page 37):** Verifying conversion of the Copper instruction wrap comparison table (`Instruction | Explanation`) and subsequent `> [!NOTE]` callout regarding NTSC/PAL line and field alternation.
70. **Multi-Page Sample Copper List Listing (Pages 122 & 123 / HRM Pages 42 & 43):** Verifying continuous assembly code formatting across consecutive pages for the `COMPLETE SAMPLE COPPER LIST`, maintaining indentation and column alignment from introductory setup through `COPPERLIST:` continuation.
71. **Playfield Dimension & Color Table Conversions (Pages 124 & 125 / HRM Pages 57 & 58):** Verifying conversion of `Table 3-1: Colors in a Single Playfield` (`Number of Colors | Number of Bit-Planes`), `Table 3-2: Portion of the Color Table`, and genlock advisory `> [!NOTE]`.
72. **HTML Table with Multi-Level Spans & Mathematical Expressions (Page 126 / HRM Page 71):** Verifying conversion of `Table 3-9: DIWSTRT AND DIWSTOP Summary` into a semantic HTML table with sub-header spans (`colspan="2"` for Nominal and Possible values), while preserving under-table clock and display window arithmetic formulas as native LaTeX math ($ ... $).
73. **Dual-Playfield Multi-Subsystem Table Disaggregation (Page 127 / HRM Page 84):** Verifying that `Table 3-12: Playfields 1 and 2 Color Registers` is cleanly separated into two distinct Markdown tables (`Playfield 1` and `Playfield 2`) preserving bit combinations and color selections.
74. **Sequential Multi-Diagram Linearization (Page 128 / HRM Page 88):** Verifying that multiple sequential memory layout diagrams (Figures 3-16, 3-17, 3-18) separated by explanatory text paragraphs are isolated into 3 distinct visual `image` crops interspersed with narrative prose, rather than merged into a single oversized crop.
75. **Display Window Screen Region Diagrams (Page 129 / HRM Page 90):** Verifying that two distinct display window area diagrams (Figures 3-19 and 3-20) are isolated into 2 independent visual `image` crops with intermediate explanatory prose preserved.
76. **High-Resolution Color Selection Table with Rowspans (Page 130 / HRM Page 109):** Verifying that `Table 3-19: High-resolution Color Selection` is converted into a semantic HTML table preserving multi-row vertical spans (`rowspan="4"` for `NOT USED IN THIS MODE`), with under-table footnotes (`*` and `**`) preserved outside the table.
77. **Sprite Data Structure & Explicit Caution Alert Blockquote (Page 131 / HRM Page 124):** Verifying that the spaceship sprite assembly data structure is formatted in ` ```assembly `, display steps (1-4) are formatted as a numbered list, and the bottom advisory warning block is formatted as a GitHub Flavored Markdown `> [!CAUTION]` alert blockquote.
78. **Spanning Header Data Bit Table (Page 132 / HRM Page 136):** Verifying that `Table 4-4: Data Words for First Line of Spaceship Sprite` is converted into a structured table with a master spanning header `Pixel Number` over 16 bit columns (15 to 0) and 4 data word rows (`Line 1` to `Line 4`).
79. **Grouped Sprite Range Table with Footnotes (Page 133 / HRM Page 145):** Verifying that `Table 4-6: Color Registers for Single Sprites` is converted into a semantic table with multi-row spans (`rowspan="4"` for sprite ranges `0 or 1`, `2 or 3`, `4 or 5`, `6 or 7`), with the under-table footnote `* Selects transparent mode.` cleanly preserved below the table.
80. **Graphic Crop Isolation & Waveform Data Table Extraction (Page 134 / HRM Page 151):** Verifying that Page 151 isolates `Figure 5-2: Digitized Amplitude Values` as a visual graphic crop at the top, while extracting and transcribing the four-column digitized waveform lookup table (`TIME | SINE | SQUARE | TRIANGLE`, times 0..19) cleanly as a structured table.
81. **Audio Memory Offset Specification as Fenced Assembly Code Block (Page 135 / HRM Page 153):** Verifying that the byte-address waveform offset mapping (`Table 5-1: Sample Audio Data Set for Channel 0` / `audiodata -> AUD0LC * 100 98`, `AUD0LC+ 2** 92 83`, etc.) is formatted cleanly as a fenced assembly code block (`` ```assembly ``) rather than an unnecessary table crop, with note annotations preserved below.
82. **Audio Volume Register Table & Assembly Strobe Listing (Page 136 / HRM Page 155):** Verifying conversion of `Table 5-2: Volume Values` (`Volume | Decibel Value`) into a clean Markdown table, alongside the `SETAUD0VOLUME:` assembly code block and preceding explanatory notes.
83. **Mathematical Expressions & Audio Clock Parameter Table with Spans (Page 137 / HRM Page 156):** Verifying that DMA timing formulas (theoretical vs practical sampling limits, microsecond intervals, fractional divisions) are transcribed as native LaTeX math ($ ... $), and the bottom clock table (`Clock Values: NTSC | PAL | units` with `Clock Constant` and `Clock Interval`) is converted into a semantic HTML table preserving column spans.
84. **Top Sampled Values Table & Period Calculation Code Block (Page 138 / HRM Page 158):** Verifying that the 8-value waveform amplitude table at the top (`Sampled Values: 0, 90, 127, 90, 0, -90, -127, -90`) is transcribed as a clean table, followed by LaTeX sampling period formulas and the `SETAUD0PERIOD:` assembly code block.
85. **DMACON Register Audio Enable Bits Table & Initialization Assembly (Page 139 / HRM Page 159):** Verifying conversion of `Table 5-3: DMA and Audio Channel Enable Bits` (`DMACON Register: Bit | Name | Function` covering bits 15, 9, 3, 2, 1, 0) into a structured table, followed by the `BEGINCHAN0:` custom register initialization assembly snippet.
86. **Sampling Rate vs Frequency Relationship Table (Page 140 / HRM Page 171):** Verifying conversion of `Table 5-6: Sampling Rate and Frequency Relationship` (`Sampling Period | Sampling Rate (KHz) | Maximum Output Frequency (KHz)`) into a clean Markdown table, followed by technical prose regarding low-pass filter bypass via the 8520 CIA LED bit.
87. **Multi-Table Calibration Layout (Four Sample Data Tables) (Page 141 / HRM Page 176):** Verifying that a page presenting four distinct waveform buffer sample listings (`256 Byte Sample`, `128 Byte Sample`, `64 Byte Sample`, `32 Byte Sample`) is decomposed and transcribed into **four separate, cleanly structured tables**, preserving all positive and negative signed byte values (-128..+127).
88. **Blitter Memory Layout Map & DMA Channel Enable Advisory Note (Page 142 / HRM Page 183):** Verifying that `Figure 6-1: How Images are Stored in Memory` (showing word addresses 20..61) is cropped cleanly as an `image` asset, while the surrounding DMA channel enable explanations and advisory notes are formatted as native Markdown prose and standard blockquotes.
89. **Blitter Minterm Truth Table with Proper Boolean Negation Representation (Page 143 / HRM Page 186):** Verifying that the blitter logic function generator truth table (`A | B | C | D | BLTCON0 position | Minterm`) is transcribed as a semantic table where negated boolean minterms are accurately represented using mathematical overbars or logical negations ($\\overline{A}\\overline{B}\\overline{C}$, $\\overline{A}\\overline{B}C$, etc. or `~A ~B ~C` / `\\bar{A}\\bar{B}\\bar{C}`) rather than mangled OCR text.
90. **Blitter Logic Equations & Markdown Note Alert Callout (Page 144 / HRM Page 187):** Verifying that minterm logic expressions ($ABC$, $AB + BC$, $(AB) + (BC)$) and the LF control byte specification are formatted cleanly with LaTeX math and code formatting, and the explicit `NOTE` on operator precedence (AND before OR) is formatted as a GitHub Flavored Markdown `> [!NOTE]` alert blockquote.
91. **Dual Minterm Bit Pattern & Logic Combination Code Blocks (Page 145 / HRM Page 191):** Verifying that the two tabular minterm bit alignment and combination diagrams at the top of Page 191 (inverse source selection producing `$0F` and the OR combination $AB + BC$ producing `$C8`) are formatted cleanly as formatted monospaced code blocks (`` ```text ``) preserving character-grid alignment across bit columns (7..0), rather than degraded into broken tables.
92. **Blitter Cycle Sequence Table Conversion (Page 146 / HRM Page 201):** Verifying conversion of `Table 6-2: Typical Blitter Cycle Sequence` (`USE Code in BLTCON0 | Active Channels`) into a clean structured table, detailing bus slot sequencing for channels A, B, C, D.
93. **Octant Line Drawing Code Bits Table & Indented Algorithm Block (Page 147 / HRM Page 203):** Verifying conversion of `Table 6-3: BLTCON1 Code Bits for Octant Line Drawing` (`BLTCON1 Code Bits | Octant #`), followed by the indented line drawing algorithm code block (`dx = x2 - x1`, `dy = y2 - y1`, octant conditions and register settings) formatted with preserved indentation in a fenced code block (`` ```text ``).
94. **Multi-Page Line Mode Register Summary Code Blocks (Pages 148 & 149 / HRM Pages 204 & 205):** Verifying that the continuous register initialization specifications and pseudo-code algorithm spanning across consecutive pages (`REGISTER SUMMARY FOR LINE MODE: Preliminary setup`, `BLTCON0`, `BLTCON1`, modulo calculations, and blit size setup) are cleanly formatted across both pages as continuous fenced code blocks (`` ```text `` or `` ```assembly ``), without breaking structure or losing indentation.
95. **Blitter Execution Speed Mathematical Calculations (Page 150 / HRM Page 206):** Verifying transcription of blitter timing formulas, clock speed parameters (NTSC 7.16 MHz vs PAL 7.09 MHz), cycle tick additions (A=free, B=+2, C/D=+2, line mode=8 ticks/pixel), and total blit duration equations ($t = \frac{n \times H \times W}{7.16}$ and $t = \frac{n \times H \times W}{7.09}$) into native LaTeX math ($ ... $).
96. **Rotated Hardware Timing Diagram Crop Normalization (Page 151 / HRM Page 208):** Verifying that `Figure 6-9: DMA Time Slot Allocation / Horizontal Line`, which is printed rotated 90 degrees counter-clockwise in the source manual, is extracted and rotated 90 degrees clockwise into an upright, readable orientation for Stage 16 architectural breakdown and RAG indexing.
97. **Anti-Table Regression Test for Display Time Slot Waveform Diagrams (Page 152 / HRM Page 210):** Verifying that `Figure 6-11: Time Slots Used by a Six Bit Plane Display` and `Figure 6-12: Time Slots Used by a High Resolution Display` are **strictly classified as `image` crops** (`detected_type: "image"`), deferred to Stage 16, and NEVER converted into HTML or Markdown tables.
98. **Beam Position Counter Register Structure Table (Page 153 / HRM Page 229):** Verifying conversion of `Table 7-5: Contents of the Beam Position Counter` (`VPOSR` read-only register bit assignments: Bit 15 LOF long-frame bit, Bits 14-1 unused, Bit 0 high vertical bit V8) into a clean structured table.
99. **Complex Spanned HTML Interrupt Priority Table (Page 154 / HRM Page 234):** Verifying conversion of the hardware interrupt priority table (Level 1–6 interrupt levels, mask bits, handler addresses, and serial/disk interrupt bit mappings) into a semantic HTML table with multi-row spans (`rowspan`).
100. **Dual Controller Port Specification Tables (Page 155 / HRM Page 241):** Verifying conversion of `Table 8-1: Typical Controller Connections` (pinout mapping for Joystick, Mouse/Trackball, Proportional Pair) and `Table 8-2: Controller Port Register Bit Allocations` as two distinct structured Markdown tables on a single page.
101. **Multi-Page Disk Subsystem Control Table Fusion (Pages 156 & 157 / HRM Pages 256 & 257):** Verifying that `Table 8-5: Disk Subsystem` (8520 CIA-A/CIA-B input/output control and sensing lines: `PA5 DSKRDY*`, `PB6-PB3 DSKSEL*`, `DSKMTR*`, `DSKSTEP*`, etc.) spanning across consecutive pages is transcribed per-page and fused in Stage 21 into **ONE unified continuous table**, stripping redundant headers.
102. **Keyboard Scan Matrix Graphics Anti-Table Image Regression (Page 158 / HRM Page 267):** Verifying that two distinct keyboard matrix and keycode layout schematics on Page 267 are **strictly classified as visual `image` crops** (`detected_type: "image"`), deferred to Stage 16, and NEVER converted into tables.
103. **Multi-Page Serial & Audio Control Register Table Fusion / Disaggregation (Pages 159 & 160 / HRM Pages 270 & 271):** Verifying handling of `Table 8-9: SERDATR / ADKCON Registers` spanning pages 270 and 271, evaluating whether the register mappings are unified into a single table or cleanly separated into dedicated per-register tables (`SERDATR` and `ADKCON`).
104. **Appendix A Continuous Hardware Register Listing Code Blocks (Pages 161–163 / HRM Pages 277–279):** Verifying that the 3-page continuous custom chip register address list (ADKCON, AUDxDAT, AUDxLCH, BLTCON0, BLTCON1, etc.) is formatted cleanly as a continuous fenced code block (` ```text ` or ` ```assembly `), preserving column alignment for address offsets, read/write flags, and target chips.
105. **Game Port / Mouse Register Mapping Formatting (Page 164 / HRM Page 293):** Verifying that `JOY0DAT` and `JOY1DAT` register definitions and bitfield mappings are cleanly formatted in structured code blocks or compact Markdown tables.
106. **Three-Page Custom Chip Register Address Map Table Fusion (Pages 165–167 / HRM Pages 301–303):** Verifying that Appendix B `Complete Custom Chip Register Map by Address` (`NAME | ADD | R/W | CHIP | FUNCTION`) spanning 3 consecutive pages is fused in Stage 21 into **ONE single continuous table**, suppressing repeated page headers.
107. **Agnus Chip Hardware Pinout Table (Page 168 / HRM Page 308):** Verifying conversion of Appendix C `AGNUS PIN ASSIGNMENT` (`PIN # | DESIGNATION | FUNCTION | DEFINITION`) into a clean, comprehensive Markdown table detailing all 84/68 Agnus package pins.
108. **Hardware Architecture Memory Map Table (Page 169 / HRM Page 312):** Verifying conversion of Appendix D `Amiga Hardware Architecture Memory Map` (address ranges `$000000`–`$FFFFFF`, Chip RAM, Auto-Config space, Custom Register Space `$DFF000`, CIA spaces) into a structured Markdown table.
109. **Parallel Interface Pinout & ASCII Timing Diagram Code Blocks (Pages 170 & 171 / HRM Pages 320 & 321):** Verifying that Appendix E DB25 parallel port pin definitions and the character-based timing diagram (`PARALLEL CONNECTOR INTERFACE TIMING, OUTPUT CYCLE` showing strobe pulses, data setup $T_1$, and handshake signals) are cleanly preserved as formatted monospaced code blocks (`` ```text ``).
110. **8520 CIA-A Register Address Map Table (Page 172 / HRM Page 336):** Verifying conversion of Appendix F `CIAA Address Map` (`Byte Address | Register Name | Data bits 7 6 5 4 3 2 1 0`) into a structured 10-column table.
111. **CIA Control Register CRA/CRB Bitfield Map Code Block (Page 173 / HRM Page 346):** Verifying formatting of Appendix F `BIT MAP OF REGISTER CRA` (`UNUSED`, `SPMODE`, `INMODE`, `LOAD`, `RUNMODE`, `OUTMODE`, `PBON`, `START`) as a structured code block preserving register bit labels.
112. **Expansion Architecture Auto-Config Nibble Diagrams as Code Blocks (Page 174 / HRM Page 356):** Verifying that Appendix G Auto-Config board offset diagrams (`$00/02`, nibbles at `$E80000`, bit brackets, and inversion descriptions) are transcribed as clean ASCII/monospaced code blocks (`` ```text ``), avoiding broken tables.
113. **Keyboard Serial Communications Handshake Waveform (Page 175 / HRM Page 362):** Verifying that Appendix H serial communications protocol description and KCLK/KDAT timing handshake diagrams are cleanly isolated as visual `image` crops or monospaced timing code blocks.
114. **Two-Page Complex Spanned Keyboard Matrix HTML Table Fusion (Pages 176 & 177 / HRM Pages 368 & 369):** Verifying that the extensive 2-page Appendix H `Matrix Table` (Row 5–0 mapped across Bit 7–0 columns, raw scan codes, and special key assignments) is transcribed as a semantic HTML table with complex multi-row spans, and fused in Stage 21 into **ONE unified continuous table**.
115. **Index Page Processing & Final Omission Rule (Page 178 / HRM Page 391):** Verifying that the printed index page (`Index 373`) is processed cleanly as text through Stages 5–19, and systematically stripped/omitted during final chapter assembly in Stage 20/21.

---

---

## 2. Page Mapping & Test Objectives Matrix

### Source Manual Breakdown
Test pages in this composite test book are sourced from four reference manuals:
- **Source Manual 1:** `68000 User's Manual`
  - **Source PDF Path:** `68000 User's Manual/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8.pdf` (Total 216 pages)
  - **Test Pages:** 1–37 (37 pages)
  - **Section Distribution Across Test Book:**
    - **Front Matter (Cover, Quick Ref, Title, Form, TOC, Lists):** Test Pages 1–14 (Source PDF Pages 1, 2, 4, 5, 6, 7, 8, 9, 12, 13, 14, 15, 16, 17 | Folios: Cover, Quick Ref, `iii`, `iv`, `v`, `vi`, `vii`, `viii`, `xi`, `xii`, `xiii`, `xiv`, `xv`, `xvi`)
    - **Section 1 (Overview):** Test Pages 15–16 (Source PDF Pages 18, 19 | Folios: `1-1`, `1-2`)
    - **Section 2 (Data Organization & Addressing Modes):** Test Pages 17–25 (Source PDF Pages 23, 24, 26, 27, 28, 29, 32, 33, 35 | Folios: `2-2`, `2-3`, `2-5`, `2-6`, `2-7`, `2-8`, `2-11`, `2-12`, `2-14`)
    - **Section 3 (Signal Description):** Test Pages 26–29 (Source PDF Pages 36, 39, 41, 44 | Folios: `3-1`, `3-4`, `3-6`, `3-9`)
    - **Section 4 (8-Bit Bus Operation):** Test Page 30 (Source PDF Page 47 | Folio: `4-2`)
    - **Section 5 (16-Bit Bus Operation & Timing):** Test Pages 31–33 (Source PDF Pages 59, 62, 70 | Folios: `5-6`, `5-9`, `5-17`)
    - **Section 6 (Exception Processing):** Test Pages 34–35 (Source PDF Pages 96, 112 | Folios: `6-3`, `6-19`)
    - **Section 7 (Instruction Execution Times):** Test Page 36 (Source PDF Page 115 | Folio: `7-2`)
    - **Index (Subject Index):** Test Page 37 (Source PDF Page 208 | Folio: `INDEX-1`)
- **Source Manual 2:** `68000 Programmer's Reference Manual`
  - **Source PDF Path:** `68000 Programmer's Reference Manual/M68000PRM.pdf` (Total 646 pages)
  - **Test Pages:** 38–63 (26 pages)
  - **Section Distribution Across Test Book:**
    - **Section 1 (Architectural Summary):** Test Pages 38–43 (Source PDF Pages 13, 16, 22, 23, 29, 32 | Folios: `1-2`, `1-5`, `1-11`, `1-12`, `1-18`, `1-21`)
    - **Section 2 (Addressing Capabilities):** Test Pages 44–46 (Source PDF Pages 42, 46, 65 | Folios: `2-1`, `2-5`, `2-24`)
    - **Section 3 (Instruction Set Summary):** Test Pages 47–49 (Source PDF Pages 80, 97, 104 | Folios: `3-9`, `3-26`, `3-33`)
    - **Section 4 (Integer Instructions):** Test Pages 50–57 (Source PDF Pages 105, 106–111, 112 | Folios: `4-1`, `4-2`–`4-7`, `4-8`)
    - **Section 5 (Floating-Point Instructions):** Test Page 58 (Source PDF Page 338 | Folio: `5-36`)
    - **Appendix A (Instruction Set Summary):** Test Pages 59–61 (Source PDF Pages 597, 598, 603 | Folios: `A-1`, `A-2`, `A-7`)
    - **Appendix B (Exception Stack Frames):** Test Pages 62–63 (Source PDF Pages 632, 639 | Folios: `B-5`, `B-12`)
- **Source Manual 3:** `A500 A2000 Technical Reference Manual`
  - **Source PDF Path:** `A500 A2000 Technical Reference Manual/Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore.pdf` (Total 308 pages)
  - **Test Pages:** 64–102 (39 pages)
  - **Section Distribution Across Test Book:**
    - **Section 1 (Introduction & System Overview):** Test Pages 64–68 (Source PDF Pages 5, 7, 13, 14, 15 | Folios: `2`, `4`, `10`, `11`, `12`)
    - **Section 3 (Expansion Architecture & System Bus):** Test Pages 69–77 (Source PDF Pages 20, 21, 29, 30, 31, 42, 43, 44, 45 | Folios: `17`, `18`, `26`, `27`, `28`, `39`, `40`, `41`, `42`)
    - **Section 4 (PC/XT Emulator, Bridgeboard & Janus Software):** Test Pages 78–89 (Source PDF Pages 109, 110, 113, 124, 126, 131, 135, 138, 139, 151–152, 155 | Folios: `109`, `110`, `113`, `124`, `126`, `131`, `135`, `138`, `139`, `151`–`152`, `155`)
    - **Section 5 (Hard Disk Controller):** Test Pages 90–91 (Source PDF Pages 178, 184 | Folios: `178`, `184`)
    - **Section 6 (Custom Chip Register Reference):** Test Pages 92–96 (Source PDF Pages 201, 202, 205, 207, 211 | Folios: `201`, `202`, `205`, `207`, `211`)
    - **Section 7 (Hardware Equations & Motherboard Jumpers):** Test Pages 97–98 (Source PDF Pages 228, 233 | Folios: `228`, `233`)
    - **Appendix E (Hardware Schematics & Connector Pinouts):** Test Pages 99–102 (Source PDF Pages 235, 237, 247, 248 | Folios: `235`, `237`, `247`, `248`)
- **Source Manual 4:** `Hardware Reference Manual`
  - **Source PDF Path:** `Hardware Reference Manual/Commodore_Amiga_Hardware_Reference_Manual_2nd.pdf` (Total 405 pages)
  - **Test Pages:** 103–160 (58 pages)
  - **Section Distribution Across Test Book:**
    - **Section 1 (Introduction):** Test Pages 103–104 (Source PDF Pages 26, 28 | Folios: `8`, `10`)
    - **Section 2 (Coprocessor Hardware):** Test Pages 105–108 (Source PDF Pages 35, 37, 42, 43 | Folios: `17`, `19`, `24`, `25`)
    - **Section 3 (Playfield Hardware):** Test Pages 109–115 (Source PDF Pages 57, 58, 71, 84, 88, 90, 109 | Folios: `39`, `40`, `53`, `66`, `70`, `72`, `91`)
    - **Section 4 (Sprite Hardware):** Test Pages 116–118 (Source PDF Pages 124, 136, 145 | Folios: `106`, `118`, `127`)
    - **Section 5 (Audio Hardware):** Test Pages 119–125 (Source PDF Pages 151, 153, 155, 156, 159, 171, 176 | Folios: `133`, `135`, `137`, `138`, `141`, `153`, `158`)
    - **Section 6 (Blitter Hardware):** Test Pages 126–135 (Source PDF Pages 183, 186, 187, 191, 201, 203, 204, 206, 208, 210 | Folios: `165`, `168`, `169`, `173`, `183`, `185`, `186`, `188`, `190`, `192`)
    - **Section 7 (System Control Hardware):** Test Pages 136–137 (Source PDF Pages 229, 234 | Folios: `211`, `216`)
    - **Section 8 (Interface Hardware):** Test Pages 138–143 (Source PDF Pages 241, 256, 257, 267, 270, 271 | Folios: `223`, `238`, `239`, `249`, `252`, `253`)
    - **Appendix A (Register Summary):** Test Pages 144–146 (Source PDF Pages 277, 279, 293 | Folios: `259`, `261`, `275`)
    - **Appendix B (Complete Chip Register Map):** Test Pages 147–149 (Source PDF Pages 301, 302, 303 | Folios: `283`, `284`, `285`)
    - **Appendix C (Chip Pinouts):** Test Page 150 (Source PDF Page 308 | Folio: `290`)
    - **Appendix D (Memory Map):** Test Page 151 (Source PDF Page 312 | Folio: `294`)
    - **Appendix E (Parallel Interface):** Test Pages 152–153 (Source PDF Pages 320, 321 | Folios: `302`, `303`)
    - **Appendix F (8520 CIA Architecture):** Test Pages 154–155 (Source PDF Pages 336, 346 | Folios: `318`, `328`)
    - **Appendix G (Expansion Architecture):** Test Page 156 (Source PDF Page 356 | Folio: `338`)
    - **Appendix H (Keyboard Interface):** Test Pages 157–159 (Source PDF Pages 362, 368, 369 | Folios: `344`, `350`, `351`)
    - **Index:** Test Page 160 (Source PDF Page 391 | Folio: `373`)

### Complete 160-Page Mapping Matrix

| Test Page | Source Manual / Book | Source PDF Page | Printed Book Folio | Section / Content Description | Pipeline Test Objective |
| :---: | :--- | :---: | :---: | :--- | :--- |
| **1** | `68000 User's Manual` | Page 1 | *Unnumbered (Cover)* | Front Cover / Title Page (`M68000UM/AD`) with Motorola logo and graphic banner | **Graphics Handling:** Test detection, bounding box extraction, and classification as an image asset (`Stage 7 -> 8 -> 10 -> 16`). |
| **2** | `68000 User's Manual` | Page 2 | *Unnumbered (Quick Ref)* | Quick Reference / Side Tab Bookmark Page (Recto) | **Tab / Index Filtering:** Test handling of edge navigation tabs ("Overview", "Introduction", etc.); verify they do not produce false visual asset crops. |
| **3** | `68000 User's Manual` | Page 4 | `iii` | Title / Publication Details (`Semiconductor Technical Data`) | **Front Matter Formatting:** Title block, copyright notices, and publication revision information. |
| **4** | `68000 User's Manual` | Page 5 | `iv` *(Blank)* | Completely Blank Page | **Blank Page Handling:** Verify Stage 5/6/7 skip or cleanly record empty pages without hallucinating content or crashing. |
| **5** | `68000 User's Manual` | Page 6 | `v` | 68K FAX-IT Documentation Comments Form | **Form / Layout Handling:** Form fields, checkbox symbols, and address blocks. |
| **6** | `68000 User's Manual` | Page 7 | `vi` | Sales Offices & Literature Distribution Directory | **Multi-Column Text:** Dense multi-column lists and addresses without tabular lines. |
| **7** | `68000 User's Manual` | Page 8 | `vii` | **TABLE OF CONTENTS** (Start) | **TOC Generation & Links:** First page of paragraph numbers, titles, and target page numbers. |
| **8** | `68000 User's Manual` | Page 9 | `viii` | **TABLE OF CONTENTS (Continued)** | **TOC Continuation:** Multi-page continuation marker resolution; test Markdown anchor link generation (`[Title](#target)`). |
| **9** | `68000 User's Manual` | Page 12 | `xi` | **TABLE OF CONTENTS (Continued)** (End) | **TOC Termination:** Conclusion of Table of Contents. |
| **10** | `68000 User's Manual` | Page 13 | `xii` | **LIST OF ILLUSTRATIONS** (Start) | **List Continuation (Start):** First page of figure titles and page numbers. |
| **11** | `68000 User's Manual` | Page 14 | `xiii` | **LIST OF ILLUSTRATIONS (Continued)** | **Header Repetition:** Test recognition of repeated `(Continued)` header; verify it is marked for suppression during chapter merge. |
| **12** | `68000 User's Manual` | Page 15 | `xiv` | **LIST OF ILLUSTRATIONS (Concluded)** | **Header Conclusion:** Test recognition and omission of `(Concluded)` header during final fusion (Stage 21). |
| **13** | `68000 User's Manual` | Page 16 | `xv` | **LIST OF TABLES** (Start) | **Table List Continuation:** First page of table titles and page numbers. |
| **14** | `68000 User's Manual` | Page 17 | `xvi` | **LIST OF TABLES (Concluded)** | **Header Conclusion:** Test recognition and omission of `(Concluded)` table list header during fusion. |
| **15** | `68000 User's Manual` | Page 18 | `1-1` | **SECTION 1: OVERVIEW** (Start) | **Chapter Detection & Prose:** Stage 4 TOC/heading detection, main section header `SECTION 1 OVERVIEW`, introductory text. |
| **16** | `68000 User's Manual` | Page 19 | `1-2` | Section 1 Content (Paragraph 1.1) | **Body Prose & Features:** Technical body prose, processor comparison features, and formatting. |
| **17** | `68000 User's Manual` | Page 23 | `2-2` | **Figure 2-1:** User Programmer's Model (D0–D7, A0–A7, PC, CCR)<br/>**Figure 2-2:** Supervisor Programmer's Model Supplement (SSP A7', SR) | **Anti-Table Regression Test:** Must NEVER be converted into an HTML or Markdown table. Must be strictly classified as `image` (`detected_type: "image"`), deferred to Stage 16. |
| **18** | `68000 User's Manual` | Page 24 | `2-3` | **Figure 2-3:** Supervisor Programmer's Model Supplement (MC68010: VBR, SFC, DFC)<br/>**Figure 2-4:** Status Register with pointer trees to condition code labels | **Anti-Table Regression Test:** Must NEVER be converted into an HTML or Markdown table. Pointer leader lines and condition code callout trees cannot be reduced to tables; must remain `image`. |
| **19** | `68000 User's Manual` | Page 26 | `2-5` | **Table 2-1:** Data Addressing Modes (14 addressing modes across 6 categories, Generation formulas, Syntax column, and definition list) | **Table Conversion & LaTeX Math Reduction:** Convert in Stage 12/13; reduce to pure GFM Markdown table in Stage 15. Preserves native LaTeX math ($d_8, d_{16}, \leftarrow, \text{EA} = \dots$) without raw HTML tags. |
| **20** | `68000 User's Manual` | Page 27 | `2-6` | **Figure 2-5:** Word Organization in Memory (spatial memory map with bit columns 15..0, external hexadecimal address pointers `$000000`..`$FFFFFE`, word/byte blocks, and jagged zig-zag tear-off break lines) | **Anti-Table Regression Test:** Spatial memory address map with jagged break lines must NEVER be converted to an HTML or Markdown table. Must remain `image` (`detected_type: "image"`), deferred to Stage 16. |
| **21** | `68000 User's Manual` | Page 28 | `2-7` | **Figure 2-6:** Data Organization in Memory (composite figure with 6 data type encodings: Bit Data, Integer Data, 16-bit Word, 32-bit Long Word, 32-bit Addresses, and BCD Data with MSD/LSD nibbles) | **Anti-Table Regression Test:** Multi-part data storage encoding figure with separate bit grids, dashed word dividers, and category headers must NEVER be converted to an HTML or Markdown table. Must remain `image` (`detected_type: "image"`), deferred to Stage 16. |
| **22** | `68000 User's Manual` | Page 29 | `2-8` | **Figure 2-7:** Memory Data Organization of the MC68008 (byte-serial memory packing with hierarchical bracket groupings from byte cells to words and long words, plus vertical address direction arrows) | **Anti-Table Regression Test:** Byte-serial memory packing diagram with hierarchical bracket groupings and address direction indicators must NEVER be converted to an HTML or Markdown table. Must remain `image` (`detected_type: "image"`), deferred to Stage 16. |
| **23** | `68000 User's Manual` | Page 32 | `2-11` | **Table 2-2:** Instruction Set Summary (Sheet 1 of 4)<br/>`Opcode | Operation |
| **24** | `68000 User's Manual` | Page 33 | `2-12` | **Table 2-2:** Instruction Set Summary (Sheet 2 of 4)<br/>`Opcode | Operation |
| **25** | `68000 User's Manual` | Page 35 | `2-14` | **Table 2-2:** Instruction Set Summary (Sheet 4 of 4)<br/>`Opcode | Operation |
| **26** | `68000 User's Manual` | Page 36 | `3-1` | **Figure 3-1:** Input and Output Signals (MC68000, MC68HC000 and MC68010) (microprocessor IC chip package diagram with directional arrow pins, address/data buses, and 6 functional grouping brackets) | **Anti-Table Regression Test (IC Pinout / Signal Diagram):** Integrated circuit pinout with directional signal arrows, active-low bars, and domain brackets must NEVER be converted to an HTML or Markdown table. Must remain `image` (`detected_type: "image"`), deferred to Stage 16 for architectural breakdown and RAG indexing. |
| **27** | `68000 User's Manual` | Page 39 | `3-4` | **Section 3.2 / 3.3:** Data Bus & Asynchronous Bus Control (`Address Strobe (AS)`, `Read/Write (R/W)`, `Upper And Lower Data Strobes (UDS, LDS)`) | **Active-Low Signal & Overbar OCR Healing:** Verifies multimodal OCR repair of active-low signal overbars from visual ground truth. Prevents OCR artifacts like `Address Strobe (~)` and ensures proper representation as `` `_AS` `` / `` `R/_W` `` (or `$\overline{\text{AS}}$` / `$\text{R}/\overline{\text{W}}$`). |
| **28** | `68000 User's Manual` | Page 41 | `3-6` | Bus Arbitration Signals (`BR`, `BG`, `BGACK`), Section 3.5 Interrupt Control (`IPL0`, `IPL1`, `IPL2`), and MC68008 interrupt priority **NOTE** block | **Markdown Callout Alert & Bus Control Formatting:** Verifies numbered list formatting (conditions 1–4), interrupt control active-low overbars ($\overline{\text{IPL0}}$-$\overline{\text{IPL2}}$), and formatting of the bottom `NOTE` block into a proper GFM Markdown alert callout (`> [!NOTE]`). |
| **29** | `68000 User's Manual` | Page 44 | `3-9` | **Table 3-3:** Function Code Outputs (`FC2 | FC1 |
| **30** | `68000 User's Manual` | Page 47 | `4-2` | **Figure 4-1:** Byte Read-Cycle Flowchart<br/>**Figure 4-2:** Read and Write-Cycle Timing Diagram | **Multi-Figure Decomposition:** Verifies that two distinct figures on a single page produce two independent `<crop>` tags and separate visual image assets for Stage 16. |
| **31** | `68000 User's Manual` | Page 59 | `5-6` | **Figure 5-7:** M6800 Cycle Timing Diagram & Section 5.1.4 Prose (`refer to Appendix B M6800 Peripheral Interface`) | **Cross-Reference Internal Anchor Linking:** Verifies that prose cross-references are converted into active Markdown links (`[Appendix B M6800 Peripheral Interface](#appendix-b-m6800-peripheral-interface)`). |
| **32** | `68000 User's Manual` | Page 62 | `5-9` | **Figure 5-10:** CPU Space Address Encoding (bit ranges 31-20, 19-16, 15-0 with dotted dividers and callout bracket) | **Anti-Table Regression (CPU Space Encoding):** Verifies that structural bus address encoding diagrams with dotted dividers and callout brackets are classified as `image` and never forced into tables. |
| **33** | `68000 User's Manual` | Page 70 | `5-17` | **Figure 5-18:** Bus Arbitration Unit State Diagrams (graphic state bubbles/arcs with bottom text legend & notes) | **Figure Crop Isolation vs. Native Legend/Notes Text:** Verifies that the crop isolates only the state machine graphs, while the abbreviation legend (`R = Bus Request Internal...`) and notes remain selectable Markdown text. |
| **34** | `68000 User's Manual` | Page 96 | `6-3` | **Table 6-1:** Reference Classification with footnote (`*Address space 3 is reserved...`) | **HTML Table Spans & Native Footnote Text:** Verifies conversion into semantic HTML table with `colspan`/`rowspan`, while the under-table footnote is preserved as native Markdown text outside the table. |
| **35** | `68000 User's Manual` | Page 112 | `6-19` | **Figure 6-9:** Special Status Word Format (16-bit register word with bit fields `RR`, `IF`, `DF`, `RM`, `HB`, `BY`, `RW`, `FC2-FC0` and bit positions 15..0, followed by bit definition list) | **Anti-Table Regression Test (Register Word Format):** Verifies that a register bitfield diagram captioned as a figure is strictly classified as `image` (`detected_type: "image"`), deferred to Stage 16 for architectural breakdown, and never converted into an HTML or Markdown table. |
| **36** | `68000 User's Manual` | Page 115 | `7-2` | **Table 7-1:** Effective Address Calculation Times<br/>**Table 7-2:** Move Byte Instruction Execution Times (`Destination` spanning 9 columns, `Source` modes) | **HTML Tables with Preserved Cell Formatting:** Verifies conversion into HTML tables preserving cell parentheses, slashes (`0(0/0)`), addressing mode notations (`(xxx).W`), and escaping `#&lt;data&gt;`; protected from reduction by Stage 15 gatekeeper. |
| **37** | `68000 User's Manual` | Page 208 | `INDEX-1` | **INDEX** (Sheet 1 of 5, INDEX-1) (Letters -A-, -B-, -C-, -D- in 2 columns) | **Index Processing & Final Omission:** Process as single-column text without image crops in Stages 7–19; systematically strip and exclude from final publication chapters in Stage 20/21. |
| **38** | `68000 Programmer's Reference Manual` | Page 13 | `1-2` | **Figure 1-1:** M68000 Family User Programming Model (D0–D7, A0–A7, PC, CCR, FP0–FP7, FPCR, FPSR, FPIAR grouped by large category brackets) | **Anti-Table Regression Test (PRM Architecture Model):** Processor programming model with multi-tier register groups and external grouping brackets must NEVER be converted to an HTML or Markdown table. Must be strictly classified as `image` (`detected_type: "image"`), deferred to Stage 16 for architectural breakdown. |
| **39** | `68000 Programmer's Reference Manual` | Page 16 | `1-5` | **Figure 1-3:** Floating-Point Control Register (16-bit register word with 10 2D branching pointer leader lines connecting bit cells to condition/mode callout labels) | **Anti-Table Regression Test (2D Pointer Tree):** Register bit diagram with 2D branching pointer trees and leader lines connecting bit cells to external mode/exception callout labels must NEVER be converted to an HTML or Markdown table. Must be strictly classified as `image` (`detected_type: "image"`), deferred to Stage 16. |
| **40** | `68000 Programmer's Reference Manual` | Page 22 | `1-11` | **Figure 1-8:** Status Register (system/user bytes with pointer callouts) and two lookup tables (`TRACE MODE` & `ACTIVE STACK`) | **Composite Graphic & Table Disaggregation:** Verifies that the composite figure is split into an `image` crop for the status register pointer diagram and two clean GFM Markdown tables for the data lookup grids. |
| **41** | `68000 Programmer's Reference Manual` | Page 23 | `1-12` | **Figure 1-9:** MC68030 Transparent Translation/MC68EC030 Access Control Register Format (32-bit register grid: bits 31..0, subfields `ADDRESS BASE`, `ADDRESS MASK`, `E`, `CI`, `R/W`, `RWM`, `FC BASE`, `FC MASK`) | **Bounded Register Bitfield Conversion:** Verifies that a 32-bit register bitfield format is converted to a semantic HTML table (`table_html` in Stage 14) with `colspan` across the bit positions and field names. |
| **42** | `68000 Programmer's Reference Manual` | Page 29 | `1-18` | **Figure 1-13:** Normalized Number Format<br/>**Figure 1-14:** Denormalized Number Format | **Anti-Table Regression Test (Real Number Formats):** Verifies that real format bit specification diagrams are classified strictly as `image` (`detected_type: "image"`), deferred to Stage 16. |
| **43** | `68000 Programmer's Reference Manual` | Page 32 | `1-21` | **Table 1-4:** Single-Precision Real Format Summary Data Format (dense data table with embedded 32-bit `s | e |
| **44** | `68000 Programmer's Reference Manual` | Page 42 | `2-1` | **Figure 2-1:** Instruction Word General Format (stacked instruction word boxes with bit numbers `15` on left and `0` on right) | **Instruction Format HTML Table:** Verifies conversion into a semantic HTML table with header row representing bit boundaries `15` and `0` spanning stacked composition rows. |
| **45** | `68000 Programmer's Reference Manual` | Page 46 | `2-5` | **Paragraphs 2.2.1, 2.2.2, 2.2.3:** Data Register Direct, Address Register Direct, Address Register Indirect Modes | **Multi-Section Crop Isolation vs. Native Paragraph Prose:** Verifies that the introductory text for each of the 3 addressing mode sections is preserved as native Markdown prose, while three independent `image` crops are extracted for the register/memory schematics. |
| **46** | `68000 Programmer's Reference Manual` | Page 65 | `2-24` | **Figure 2-5:** No Memory Indirect Action (upper 4-column lookup table `BR | Xn |
| **47** | `68000 Programmer's Reference Manual` | Page 80 | `3-9` | **Table 3-5:** Shift and Rotate Operation Format (instruction operation table featuring graphical bit-shift cartoons and flip-flop routing schematics in `Operation` column) | **Complex Graphical Table Bailout to Image:** Verifies that despite the `Table` caption, the asset is bailed out to `image` due to visual bit-shift schematics, and Stage 16 generates an exhaustive architectural breakdown in Markdown and RAG text. |
| **48** | `68000 Programmer's Reference Manual` | Page 97 | `3-26` | **Figure 3-2:** Rounding Algorithm Flowchart (multi-branch algorithmic flowchart with decision diamonds and rounding mode paths) | **Algorithmic Flowchart Bailout to Image:** Verifies that algorithmic decision flowcharts are classified strictly as `image` (`detected_type: "image"`), deferred to Stage 16. |
| **49** | `68000 Programmer's Reference Manual` | Page 104 | `3-33` | **Figure 3-3:** Instruction Description Format (standard typographic template and documentation guide with field callouts) | **Anti-Table Regression Test (Structural Template):** Documentation format guide showing instructional layout brackets must remain `image` (`detected_type: "image"`), deferred to Stage 16. |
| **50** | `68000 Programmer's Reference Manual` | Page 105 | `4-1` | **Section 4: Integer Instructions** (Introduction & two side-by-side addressing mode lookup tables: `(bd,An,Xn)` and `(bd,PC,Xn)`) | **Side-by-Side Table Detection:** Recognizes and isolates two side-by-side addressing mode tables at bottom of page into separate, correctly formatted tables. |
| **51** | `68000 Programmer's Reference Manual` | Page 106 | `4-2` | **ABCD Instruction** (Add Decimal with Extend - syntax, operation, operand sizes) | **Instruction Entry Transcription:** Standard instruction specification layout. |
| **52** | `68000 Programmer's Reference Manual` | Page 107 | `4-3` | **ABCD Instruction (Cont.)** (Instruction fields, condition codes, effective address modes) | **Instruction Entry Continuation:** Intermediate instruction specification continuation. |
| **53** | `68000 Programmer's Reference Manual` | Page 108 | `4-4` | **ADD Instruction** (Add - Syntax: `ADD <ea>, Dn`, `ADD Dn, <ea>`, Operation: `Source + Destination -> Destination`, Attributes) | **Multi-Page Instruction Fusion (Start):** First page of 3-page `ADD` instruction specification; establishes base instruction header `## ADD`. |
| **54** | `68000 Programmer's Reference Manual` | Page 109 | `4-5` | **ADD Instruction (Cont.)** (Effective address encoding tables for `ADD <ea>, Dn` and `ADD Dn, <ea>`) | **Multi-Page Instruction Fusion (Cont.):** Intermediate instruction specification. Redundant running headers and duplicate `ADD` banners must be suppressed during Stage 21 fusion. |
| **55** | `68000 Programmer's Reference Manual` | Page 110 | `4-6` | **ADD Instruction (Concl.)** (Condition codes, instruction register encoding fields `Opmode`, `Register`, `Effective Address`) | **Multi-Page Instruction Fusion (End):** Conclusion of `ADD` instruction. Verified clean fusion into one unbroken instruction entry in `build/02_final_chapters/`. |
| **56** | `68000 Programmer's Reference Manual` | Page 111 | `4-7` | **ADDA Instruction** (Add Address - Syntax: `ADDA <ea>, An`, Operation: `Source + Destination -> Destination`, Instruction Format) | **Instruction Entry Boundary:** Verifies clean transition from `ADD` to subsequent instruction `ADDA`. |
| **57** | `68000 Programmer's Reference Manual` | Page 112 | `4-8` | **ADDA Instruction (Cont.)** (Effective addressing mode tables printed in 4 side-by-side column blocks: 2 for base modes, 2 for 68020+) | **Side-by-Side Column Unrolling:** Unrolls 4 space-saving side-by-side printed table blocks into 2 continuous single-column tables (base modes and 68020+ modes). |
| **58** | `68000 Programmer's Reference Manual` | Page 338 | `5-36` | **FDIV Instruction** (Divide - Operation Table with dual left/right text alignment within single column) | **Dual-Alignment HTML Table:** Transcribes `FDIV` Operation Table into a semantic HTML table preserving dual left/right text and sign alignments within single columns. |
| **59** | `68000 Programmer's Reference Manual` | Page 597 | `A-1` | **Appendix A / Table A-1:** M68000 Family Instruction Set (Sheet 1 of 7: `Instruction | M68000..68040 |
| **60** | `68000 Programmer's Reference Manual` | Page 598 | `A-2` | **Table A-1:** M68000 Family Instruction Set (Sheet 2 of 7: `BSR` through `DIVSL`) | **7-Page Appendix Table Unification (Cont.):** Second sheet; suppresses repeated running headers and intermediate column names across continuation markers. |
| **61** | `68000 Programmer's Reference Manual` | Page 603 | `A-7` | **Table A-1:** M68000 Family Instruction Set (Sheet 7 of 7: `RTS` through `UNLK`) | **7-Page Appendix Table Unification (End):** Final sheet concluding Table A-1. Verified seamless fusion into one unified table entity in `build/02_final_chapters/`. |
| **62** | `68000 Programmer's Reference Manual` | Page 632 | `B-5` | **Figure B-7:** MC68EC040/LC040 Floating-Point Unimplemented Stack Frame<br/>**Figure B-8:** MC68040 Access Error Stack Frame | **Anti-Table Regression (Exception Stack Frames):** Stack frame layout diagrams with byte offset callouts (`SP`, `+$02`, `+$06`...) must remain `image` (`detected_type: "image"`), deferred to Stage 16. |
| **63** | `68000 Programmer's Reference Manual` | Page 639 | `B-12` | **Figure B-21:** MC68040 Idle Stack Frame<br/>**Figure B-22:** MC68040 Unimplemented Instruction Stack Frame | **Anti-Table Regression (Idle/Unimplemented Stack Frames):** Stack frame diagrams showing internal CPU state blocks with byte offset labels (`$00`, `$04`, `$08`...) must remain `image` (`detected_type: "image"`), deferred to Stage 16. |
| **64** | `A500 A2000 Technical Reference Manual` | Page 5 | `2` | **Figure 1.1:** Key Codes (Commodore Amiga 500/2000 physical keyboard layout diagram with matrix key positions and raw scan codes `$5A`, `$5B`, etc.) | **Keyboard Diagram Image Classification:** Verify that physical hardware keyboard layout maps with scan codes are classified strictly as `image` (`detected_type: "image"`), deferred to Stage 16, and never forced into an HTML table. |
| **65** | `A500 A2000 Technical Reference Manual` | Page 7 | `4` | Parallel Port Centronics DB25 connector diagram printed side-by-side with DB25 pin comparison table (`PIN | A1000 |
| **66** | `A500 A2000 Technical Reference Manual` | Page 13 | `10` | **Table 1-1:** RAW KEY CODES (Sheet 1 of 3: Codes `$00`–`$2B` with Key, Raw, Unshifted, Shifted, Caps Lock, Ctrl, Alt) | **3-Page Key Code Table Unification (Start):** First sheet of 3-page Amiga raw key code translation table; cropped and converted in Stages 7–15; establishes base table for Stage 21 fusion. |
| **67** | `A500 A2000 Technical Reference Manual` | Page 14 | `11` | **Table 1-1:** RAW KEY CODES (Sheet 2 of 3: Codes `$2C`–`$57`) with intermediate footnotes (`*In shifted Forward Arrow...` and `<CSI> stands for...`) | **3-Page Key Code Table Unification & Footnote Relocation:** Second sheet; continuous rows merged across page boundary; intermediate footnotes are extracted and relocated to the very end of the unified table following Sheet 3. |
| **68** | `A500 A2000 Technical Reference Manual` | Page 15 | `12` | **Table 1-1:** RAW KEY CODES (Sheet 3 of 3: Codes `$58`–`$7F` concluded) | **3-Page Key Code Table Unification (End):** Final sheet concluding Table 1-1; verifies seamless fusion into a single consolidated table with relocated footnotes at the bottom in `build/02_final_chapters/`. |
| **69** | `A500 A2000 Technical Reference Manual` | Page 20 | `17` | Section 3.1 Designing Hardware for the Amiga Expansion Architecture with explicit bottom **WARNING** block | **GFM Alert Warning Formatting:** Verify that explicit danger/warning blocks are transformed into standardized GitHub Flavored Markdown alert blockquotes (`> [!WARNING]`). |
| **70** | `A500 A2000 Technical Reference Manual` | Page 21 | `18` | **Figure 3.1:** Expansion Architecture Overview and explicit top **NOTE** block | **GFM Alert Note Formatting & Diagram:** Verify that note advisory blocks are transformed into standardized GitHub Flavored Markdown alert blockquotes (`> [!NOTE]`), and architectural overview diagrams remain `image`. |
| **71** | `A500 A2000 Technical Reference Manual` | Page 29 | `26` | Auto-Config Notes & Address Specification Table: nibbles `(00/02)` board type and memory size definitions | **Bit Descriptions as Structured Text:** Verify that register bitfields and memory size definitions with bit masks (`000 = 8MB`...`111 = 4MB`) are represented as clean native Markdown text / structured lists rather than visual crops. |
| **72** | `A500 A2000 Technical Reference Manual` | Page 30 | `27` | Auto-Config Registers: nibbles `(04/06)` through `(3C/3E)` product numbers, mfg numbers, serial numbers, ROM vectors | **Bit Structures & Horizontal Line Text:** Verify that register bit structures (`7654 3210`) with horizontal lines and mixed descriptions are formatted as structured text / monospaced blocks, never forced into graphics or tables. |
| **73** | `A500 A2000 Technical Reference Manual` | Page 31 | `28` | Auto-Config CSR, Base Address, Shut-Up address (text) and Reserved Address Table `(50/52)`–`(7C/7E)` | **Hybrid Text & Reserved Table Conversion:** Verify that upper register descriptions remain structured text while the repetitive reserved register block at the bottom (`(50/52)`–`(7C/7E)`) is converted into a clean Markdown table. |
| **74** | `A500 A2000 Technical Reference Manual` | Page 42 | `39` | A2000 System Bus Loading: Standard Load and Drive Values Table + System Bus Signal Drive Table (Start) | **Dual Table Detection on Single Page:** Verify detection of two distinct tables on a single page: Standard load/drive table (Table 1) and signal drive table start (Table 2). |
| **75** | `A500 A2000 Technical Reference Manual` | Page 43 | `40` | System Bus Signal Drive Table (Continuation: `/EINT7` through `/XCLKEN`) | **Multi-Page Bus Signal Table Fusion:** Verify that Table 2 from Page 87 and the continuation on Page 88 are fused into **ONE single unified table** in Stage 21, suppressing duplicate column headers (`Named Signals |
| **76** | `A500 A2000 Technical Reference Manual` | Page 44 | `41` | `TABLE 3-2 PAL16L8 STEERING1SOR17 REV3` Amiga 2000 Steering Logic Boolean Equations & Pinout | **PAL Logic Equations in Code Block:** Verify that PAL boolean equations and signal definitions are formatted in a fenced code block (` ```text ` or ` ```pal `), NEVER as an HTML table or image crop. |
| **77** | `A500 A2000 Technical Reference Manual` | Page 45 | `42` | `TABLE 3-3 PAL16R6 ARBITRATE REV1` Amiga 2000 Bus Arbitration PAL Equations & Pinout | **PAL Arbitration Equations in Code Block:** Verify that registered PAL equations (`BG1 = BGIN * /BGOLD...`) are formatted in a fenced code block (` ```text ` or ` ```pal `), NEVER as an HTML table or image crop. |
| **78** | `A500 A2000 Technical Reference Manual` | Page 109 | `109` | Section 4.1 PC/XT Emulator for AMIGA 2000: Interface Memory Map Table (`Offset Address | Size |
| **79** | `A500 A2000 Technical Reference Manual` | Page 110 | `110` | Section 4.1 PC Memory and I/O Map (Table 1) + Amiga Memory Map (Table 2) | **Dual Memory Map Tables:** Verify that the two distinct memory map tables on Page 110 are converted into two separate, properly formatted tables. |
| **80** | `A500 A2000 Technical Reference Manual` | Page 113 | `113` | Section 4.1 PC/AT I/O Address to Amiga Interface Offset Map (`379`–`3BF` addresses) | **Bridgeboard I/O Map Formatting:** Evaluate whether the PC/AT port mapping is transcribed as a monospaced code block or structured tabular grid. |
| **81** | `A500 A2000 Technical Reference Manual` | Page 124 | `124` | Section 4.2 PC/XT BIOS Tele-Type Output (`AH = 0EH`) and bottom bitfield graphics (`Bits of AL` & `Bits of AH`) | **BIOS Calling Conventions & Bitfield Diagrams:** Verify input/output parameter formatting and evaluate whether bottom bitfield drawings are cropped as graphics or parsed as bitfield tables. |
| **82** | `A500 A2000 Technical Reference Manual` | Page 126 | `126` | Section 4.2 EIA DSR Entry Point / Comm Port Init (`AH = 00H`) with baud rate and status parameters | **Serial Port Calling Conventions:** Verify clean formatting of multi-parameter BIOS calling conventions and return line status bits. |
| **83** | `A500 A2000 Technical Reference Manual` | Page 131 | `131` | Section 4.3 `janus.library` API reference: `SetJanusHandler(jintnum, intserver)` with CPU registers `D0`, `A1` | **Library API Prototype & Register Bindings:** Verify that library function prototypes are formatted in backticks and register assignments (`D0`, `A1`) are documented in clean parameter lists. |
| **84** | `A500 A2000 Technical Reference Manual` | Page 135 | `135` | Section 4.4 Include Files: `janus.[h|i]` and start of `janus_i86block.i` assembly structures | **Multi-Page Assembly Listing (Start):** First page of 5-page assembly include listing (`janus_i86block.i`); verify clean formatting in ` ```assembly `. |
| **85** | `A500 A2000 Technical Reference Manual` | Page 138 | `138` | Section 4.4 Include Files: Disk error constants and conclusion of `janus_i86block.i` (`ENDC JANUS_I86BLOCK_I`) | **Multi-Page Assembly Listing (End):** Fourth page concluding `janus_i86block.i`; verifies clean code block closure before file boundary. |
| **86** | `A500 A2000 Technical Reference Manual` | Page 139 | `139` | Section 4.4 Include Files: Start of new file `janus.i` with `STRUCTURE JanusResource` and `STRUCTURE JanusAmiga` | **File Boundary Demarcation:** Verifies that the start of the new include file `janus.i` is clearly demarcated with a distinct file header and newly opened code block. |
| **87** | `A500 A2000 Technical Reference Manual` | Page 151 | `151` | Section 4.4 Include Files: `janus.h` C header start with `struct JanusMemHead`, `JanusMemChunk`, `JanusList` | **Multi-File C Code Block (Start):** First page of C structure definitions formatted in ` ```c `. |
| **88** | `A500 A2000 Technical Reference Manual` | Page 152 | `152` | Section 4.4 Include Files: Conclusion of `janus.h` and start of new C header `janus_memrw.h` | **C Header File Boundary Demarcation:** Verifies that when `janus.h` ends and `janus_memrw.h` begins, the two files are separated into distinct C code blocks. |
| **89** | `A500 A2000 Technical Reference Manual` | Page 155 | `155` | Section 4.5 PC Janus Service: INT JANUS software interrupt calling conventions (`J_GET_SERVICE`, `J_ALLOC_MEM`) | **PC Service API Calling Conventions:** Verify clean Markdown formatting for INT JANUS software interrupts with register inputs/returns (`AH`, `AL`, `ES:DI`, `DX`, `BX`). |
| **90** | `A500 A2000 Technical Reference Manual` | Page 178 | `178` | Section 5 Hard Disk Controller: `Table 5-6. Command Summary` (`Description | OpcodeNum |
| **91** | `A500 A2000 Technical Reference Manual` | Page 184 | `184` | Section 5 Hard Disk Controller: `Table 5-10. Change Command Block Address` with multi-byte DMA address spans | **HTML Table with Colspans:** Verify conversion into a semantic HTML table preserving multi-byte DMA address column spans (`A23..A16`), protected from reduction by Stage 15 gatekeeper. |
| **92** | `A500 A2000 Technical Reference Manual` | Page 201 | `201` | Section 6 Custom Chip Register Reference: `BLTCON0`/`BLTCON1` bit descriptions and Line mode table start | **Blitter Register Bit Descriptions & Table Start:** Verify register bit definitions as formatted text and start of line mode bit table. |
| **93** | `A500 A2000 Technical Reference Manual` | Page 202 | `202` | Section 6 Custom Chip Register Reference: Line draw mode bit table + footnotes + Blitter window start/size table | **Dual Custom Chip Tables & Footnotes:** Verify conversion of line draw bit table with footnotes, followed by the blitter window start/size table. |
| **94** | `A500 A2000 Technical Reference Manual` | Page 205 | `205` | Section 6 Custom Chip Register Reference: Copper instruction registers (`COP1LCH`–`COPINS`) and instruction format table | **Copper Instruction Format Table:** Verify conversion into semantic HTML table with multi-bit instruction field spans (`MOVE`, `WAIT`, `SKIP`) and comparison mask footnotes (`VE`, `HE`). |
| **95** | `A500 A2000 Technical Reference Manual` | Page 207 | `207` | Section 6 Custom Chip Reference: `DDFSTRT`/`DDFSTOP` horizontal timing tables & `DMACON`/`DMACONR` bit assignments | **Display Data Fetch Timing & Register Bitfields:** Verify transcription/conversion of `DDFSTRT` and `DDFSTOP` tables (`PURPOSE |
| **96** | `A500 A2000 Technical Reference Manual` | Page 211 | `211` | Section 6 Custom Chip Reference: `DMA Time Slot Allocation / Horizontal Line (Cont'd)` with text wrapping around diagram | **Surrounding Prose & Text-Wrap Linearization:** Verify that prose flanking the horizontal DMA slot allocation diagram is linearized: top prose -> diagram `<crop>` -> bottom continuation prose. |
| **97** | `A500 A2000 Technical Reference Manual` | Page 228 | `228` | Section 7.3 A2000 PAL Equations: `PAL20L8` Memory and DTACK Decoder for A2500 Mainboard (U26) | **PAL Equations in Fenced Code Block:** Verify that PAL20L8 equations and pin declarations (`IF (OVR) /VPA = ...`) are formatted in a fenced code block (` ```text ` or ` ```pal `), NEVER as an HTML table or image crop. |
| **98** | `A500 A2000 Technical Reference Manual` | Page 233 | `233` | Section 7.4 List of B2000 Motherboard Jumpers: `J101`, `J200`, `J301`, `J500` pin diagrams & setting descriptions | **Motherboard Jumper Pin Diagrams as Images:** Verify that jumper pin diagrams (showing pins 1, 2, 3 with shunt positions) are cropped as visual `image` crops, while surrounding setting descriptions remain selectable Markdown text. |
| **99** | `A500 A2000 Technical Reference Manual` | Page 235 | `235` | Appendix E Hardware Schematics: Amiga 2000 System Schematic Foldout (Sheet 1) | **Large-Format Hardware Schematic Extraction:** Verify that the high-resolution schematic foldout (1831x2160 pt) is cropped cleanly without perimeter loss for Stage 16 architectural breakdown and RAG summary. |
| **100** | `A500 A2000 Technical Reference Manual` | Page 237 | `237` | Appendix E Hardware Schematics: Backplane Bus Expansion Signal Interconnection Diagram & Routing Table | **Backplane Bus Expansion Interconnection:** Verify handling of expansion slot interconnects (`J1`–`J5`, `LOCAL_OWN`, `SLAVE`, `INT6`, `CONFIG_OUT`) and evaluate tabular signal interconnection grid vs schematic diagram. |
| **101** | `A500 A2000 Technical Reference Manual` | Page 247 | `247` | Appendix E Component Pinouts: `A2000 Keyboard Connector` (Text, DIN connector drawing, pinout table) | **Tri-Part Layout Linearization:** Verify sequential vertical linearization: introductory text -> connector mechanical drawing `<crop>` (image) -> `Pin Name Description` pinout table (GFM table). |
| **102** | `A500 A2000 Technical Reference Manual` | Page 248 | `248` | Appendix E Peripheral Specifications: `Amiga 500/2000 Mouse` (Mechanical dimension drawing & `CONNECTION TABLE`) | **Peripheral Drawing & Connection Table:** Verify mechanical drawing `<crop>` (image) -> `CONNECTION TABLE` (`PIN |
| **103** | `Hardware Reference Manual` | Page 26 | `8` | Section 1 Introduction: Custom chip register offsets (`hardware/custom.i` and `hardware/custom.h`) dual code examples | **Dual-Language Code Blocks:** Verify that assembly and C code examples documenting custom chip offsets are formatted as two distinct fenced code blocks (`` ```assembly `` and `` ```c ``). |
| **104** | `Hardware Reference Manual` | Page 28 | `10` | Section 1 Introduction: Special CPU register instructions, TAS warning alert, user/super mode table, and strobe strobe note | **Sandwich Alert & Table Layout:** Verify formatting of top `> [!NOTE]` (TAS warning), middle Markdown table (`| CPU |
| **105** | `Hardware Reference Manual` | Page 35 | `17` | Section 2 Coprocessor Hardware: Copper `WAIT` instruction bitfield definitions and consecutive assembly instruction examples | **Consecutive Instruction Code Blocks:** Verify that two consecutive assembly code examples (`DC.W $9601,$FF00` and `DC.W $FFFF,$FFFE`) are formatted as separate fenced code blocks with inline comments. |
| **106** | `Hardware Reference Manual` | Page 37 | `19` | Section 2 Coprocessor Hardware: Copper vertical beam position comparison table (`Instruction | Explanation`) and field alternation note |
| **107** | `Hardware Reference Manual` | Page 42 | `24` | Section 2 Coprocessor Hardware: `COMPLETE SAMPLE COPPER LIST` (Sheet 1) introductory register setup table and start of assembly listing | **Multi-Page Assembly Listing Continuation (Start):** Verify clean formatting of introductory setup table and beginning of assembly code block with proper indentation. |
| **108** | `Hardware Reference Manual` | Page 43 | `25` | Section 2 Coprocessor Hardware: `COMPLETE SAMPLE COPPER LIST` (Sheet 2) continuation of assembly listing (`COPPERLIST:`) | **Multi-Page Assembly Listing Continuation (End):** Verify seamless continuation of the assembly listing across the page seam without breaking code block formatting or losing indentation. |
| **109** | `Hardware Reference Manual` | Page 57 | `39` | Section 3 Playfield Hardware: Playfield dimensions prose and `Table 3-1: Colors in a Single Playfield` (`Number of Colors | Number of Bit-Planes`) |
| **110** | `Hardware Reference Manual` | Page 58 | `40` | Section 3 Playfield Hardware: `Table 3-2: Portion of the Color Table` (`Register Name | Contents |
| **111** | `Hardware Reference Manual` | Page 71 | `53` | Section 3 Playfield Hardware: `Table 3-9: DIWSTRT AND DIWSTOP Summary` with nominal/possible value subheaders, plus display arithmetic | **HTML Table with Spans & Native LaTeX Math:** Verify conversion of `Table 3-9` into an HTML table with `colspan="2"` subheaders, and preserve bottom display window/clock calculation formulas as native LaTeX math ($ ... $). |
| **112** | `Hardware Reference Manual` | Page 84 | `66` | Section 3 Playfield Hardware: `Table 3-12: Playfields 1 and 2 Color Registers - High-resolution Mode` with grouped columns | **Dual-Playfield Grouped Table Separation:** Verify that the side-by-side grouped `Table 3-12` is disaggregated into two distinct Markdown tables (`Playfield 1 Color Registers` and `Playfield 2 Color Registers`). |
| **113** | `Hardware Reference Manual` | Page 88 | `70` | Section 3 Playfield Hardware: Bitplane modulo data fetch with three sequential memory layout diagrams (Figures 3-16, 3-17, 3-18) | **Sequential Multi-Diagram Linearization:** Verify that three distinct memory layout diagrams separated by explanatory text paragraphs are isolated into 3 separate visual `image` crops interspersed with narrative prose. |
| **114** | `Hardware Reference Manual` | Page 90 | `72` | Section 3 Playfield Hardware: Display window horizontal and vertical start positions with two screen area diagrams (Figures 3-19, 3-20) | **Display Window Screen Region Diagrams:** Verify that two distinct display window area diagrams are isolated into 2 independent visual `image` crops with intermediate explanatory prose preserved. |
| **115** | `Hardware Reference Manual` | Page 109 | `91` | Section 3 Playfield Hardware: `Table 3-19: High-resolution Color Selection` with multi-row spans and footnotes | **High-Resolution Color Selection Table with Rowspans:** Verify conversion of `Table 3-19` into a semantic HTML table preserving multi-row vertical spans (`rowspan="4"` for `NOT USED IN THIS MODE`), with under-table footnotes (`*` and `**`) preserved outside the table. |
| **116** | `Hardware Reference Manual` | Page 124 | `106` | Section 4 Sprite Hardware: Spaceship sprite data structure, display steps (1-4), and advisory warning block | **Sprite Data Structure & Explicit Caution Alert Blockquote:** Verify assembly sprite data structure formatting in ` ```assembly `, numbered list for display steps (1-4), and bottom advisory warning block formatted as a GitHub Flavored Markdown `> [!CAUTION]` alert blockquote. |
| **117** | `Hardware Reference Manual` | Page 136 | `118` | Section 4 Sprite Hardware: Attached sprite rules, color matrix, and `Table 4-4: Data Words for First Line of Spaceship Sprite` | **Spanning Header Data Bit Table:** Verify conversion of `Table 4-4` into a structured table with a master spanning header `Pixel Number` over 16 bit columns (15 to 0) and 4 data word rows (`Line 1` to `Line 4`). |
| **118** | `Hardware Reference Manual` | Page 145 | `127` | Section 4 Sprite Hardware: `Table 4-6: Color Registers for Single Sprites` with grouped sprite rowspans and footnote | **Grouped Sprite Range Table with Footnotes:** Verify conversion of `Table 4-6` into a semantic table with multi-row spans (`rowspan="4"` for sprite ranges `0 or 1`, `2 or 3`, `4 or 5`, `6 or 7`), with the under-table footnote `* Selects transparent mode.` cleanly preserved below the table. |
| **119** | `Hardware Reference Manual` | Page 151 | `133` | Section 5 Audio Hardware: `Figure 5-2: Digitized Amplitude Values` & Waveform Data Table (`TIME | SINE |
| **120** | `Hardware Reference Manual` | Page 153 | `135` | Section 5 Audio Hardware: `Table 5-1: Sample Audio Data Set for Channel 0` (`AUD0LC` byte address mapping) & Notes | **Fenced Assembly Code Block Formatting:** Format memory offset listing as a fenced code block (`` ```assembly ``), with notes cleanly preserved underneath. |
| **121** | `Hardware Reference Manual` | Page 155 | `137` | Section 5 Audio Hardware: `Table 5-2: Volume Values` & `SETAUD0VOLUME:` Assembly Strobe Listing | **Markdown Table & Code Block:** Convert volume lookup table into a clean table; format register write in assembly code block; preserve negative decibel notes. |
| **122** | `Hardware Reference Manual` | Page 156 | `138` | Section 5 Audio Hardware: DMA Sampling Calculations & `Clock Values` Table (`NTSC | PAL |
| **123** | `Hardware Reference Manual` | Page 159 | `141` | Section 5 Audio Hardware: `Table 5-3: DMA and Audio Channel Enable Bits` & `BEGINCHAN0:` Assembly Listing | **Register Bit Table & Code Block:** Convert DMACON audio enable bit table (`Bit |
| **124** | `Hardware Reference Manual` | Page 171 | `153` | Section 5 Audio Hardware: `Table 5-6: Sampling Rate and Frequency Relationship` & Low-Pass Filter Bypass Prose | **Markdown Table & Technical Prose:** Convert sampling rate/frequency table; transcribe CIA 8520 power LED filter bypass notes as clean prose. |
| **125** | `Hardware Reference Manual` | Page 176 | `158` | Section 5 Audio Hardware: Four Waveform Sample Tables (`256 Byte Sample`, `128 Byte Sample`, `64 Byte Sample`, `32 Byte Sample`) | **Multi-Table Page Decomposition:** Decompose and transcribe page into four distinct structured tables, preserving signed byte values (-128..+127). |
| **126** | `Hardware Reference Manual` | Page 183 | `165` | Section 6 Blitter Hardware: `Figure 6-1: How Images are Stored in Memory` & DMA Channel Enable Explanations | **Memory Map Crop & Prose Flow:** Isolate Figure 6-1 memory address grid as an `image` crop; transcribe channel enable descriptions as clean Markdown prose. |
| **127** | `Hardware Reference Manual` | Page 186 | `168` | Section 6 Blitter Hardware: Blitter Function Generator Truth Table with Minterms (`A | B |
| **128** | `Hardware Reference Manual` | Page 187 | `169` | Section 6 Blitter Hardware: Minterm Logic Equations, LF Control Byte & Operator Precedence NOTE Callout | **Boolean Logic Equations & Note Alert:** Format logic equations with LaTeX math; convert explicit operator precedence `NOTE` into a GitHub Flavored Markdown `> [!NOTE]` callout. |
| **129** | `Hardware Reference Manual` | Page 191 | `173` | Section 6 Blitter Hardware: Dual Minterm Bit Pattern Alignment & Logic Combination Diagrams (`$0F`, `$C8`) | **Monospaced Code Block Formatting:** Format both minterm bit alignment diagrams at the top as clean monospaced code blocks (`` ```text ``) preserving character grids. |
| **130** | `Hardware Reference Manual` | Page 201 | `183` | Section 6 Blitter Hardware: `Table 6-2: Typical Blitter Cycle Sequence` (`USE Code in BLTCON0 | Active Channels`) |
| **131** | `Hardware Reference Manual` | Page 203 | `185` | Section 6 Blitter Hardware: `Table 6-3: BLTCON1 Code Bits for Octant Line Drawing` & Indented Octant Setup Code Block | **Octant Table & Indented Code Block:** Convert Table 6-3 into a structured table; format octant line algorithm in a fenced code block with strict indentation preservation. |
| **132** | `Hardware Reference Manual` | Page 204 | `186` | Section 6 Blitter Hardware: `REGISTER SUMMARY FOR LINE MODE` (Part 1: Preliminary setup, coordinate delta, initial register assignments) | **Line Mode Register Setup Code Block (Part 1):** Format entire register setup and decision logic in a clean fenced code block (`` ```text ``). |
| **133** | `Hardware Reference Manual` | Page 206 | `188` | Section 6 Blitter Hardware: Blitter Speed formulas ($t = \frac{n \times H \times W}{7.16}$ / $7.09$), cycle tick counts, and clock frequencies | **Blitter Speed Mathematical Expressions:** Transcribe blit execution time equations, clock frequencies, and tick counts as native LaTeX math ($ ... $). |
| **134** | `Hardware Reference Manual` | Page 208 | `190` | Section 6 Blitter Hardware: `Figure 6-9: DMA Time Slot Allocation / Horizontal Line` (sideways diagram) | **Rotated Graphic Crop Normalization:** Extract visual crop of Figure 6-9 and rotate 90° clockwise so that horizontal line DMA slot allocation is upright and legible. |
| **135** | `Hardware Reference Manual` | Page 210 | `192` | Section 6 Blitter Hardware: `Figure 6-11: Time Slots Used by a Six Bit Plane Display` & `Figure 6-12: Time Slots Used by a High Resolution Display` | **Anti-Table Regression Test (Dual Time Slot Figures):** Strictly classify both Figure 6-11 and Figure 6-12 as visual `image` crops; verify they are NEVER converted into tables. |
| **136** | `Hardware Reference Manual` | Page 229 | `211` | Section 7 System Control Hardware: `Table 7-5: Contents of the Beam Position Counter` (`VPOSR` bitfields) | **Beam Position Counter Register Table:** Convert Table 7-5 into a clean structured table detailing LOF, unused bits, and vertical position bit V8. |
| **137** | `Hardware Reference Manual` | Page 234 | `216` | Section 7 System Control Hardware: Hardware interrupt priority levels, serial/disk bits & spanned priority table | **Complex Spanned HTML Interrupt Table:** Convert hardware interrupt priority matrix with multi-row spans into a semantic HTML table. |
| **138** | `Hardware Reference Manual` | Page 241 | `223` | Section 8 Interface Hardware: `Table 8-1: Typical Controller Connections` & `Table 8-2` controller port allocations | **Dual Controller Port Tables:** Transcribe both controller pinout and register bit allocation tables cleanly as separate Markdown tables. |
| **139** | `Hardware Reference Manual` | Page 256 | `238` | Section 8 Interface Hardware: `Table 8-5: Disk Subsystem` (Sheet 1: `CIAAPRA` input signals: `PA5 DSKRDY*`, etc.) | **Multi-Page Disk Table Fusion (Sheet 1):** Transcribe first sheet of 8520 CIA disk subsystem control table; establish base header for Stage 21 fusion. |
| **140** | `Hardware Reference Manual` | Page 257 | `239` | Section 8 Interface Hardware: `Table 8-5: Disk Subsystem` (Sheet 2: `CIABPRB` output signals: `PB6-PB3 DSKSEL*`, `DSKMTR*`, etc.) | **Multi-Page Disk Table Fusion (Sheet 2):** Seamlessly fuse second sheet of disk subsystem table in Stage 21, removing duplicate headers. |
| **141** | `Hardware Reference Manual` | Page 267 | `249` | Section 8 Interface Hardware: Keyboard matrix schematics and keycode layout graphics | **Anti-Table Regression Test (Keyboard Schematics):** Strictly classify both keyboard matrix graphics as visual `image` crops; never force into tables. |
| **142** | `Hardware Reference Manual` | Page 270 | `252` | Section 8 Interface Hardware: `Table 8-9: SERDATR / ADKCON Registers` (Part 1: `SERDATR` bit definitions) | **Serial Data Register Table:** Convert SERDATR bitfield definitions (bits 15-0, OVRUN, RBF, TBE, etc.) into a structured semantic table. |
| **143** | `Hardware Reference Manual` | Page 271 | `253` | Section 8 Interface Hardware: `Table 8-9: SERDATR / ADKCON Registers` (Part 2: `ADKCON` bit definitions) | **Audio/Disk Control Register Table:** Convert ADKCON bitfield definitions into a structured semantic table, evaluating fusion with SERDATR. |
| **144** | `Hardware Reference Manual` | Page 277 | `259` | Appendix A Register Summary: Custom chip register address listings (`ADKCON`, `AUDxDAT`, `AUDxLCH`, etc.) | **Register Summary Code Block (Part 1):** Format custom chip address listings in a continuous fenced code block preserving tabular character grid. |
| **145** | `Hardware Reference Manual` | Page 279 | `261` | Appendix A Register Summary: Custom chip register address listings (`BLTCON0`, `BLTCON1`, blitter control notes) | **Register Summary Code Block (Part 3):** Conclude blitter register address specifications and operational notes cleanly. |
| **146** | `Hardware Reference Manual` | Page 293 | `275` | Appendix A Register Summary: Game controller and mouse data register specifications (`JOY0DAT`, `JOY1DAT`) | **Controller Data Register Formatting:** Format joystick and mouse coordinate registers in structured code blocks or compact Markdown tables. |
| **147** | `Hardware Reference Manual` | Page 301 | `283` | Appendix B Complete Chip Register Map: `NAME | ADD |
| **148** | `Hardware Reference Manual` | Page 302 | `284` | Appendix B Complete Chip Register Map: `NAME | ADD |
| **149** | `Hardware Reference Manual` | Page 303 | `285` | Appendix B Complete Chip Register Map: `NAME | ADD |
| **150** | `Hardware Reference Manual` | Page 308 | `290` | Appendix C Chip Pinouts: `AGNUS PIN ASSIGNMENT` (`PIN # | DESIGNATION |
| **151** | `Hardware Reference Manual` | Page 312 | `294` | Appendix D Memory Map: `Amiga Hardware Architecture Memory Map` table | **Memory Map Table:** Convert system hardware memory space table ($000000-$FFFFFF) into a clean Markdown table. |
| **152** | `Hardware Reference Manual` | Page 320 | `302` | Appendix E Parallel Interface: DB25 connector pinout specifications | **Parallel Port Pinout Code Block:** Format parallel port DB25 pin functions and Centronics differences in a clean code block. |
| **153** | `Hardware Reference Manual` | Page 321 | `303` | Appendix E Parallel Interface: `PARALLEL CONNECTOR INTERFACE TIMING, OUTPUT CYCLE` | **Timing Waveform Code Block:** Format character-based parallel timing diagram (PA<7:0>, DRDY*, T1 setup) as a monospaced code block (`` ```text ``). |
| **154** | `Hardware Reference Manual` | Page 336 | `318` | Appendix F 8520 CIA: `CIAA Address Map` (`Byte Address | Register Name |
| **155** | `Hardware Reference Manual` | Page 346 | `328` | Appendix F 8520 CIA: `BIT MAP OF REGISTER CRA` (CRA/CRB control bit definitions) | **CIA Control Register Map:** Format CRA control register bitfield layout into a structured code block or compact table. |
| **156** | `Hardware Reference Manual` | Page 356 | `338` | Appendix G Expansion: Auto-Config board offset diagrams ($00/02) and nibble callouts | **Auto-Config Nibble Diagrams Code Block:** Format board offset bit groupings and inverted nibble representations as monospaced code blocks. |
| **157** | `Hardware Reference Manual` | Page 362 | `344` | Appendix H Keyboard: Keyboard communications handshake timing and serial protocol | **Keyboard Protocol Waveform:** Isolate handshake timing diagram as an `image` crop or monospaced timing block. |
| **158** | `Hardware Reference Manual` | Page 368 | `350` | Appendix H Keyboard: `Matrix Table` (Sheet 1: Rows 5–0, keycode assignments) | **Keyboard Matrix HTML Table Fusion (Sheet 1):** Transcribe initial sheet of keyboard scan matrix; establish multi-row span header for Stage 21. |
| **159** | `Hardware Reference Manual` | Page 369 | `351` | Appendix H Keyboard: `Matrix Table` (Sheet 2: Column mapping, bit assignments, scan codes) | **Keyboard Matrix HTML Table Fusion (Sheet 2):** Conclude 2-page unified keyboard matrix HTML table with multi-row spans in Stage 21. |
| **160** | `Hardware Reference Manual` | Page 391 | `373` | Index: Alphabetical subject index (`60 Pin Edge Connector`, `68000`, `8520`, etc.) | **Index Processing & Final Omission:** Process as text without crops in Stages 7–19; systematically strip and exclude from final publication chapters in Stage 20/21. |

---

## 3. Key Pipeline Verification Focus Areas

### A. Title Cover Graphics (Page 1)
- Ensure the Motorola logo and cover artwork are isolated into a 1000x1000 `<crop>` tag during Stage 7.
- Confirm Stage 8 crops the high-resolution PNG.
- Confirm Stage 10 validates the crop without clipping text labels.
- Confirm Stage 16 produces both an architectural breakdown Markdown fragment and `.txt` RAG file.

### B. Header Continuation & Conclusion Suppression (Pages 8–17)
- **Problem Statement:** In printed technical manuals, multi-page lists repeat the main header on each page with `(Continued)` or `(Concluded)`. In continuous Markdown documents, repeating these headers every 30 lines breaks document flow.
- **Verification Rule:**
  - Stage 7 transcribes the raw list content.
  - Stage 20 inserts `<continuation-marker>` across consecutive list pages.
  - Stage 21 fuses the multi-page lists into a single continuous list, stripping the repeated running headers (`(Continued)`, `(Concluded)`).

### C. TOC Markdown Link Generation (Pages 8–12)
- Validate whether paragraph entries in the Table of Contents are transformed into valid GitHub Flavored Markdown internal anchors (e.g. `[1.1 MC68000](#11-mc68000)`).

### D. Anti-Table Regression Test: Programmer Models, Pointer Trees, Memory Maps, Data Storage Encodings & IC Pinout Diagrams (Pages 17–26/ Source Pages 23–24, 27–29, 36)
- **Problem Statement:** An overzealous table converter in Stage 12, 13, or 14 might attempt to force complex register model diagrams, status register pointer diagrams, spatial memory organization maps, multi-part data storage encoding figures, byte-serial memory packing diagrams, or integrated circuit chip pinout schematics into an HTML `<table>` or Markdown grid. This destroys the visual grouping brackets, pointer callouts, jagged break lines, directional pin arrows, and composite multi-tier alignment.
- **Verification Rule:**
  - **Figure 2-1 & Figure 2-2 / 2-3 (Programmer's Models):** These show entire CPU register hierarchies (D0–D7 32-bit registers, A0–A7 address registers, stack pointers, program counters) with external grouping brackets ("EIGHT DATA REGISTERS", "SEVEN ADDRESS REGISTERS") and internal bit subdivisions. They are high-level structural architectural figures.
  - **Figure 2-4 (Status Register):** Displays a 16-bit register with hierarchical bracket callouts ("SYSTEM BYTE", "USER BYTE") and branching vertical leader lines pointing down to condition code definitions ("TRACE MODE", "SUPERVISOR STATE", "INTERRUPT MASK", "CONDITION CODES").
  - **Figure 2-5 (Word Organization in Memory):** Displays spatial memory organization with bit tick marks (15..0), external hexadecimal address labels (`$000000`, `$000002`, `$FFFFFE`), word/byte divisions, vertical ellipsis dots, and jagged zig-zag tear-off break lines indicating omitted address spans.
  - **Figure 2-6 (Data Organization in Memory):** Displays a 6-part composite encoding diagram showing bit data, integer bytes, 16-bit words, 32-bit long words, 32-bit addresses, and BCD nibbles with separate bit grids, dashed dividers, and MSD/LSD callouts.
  - **Figure 2-7 (Memory Data Organization of the MC68008):** Displays narrow 8-bit bus memory packing with multi-tier bracket groupings connecting byte cells into high/low order words and long words, alongside vertical address flow indicators ("LOWER ADDRESSES" / "HIGHER ADDRESSES").
  - **Figure 3-1 (Input and Output Signals):** Displays a central microprocessor integrated circuit (IC) rectangular package block with incoming and outgoing directional signal arrows, active-low inverted overbars ($\overline{\text{AS}}$, $\text{R}/\overline{\text{W}}$, $\overline{\text{UDS}}$, $\overline{\text{LDS}}$, $\overline{\text{DTACK}}$, $\overline{\text{BR}}$, $\overline{\text{BG}}$, $\overline{\text{BGACK}}$, $\overline{\text{IPL0}}$-$\overline{\text{IPL2}}$, $\overline{\text{BERR}}$, $\overline{\text{RESET}}$, $\overline{\text{HALT}}$), bus width brackets (`A23-A1`, `D15-D0`), and functional domain brackets ("PROCESSOR STATUS", "MC6800 PERIPHERAL CONTROL", "SYSTEM CONTROL", "ASYNCHRONOUS BUS CONTROL", "BUS ARBITRATION CONTROL", "INTERRUPT CONTROL"). This is a hardware functional pinout schematic, NOT a data table, and must NEVER be converted to an HTML or Markdown table.
  - **Pipeline Contract:** In Stages 12, 13, and 14, these assets MUST be classified as `detected_type: "image"` with `status: "checked"` and `conversion_status: "pending"`. In Stage 16, they receive standard image links (`![Figure ...](...)`), collapsible technical breakdowns, and companion RAG `.txt` descriptions.

### E. Table Reduction & Mathematical Expressions (Page 19/ Source Page 26)
- **Problem Statement:** Technical tables with addressing mode calculations and mathematical formulas must not lose subscript formatting ($d_8, d_{16}$) or replacement arrows ($\leftarrow$), and should be converted to clean, standard GitHub Flavored Markdown tables rather than bulky HTML blocks whenever possible.
- **Verification Rule:**
  - **Stage 12 / 13:** Transcribes `Table 2-1. Data Addressing Modes` into semantic HTML (`Mode | Generation | Syntax`) with category rows (`Register Direct Addressing`, `Absolute Data Addressing`, etc.).
  - **Stage 15 Gatekeeper & Agent:** Verifies table eligibility for reduction. In Step 2, the multimodal Agent transcribes the table into clean GFM pipe syntax:
    - Native LaTeX subscripts: `$d_8$`, `$d_{16}$` (never HTML `<sub>`).
    - Replacement arrows: `$\leftarrow$` (e.g. `$An \leftarrow An - N$`, `$\text{EA} = (An), An \leftarrow An + N$`).
    - Effective address equations: `$\text{EA} = (\text{PC}) + d_{16}$`, `$\text{EA} = (An) + (Xn) + d_8$`.
    - Zero raw HTML tags (`<span>`, `<font>`, `<sub>`).
    - Explanatory definitions (`NOTES:`, `EA = Effective Address`, etc.) formatted cleanly as a Markdown bulleted list directly underneath the table.

### F. Multi-Page Spanning Table Fusion Test (Pages 23 & 25/ Source Pages 32–35)
- **Problem Statement:** In printed reference manuals, extensive reference tables (such as instruction summaries, opcode matrices, or electrical characteristics) span across multiple physical pages with repeated sheet titles (e.g., `Table 2-2. Instruction Set Summary (Sheet 1 of 4)` through `(Sheet 4 of 4)`) and repeated column header rows (`Opcode | Operation | Syntax`). If transcribed naively without fusion, the final document is fractured into disconnected table snippets separated by page breaks and duplicate headings.
- **Verification Rule:**
  - **Stages 7–11:** Transcribe and crop each page's table fragment independently as per-page visual assets.
  - **Stages 12–15:** Convert each sheet fragment into a semantic table (either HTML or GFM Markdown table fragment).
  - **Stage 18 (Embed):** Injects the converted table fragments into their respective per-page markdown documents (`page_0026-embed.md` through `page_0029-embed.md`).
  - **Stage 20 (Prepare Chapters):** Assembles consecutive pages into chapter drafts in `build/02_detect_cont_chapters/` and injects `<continuation-marker>` tags between consecutive page documents.
  - **Stage 21 (Merge Chapters):** The multimodal Stage 21 worker detects the consecutive `Table 2-2. Instruction Set Summary` sheet fragments across the continuation boundary and executes semantic fusion:
    - Fuses all 4 fragments into a single continuous table entity.
    - Strips intermediate sheet titles: `Table 2-2. Instruction Set Summary (Sheet 2 of 4)`, `(Sheet 3 of 4)`, `(Sheet 4 of 4)`.
    - Keeps the primary title `### Table 2-2. Instruction Set Summary` without the `(Sheet 1 of 4)` suffix.
    - Suppresses repeated intermediate column headers (`| Opcode | Operation | Syntax |`).
    - Merges data rows continuously from `ABCD` (Sheet 1) through `UNLK` (Sheet 4).

### G. Active-Low Bus Signals & Overbar OCR Healing (Page 27/ Source Page 39)
- **Problem Statement:** In Motorola 68000 documentation, active-low bus signals feature a printed overbar (macron): $\overline{\text{AS}}$, $\text{R}/\overline{\text{W}}$, $\overline{\text{UDS}}$, $\overline{\text{LDS}}$, etc. For Read/Write, the slash rises into an overline strictly across the `W` ($\text{R}/\overline{\text{W}}$), denoting that read cycles are active-high and write cycles are active-low. In the source PDF text layer, the scanner misrecognized the overbar as a tilde `~` (producing corrupt text like `Address Strobe (~)`), or omitted it completely (producing plain `R/W` or `UDS`).
- **Verification Rule:**
  - **Visual Ground Truth Inspection:** Stage 7 and Stage 19 models MUST cross-reference the visual page preview (`page_0031.png`) rather than blindly trusting the corrupted OCR text stream.
  - **Healed Signal Syntax:**
    - Standard Retrocomputing Notation: `` `_AS` ``, `` `R/_W` ``, `` `_UDS` ``, `` `_LDS` ``.
    - Typographical LaTeX Math Notation: `$\overline{\text{AS}}$`, `$\text{R}/\overline{\text{W}}$`, `$\overline{\text{UDS}}$`, `$\overline{\text{LDS}}$`.
  - **Zero OCR Garbage:** Absolutely no stray tildes (`Address Strobe (~)`), mangled symbols, or lost negations where an overbar is visually evident.

### H. Markdown Callout Alert Style for Advisory Notes (Page 28/ Source Page 41)
- **Problem Statement:** In technical documentation, advisory notices (e.g. `NOTE`, `WARNING`, `CAUTION`) are set apart visually (centered headers, indented margins). If transcribed as plain body prose with a floating `NOTE` word, the advisory hierarchy is lost.
- **Verification Rule:**
  - **GitHub Flavored Markdown Alert Syntax:** Explicit notes in the source publication MUST be formatted as GFM callout blockquotes:
    ```markdown
    > [!NOTE]
    > [Source note text, transcribed locally during testing.]
    ```
  - **Signal & Register Normalization:** Hardware registers, processor models (`MC68008`), and active-low bus signals (`_IPL0`, `_IPL1`, `_IPL2`, `_IPL0`/`_IPL2`) inside the callout text must be properly wrapped in backticks and normalized.
  - **Content Verification:** Check the source note locally for its description of the 48-pin variant's two interrupt inputs, shared signal connection, and supported priority levels (0, 2, 5, 7).
  - **Prohibition on Spontaneous Callouts:** Ordinary prose paragraphs must NEVER be arbitrarily converted into callout alerts; alert formatting is reserved strictly for blocks explicitly designated as notes/warnings in the source publication.

### I. Multi-Column & Multi-Row HTML Table Spans (Pages 29 & 34/ Source Pages 44 & 96)
- **Problem Statement:** Tables with grouped column headers (`colspan`) or category-spanning rows (`rowspan`) cannot be converted into pure Markdown pipe tables without breaking table structure or losing hierarchy. Furthermore, Stage 15 table reduction must not collapse or corrupt tables with spans.
- **Verification Rule:**
  - **Stage 12 / 13:** Transcribes `Table 3-3. Function Code Outputs` (`FC2 | FC1 | FC0` spanning under `Function Code Output`, and `rowspan="2"` on `Address Space Type`) and `Table 6-1. Reference Classification` into semantic HTML tables (`<table>`, `<tr>`, `<th>`, `<td>`).
  - **Stage 15 Gatekeeper:** Detects `colspan` and `rowspan` attributes and automatically rejects reduction to Markdown, keeping them as high-fidelity semantic HTML tables.
  - **Under-Table Footnotes:** Footnotes beneath the table (e.g. `*Address space 3 is reserved for user definition...` on Page 96) must NOT be captured as table rows, but placed directly beneath the table as native Markdown text.

### J. Multi-Figure Page Decomposition (Page 30/ Source Page 47)
- **Problem Statement:** When a single manual page contains multiple distinct graphical entities (e.g. an upper flowchart and a lower timing waveform), an imprecise model might group them into a single oversized crop box, or drop one of the figures.
- **Verification Rule:**
  - **Distinct Bounding Boxes:** Stage 7 must generate two separate, non-overlapping `<crop>` tags:
    1. `Figure 4-1. Byte Read-Cycle Flowchart` (upper flowchart)
    2. `Figure 4-2. Read and Write-Cycle Timing Diagram` (lower timing waveforms)
  - **Stage 8 & Stage 10:** Each crop is independently extracted to `assets/` and independently validated against perimeter ink boundaries.
  - **Stage 16:** Produces independent architectural breakdowns and RAG summaries for each figure.

### K. Cross-Reference Internal Anchor Linking (Page 31/ Source Page 59)
- **Problem Statement:** Reference manuals frequently direct readers to other sections or appendices (e.g. `(refer to Appendix B M6800 Peripheral Interface.)`). If transcribed as static plain text, navigational utility is lost.
- **Verification Rule:**
  - **Stage 7 & Stage 19:** Cross-reference phrases such as `refer to Appendix B...` or `see Section 5.1` must be transcribed as active Markdown anchor links:
    ```markdown
    (refer to [Appendix B M6800 Peripheral Interface](#appendix-b-m6800-peripheral-interface))
    ```
  - **Deterministic Slug Generation:** Heading slugs follow GitHub anchor convention (lowercase, hyphens, alphanumeric only).

### L. Anti-Table Regression for CPU Space Encodings (Page 32/ Source Page 62)
- **Problem Statement:** `Figure 5-10. CPU Space Address Encoding` shows bit partitions (31–20, 19–16, 15–0) with dotted column dividers and an external callout bracket ("CPU SPACE TYPE FIELD"). A naive classifier might mistake this for a table.
- **Verification Rule:**
  - **Stage 14 Bailout:** Classified strictly as `image` (`detected_type: "image"`).
  - **Stage 16 Breakdown:** Analyzed as a visual architecture diagram describing bus address line encoding.

### M. Figure Crop Isolation vs. Native Legend & Notes Text (Page 33/ Source Page 70)
- **Problem Statement:** `Figure 5-18. Bus Arbitration Unit State Diagrams` contains state graph bubbles and arcs, followed by an abbreviation legend (`R = Bus Request Internal...`) and numbered notes (`Notes: 1. State machine will not change...`). Including text paragraphs inside the crop destroys searchability and accessibility.
- **Verification Rule:**
  - **Tight Crop Box:** The `<crop>` tag in Stage 7 isolates ONLY the active state diagrams (bubbles and transition arcs).
  - **Native Text Transcription:** The abbreviation legend and `Notes:` block must be transcribed directly into the Markdown document as selectable body prose underneath the image link.

### N. Blank Page Handling (Page 38 / Source Page 93)
- **Problem Statement:** Blank pages in technical manuals (often used to force chapter starts onto odd-numbered recto pages) can cause OCR or inference hallucinations.
- **Verification Rule:**
  - **Stage 5 / Stage 7:** Detects empty content (aside from running headers/footers or section tab markings) and emits an empty or minimal Markdown document without hallucinating text.
  - **Stage 20 / Stage 21:** Merges smoothly across blank page boundaries without introducing artifacts.

### O. Anti-Table Regression for Register Word Formats (Figure 6-9) (Page 35/ Source Page 112)
- **Problem Statement:** In Motorola technical publications, register bitfield diagrams and status word layouts are captioned as figures (e.g. `Figure 6-9. Special Status Word Format`). If an inference model attempts to force register bit boxes, bit ranges (15..0), and definition lists into an HTML table, the visual register layout is degraded.
- **Verification Rule:**
  - **Stage 14 Bailout:** Must be classified strictly as `image` (`detected_type: "image"`, `status: "checked"`, `conversion_status: "pending"`).
  - **Stage 16 Conversion:** Receives standard image link (`![Figure 6-9...](...)`), a collapsible architectural breakdown of the bit positions, and a companion RAG `.txt` description.
  - **Pipeline Contract:** Must NEVER be converted into an HTML or Markdown table.

### P. Table Cell Text Formatting & HTML Entity Escaping (Page 36/ Source Page 115)
- **Problem Statement:** Instruction execution time tables (`Table 7-1`, `Table 7-2`) contain dense technical notation inside individual cells: parenthesized clock and cycle counts `0(0/0)`, `8(2/0)`, `48(11/1)`, addressing mode syntax `(xxx).W`, `(xxx).L`, `(d16, An)`, and immediate data `#<data>`. Unescaped `<` and `>` brackets cause browsers to parse cell text as malformed HTML tags, and naively flattened tables lose column headers spanning 9 destination modes.
- **Verification Rule:**
  - **Stage 13 HTML Conversion:** Synthesizes `Table 7-1` and `Table 7-2` with `colspan="9"` for `Destination`, maintaining exact cell notation.
  - **HTML Entity Escaping:** Immediate data `#<data>` is strictly escaped as `&lt;data&gt;` or wrapped in `<code>#&lt;data&gt;</code>` inside table cells.
  - **Stage 15 Gatekeeper Protection:** The presence of `colspan="9"` and multi-tier subheaders automatically ensures `reduced_to_markdown: False`, preserving them as high-fidelity semantic HTML tables.

### Q. Back-of-the-Book Multi-Page Subject Index (Page 37/ Source Pages 208–212)
- **Problem Statement:** The back-of-the-book subject index spans 5 pages in a 2-column layout with alphabetical division banners (`-A-`, `-B-`, etc.) and page references (e.g. `3-3, 3-4`, `5-15`). Naive processing might attempt to crop columns as visual assets, leave side-by-side multi-column lines, or leave static, unlinked page numbers.
- **Verification Rule:**
  - **Stage 4 Detection:** Automatically identified as chapter type `index` (`title: "INDEX"`, `start_page: 42`).
  - **Stage 7 Linearization:** Unrolls the 2-column layout into a single, clean vertical list top-to-bottom. Zero `<crop>` tags are generated.
  - **Active Anchor Linking:** Reference numbers (e.g. `3-3, 3-4`, `5-15`) are transformed into active internal Markdown anchor links (`[3-3](#section-33)`).
  - **Stage 20 & Stage 21 Fusion:** Assembles the 5 index sheets across `<continuation-marker>` boundaries, strips running headers (`INDEX-1` through `INDEX-5`), and fuses them into a single continuous alphabetical index.

### R. Multi-Manual Anti-Table Regression for PRM Programming Models & Pointer Trees (Pages 38 & 39/ PRM Pages 13 & 16)
- **Problem Statement:** In the *68000 Programmer's Reference Manual*, architectural diagrams such as Figure 1-1 (User Programming Model) and Figure 1-3 (Floating-Point Control Register) contain complex multi-tier register groupings, floating-point register banks (FP0–FP7), and 2D branching pointer trees connecting individual bit cells (bits 15..0) to external mode/exception callout labels (`ROUNDING MODE`, `ROUNDING PRECISION`, `UNDERFLOW`, `OVERFLOW`). If an inference model in Stage 12, 13, or 14 attempts to transcribe these into HTML tables or Markdown grids, the 2D spatial pointer topology is destroyed and the output becomes malformed or illegible.
- **Verification Rule:**
  - **Figure 1-1 (Page 47 / PRM Page 13):** Structural architectural figure with external grouping brackets ("DATA REGISTERS", "ADDRESS REGISTERS", "FLOATING-POINT DATA REGISTERS", "CONTROL REGISTERS"). Must be strictly classified as `detected_type: "image"` with `status: "checked"` and deferred to Stage 16.
  - **Figure 1-3 (Page 48 / PRM Page 16):** Register bit diagram featuring 10 2D branching leader lines pointing down to external callout labels. Explicitly prohibited from table conversion; must be classified as `detected_type: "image"` with `status: "checked"` and deferred to Stage 16.
  - **Pipeline Contract:** Neither figure may ever produce an HTML table fragment or Markdown table. Both must be converted in Stage 16 into standard Markdown image links with detailed collapsible architectural breakdowns and companion RAG text files.

### S. Composite Register & Table Disaggregation (Page 40/ PRM Page 22, Figure 1-8)
- **Problem Statement:** `Figure 1-8. Status Register` combines a visual register word diagram (with leader pointer lines from system/user bytes) and two lookup tables (`TRACE MODE` with conditions 00, 01, 10, 11, and `ACTIVE STACK` with conditions 0x, 10, 11). Grouping them into a single crop or attempting to turn the whole asset into an HTML table creates structural distortion.
- **Verification Rule:**
  - **Decomposition:** The status register word diagram with pointer lines is cropped as an `image` for Stage 16.
  - **Table Conversion:** The two lookup tables (`TRACE MODE` and `ACTIVE STACK`) are transcribed as clean, native GitHub Flavored Markdown tables directly in the document flow.

### T. Bounded 32-Bit Register Bitfield Conversion (Page 41/ PRM Page 23, Figure 1-9)
- **Problem Statement:** `Figure 1-9. MC68030 Transparent Translation/MC68EC030 Access Control Register Format` displays a bounded 32-bit word grid (bits 31..0) with explicit bit divisions (`ADDRESS BASE`, `ADDRESS MASK`, `E`, `CI`, `R/W`, `RWM`, `FC BASE`, `FC MASK`).
- **Verification Rule:**
  - **Stage 14 HTML Table:** Must be classified as `table_html` (`conversion_stage: 14`) and transcribed into a clean semantic HTML table using `colspan` attributes to align bit spans (31..24, 23..16, 15, 14..11, 10, 9, 8, 7, 6..4, 3, 2..0).

### U. Real Format Diagram Bailout to Image (Page 42/ PRM Page 29, Figures 1-13 & 1-14)
- **Problem Statement:** `Figure 1-13. Normalized Number Format` and `Figure 1-14. Denormalized Number Format` specify floating-point bit encodings with variable mantissa bit patterns and exponent range constraints.
- **Verification Rule:**
  - **Stage 14 Bailout:** Both figures are classified strictly as `image` (`detected_type: "image"`), deferred to Stage 16 for architectural breakdown and RAG text summary.

### V. Nested HTML Table Embedding (Page 43/ PRM Page 32, Table 1-4)
- **Problem Statement:** `Table 1-4. Single-Precision Real Format Summary Data Format` contains an embedded 32-bit field breakdown grid (`31 30 / 23 22 / 0` -> `s / e / f`) within the top cell under `Data Format`. Flattening this destroys the field hierarchy.
- **Verification Rule:**
  - **Stage 13 HTML Conversion:** Transcribes Table 1-4 into a semantic HTML table with a **nested HTML sub-table** (`<table>` inside the first `<td>` or `<th>`) preserving the bit span layout (`colspan` for bits 30..23 and 22..0).
  - **Stage 15 Gatekeeper:** Nested tables and rowspans automatically reject Markdown reduction, preserving the full HTML structure.

### W. Instruction Word Format Table with Bit Boundaries (Page 44/ PRM Page 42, Figure 2-1)
- **Problem Statement:** `Figure 2-1. Instruction Word General Format` displays stacked instruction word composition boxes with bit position markers `15` at the top left and `0` at the top right.
- **Verification Rule:**
  - **Stage 14 HTML Conversion:** Rendered as an HTML table with an upper header row establishing the bit boundary span (`15` ... `0`) and stacked cells for the operation word and extension word specifiers.

### X. Multi-Section Crop Isolation vs. Native Paragraph Prose (Page 45/ PRM Page 46)
- **Problem Statement:** Page 46 contains three consecutive addressing mode subsections (2.2.1 Data Register Direct Mode, 2.2.2 Address Register Direct Mode, 2.2.3 Address Register Indirect Mode). Each section has an introductory paragraph followed by an addressing diagram. If the entire section is cropped as an image, text searchability and heading hierarchy are lost.
- **Verification Rule:**
  - **Native Prose:** Section headings (`### 2.2.1 Data Register Direct Mode`, etc.) and body text paragraphs are transcribed as native Markdown prose.
  - **Crop Isolation:** Three separate, tight `<crop>` tags are generated exclusively around the graphical diagrams.

### Y. Split Table & Arithmetic Graph Decomposition (Page 46/ PRM Page 65, Figure 2-5)
- **Problem Statement:** `Figure 2-5. No Memory Indirect Action` contains an upper 4-column mode selection table (`BR | Xn | bd | Addressing Mode`) followed by an address calculation flowchart with addition bubbles and memory pointer lines.
- **Verification Rule:**
  - **Upper Table:** Transcribed as a pure GFM Markdown table.
  - **Lower Schematic:** Cropped as an `image` for Stage 16, preserving the arithmetic addition bubbles and pointer arrows.

### Z. Complex Graphical Table Bailout & Flowcharts (Pages 47 & 48/ PRM Pages 80 & 97)
- **Problem Statement:**
  - `Table 3-5. Shift and Rotate Operation Format` is titled as a table, but its `Operation` column contains complex graphical bit-shift diagrams with carry/extend flip-flop routing and arrows.
  - `Figure 3-2. Rounding Algorithm Flowchart` is an algorithmic decision flowchart.
- **Verification Rule:**
  - **Table 3-5:** Classified as `image` (deferred to Stage 16). The complex bit-shift animations and diagrams are documented via Stage 16 architectural breakdown.
  - **Figure 3-2:** Classified as `image` (deferred to Stage 16).

### AA. Instruction Description Format Structural Template Bailout (Page 49/ PRM Page 104, Figure 3-3)
- **Problem Statement:** `Figure 3-3. Instruction Description Format` provides a visual layout diagram explaining the standard typographic and structural format Motorola used across all instruction dictionary entries in Sections 4 and 5 (showing field brackets, instruction mnemonics, operand syntax placeholders, and operation definitions). An automated parser could mistake these structural boxes for an HTML table.
- **Verification Rule:**
  - **Stage 14 Bailout:** Classified strictly as `image` (`detected_type: "image"`), deferred to Stage 16.
  - **Stage 16 Conversion:** Receives standard image link (`![Figure 3-3...](...)`), an architectural breakdown documenting Motorola's instruction description template conventions, and companion RAG `.txt` description.

### AB. Side-by-Side Addressing Mode Tables (Page 50/ PRM Page 105)
- **Problem Statement:** At the bottom of Section 4's opening page, two addressing mode lookup tables (`(bd,An,Xn)` and `(bd,PC,Xn)`) are printed side-by-side across two horizontal column spans.
- **Verification Rule:**
  - **Stage 7 Transcription:** Recognizes and isolates the two side-by-side addressing mode tables, transcribing them cleanly into sequential Markdown/HTML tables rather than merging their rows into a garbled multi-column composite.

### AC. Multi-Page Instruction Specification Fusion without Header Repetition (Pages 51 & 56/ PRM Pages 106–111)
- **Problem Statement:** In Section 4 (Integer Instructions), individual instruction entries span across multiple printed pages. In particular, the `ADD` instruction begins on Page 62 (PRM Page 108 / Folio `4-4`), continues on Page 63 (PRM Page 109 / Folio `4-5`), and concludes on Page 64 (PRM Page 110 / Folio `4-6`). Each physical page repeats running headers (`ADD`, `MOTOROLA`, page folios). If merged naively, the resulting chapter document will contain repeated `## ADD` headers on each page seam and duplicate banner blocks.
- **Verification Rule:**
  - **Stage 7–11 Transcription:** Each page is transcribed cleanly with its localized content.
  - **Stage 20 Preparation:** Injects `<continuation-marker>` tags between consecutive instruction pages.
  - **Stage 21 Chapter Fusion:** The multimodal Stage 21 worker recognizes the continuous multi-page `ADD` specification:
    - Fuses all three pages (Pages 62, 63, 64) into **one single unbroken `## ADD` instruction entry**.
    - Completely suppresses redundant `ADD` banner titles, running headers, and folio marks on continuation pages (Pages 63 and 64).
    - Seamlessly sequences: Operation -> Syntax -> Attributes -> Description -> Effective Address Encoding Tables -> Condition Codes -> Instruction Fields -> Instruction Format.
    - Transitions cleanly to the next instruction (`ADDA`) on Page 65 without cross-instruction contamination.

### AD. Side-by-Side Effective Address Column Unrolling (Page 57/ PRM Page 112)
- **Problem Statement:** On Page 66 (ADDA instruction continuation), the effective address mode tables are split into 4 side-by-side column blocks (2 for base 68000 addressing modes, and 2 for 68020+ addressing modes) solely to save vertical space on the physical printed sheet. In Markdown, side-by-side tables are difficult to render legibly.
- **Verification Rule:**
  - **Unrolling Protocol:** In Stages 12–15, detect that these 4 blocks represent two logical sets of data modes that were split horizontally.
  - **Sequential Single-Column Output:** Unroll the 4 column blocks into **2 continuous single-column tables** (one for base addressing modes, one for 68020+ extended modes), preserving clear, linear vertical reading flow.

### AE. Dual Left/Right Text Alignment in Single Column HTML Tables (Page 58/ PRM Page 338, FDIV Operation Table)
- **Problem Statement:** In the `FDIV` floating-point instruction `Operation Table`, individual table cells contain dual-aligned text within the same column: mathematical signs/operands (such as `+` and `-`) are aligned strictly to the left, while numeric values or infinity symbols (such as `+0.0`, `$\infty$`, `NaN`) are aligned to the right. Flattening these cells into plain strings concatenates them ambiguously.
- **Verification Rule:**
  - **Semantic HTML Table Conversion:** Converted as a semantic HTML table (`table_html` in Stage 13/14).
  - **Dual Alignment Preservation:** Individual table cells preserve the dual alignment using clean inline styling or spans (e.g. `<span style="float: left;">+</span><span style="float: right;">+0.0</span>`) to ensure exact visual and semantic fidelity with the printed Motorola specification.

### AF. Multi-Page Spanning Appendix Table Unification (Pages 59 & 61/ PRM Pages 597–603, Table A-1)
- **Problem Statement:** Appendix A contains `Table A-1. M68000 Family Instruction Set Summary`, which spans **7 continuous printed pages** from `ABCD` on Page 68 through `UNLK` on Page 74. Each physical page repeats running headers, table titles (`Table A-1. M68000 Family Instruction Set Summary (Continued)`), and column headers (`Instruction | M68000 | M68008 | M68010 | M68020 | M68030 | M68040 | Operation`).
- **Verification Rule:**
  - **Per-Page Processing (Stages 7–19):** Each sheet fragment is cropped, converted, and embedded into its respective page markdown document.
  - **Stage 20 Preparation:** Links all 7 pages with `<continuation-marker>` tags inside `build/02_detect_cont_chapters/appendix-a.md`.
  - **Stage 21 Chapter Fusion:** The Stage 21 worker executes comprehensive multi-page table unification:
    - Fuses all 7 sheets into **ONE single continuous Markdown/HTML table**.
    - Retains only the initial title `### Table A-1. M68000 Family Instruction Set Summary` (stripping `Sheet 1 of 7` and all subsequent `(Continued)` / `(Sheet X of 7)` subtitles).
    - Completely strips intermediate repeated column header rows (`Instruction | M68000 | ...`) across all 6 continuation seams.
    - Preserves continuous alphabetical sorting and instruction row sequences from `ABCD` through `UNLK`.

### AG. Exception & Stack Frame Diagram Bailouts (Pages 62 & 63/ PRM Pages 632 & 639, Figures B-7, B-8, B-21, B-22)
- **Problem Statement:** Appendix B documents exception processing stack frame formats (`Figure B-7. MC68EC040/LC040 Floating-Point Unimplemented Stack Frame`, `Figure B-8. MC68040 Access Error Stack Frame`, `Figure B-21. MC68040 Idle Stack Frame`, `Figure B-22. MC68040 Unimplemented Instruction Stack Frame`). These show internal CPU stack frame memory blocks with stacked word boxes, hexadecimal byte offset callouts (`SP`, `+$02`, `+$06`, `+$08`, `$00`, `$04`, `$08`, `+$14`...), internal register state fields, and format/vector offset words. An automated classifier could erroneously attempt to parse the word frames into an HTML table.
- **Verification Rule:**
  - **Stage 14 Bailout:** All four figures are classified strictly as `image` (`detected_type: "image"`), deferred to Stage 16.
  - **Stage 16 Conversion:** Receives standard image links (`![Figure B-X...](...)`), detailed architectural breakdowns of stack frame layout, format codes, and vector offsets, and companion RAG `.txt` files.
  - **Pipeline Contract:** Stack frame figures must NEVER be converted to HTML or Markdown tables.

### AH. Keyboard Layout Diagram Image Bailout (Page 64/ A500 TRM Page 5, Figure 1.1)
- **Problem Statement:** Page 5 displays `Figure 1.1 Key Codes`, which illustrates the physical Commodore Amiga 500/2000 keyboard key matrix with individual key caps and hexadecimal scan code labels (`$5A`, `$5B`, `$40`, etc.). Because keys are arranged in rows and columns, a tabular converter might mistakenly attempt to represent the keyboard as an HTML table grid, losing the physical visual arrangement, irregular keycap widths, and matrix wiring.
- **Verification Rule:**
  - **Stage 14 Bailout:** Classify `Figure 1.1` strictly as `image` (`detected_type: "image"`), deferred to Stage 16.
  - **Stage 16 Conversion:** Generates a standard image link (`![Figure 1.1 Key Codes](...)`), a detailed architectural breakdown of keyboard matrices and scan code groups, and a companion RAG `.txt` document.
  - **Pipeline Contract:** Physical keyboard layout diagrams must NEVER be converted to HTML or Markdown tables.

### AI. Side-by-Side Graphic and Table Vertical Linearization (Page 65/ A500 TRM Page 7)
- **Problem Statement:** Page 7 displays a physical DB25 Centronics connector pinout illustration printed side-by-side horizontally with a DB25 pin assignment comparison table (`PIN | A1000 | A500/A2000 | PC10`). Multi-column print layouts cannot be rendered side-by-side in standard Markdown without messy inline styling or breaking responsive reading flow.
- **Verification Rule:**
  - **Sequential Linearization:** Stage 7 transcribes the layout vertically from top to bottom.
  - **Order of Precedence:** The connector illustration is extracted as a visual image `<crop>` tag first, followed immediately by the tabular comparison markup (or table `<crop>` tag) directly underneath.
  - **Stage 18 / 19 / 21 Output:** In the embedded and final chapter Markdown, the connector graphic appears first, and the Centronics pin assignment table follows cleanly underneath it.

### AJ. Multi-Page Spanning Table Unification with Intermediate Footnote Relocation (Pages 66 & 68/ A500 TRM Pages 13–15, Table 1-1)
- **Problem Statement:** `Table 1-1 RAW KEY CODES` spans **3 physical pages** (Pages 13, 14, 15). Page 14 (Sheet 2) includes two explanatory footnotes directly below its row segment (`*In shifted Forward Arrow...` and `<CSI> stands for Control Sequence Introducer...`). If footnotes remain embedded between Page 14 and Page 15, the table is fractured into two broken pieces, or footnotes become stranded inside table data cells.
- **Verification Rule:**
  - **Stages 7–19:** Each page's table rows are transcribed and converted cleanly.
  - **Stage 20 Preparation:** Links Pages 79–81 with `<continuation-marker>` tags in `build/02_detect_cont_chapters/`.
  - **Stage 21 Chapter Fusion:**
    - Merges all three sheets into **ONE unified continuous table**.
    - Extracts the intermediate footnotes on Page 14 and relocates them to the very end of the consolidated table following the conclusion of Sheet 3 on Page 15.
    - Strips repeated table headers (`Table 1-1 RAW KEY CODES (Continued)`) and column header rows across continuation seams.

### AK. Alert Blockquote Formatting for Warnings and Notes (Pages 69 & 70/ A500 TRM Pages 20 & 21)
- **Problem Statement:** Technical hardware documentation contains critical caution notices and architectural notes (`WARNING` on Page 20; `NOTE` on Page 21). When parsed as regular prose or unadorned bold text, their critical visual emphasis is lost.
- **Verification Rule:**
  - **Stage 7 & Stage 19:** Multimodal transcription identifies explicit warning/caution boxes and note blocks.
  - **GitHub Flavored Markdown Callout Syntax:** Formats warnings as `> [!WARNING]` blockquotes, and notes as `> [!NOTE]` blockquotes.
  - **Content Integrity:** Retains all warning details regarding bus contention, electrical damage, or architectural expansion constraints.

### AL. Auto-Config Register Bit Descriptions & Structures as Formatted Text (Pages 71 & 73/ A500 TRM Pages 29–31)
- **Problem Statement:** Auto-config documentation provides detailed register bit assignments, nibble addresses `(00/02)`, `(04/06)`...`(3C/3E)`, bitfields `7654 3210`, horizontal dashed dividers, and associated field definitions (memory sizes `000 = 8MB`...`111 = 4MB`, board types, manufacturer codes, product IDs, ROM vectors, serial numbers). An automated crop detector might erroneously crop these bitfields and notes as visual tables or diagram images.
- **Verification Rule:**
  - **Structured Markdown & Text Blocks:** All bit descriptions, bit masks, and horizontal divider structures must remain native, selectable text formatted as clean Markdown lists or monospaced blocks (`` ```text ``).
  - **Zero Visual Cropping:** Do NOT create `<crop>` tags for these bitfield description lists.

### AM. Reserved Address Register Block Table Conversion (Page 73/ A500 TRM Page 31)
- **Problem Statement:** At the bottom of Page 31, a repetitive sequence of 12 register offsets `(50/52)` through `(7C/7E)` is listed with bit pattern `7654 3210` and `Reserved, must be 00`. In running text, this repetitive listing is bulky and awkward.
- **Verification Rule:**
  - **Markdown Table Formatting:** Convert this repetitive offset sequence into a clean Markdown table (`| Offset | Bits | Value / Description |`).
  - **Footnote Retention:** The bottom note regarding inverted values (`FF rather than 00`) is preserved as clean Markdown text directly below the table.

### AN. Multi-Page Bus Signal Drive Loading Table Fusion (Pages 74 & 75/ A500 TRM Pages 42 & 43)
- **Problem Statement:** Page 42 contains two tables: (1) Standard Load and Drive Values Table, and (2) the start of the System Bus Signal Drive Table (`Named Signals | DIR | Expansion Slots (each) | Coprocessor Slot | Video Slot`). The Signal Drive Table continues across the page boundary onto Page 43 (`/EINT7` through `/XCLKEN`). Leaving them disjointed creates two partial tables with repeated column headers.
- **Verification Rule:**
  - **Multi-Table Detection on Page 87:** Stage 7 extracts Table 1 and Table 2 independently.
  - **Stage 21 Table Fusion:** The Stage 21 worker detects the continuation of the Signal Drive Table across the continuation boundary and fuses the rows from Page 87 and Page 88 into **ONE single unified table**, stripping the repeated column headers.

### AO. Programmable Logic Specifications & PAL Equations in Fenced Code Blocks (Pages 76 & 77/ A500 TRM Pages 44 & 45)
- **Problem Statement:** Pages 44 and 45 contain hardware logic equations for PAL devices (`TABLE 3-2 PAL16L8 STEERING1SOR17 REV3` and `TABLE 3-3 PAL16R6 ARBITRATE REV1`). Despite having "TABLE" in their printed titles, these contain pin definitions, Boolean logic equations (`DBOE = AS * /RD * /BERR + ...`), and comments. Converting them to HTML `<table>` structures or cropping them as images severely degrades their readability and searchability.
- **Verification Rule:**
  - **Fenced Code Block Syntax:** Wrap PAL equations, pinouts, and Boolean terms in clean fenced code blocks (`` ```text `` or `` ```pal ``).
  - **Never HTML / Never Crop:** Must NEVER be converted into HTML tables and must NEVER be cropped as visual images.

### AP. Dual Memory Map & Emulator Tables (Pages 78 & 79/ A500 TRM Pages 109 & 110)
- **Problem Statement:** Section 4.1 documents the PC/XT emulator for Amiga 2000. Page 109 contains the `INTERFACE MEMORY MAP` table, while Page 110 contains two separate consecutive tables: (1) `PC MEMORY AND I/O MAP` and (2) `AMIGA MEMORY MAP`. Both tables on Page 110 share similar column concepts but map addresses in opposite directions.
- **Verification Rule:**
  - **Independent Table Conversion:** Converted as separate, properly structured semantic tables.
  - **Preserved Address Offsets & Notes:** Retain all address ranges (`0000 ... 03FF`, `A0000 ... AFFFF`), sizes (`64K DISK BUFFER RAM`), access types (`B`, `W`, `G`), and mode register selection footnotes (`*`).

### AQ. Bridgeboard PC/AT I/O Address Map Formatting (Page 80/ A500 TRM Page 113)
- **Problem Statement:** Section 4.1 documents the PC/AT I/O address mapping to Amiga interface offsets (`379`–`3BF` addresses with associated usage and offset addresses `INTERFACE / Amiga`). In technical manuals, port address maps can appear as monospaced lists, pseudo-code tables, or raw alignment columns.
- **Verification Rule:**
  - **Structure Evaluation:** Stage 7 evaluates whether the address mapping is structured as a clean GitHub Flavored Markdown table (`| PC/AT I/O Address | Usage | Offset Address INTERFACE | Offset Address Amiga |`) or a formatted code block (` ```text `).
  - **Zero Crop Hallucination:** Under no circumstances should this textual port table be cropped as an image.

### AR. BIOS Calling Conventions & Bottom Bitfield Graphics (Pages 81 & 82/ A500 TRM Pages 124 & 126)
- **Problem Statement:**
  - Page 94 (TRM Page 124) documents PC/XT BIOS Tele-Type output routines (`AH = 0EH`) with CPU register calling parameters, and concludes at the bottom with two bitfield diagrams (`Bits of AL` equipment list and `Bits of AH` memory size).
  - Page 95 (TRM Page 126) documents EIA DSR Entry Point / Comm Port initialization (`AH = 00H`) with multi-parameter register inputs (baud rate, parity, stop bits) and return status bit words.
- **Verification Rule:**
  - **API Calling Convention Formatting:** Register calling conventions (`AH`, `AL`, `DX`, `ES:DI`) are formatted as structured parameter lists with inline code backticks (` `AH = 0EH` `).
  - **Bottom Bitfield Evaluation:** The bottom bitfield graphics (`Bits of AL` and `Bits of AH`) are evaluated: if bounded and grid-like, converted as semantic bitfield tables; if containing graphical pointer lines or irregular free-form layouts, cleanly cropped as visual images for Stage 16.

### AS. Janus Library & Service API Parameter Formatting (Pages 83 & 89/ A500 TRM Pages 131 & 155)
- **Problem Statement:**
  - Page 96 (TRM Page 131) details `janus.library` API function prototypes such as `SetJanusHandler(jintnum, intserver)` with associated 68000 CPU register assignments (`D0`, `A1`).
  - Page 104 (TRM Page 155) documents PC Janus Service calling conventions using software interrupt `INT JANUS` (`J_GET_SERVICE`, `J_ALLOC_MEM`, `J_FREE_MEM`) with x86 CPU register bindings (`AH`, `AL`, `ES:DI`, `DX`, `BX`).
- **Verification Rule:**
  - **Syntax & Prototype Highlighting:** Function prototypes are formatted as inline code blocks or heading anchors (e.g. `### SetJanusHandler()` or `` `SetJanusHandler(jintnum, intserver)` ``).
  - **Register Binding Lists:** Register mappings (`D0`, `A1`, `AH`, `AL`, `ES:DI`, `DX`, `BX`) are presented as clean, readable Markdown parameter lists or tables, never garbled into unformatted prose.

### AT. Multi-Page Source Code Listings & File Boundary Demarcation (Pages 84 & 88/ A500 TRM Pages 135–139 & 151–152)
- **Problem Statement:**
  - Pages 97–101 (TRM Pages 135–139) contain multi-page assembly include listings: `janus_i86block.i` runs continuously from Page 97 through Page 100, concluding with `ENDC JANUS_I86BLOCK_I`. On Page 101, a completely **new file** begins: `janus.i` (`STRUCTURE JanusResource` and `STRUCTURE JanusAmiga`).
  - Pages 102–103 (TRM Pages 151–152) contain C header listings: `janus.h` runs from Page 102 onto Page 103, where it concludes and a **new C file** `janus_memrw.h` begins (`#define JANUS_MEMRW_H`).
  - If multi-page code is run without boundary demarcation, distinct source files are concatenated into a single malformed block.
- **Verification Rule:**
  - **Clean Fenced Code Blocks:** Assembly code is formatted in `` ```assembly `` (or `` ```m68k ``), and C code in `` ```c ``. Column alignments for instructions, operands, structure offsets, and comments are strictly preserved.
  - **File Boundary Demarcation:** When one source file concludes and another begins (e.g., between `janus_i86block.i` and `janus.i` on Page 101, and between `janus.h` and `janus_memrw.h` on Page 103):
    1. The preceding code block is cleanly closed with ```.
    2. A clear file header (e.g. `#### `janus.i`` or `#### `janus_memrw.h``) and explanatory commentary are inserted in Markdown prose.
    3. A new code block is opened with the appropriate language identifier.

### AU. HTML Tables with Multi-Byte Column Spans & Custom Chip Formats (Pages 90 & 94/ A500 TRM Pages 178, 184, 201, 202, 205)
- **Problem Statement:**
  - Page 105 (TRM Page 178) contains `Table 5-6. Command Summary` (`Description | OpcodeNum | BCNT | Options | Error Codes`) with footnotes underneath.
  - Page 106 (TRM Page 184) contains `Table 5-10. Change Command Block Address` featuring multi-byte DMA address column spans (`A23..A16`, `A15..A08`, `A07..A00`).
  - Page 107 (TRM Page 201) contains custom chip Blitter register bit definitions (`BLTCON0`, `BLTCON1`) followed by the start of the Line mode table.
  - Page 108 (TRM Page 202) contains the Blitter line draw mode bit table with footnotes, followed by a second table (`Blitter start and size`).
  - Page 109 (TRM Page 205) contains Copper registers and the instruction format table (`MOVE`, `WAIT`, `SKIP`) with multi-column bit spans and comparison mask footnotes (`VE`, `HE`).
- **Verification Rule:**
  - **Multi-Byte Column Spans in HTML Tables:** Tables with bit/byte column spans (such as `Table 5-10` with DMA byte groups and `Table 6-x` with Copper instruction fields) MUST be converted as semantic HTML tables (`table_html` in Stage 13/14) utilizing `colspan`.
  - **Stage 15 Gatekeeper Protection:** The presence of `colspan` attributes automatically protects these tables from reduction to standard Markdown, preserving their multi-byte layout.
  - **Footnote Preservation:** Footnotes underneath `Table 5-6`, the Blitter line mode table, and Copper instruction format are preserved as clean Markdown text directly below the respective tables.
  - **Register Bitfield Text Formatting:** Blitter register bit definitions on Page 107 are maintained as structured Markdown text, transitioning into tabular format only for true data tables.

### AV. Multi-Table Timing & Bit Assignments on Single Page (Page 95/ A500 TRM Page 207)
- **Problem Statement:** Page 207 details display data fetch horizontal timing. It contains:
  1. Register bit assignment table (`BIT# | USE`) with `X` and `H8`–`H3` bits.
  2. `DDFSTRT (Left edge of display data fetch)` table (`PURPOSE | H8 H7 H6 H5 H4`) with rows for Extra wide, wide, normal, narrow.
  3. `DDFSTOP (Right edge of display data fetch)` table (`PURPOSE | H8 H7 H6 H5 H4`) with rows for narrow, normal, wide.
  4. `DMACON`/`DMACONR` register bit descriptions (`SET/CLR`, `BBUSY`, `BZERO`, `BLTPRI` / "Blitter Nasty").
  Attempting to force all four elements into a single composite table creates garbled column headers.
- **Verification Rule:**
  - **Discrete Entity Separation:** `DDFSTRT` and `DDFSTOP` are transcribed as two separate, clean Markdown/HTML tables.
  - **Register Bitfield Structured Text:** Register bit assignments and `DMACON`/`DMACONR` control bits are formatted as structured text / bulleted lists with inline code styling.

### AW. Text Flow Wrapping / Linearization around Diagram (Page 96/ A500 TRM Page 211)
- **Problem Statement:** Page 211 documents `DMA Time Slot Allocation / Horizontal Line (Cont'd)`. The printed sheet has introductory text ("Hardware stop installed here. Data fetch cannot begin any sooner than cycle 18..."), a central timing diagram showing sprite DMA cycles and hardware stops, and concluding text below explaining how wide displays steal cycles from sprites. Multi-column wrap formatting breaks standard Markdown rendering.
- **Verification Rule:**
  - **Sequential Top-to-Bottom Linearization:** The page must be linearized into:
    1. Section header and introductory narrative paragraph.
    2. Visual `<crop>` tag for the horizontal DMA cycle allocation diagram.
    3. Concluding explanatory paragraph underneath the graphic.

### AX. PAL20L8 Logic Specification in Fenced Code Block (Page 97/ A500 TRM Page 228)
- **Problem Statement:** Section 7.3 contains `PAL20L8 PAL DESIGN SPECIFICATION` for the A2500 memory and DTACK decoder (U26), containing pinout vectors and boolean logic equations (`IF (OVR) /VPA = /AS*A23*/A22*A21...`). Because this specification is titled with design parameters, an automated parser could mistake it for a data table or attempt to crop it as an image.
- **Verification Rule:**
  - **Fenced Code Block Syntax:** Formatted strictly in a fenced code block (`` ```text `` or `` ```pal ``).
  - **Zero Crop / Never HTML Table:** Must NEVER be cropped as an image and must NEVER be converted to an HTML table.

### AY. Motherboard Jumper Pin Configuration Diagrams (Page 98/ A500 TRM Page 233)
- **Problem Statement:** Section 7.4 contains `List of B2000 Motherboard Jumpers` with visual jumper diagrams (`J101`, `J200`, `J301`, `J500`) showing physical shunt positions across pins 1, 2, and 3, accompanied by paragraphs explaining address bit selection (e.g. A23 vs A19 for Fat Agnus).
- **Verification Rule:**
  - **Visual Image Crops:** Jumper pin diagrams are cropped as visual `image` crops for Stage 16.
  - **Native Prose Preservation:** Surrounding explanatory text detailing jumper functions and positions remains native selectable Markdown prose.

### AZ. Large-Format Hardware Schematic Foldout Extraction (Page 99/ A500 TRM Page 235)
- **Problem Statement:** Appendix E contains large-format hardware foldouts (sheet size 1831x2160 pt) detailing complete motherboard schematics. Standard crop margins calibrated for letter/A4 pages could truncate outer bus rails or perimeter IC pin numbers.
- **Verification Rule:**
  - **Perimeter Ink Preservation:** Bounding box in Stage 7 and Stage 8 must encompass the entire active schematic canvas without clipping outer bus nets, power rails, or component callouts.
  - **Stage 16 Breakdown:** Generates a comprehensive architectural breakdown and RAG index document detailing bus connections and major IC subsystems.

### BA. Backplane Signal Interconnection & Bus Routing Table (Page 100/ A500 TRM Page 237)
- **Problem Statement:** Page 237 documents the Amiga 2000 expansion backplane, illustrating slot signal interconnections across slots J1 through J5 (`LOCAL_OWN`, `SLAVE`, `INT6`, `CONFIG_OUT`, `CONFIG_IN`) and logic gating.
- **Verification Rule:**
  - **Signal Routing Evaluation:** Evaluate whether the bus signal interconnects are parsed as a structured signal routing table or classified as an architectural schematic `image` crop.

### BB. Tri-Part Hardware Connector Linearization (Page 101/ A500 TRM Page 247)
- **Problem Statement:** Page 247 documents the `A2000 Keyboard Connector`, featuring:
  1. Header and introductory text.
  2. A physical 5-pin DIN connector mechanical diagram.
  3. A `Pin Name Description` pinout table (Pins 1–6: KCLK, KDAT, NC, GND, +5V, SHIELD).
  If merged into a single image crop, the pinout table loses text searchability. If converted without the diagram, physical pin numbering is lost.
- **Verification Rule:**
  - **Tri-Part Linearization:** Emits:
    1. Header prose: `## A2000 Keyboard Connector`
    2. Tight visual `<crop>` for the DIN connector pinout diagram.
    3. Clean GFM table for the pin assignments: `| Pin | Name | Description |`.

### BC. Peripheral Dimension Drawing & Pin Connection Table (Page 102/ A500 TRM Page 248)
- **Problem Statement:** Page 248 documents the `Amiga 500/2000 Mouse`, containing a mechanical dimensional drawing alongside a `CONNECTION TABLE` (`PIN | FUNCTION`). In the scanned manual, the drawing and table are printed in landscape orientation.
- **Verification Rule:**
  - **Linearization:** Mechanical mouse outline is cropped as a visual `image`, and the `CONNECTION TABLE` is transcribed directly as a clean GitHub Flavored Markdown table (`| Pin | Function |`) with signals (`BUTTON #1 (LEFT)`, `BUTTON #2 (RIGHT)`, `+5V`, `GND`).

### BD. Dual-Language Code Blocks (Page 103/ HRM Page 26)
- **Problem Statement:** In Section 1 (Introduction), custom chip register offsets are documented with back-to-back assembly (`hardware/custom.i`) and C (`hardware/custom.h`) examples. If combined into a single block or untagged block, syntax highlighting is lost.
- **Verification Rule:**
  - **Code Block Separation:** Transcribe the assembly snippet into a fenced code block tagged with `assembly` (or `asm`), and the C structure example into a separate fenced code block tagged with `c`.

### BE. Sandwich Alert & Table Layout (Page 104/ HRM Page 28)
- **Problem Statement:** Page 28 features a multi-element vertical "sandwich":
  1. An introductory warning regarding the 68000 `TAS` instruction.
  2. A compatibility table (`| CPU | User Mode | Super Mode |`).
  3. A technical note warning about `CLR.W` vs `MOVE.W` when accessing strobe registers.
- **Verification Rule:**
  - **Alert & Table Sequencing:**
    - Top advisory formatted as a GitHub Flavored Markdown note: `> [!NOTE]` (TAS instruction warning).
    - Middle compatibility grid converted into a clean GFM table (`| CPU | User Mode | Super Mode |`).
    - Bottom advisory formatted as a GitHub Flavored Markdown note: `> [!NOTE]` (MOVE.W vs CLR.W strobe register behavior).

### BF. Consecutive Instruction Code Blocks (Page 105/ HRM Page 35)
- **Problem Statement:** Page 35 defines the bitfield composition of the Copper `WAIT` instruction, followed at the bottom by two separate assembly instruction examples demonstrating beam wait and end-of-list wait (`DC.W $9601,$FF00` and `DC.W $FFFF,$FFFE`).
- **Verification Rule:**
  - **Independent Code Blocks:** Verify that the two assembly examples are rendered as two distinct fenced code blocks (` ```assembly `) with their accompanying explanatory comments, preserving column alignment.

### BG. Instruction Comparison Table & Field Alternation Alert (Page 106/ HRM Page 37)
- **Problem Statement:** Page 37 provides a comparison of Copper instructions (`Instruction | Explanation`) demonstrating vertical line 255 wrap and total scan lines (256 + 6 = 262), followed by a crucial hardware note on NTSC/PAL line and field alternation.
- **Verification Rule:**
  - **Table & Callout Formatting:**
    - The upper two-column comparison is transcribed as a standard GitHub Flavored Markdown table (`| Instruction | Explanation |`).
    - The bottom hardware note regarding NTSC (262 lines/field) and PAL (312 lines/field) vertical beam alternation is formatted as a `> [!NOTE]` alert blockquote.

### BH. Multi-Page Copper List Assembly Continuation (Pages 107 & 108/ HRM Pages 42 & 43)
- **Problem Statement:** Pages 42 and 43 contain the `COMPLETE SAMPLE COPPER LIST`. Page 42 establishes the register initialization table and initial allocation code; Page 43 continues the assembly listing with `COPPERLIST:`, defining bitplane pointers and color palette registers. If the listing is split or interrupted by redundant headers, the code listing integrity is broken.
- **Verification Rule:**
  - **Assembly Continuation:** Ensure assembly code indentation, label definitions, opcodes, and operands are formatted continuously across the page boundary, with Stage 21 fusing the code smoothly into a unified block.

### BI. Playfield Dimensions & Color Table Conversions (Pages 109 & 110/ HRM Pages 57 & 58)
- **Problem Statement:** Section 3 Playfield Hardware introduces playfield screen geometry and color allocation:
  - Page 57: Bottom `Table 3-1: Colors in a Single Playfield` (`Number of Colors | Number of Bit-Planes`).
  - Page 58: `Table 3-2: Portion of the Color Table` (`Register Name | Contents | Meaning`) and an advisory note regarding genlock color 00 transparency.
- **Verification Rule:**
  - **Table & Alert Transcription:**
    - `Table 3-1` is transcribed as a clean GFM Markdown table.
    - `Table 3-2` is transcribed as a clean GFM Markdown table.
    - The genlock color 00 note is formatted as a GitHub Flavored Markdown `> [!NOTE]` alert blockquote.

### BJ. HTML Table with Multi-Level Spans & Native LaTeX Math (Page 111/ HRM Page 71)
- **Problem Statement:** Page 71 features `Table 3-9: DIWSTRT AND DIWSTOP Summary`, which contains multi-tier hierarchical subheaders:
  - Columns: `Screen Type`, `Nominal Values` (with subheaders `MIN` and `MAX`), and `Possible Values` (with subheaders `MIN` and `MAX`).
  - Under the table, calculations define display window width and horizontal pixel clocks (`$F4 - $2C = $C8 (hex) = 200 (decimal)`, `$81 / 2 = $40.5`).
- **Verification Rule:**
  - **Semantic HTML Table:** `Table 3-9` must be converted into a semantic HTML table with `colspan="2"` for `Nominal Values` and `colspan="2"` for `Possible Values`.
  - **Mathematical Expressions:** The arithmetic formulas under the table must be preserved in native LaTeX math syntax (`$ ... $`), e.g., `$F4 - $2C = $C8 \text{ (hex)} = 200 \text{ (decimal)}$` and `$81 / 2 = 40.5$`, avoiding plain ASCII mangling.

### BK. Dual-Playfield Multi-Subsystem Table Disaggregation (Page 112/ HRM Page 84)
- **Problem Statement:** Page 84 presents `Table 3-12: Playfields 1 and 2 Color Registers - High-resolution Mode`. The table groups color registers for two distinct hardware subsystems (Playfield 1 and Playfield 2) side-by-side:
  - Playfield 1 columns: `Playfield 1 Color | High-Resolution Bit Combinations | Low-Resolution Bit Combinations`.
  - Playfield 2 columns: `Playfield 2 Color | High-Resolution Bit Combinations | Low-Resolution Bit Combinations`.
  In a linear Markdown document, cramming both subsystems into a single wide table causes awkward horizontal scrolling and obscures the functional independence of each playfield.
- **Verification Rule:**
  - **Table Disaggregation:** Per prompt Rule 7, disaggregate `Table 3-12` into two separate, clean Markdown tables:
    1. `Table 3-12A: Playfield 1 Color Registers - High-Resolution Mode`
    2. `Table 3-12B: Playfield 2 Color Registers - High-Resolution Mode`

### BL. Sequential Multi-Diagram Linearization (Page 113/ HRM Page 88)
- **Problem Statement:** Page 88 documents bitplane modulo data fetch, containing three sequential memory layout diagrams:
  1. `Figure 3-16: Data Fetch for the Second Line When Modulo = 40` (showing START+80, START+82, START+84, START+118).
  2. Intermediate prose detailing vertical blanking routine and starting pointers at location START+40.
  3. `Figure 3-17: Data Layout for First Line - Right Half of Big Picture` (START+40, START+42, START+44, START+78).
  4. Intermediate prose explaining bitplane pointers containing START+80 and adding modulo.
  5. `Figure 3-18: Data Layout for Second Line - Right Half of Big Picture` (START+120, START+122, START+124, START+158).
  6. Concluding prose explaining byte fetch requirements in high-resolution mode (80 bytes vs 40 bytes).
  Merging all three figures into one massive crop would destroy the narrative reading flow and swallow the explanatory text paragraphs.
- **Verification Rule:**
  - **Sequential Linearization:** Isolate three separate visual `<crop>` tags for Figures 3-16, 3-17, and 3-18, strictly preserving the interleaving narrative prose in proper linear sequence:
    prose -> Figure 3-16 `<crop>` -> intermediate prose -> Figure 3-17 `<crop>` -> intermediate prose -> Figure 3-18 `<crop>` -> concluding prose.

### BM. Display Window Screen Region Diagrams (Page 114/ HRM Page 90)
- **Problem Statement:** Page 90 defines the display window horizontal and vertical starting positions:
  1. `Figure 3-19: Display Window Horizontal Starting Position` (illustrating FULLSCREEN AREA with 0, 255, 361, and HSTART region).
  2. Intermediate prose regarding VSTART bit allocation (bits 8-15) and counting 256 positions down from top of display.
  3. `Figure 3-20: Display Window Vertical Starting Position` (illustrating FULLSCREEN AREA with 0, 255, 262, and VSTART region).
  4. Concluding prose regarding low-res/interlaced starting positions and setting DIWSTRT.
- **Verification Rule:**
  - **Dual-Figure Separation:** Isolate two independent visual `<crop>` tags for Figures 3-19 and 3-20, preserving the intermediate explanatory text between them.

### BN. High-Resolution Color Selection Table with Rowspans (Page 115/ HRM Page 109)
- **Problem Statement:** Page 109 features `Table 3-19: High-resolution Color Selection`. The table defines color register selection for single playfield (4 planes) vs dual playfields (Playfield 1 and Playfield 2), with:
  - Header: `Single Playfield planes 4,3,2,1 | Dual Playfields Playfield 1 Bit-planes 3,1 | Playfield 2 Bit-planes 4,2 | Color Register Number`.
  - Merged rows: `NOT USED IN THIS MODE` spanning 4 vertical rows for Playfield 1 (rows 4–7) and 4 vertical rows for Playfield 2 (rows 12–15).
  - External footnotes: `* Selects "transparent" mode.` and `** Color register 0 always defines the background color.`.
- **Verification Rule:**
  - **Semantic HTML Table:** Convert into a semantic HTML table preserving multi-row vertical spans (`rowspan="4"` for `NOT USED IN THIS MODE`).
  - **External Footnotes:** Preserve both footnotes cleanly outside the table directly underneath.

### BO. Sprite Data Structure & Explicit Caution Alert Blockquote (Page 116/ HRM Page 124)
- **Problem Statement:** Page 124 documents sprite data structures and display sequencing:
  1. Introductory prose and spaceship sprite assembly data structure (`SPRITE:` with `DC.W $6D60,$7200...`).
  2. Section header `## Displaying a Sprite` and 4 numbered operational steps.
  3. A prominent hardware `CAUTION` warning against turning off sprite DMA during active display, which causes a vertical streak/bar artifact.
- **Verification Rule:**
  - **Code Block & Alert Formatting:**
    - Format sprite data structure in an assembly fenced code block (` ```assembly `).
    - Format display steps 1–4 as a clean Markdown numbered list.
    - Format the `CAUTION` warning as a GitHub Flavored Markdown alert blockquote:
      ```markdown
      > [!CAUTION]
      > [Source caution text, transcribed locally during testing.]
      ```
    - Check the warning locally for the active-display condition (`VSTART` to `VSTOP`), repeated sprite-line artifact, and advice to disable DMA outside sprite display.

### BP. Spanning Header Data Bit Table (Page 117/ HRM Page 136)
- **Problem Statement:** Page 136 presents `Table 4-4: Data Words for First Line of Spaceship Sprite`. The table maps binary color selector bits for attached sprites:
  - Top master header: `Pixel Number` spanning across all 16 bit columns (15 down to 0).
  - Four data rows: `Line 1`, `Line 2`, `Line 3`, `Line 4` representing high-order and low-order words for Sprite 1 and Sprite 0.
- **Verification Rule:**
  - **Master Category Header:** Transcribe or crop as a structured semantic table preserving the master spanning header (`<th colspan="16">Pixel Number</th>` in HTML table) over the individual 16 bit columns (15 to 0).

### BQ. Grouped Sprite Range Table with Footnotes (Page 118/ HRM Page 145)
- **Problem Statement:** Page 145 presents `Table 4-6: Color Registers for Single Sprites`:
  - Columns: `Single Sprites Sprite | Value | Color Register`.
  - Grouped sprite rows: ranges `0 or 1`, `2 or 3`, `4 or 5`, and `6 or 7` each spanning 4 rows for values `00`, `01`, `10`, `11`.
  - Under-table footnote: `* Selects transparent mode.`.
- **Verification Rule:**
  - **Grouped Rowspans & Footnote:** Convert into a semantic table preserving `rowspan="4"` for each sprite pair range, and position the footnote `* Selects transparent mode.` cleanly underneath the table.

### BR. Graphic Crop Isolation & Waveform Data Table Extraction (Page 119/ HRM Page 151)
- **Problem Statement:** Page 151 (Folio 133) presents `Figure 5-2: Digitized Amplitude Values` illustrating discrete waveform sampling steps, followed immediately by a four-column data lookup table:
  - Columns: `TIME | SINE | SQUARE | TRIANGLE` covering discrete time units 0 through 19.
  - Splicing both the visual graph and the numerical data into a single visual crop prevents programmatic text search and violates the rule to transcribe tabular data as native text.
- **Verification Rule:**
  - **Graphic Isolation:** Isolate `Figure 5-2` as an `image` crop (`detected_type: "image"`), deferred to Stage 16.
  - **Data Table Extraction:** Transcribe the numerical table underneath into a clean Markdown table with columns `Time | Sine | Square | Triangle`, preserving exact integer amplitudes (-127 to +127).

### BS. Audio Memory Offset Specification as Fenced Assembly Code Block (Page 120/ HRM Page 153)
- **Problem Statement:** Page 153 (Folio 135) contains `Table 5-1: Sample Audio Data Set for Channel 0`. Despite the word "Table", the content specifies contiguous word-aligned chip memory addresses (`audiodata -> AUD0LC *`, `AUD0LC+ 2**`, ..., `AUD0LC+ 30`) mapped to high and low byte waveform sample values (`100 98`, `92 83`, ..., `92 98`). Cropping this as an image or rendering as an HTML table is cumbersome; it represents architectural memory definitions best structured as a formatted assembly code block.
- **Verification Rule:**
  - **Assembly Code Block:** Format the memory address and sample value definitions as a fenced assembly block (`` ```assembly ``), aligning offsets and sample bytes cleanly.
  - **Footnotes & Notes:** Preserve the `*` and `**` memory alignment notes directly underneath as native Markdown text.

### BT. Audio Volume Register Table & Assembly Strobe Listing (Page 121/ HRM Page 155)
- **Problem Statement:** Page 155 (Folio 137) provides `Table 5-2: Volume Values` (`Volume | Decibel Value`), followed by an assembly register strobe routine (`SETAUD0VOLUME: LEA CUSTOM,a0; MOVE.W #48, AUD0VOL(a0)`), accompanied by narrative notes explaining negative decibel representation relative to 0 dB maximum.
- **Verification Rule:**
  - **Markdown Table:** Convert `Table 5-2` into a clean Markdown table (`| Volume | Decibel Value |`).
  - **Code Block:** Format the assembly volume setup routine in a fenced code block (`` ```assembly ``).
  - **Explanatory Prose:** Maintain narrative flow for decibel recording level explanations without dropping context.

### BU. Mathematical Expressions & Audio Clock Parameter Table with Spans (Page 122/ HRM Page 156)
- **Problem Statement:** Page 156 (Folio 138) contains high-density hardware timing calculations for audio DMA sample rates:
  - Formulas for theoretical maximum rate ($2 	imes 262.5 	imes 59.94 = 31,469	ext{ samples/sec}$), hardware limit ($28,867	ext{ samples/sec}$), system timing interval ($0.279365\ \mu	ext{s}$), and fractional minimum period derivations.
  - A bottom parameter table `Clock Values` comparing `NTSC` vs `PAL` clock constants (`3579545` vs `3546895`) and clock intervals (`0.279365` vs `0.281937`) with a units column.
- **Verification Rule:**
  - **Native LaTeX Math:** Transcribe all mathematical formulas, fractional ratios, and unit derivations in native LaTeX math syntax (`$ ... $` and `$$ ... $$`).
  - **Clock Values Table:** Convert the clock table into a clean semantic HTML table preserving multi-column headers and unit spans.

### BV. DMACON Register Audio Enable Bits Table & Initialization Assembly (Page 123/ HRM Page 159)
- **Problem Statement:** Page 159 (Folio 141) documents `Table 5-3: DMA and Audio Channel Enable Bits` (`DMACON Register: Bit | Name | Function`) detailing control bits (Bit 15 `SET/CLR`, Bit 9 `DMAEN`, Bits 3-0 `AUD3EN`-`AUD0EN`), followed by an assembly initialization block (`BEGINCHAN0:`).
- **Verification Rule:**
  - **Register Table:** Convert `Table 5-3` into a structured semantic table preserving bit numbers, register names, and function descriptions.
  - **Assembly Code:** Format the `BEGINCHAN0:` routine in an assembly code block (`` ```assembly ``).

### BW. Sampling Rate vs Frequency Relationship Table (Page 124/ HRM Page 171)
- **Problem Statement:** Page 171 (Folio 153) presents `Table 5-6: Sampling Rate and Frequency Relationship` (`Sampling Period | Sampling Rate (KHz) | Maximum Output Frequency (KHz)`), alongside technical prose detailing the CIA 8520 low-pass filter bypass controlled via the power LED brightness bit.
- **Verification Rule:**
  - **Markdown Table:** Convert `Table 5-6` into a clean Markdown table.
  - **Prose & CIA Reference:** Accurately transcribe and preserve the technical notes regarding 8520 CIA filter control and direct (non-DMA) audio output considerations.

### BX. Multi-Table Calibration Layout (Four Sample Data Tables) (Page 125/ HRM Page 176)
- **Problem Statement:** Page 176 (Folio 158) contains four distinct waveform sample data sets arranged on a single page:
  - `256 Byte Sample`
  - `128 Byte Sample`
  - `64 Byte Sample`
  - `32 Byte Sample`
  Lumping all four tables together or cropping them as a single image destroys tabular data utility.
- **Verification Rule:**
  - **Four Independent Tables:** Transcribe the page into **four separate, cleanly structured tables**, each titled with its respective sample size header, preserving all positive and negative signed byte values (-128..+127).

### BY. Blitter Memory Layout Map & DMA Channel Enable Advisory Note (Page 126/ HRM Page 183)
- **Problem Statement:** Page 183 (Folio 165) illustrates `Figure 6-1: How Images are Stored in Memory`, showing a memory word address map (addresses 20 through 61) representing a single bitplane image, accompanied by text detailing independent enablement of channels `SRCA`, `SRCB`, `SRCC`, and `DEST` in `BLTCON0`.
- **Verification Rule:**
  - **Figure Crop:** Isolate `Figure 6-1` cleanly as an `image` crop (`detected_type: "image"`), deferred to Stage 16.
  - **Prose Formatting:** Format surrounding register explanations and DMA cycle notes as clean Markdown prose.

### BZ. Blitter Minterm Truth Table with Proper Boolean Negation Representation (Page 127/ HRM Page 186)
- **Problem Statement:** Page 186 (Folio 168) introduces the Blitter Function Generator truth table (`A | B | C | D | BLTCON0 position | Minterm`). In the scanned PDF, inverted boolean inputs are printed with overbars ($\overline{A}\overline{B}\overline{C}$, $\overline{A}\overline{B}C$, etc.). OCR typically drops these overbars or mangles them into unreadable characters.
- **Verification Rule:**
  - **Semantic Truth Table:** Convert into a clean semantic table with columns `A | B | C | D | BLTCON0 Position | Minterm`.
  - **Accurate Negation Representation:** Ensure all negated minterm terms are accurately represented using mathematical overbars (`$\overline{A}\overline{B}\overline{C}$`) or logical not syntax (`~A~B~C` / `ar{A}ar{B}ar{C}`) rather than plain un-negated letters.

### CA. Blitter Logic Equations & Markdown Note Alert Callout (Page 128/ HRM Page 187)
- **Problem Statement:** Page 187 (Folio 169) explains designing the LF control byte using logic equations and minterms, including algebraic boolean equations ($AB + BC$, $(AB) + (BC)$), and features an explicit advisory `NOTE` regarding boolean operator precedence (AND has higher precedence than OR).
- **Verification Rule:**
  - **Logic Equations:** Format all boolean algebra expressions cleanly with LaTeX math or inline code formatting.
  - **Markdown Alert Callout:** Format the explicit precedence advisory note as a standard GitHub Flavored Markdown note alert blockquote:
    ```markdown
    > [!NOTE]
    > [Source operator-precedence note, transcribed locally during testing.]
    ```
  - **Content Verification:** Check the note locally for implicit AND, `+` as OR, and AND taking precedence over OR; retain the equation `$AB + BC = (AB) + (BC)$`.

### CB. Dual Minterm Bit Pattern & Logic Combination Code Blocks (Page 129/ HRM Page 191)
- **Problem Statement:** Page 191 (Folio 173) displays two character-aligned minterm bit selection and combination diagrams at the top of the page:
  1. Inverse source selection:
     ```text
     Minterm Numbers   7 6 5 4 3 2 1 0
     Selected Minterms 0 0 0 0 1 1 1 1
     0 F               equals $0F
     ```
  2. Combining minterms with OR logic ($AB + BC$):
     ```text
     Minterm Numbers   7 6 5 4 3 2 1 0
     AB                1 1 0 0 0 0 0 0
     BC                1 0 0 0 1 0 0 0
     AB+BC             1 1 0 0 1 0 0 0
     C 8               equals $C8
     ```
  Converting these aligned bit positions into HTML tables causes column misalignment and disrupts readable hexadecimal byte synthesis.
- **Verification Rule:**
  - **Monospaced Code Blocks:** Format both diagrams as clean monospaced text code blocks (`` ```text ``), strictly preserving monospace column alignment across bit positions 7 down to 0 and their corresponding hexadecimal byte results (`$0F`, `$C8`).

### CC. Blitter Cycle Sequence Table Conversion (Page 130/ HRM Page 201)
- **Problem Statement:** Page 201 (Folio 183) documents `Table 6-2: Typical Blitter Cycle Sequence`. The table maps blitter channel USE codes in `BLTCON0` to active DMA cycles across bus time slots (e.g., Code F = A, B, C, D; Code E = A, B, C; Code 0 = no DMA channels).
- **Verification Rule:**
  - **Structured Semantic Table:** Convert `Table 6-2` into a clean Markdown table with headers `USE Code in BLTCON0 | Active Channels`, preserving the sub-channel sequence listings accurately.

### CD. Octant Line Drawing Code Bits Table & Indented Algorithm Block (Page 131/ HRM Page 203)
- **Problem Statement:** Page 203 (Folio 185) features:
  1. `Table 6-3: BLTCON1 Code Bits for Octant Line Drawing` mapping octants 0–7 to bit combinations (bits 4, 3, 2).
  2. An indented pseudo-code setup algorithm calculating coordinate deltas (`dx = x2 - x1`, `dy = y2 - y1`), octant classification, and parameter setup.
  Loss of indentation in the algorithm block ruins code readability.
- **Verification Rule:**
  - **Octant Table:** Convert `Table 6-3` into a clean semantic Markdown table.
  - **Preserved Indentation in Code Block:** Format the octant line setup logic in a fenced code block (`` ```text ``), strictly preserving hierarchical indentation and conditional blocks (`if`, `then`, `else`).

### CE. Multi-Page Line Mode Register Summary Code Blocks (Page 132/ HRM Pages 204 & 205)
- **Problem Statement:** Pages 204 and 205 (Folios 186 and 187) define `REGISTER SUMMARY FOR LINE MODE`. This two-page continuous specification contains setup formulas, register assignments (`BLTCON0`, `BLTCON1`, `BLTAFWM`, `BLTALWM`, `BLTAMOD`, `BLTBMOD`, `BLTAPT`, `BLTBPT`, `BLTCPT`, `BLTDPT`, `BLTSIZE`), and conditional branches for exclusive-or vs standard line mode. Treating parts of this as body text creates fragmented paragraphs.
- **Verification Rule:**
  - **Continuous Code Block Formatting:** Format the register summary specifications on both Page 204 and Page 205 as clean fenced code blocks (`` ```text `` or `` ```assembly ``), ensuring mathematical conditions (e.g., `4 * dy - 2 * dx < 0`) and register bitfield settings are completely contained within structured code formatting.

### CF. Blitter Execution Speed Mathematical Calculations (Page 133/ HRM Page 206)
- **Problem Statement:** Page 206 (Folio 188) documents blitter throughput and execution timing:
  - System clock specifications (7.16 MHz for NTSC, 7.09 MHz for PAL).
  - Clock tick cycle costs per channel combination (A and D = 4 ticks; B and D = 6 ticks; B, C, and D = 8 ticks; line mode = 8 ticks per pixel).
  - Formulas for total execution time in microseconds ($t = \frac{n \times H \times W}{7.16}$ for NTSC, $t = \frac{n \times H \times W}{7.09}$ for PAL).
- **Verification Rule:**
  - **Native LaTeX Formulas:** Transcribe all timing formulas, division operations, and variable definitions ($n$, $H$, $W$) in native LaTeX math syntax (`$ ... $` and `$$ ... $$`), avoiding plain ASCII distortion.

### CG. Rotated Hardware Timing Diagram Crop Normalization (Page 134/ HRM Page 208)
- **Problem Statement:** Page 208 (Folio 190) presents `Figure 6-9: DMA Time Slot Allocation / Horizontal Line`. In the printed manual, this extensive horizontal timing diagram (displaying cycles from `$00` through `$E0`, sprite DMA windows, audio DMA, memory refresh, and display data fetch stops) is printed sideways (rotated 90 degrees counter-clockwise relative to the page text). A raw vertical crop leaves the diagram unreadable on screen.
- **Verification Rule:**
  - **90° Clockwise Rotation:** When extracting the visual crop of `Figure 6-9` in Stage 8, apply a 90-degree clockwise rotation so that the resulting PNG asset is upright, legible, and oriented horizontally before Stage 10 visual evaluation and Stage 16 architectural breakdown.

### CH. Anti-Table Regression Test for Display Time Slot Waveform Diagrams (Page 135/ HRM Page 210)
- **Problem Statement:** Page 210 (Folio 192) contains two bus allocation waveform diagrams:
  1. `Figure 6-11: Time Slots Used by a Six Bit Plane Display` (illustrating memory time slots $T$ through $T+7$ with bitplane numbers 4, 6, 2, 3, 5, 1).
  2. `Figure 6-12: Time Slots Used by a High Resolution Display`.
  Heuristics looking for grid boxes or numbers might mistakenly attempt to parse these timing allocation diagrams as tables.
- **Verification Rule:**
  - **Strict Image Classification:** Strictly classify both `Figure 6-11` and `Figure 6-12` as visual `image` crops (`detected_type: "image"`), deferred to Stage 16. Verify that neither diagram is converted into an HTML or Markdown table.

### CI. Beam Position Counter Register Structure Table (Page 136/ HRM Page 229)
- **Problem Statement:** Page 229 (Folio 211) provides `Table 7-5: Contents of the Beam Position Counter`. The table documents the read-only `VPOSR` register:
  - Bit 15: `LOF` (Long-frame bit used for interlace initialization).
  - Bits 14-1: Unused.
  - Bit 0: High bit of vertical position (V8).
- **Verification Rule:**
  - **Structured Semantic Table:** Convert `Table 7-5` into a clean Markdown table with headers `Bit | Name | Function`, accurately reflecting register bitfields.

### CJ. Complex Spanned HTML Interrupt Priority Table (Page 137/ HRM Page 234)
- **Problem Statement:** Page 234 (Folio 216) presents the hardware interrupt priority matrix detailing 6 interrupt priority levels, corresponding mask bits in `INTENA`/`INTREQ`, CPU interrupt autovectors, and peripheral sources (e.g., Level 1: TBE, DSKBLK, SOFT; Level 5: RBF, DSKSYNC). Several levels group multiple peripheral sources together with complex vertical spans.
- **Verification Rule:**
  - **HTML Table with Spans:** Convert into a semantic HTML table preserving multi-row vertical spans (`rowspan`) across shared interrupt priority levels.

### CK. Dual Controller Port Specification Tables (Page 138/ HRM Page 241)
- **Problem Statement:** Page 241 (Folio 223) features two related but distinct controller specification tables:
  1. `Table 8-1: Typical Controller Connections` (mapping 9-pin D-sub connections across Joysticks, Mice, Trackballs, Driving Controllers, and Proportional Pairs).
  2. `Table 8-2: Controller Port Register Bit Allocations`.
  Lumping both tables together causes column confusion.
- **Verification Rule:**
  - **Two Independent Tables:** Transcribe both tables cleanly as two distinct, properly structured Markdown tables with their respective descriptive titles.

### CL. Multi-Page Disk Subsystem Control Table Fusion (Pages 139 & 140/ HRM Pages 256 & 257)
- **Problem Statement:** Pages 256 and 257 (Folios 238 and 239) present `Table 8-5: Disk Subsystem`, which details the 8520 CIA-A and CIA-B port bit allocations for floppy disk control:
  - Page 256 (Sheet 1): `CIAAPRA` ($BFE001) input bits (`PA5 DSKRDY*`, `PA4 DSKTRACK0*`, `PA3 DSKPROT*`, `PA2 DSKCHANGE*`).
  - Page 257 (Sheet 2): `CIABPRB` ($BFD100) output bits (`PB6-PB3 DSKSEL*`, `PB2 DSKMOTOR*`, `PB1 DSKDIRECTION`, `PB0 DSKSTEP*`).
  Leaving these as two fragmented tables breaks the reference flow.
- **Verification Rule:**
  - **Multi-Page Table Fusion:** Transcribe per page in Stages 7–15, tag across the page boundary with `<continuation-marker>` in Stage 20, and fuse into **ONE unified continuous table** in Stage 21, suppressing duplicate headers.

### CM. Keyboard Scan Matrix Graphics Anti-Table Image Regression (Page 141/ HRM Page 267)
- **Problem Statement:** Page 267 (Folio 249) features two visual schematics illustrating keyboard keycode layouts, key matrix connections, and scan code encoding. Parsing tools might mistake the key matrix grid for a data table.
- **Verification Rule:**
  - **Strict Image Classification:** Strictly classify both keyboard matrix graphics as visual `image` crops (`detected_type: "image"`), deferred to Stage 16 for architectural breakdown and RAG indexing.

### CN. Multi-Page Serial & Audio Control Register Table Fusion / Disaggregation (Pages 142 & 143/ HRM Pages 270 & 271)
- **Problem Statement:** Pages 270 and 271 (Folios 252 and 253) document `Table 8-9: SERDATR / ADKCON Registers`:
  - Page 270: `SERDATR` bit definitions (Bits 15-0: OVRUN, RBF, TBE, TSRE, RXD, STP, DB8-DB0).
  - Page 271: `ADKCON` audio/disk control bit definitions (Bits 15-0: SET/CLR, PRECOMP, MFMPREC, UARTBRK, etc.).
- **Verification Rule:**
  - **Register Table Conversion:** Convert both register definitions into structured semantic tables, maintaining clear separation between `SERDATR` and `ADKCON` functionality.

### CO. Appendix A Continuous Hardware Register Listing Code Blocks (Pages 144 & 145/ HRM Pages 277–279)
- **Problem Statement:** Pages 277, 278, and 279 (Folios 259, 260, 261) present the opening three pages of Appendix A (Register Summary). The pages list hardware register offsets, read/write permissions, controlling custom chips (Agnus, Denise, Paula), and bit functions in a character-aligned columnar format.
- **Verification Rule:**
  - **Continuous Code Block Formatting:** Format the register listings across all three pages as clean, continuous fenced code blocks (`` ```text `` or `` ```assembly ``), ensuring column offsets and comments remain perfectly aligned.

### CP. Game Port / Mouse Register Mapping Formatting (Page 146/ HRM Page 293)
- **Problem Statement:** Page 293 (Folio 275) documents game controller registers `JOY0DAT` ($00A) and `JOY1DAT` ($00C), detailing vertical and horizontal mouse/joystick coordinate counters.
- **Verification Rule:**
  - **Structured Formatting:** Transcribe register bit assignments and coordinate formats cleanly into structured code blocks or compact Markdown tables.

### CQ. Three-Page Custom Chip Register Address Map Table Fusion (Pages 147 & 149/ HRM Pages 301–303)
- **Problem Statement:** Pages 301, 302, and 303 (Folios 283, 284, 285) comprise Appendix B `Complete Custom Chip Register Map by Address`:
  - Columns: `NAME | ADD | R/W | CHIP | FUNCTION`.
  - Spans consecutive address ranges from `$000` through `$1DC`.
  In a reference manual, breaking this master lookup table across three separate page fragments disrupts searchability.
- **Verification Rule:**
  - **Three-Page Unified Table:** Transcribe each page into structured tables, and fuse all three pages in Stage 21 into **ONE single continuous Markdown table**, stripping repeated table column headers and page footers.

### CR. Agnus Chip Hardware Pinout Table (Page 150/ HRM Page 308)
- **Problem Statement:** Page 308 (Folio 290) presents Appendix C `AGNUS PIN ASSIGNMENT`:
  - Columns: `PIN # | DESIGNATION | FUNCTION | DEFINITION`.
  - Maps all package pins (D8-D0 data bus, address bus, DMA channels, power, clock).
- **Verification Rule:**
  - **Comprehensive Pinout Table:** Convert into a clean, complete Markdown table detailing every pin, designation, and electrical direction (I/O, Input, Output).

### CS. Hardware Architecture Memory Map Table (Page 151/ HRM Page 312)
- **Problem Statement:** Page 312 (Folio 294) provides the Amiga hardware architecture memory map table:
  - Address ranges from `$000000` through `$FFFFFF` (16 MB physical address space).
  - Categorizes Chip RAM, Auto-Config expansion spaces, custom chip registers (`$DFF000`), CIA-A (`$BFE001`), CIA-B (`$BFD000`), and Kickstart ROM.
- **Verification Rule:**
  - **Structured Markdown Table:** Convert into a clean Markdown table with headers `Address Range | Description | Allocation / Function`.

### CT. Parallel Interface Pinout & ASCII Timing Diagram Code Blocks (Pages 152 & 153/ HRM Pages 320 & 321)
- **Problem Statement:** Pages 320 and 321 (Folios 302 and 303) document the Amiga DB25 parallel interface:
  - Page 320 describes DB25 pin assignments, Centronics compatibility differences, and port directions.
  - Page 321 displays `PARALLEL CONNECTOR INTERFACE TIMING, OUTPUT CYCLE`, an ASCII-art timing diagram showing data bus setup time ($T_1$), data ready strobe (`DRDY*`), and acknowledge handshaking.
- **Verification Rule:**
  - **Preserved Code Blocks:** Format both the pinout specification on Page 320 and the ASCII timing diagram on Page 321 as clean fenced code blocks (`` ```text ``), ensuring character-grid alignment is preserved.

### CU. 8520 CIA-A Register Address Map Table (Page 154/ HRM Page 336)
- **Problem Statement:** Page 336 (Folio 318) presents Appendix F `CIAA Address Map`:
  - Columns: `Byte Address | Register Name | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0` (Data bits).
  - Documents all 16 internal CIA registers (`PRA`, `PRB`, `DDRA`, `DDRB`, `TALO`, `TAHI`, `TBLO`, `TBHI`, `TODLO`, `TODMID`, `TODHI`, `SDR`, `ICR`, `CRA`, `CRB`).
- **Verification Rule:**
  - **10-Column Data Table:** Convert into a clean, comprehensive 10-column Markdown table preserving bit names and hexadecimal byte offsets.

### CV. CIA Control Register CRA/CRB Bitfield Map Code Block (Page 155/ HRM Page 346)
- **Problem Statement:** Page 346 (Folio 328) details `BIT MAP OF REGISTER CRA` and `CRB`:
  - Documents bit layout (`UNUSED`, `SPMODE`, `INMODE`, `LOAD`, `RUNMODE`, `OUTMODE`, `PBON`, `START`).
  - Contains register bit functional descriptions.
- **Verification Rule:**
  - **Register Bitfield Formatting:** Format the register bit mapping in a clean fenced code block (`` ```text ``) or structured table, avoiding dropped bitfield definitions.

### CW. Expansion Architecture Auto-Config Nibble Diagrams as Code Blocks (Page 156/ HRM Page 356)
- **Problem Statement:** Page 356 (Folio 338) details Auto-Config address offset structures:
  - Board Offset `($00/02)` with nibble brackets (`\___ ___/`, `Nibble at $E80000`).
  - Bit positions 7–4 and inverted nibble definitions.
  Converting these diagrammatic callout brackets into HTML tables causes broken geometry.
- **Verification Rule:**
  - **Monospaced Code Block Formatting:** Format all offset bit layouts, brackets, and nibble flowcharts as clean monospaced text code blocks (`` ```text ``).

### CX. Keyboard Serial Communications Handshake Waveform (Page 157/ HRM Page 362)
- **Problem Statement:** Page 362 (Folio 344) documents Amiga keyboard serial transmission protocol:
  - Explains serial bit rotation, KDAT/KCLK timing pulses, and the 85-microsecond handshake low pulse.
  - Contains handshake timing waveforms.
- **Verification Rule:**
  - **Graphic Isolation / Code Block:** Isolate the timing waveform as a visual `image` crop (`detected_type: "image"`) or transcribe as an ASCII timing diagram code block.

### CY. Two-Page Complex Spanned Keyboard Matrix HTML Table Fusion (Pages 158 & 159/ HRM Pages 368 & 369)
- **Problem Statement:** Pages 368 and 369 (Folios 350 and 351) present the full keyboard `Matrix Table`:
  - Page 368 lists Rows 5 down to 0 with key assignments across multiple columns.
  - Page 369 maps column lines to Bits 7 down to 0, detailing raw key codes and matrix cross-points.
  - Features complex multi-row cells grouping keys by modifier states.
- **Verification Rule:**
  - **Complex Spanned HTML Table:** Convert into a semantic HTML table preserving multi-row vertical spans (`rowspan`).
  - **Multi-Page Fusion:** Fuse both pages in Stage 21 into **ONE single continuous HTML table**, eliminating intermediate page breaks and repeated column headers.

### CZ. Index Page Processing & Final Omission Rule (Page 160/ HRM Page 391)
- **Problem Statement:** Page 391 (Folio 373) contains the first sheet of the printed subject index (`INDEX: 60 Pin Edge Connector, 68000, 8520, etc.`). In digital Markdown documentation, printed page numbers do not correspond to markdown file or section line numbers, making static printed index pages redundant, misleading, and obsolete.
- **Verification Rule:**
  - **Upstream Text Processing:** Transcribe as clean selectable text in Stages 7–19 without image crops.
  - **Final Chapter Omission:** In Stage 20 and Stage 21, systematically strip and exclude the index chapter from the final publication output (`build/02_final_chapters/`).

---

---

## 4. History of Changes

| Date | Action | Description |
| :---: | :--- | :--- |
| **2026-10-01** | Initial Assembly | Replaced legacy 308-page PDF with 19-page test slice from `68000 User's Manual` (Pages 1–19). Reset `build/` directory for clean testing. |
| **2026-10-01** | Expansion (Pages 20–21) | Appended Source Pages 23 and 24 (Figures 2-1, 2-2, 2-3, 2-4) to test anti-table regression and enforce guaranteed `image` classification. Total pages: 21. |
| **2026-10-01** | Expansion (Page 22) | Appended Source Page 26 (Table 2-1: Data Addressing Modes) to test table conversion and Stage 15 reduction to GFM Markdown with native LaTeX mathematical expressions ($d_8, d_{16}, \leftarrow, \text{EA} = \dots$). Total pages: 22. |
| **2026-10-01** | Expansion (Page 23) | Appended Source Page 27 (Figure 2-5: Word Organization in Memory) to test anti-table regression for spatial memory maps with jagged break lines. Total pages: 23. |
| **2026-10-01** | Expansion (Page 24) | Appended Source Page 28 (Figure 2-6: Data Organization in Memory) to test anti-table regression for multi-part composite data storage encoding figures. Total pages: 24. |
| **2026-10-01** | Expansion (Page 25) | Appended Source Page 29 (Figure 2-7: Memory Data Organization of the MC68008) to test anti-table regression for byte-serial memory packing diagrams with hierarchical bracket groupings. Total pages: 25. |
| **2026-10-01** | Expansion (Pages 26–29) | Appended Source Pages 32–35 (Table 2-2: Instruction Set Summary, Sheets 1–4 of 4) to establish a dedicated multi-page spanning table fusion test for Stage 20 and Stage 21. Total pages: 29. |
| **2026-10-01** | Expansion (Page 30) | Appended Source Page 36 (Figure 3-1: Input and Output Signals) to test anti-table regression for integrated circuit (IC) pinout block diagrams with directional signal arrows and functional grouping brackets. Total pages: 30. |
| **2026-10-01** | Expansion (Page 31) | Appended Source Page 39 (Section 3.3 Asynchronous Bus Control) to test active-low bus signal representation ($\overline{\text{AS}}$, $\text{R}/\overline{\text{W}}$, $\overline{\text{UDS}}$, $\overline{\text{LDS}}$) and heal scanned OCR overbar corruption (e.g. `Address Strobe (~)` -> `_AS` / `$\overline{\text{AS}}$`). Total pages: 31. |
| **2026-10-01** | Expansion (Page 32) | Appended Source Page 41 (Bus Arbitration, Interrupt Control, and MC68008 NOTE block) to test proper GitHub Flavored Markdown alert blockquote formatting (`> [!NOTE]`). Total pages: 32. |
| **2026-10-01** | Expansion (Pages 33–39) | Appended Source Pages 44, 47, 59, 62, 70, 93, 96 to test HTML table spans (Tables 3-3 & 6-1), multi-figure page decomposition (Figures 4-1 & 4-2), cross-reference links (`refer to Appendix B`), CPU space encoding bailout (Figure 5-10), crop isolation vs. text notes/legends (Figure 5-18), and blank page handling. Total pages: 39. |
| **2026-10-01** | Expansion (Pages 40–46) | Appended Source Pages 112, 115, 208–212 to test Figure-captioned bitfield table conversion (Figure 6-9), HTML table cell text formatting and entity escaping (Tables 7-1 & 7-2), and multi-page 2-column index linearization into single-column Markdown with active links (Pages 208–212). Total pages: 46. |
| **2026-10-01** | Enhanced Mapping Matrix | Upgraded Section 2 with dedicated Source Manual Breakdown and explicit columns for Source Manual / Book, Source PDF Page, and Printed Book Folio across all 46 test pages. |
| **2026-10-01** | Multi-Manual Expansion (Pages 47–48) | Appended Source Pages 13 and 16 from `68000 Programmer's Reference Manual` (Figure 1-1: M68000 Family User Programming Model, Figure 1-3: Floating-Point Control Register) to establish multi-manual test suite and test anti-table regression for PRM programming models and 2D branching pointer trees. Total pages: 48. |
| **2026-10-01** | Register Map Policy Adjustment | Withdrew table conversion for Figure 6-9 (Special Status Word Format) on Page 40; reclassified as an anti-table regression test strictly preserving image status across all figures/diagrams. |
| **2026-10-01** | Multi-Manual PRM Expansion (Pages 49–57) | Appended 9 new test pages from `68000 Programmer's Reference Manual` (Pages 22, 23, 29, 32, 42, 46, 65, 80, 97) covering composite figure splitting (Figure 1-8), 32-bit register bitfield tables (Figure 1-9), nested HTML tables (Table 1-4), instruction word bit tables (Figure 2-1), multi-section prose vs. crop isolation (Page 46), table/diagram splits (Figure 2-5), complex graphical table image bailout (Table 3-5), and flowchart image handling (Figure 3-2). Total pages: 57. |
| **2026-10-01** | Multi-Manual PRM Expansion (Pages 58–76) | Appended 19 new test pages from `68000 Programmer's Reference Manual` (Pages 104, 105, 106–111, 112, 338, 597–603, 632, 639) to validate instruction description format bailout (Figure 3-3), side-by-side table detection (Page 105), multi-page ADD instruction fusion without header repetition (Pages 108–110), side-by-side addressing mode column unrolling (Page 112), dual left/right aligned HTML tables (FDIV Page 338), 7-page appendix table unification (Table A-1 Pages 597–603), and exception stack frame image bailouts (Figures B-7, B-8, B-21, B-22). Total pages: 76. |
| **2026-10-01** | Tri-Manual Expansion (Pages 77–83) | Appended 7 new test pages from `A500 A2000 Technical Reference Manual` (Pages 5, 7, 13–15, 20, 21) covering keyboard scan code layout image bailout (Figure 1.1), side-by-side connector graphic and table vertical linearization, 3-page raw key code table unification with intermediate footnote relocation (Table 1-1), and standardized GitHub Flavored Markdown alert blockquotes (`> [!WARNING]` and `> [!NOTE]`). Total pages: 83. |
| **2026-10-01** | Expansion to 92 Pages | Appended 9 new test pages from `A500 A2000 Technical Reference Manual` (Pages 29–31, 42–45, 109, 110) covering Auto-Config bit descriptions as text, reserved address register table conversion, bus signal drive multi-page table fusion, PAL16L8/PAL16R6 equations as fenced code blocks, and dual emulator memory mapping tables. Total pages: 92. |
| **2026-10-01** | Expansion to 109 Pages | Appended 17 new test pages from `A500 A2000 Technical Reference Manual` (Pages 113, 124, 126, 131, 135–139, 151, 152, 155, 178, 184, 201, 202, 205) covering PC/AT I/O mapping, BIOS/Janus API calling conventions with register bindings (D0/A1, AH/AL/ES:DI), multi-page assembly and C code listings with clear file boundary demarcation (`janus_i86block.i` -> `janus.i`, `janus.h` -> `janus_memrw.h`), HDC command summaries, HTML tables with multi-byte colspans (`Table 5-10`), and Amiga custom chip Blitter & Copper register/instruction tables with footnotes. Total pages: 109. |
| **2026-10-01** | Expansion to 117 Pages | Appended 8 new test pages from `A500 A2000 Technical Reference Manual` (Pages 207, 211, 228, 233, 235, 237, 247, 248; Page 236 skipped per user instruction) covering DDFSTRT/DDFSTOP timing tables & bit assignments, DMA time slot diagram text-wrap flow, PAL20L8 logic in code block, motherboard jumper diagrams as images, large-format schematic foldout (Page 235), backplane signal routing, A2000 keyboard connector tri-part layout, and mouse mechanical drawing & connection table. Total pages: 117. |
| **2026-10-01** | Multi-Manual Expansion to 127 Pages (Source 4) | Appended 10 target pages from `Hardware Reference Manual` (Source PDF Pages 26, 28, 35, 37, 42, 43, 57, 58, 71, 84). Suite now encompasses all 4 major technical manuals (46 UM, 30 PRM, 41 A500 TRM, 10 HRM = 127 pages). Codified test rules for dual-language code blocks, sandwich alert/table layouts, Copper sample assembly continuations, multi-level span HTML tables with native LaTeX math, and grouped dual-playfield table disaggregation. |
| **2026-10-01** | Expansion to 133 Pages (HRM Part 2) | Appended 6 new target pages from `Hardware Reference Manual` (Source PDF Pages 88, 90, 109, 124, 136, 145). Suite now totals 133 pages (46 UM, 30 PRM, 41 A500 TRM, 16 HRM). Codified test rules for sequential multi-diagram linearization (3 figures on Page 88, 2 figures on Page 90), high-resolution color table with rowspans, sprite assembly data structures and GFM `> [!CAUTION]` alert blockquotes, 16-bit spanning header tables (`Table 4-4`), and grouped sprite range tables with footnotes (`Table 4-6`). |
| **2026-10-01** | Expansion to 145 Pages (HRM Audio & Blitter) | Appended 12 target pages from `Hardware Reference Manual` (Source PDF Pages 151, 153, 155, 156, 158, 159, 171, 176, 183, 186, 187, 191). Suite now totals 145 pages (46 UM, 30 PRM, 41 A500 TRM, 28 HRM). Codified test rules for waveform graphic crop vs data table extraction, assembly audio offset code block, volume values table, LaTeX DMA sampling math & clock spans, top sample values table, DMACON enable bits table, sampling rate/frequency table, four-table waveform buffer page decomposition, blitter memory map crop, minterm truth table with boolean negations, minterm logic equations with GFM `> [!NOTE]` alert, and dual minterm bit alignment monospaced code blocks. |
| **2026-10-01** | Expansion to 152 Pages (HRM Blitter Line Mode & Timing) | Appended 7 target pages from `Hardware Reference Manual` (Source PDF Pages 201, 203, 204, 205, 206, 208, 210). Suite now totals 152 pages (46 UM, 30 PRM, 41 A500 TRM, 35 HRM). Codified test rules for blitter cycle sequence table, octant line drawing table with indented setup code block, multi-page line mode register summary code blocks (Pages 204-205), blitter speed LaTeX calculations (7.16 MHz NTSC / 7.09 MHz PAL), 90-degree clockwise rotated crop normalization for Figure 6-9 DMA time slot allocation, and anti-table regression test for dual time slot display figures (Figures 6-11 and 6-12). |
| **2026-10-01** | Expansion to 168 Pages (HRM System Control, Interface & Appendices) | Appended 16 target pages from `Hardware Reference Manual` (Source PDF Pages 229, 234, 241, 256, 257, 267, 270, 271, 277, 278, 279, 293, 301, 302, 303, 308). Suite now totals 168 pages (46 UM, 30 PRM, 41 A500 TRM, 51 HRM). Codified test rules for beam position counter table, spanned interrupt priority HTML table, dual controller port tables, 2-page disk subsystem table fusion (Table 8-5), keyboard matrix graphics image regression, serial/audio register tables (SERDATR/ADKCON), Appendix A 3-page continuous register code blocks (Pages 277-279), joystick data register formatting, Appendix B 3-page master register address map table fusion (Pages 301-303), and complete Agnus chip pinout table. |
| **2026-10-01** | Expansion to 177 Pages (HRM Appendices D–H Complete) | Appended 9 target pages from `Hardware Reference Manual` (Source PDF Pages 312, 320, 321, 336, 346, 356, 362, 368, 369). Suite now totals 177 pages (46 UM, 30 PRM, 41 A500 TRM, 60 HRM). Codified test rules for Appendix D hardware memory map table ($000000-$FFFFFF), Appendix E parallel port pinout and ASCII output timing waveform code blocks, Appendix F 10-column CIA-A register address map and CRA bitfield map, Appendix G Auto-Config nibble bracket callout code blocks, Appendix H keyboard handshake protocol waveform, and 2-page unified keyboard matrix HTML table with multi-row spans. |
| **2026-10-01** | Expansion to 178 Pages & Global Index Policy Update | Appended HRM Page 391 (Index 373) as Test Page 178. Suite now totals 178 pages (46 UM, 30 PRM, 41 A500 TRM, 61 HRM). Enforced updated global architectural policy per user instruction: all back-of-the-book printed index pages (Test Pages 42–46 from UM and Test Page 178 from HRM) must be transcribed through Stages 5–19 as clean text, but must be systematically stripped / excluded from the final merged publication chapters in Stage 20/21. |
| **2026-10-01** | Suite Pruning to 160 Pages (Optimization) | Pruned 18 redundant pages across all 4 manuals (Points 1, 2, 3, 4, 6 approved by user; Section 4 instruction suite preserved). Removed: 4 redundant index pages (UM pp. 43–46), 5 oversized table continuation sheets (Table A-1 pp. 70–73, Table 2-2 p. 28), 3 multi-page assembly/register repeats (Janus pp. 98–99, Appendix A p. 162), 4 front-matter duplicates (blank page 38, side-tab page 3, TOC pp. 10–11), and 2 flat table/code duplicates (HRM p. 138, p. 149). Suite reduced from 178 to 160 pages (37 UM, 26 PRM, 39 TRM, 58 HRM) while preserving 100% of distinct retrocomputing challenges and edge cases. |
