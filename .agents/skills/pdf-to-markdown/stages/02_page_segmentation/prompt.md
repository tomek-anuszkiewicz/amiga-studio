# Page Layout & Semantic Text Block Classification Prompt

You are an expert technical document layout and typography analyzer for computer manuals.
Inspect the attached 300 DPI high-resolution page image alongside the extracted text block bounding boxes.
Classify each of the extracted text blocks into exactly ONE semantic type based on its visual appearance and position:

- `header`: Running top-margin document header or chapter title rule at the very top of the page.
- `footer`: Running bottom-margin footer or page number at the very bottom of the page.
- `toc_header`: Prominent Table of Contents title banner (e.g. 'Contents', 'Table of Contents').
- `toc`: Table of contents entries, chapter listings, and page number entries.
- `chapter`: A new chapter start. The opening segment on a page indicating a new chapter (e.g. 'Chapter 1', 'Chapter 2', 'Appendix A', or major standalone chapter opening banner). It is the first segment on a page indicating a new chapter.
- `heading`: Section headings, subheadings, topic titles, and data block titles within an ongoing chapter (e.g. 'Copper Instruction Summary', 'Register Map', '256 Byte Sample', '128 Byte Sample'). A `heading` block must consist solely of the title itself. If a paragraph begins with a run-in heading followed immediately by body text sentences in the same block (e.g. '1.2.3.4 ACCRUED EXCEPTION BYTE. The AEXC byte contains five exception bits...'), you MUST classify the block as `prose` (NOT `heading`), so that the run-in title can be formatted inline as bold without turning the entire paragraph into a heading. Do NOT classify standalone subsection titles as table captions unless it is a formal table title starting with 'Table X-Y:'. Use heading_level=1 for major sections, 2 for subsections, 3 for sub-headers.
- `prose`: Standard narrative prose body paragraphs. Includes paragraphs that start with run-in subsection headings followed by body text.
- `code_block`: Monospace code listings, assembly language, memory hex dumps, or preformatted numeric waveform/sample data arrays (e.g. 16 values per row).
- `table`: Formal tabular data grids, multi-column register bit assignments, and structured parameter lists. Do NOT classify formal table caption lines (e.g. 'Table 5-8: Five Octave Even-tempered Scale') as table—classify the title line separately as caption.
- `graphic`: Circuit schematics, timing waveforms, block diagrams, IC pinouts, photographs, and diagram artwork. NEVER classify tables, table titles, or figure captions as graphic. For any block categorized as graphic, specify 'graphic_bbox_norm': [x0, y0, x1, y1] in normalized coordinates (0.0 to 1.0) enclosing ONLY the visual artwork/diagram area on the page (the schematic drawing, plots, state circles, waveforms), EXCLUDING any textual figure caption line (which must be classified separately as caption). For all other block types, graphic_bbox_norm must be null.
- `caption`: Formal caption or title lines for figures or tables (e.g. 'Figure 5-2: Digitized Amplitude Values', 'Table 5-8: Five Octave Even-tempered Scale'). NEVER classify captions as graphic or table—classify the caption line itself as caption.

Return a strict JSON array of objects with fields:
[{"idx": 0, "type": "prose", "heading_level": null, "graphic_bbox_norm": null}, {"idx": 3, "type": "graphic", "heading_level": null, "graphic_bbox_norm": [x0, y0, x1, y1]}, ...]
