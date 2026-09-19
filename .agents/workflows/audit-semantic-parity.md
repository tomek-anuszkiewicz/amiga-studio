---
name: audit-semantic-parity
description: Execute inference-driven bidirectional code-to-docs and docs-to-code semantic parity audit for Amiga 500 subsystems
---

# Workflow: Inference-Driven Bidirectional Semantic Parity Audit

Use this workflow to conduct a deep, qualitative architectural audit that assesses bidirectional parity between active Rust code (`crates/<crate>/src/`) and living Obsidian design documentation (`Obsidian/Amiga/Design/<Spec>.md`).

---

## 1. Triggering the Workflow

Execute directly in the agent session using:

```text
/audit-semantic-parity [subsystem]
```

### Supported Subsystem Targets:
- `interrupts`: [`crates/interrupts`](../../crates/interrupts) $\leftrightarrow$ [`Interrupts.md`](../../Obsidian/Amiga/Design/Interrupts.md)
- `paula`: [`crates/paula`](../../crates/paula) $\leftrightarrow$ [`Paula.md`](../../Obsidian/Amiga/Design/Paula.md)
- `audio`: [`crates/audio`](../../crates/audio) $\leftrightarrow$ [`Audio.md`](../../Obsidian/Amiga/Design/Audio.md)
- `floppy`: [`crates/floppy`](../../crates/floppy) $\leftrightarrow$ [`Floppy.md`](../../Obsidian/Amiga/Design/Floppy.md)
- `agnus`: [`crates/agnus`](../../crates/agnus) $\leftrightarrow$ [`Agnus.md`](../../Obsidian/Amiga/Design/Agnus.md)
- `copper`: [`crates/copper`](../../crates/copper) $\leftrightarrow$ [`Copper.md`](../../Obsidian/Amiga/Design/Copper.md)
- `blitter`: [`crates/blitter`](../../crates/blitter) $\leftrightarrow$ [`Blitter.md`](../../Obsidian/Amiga/Design/Blitter.md)
- `dma`: [`crates/dma`](../../crates/dma) $\leftrightarrow$ [`DMA.md`](../../Obsidian/Amiga/Design/DMA.md)
- `denise`: [`crates/denise`](../../crates/denise) $\leftrightarrow$ [`Denise.md`](../../Obsidian/Amiga/Design/Denise.md)
- `sprites`: [`crates/sprites`](../../crates/sprites) $\leftrightarrow$ [`Sprites.md`](../../Obsidian/Amiga/Design/Sprites.md)
- `frame_builder`: [`crates/frame_builder`](../../crates/frame_builder) $\leftrightarrow$ [`Frame Buffer.md`](../../Obsidian/Amiga/Design/Frame%20Buffer.md)
- `memory_bus`: [`crates/memory_bus`](../../crates/memory_bus) $\leftrightarrow$ [`MemoryBus.md`](../../Obsidian/Amiga/Design/MemoryBus.md)
- `cia`: [`crates/cia`](../../crates/cia) $\leftrightarrow$ [`CIA.md`](../../Obsidian/Amiga/Design/CIA.md)
- `m68000`: [`crates/m68000`](../../crates/m68000) $\leftrightarrow$ [`CPU Motorola M68000.md`](../../Obsidian/Amiga/Design/CPU%20Motorola%20M68000.md)
- `machine_loop`: [`crates/machine_loop`](../../crates/machine_loop) $\leftrightarrow$ [`Main loop A500.md`](../../Obsidian/Amiga/Design/Main%20loop%20A500.md)

*Default (no argument):* If no target is specified, the agent identifies recently modified subsystems from `git status` / `git diff --stat` or audits the primary coordinator (`paula` or `agnus`).

---

## 2. Audit Execution Protocol

Follow the operational instructions in [`audit-semantic-parity`](../skills/audit-semantic-parity/SKILL.md):

1. **Step 1 (Source Extraction):**
   - Read the crate's root source file (`crates/<crate>/src/<crate>.rs`) and secondary modules.
   - Catalog public types, state struct fields, register bitmasks, methods, and timing models.

2. **Step 2 (Specification Extraction):**
   - Read the authoritative design specification (`Obsidian/Amiga/Design/<Spec>.md`).
   - Catalog section headings, register tables, causal descriptions, and timing claims.

3. **Step 3 (3-Vector Inference Evaluation):**
   - **Vector 1 (Forward Parity):** Identify code mechanics (registers, bits, state machines, errata) missing from docs (**Blind Spots**).
   - **Vector 2 (Reverse Parity):** Identify spec claims, speculative snippets, or hypothetical modes missing from code (**Ghost Features**).
   - **Vector 3 (Silicon Rigor):** Score the depth from 0% to 100% based on bit-level accuracy and Inverted Pyramid structure.

4. **Step 4 (Report Emission):**
   - Format and output the standardized report using the template from `audit-semantic-parity/SKILL.md`.

5. **Step 5 (Remediation):**
   - If `NEEDS_DOCS_ENRICHMENT`, update the design specification and run `audit-docs-quality` to verify linking and checkpoints.
   - If `NEEDS_CODE_ALIGNMENT` or `SPEC_DIVERGENCE`, escalate to the user before modifying production code per `spec-compliance.md`.
