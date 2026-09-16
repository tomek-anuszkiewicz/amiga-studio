---
trigger: model_decision
description: Mandatory dual-layer linking standard, inverted pyramid information hierarchy, and zero broken links across Obsidian design documentation.
---

# Obsidian Vault Linking, Information Hierarchy & Graph Integrity Rule

This rule governs all design specifications, hardware reference links, architectural guidelines, and markdown documentation located under `Obsidian/Amiga/Design/`.
Whenever creating, modifying, updating, or refactoring documentation in this vault, the agent must actively and systematically maintain the knowledge graph structure, dual-layer linking, and information hierarchy.

## 1. Mandatory Obsidian Properties (YAML Frontmatter at Line 1)

Every markdown document in `Obsidian/Amiga/` (especially under `Obsidian/Amiga/Design/`) must begin with an active **Obsidian Properties** block (YAML frontmatter bounded by `---` lines at line 1).

### Standard Properties Schema
```yaml
---
title: "Document / Subsystem Title"
aliases: ["Alternate Title", "Chip Code", "Mnemonic"]
tags: ["amiga", "design", "subsystem-name"]
category: "Design" # "Design" | "Reference"
subsystem: "agnus" # "agnus" | "denise" | "paula" | "m68000" | "memory_bus" | "cia" | "gui" | "debugger"
status: "active" # "active" | "completed" | "draft"
created: YYYY-MM-DD
updated: YYYY-MM-DD
related: ["[SiblingDoc.md](SiblingDoc.md)"]
---
```

### Continuous Evaluation & Maintenance Contract
- **Evaluate on Every Edit:** Whenever an agent creates, refactors, or modifies any document in the vault, it **must inspect, evaluate, and update its properties block**:
  - Bump `updated` to the current date (`YYYY-MM-DD`).
  - Verify and update `status` if the subsystem or milestone transitioned (e.g. `draft` $\rightarrow$ `active` $\rightarrow$ `completed`).
  - Refresh `related` links and `tags` if new subsystem couplings or hardware dependencies were introduced.
- **Line 1 Placement:** The properties block must strictly reside at the very top of the file (Line 1 `---`) so Obsidian natively indexes it as note properties.

---

## 2. Mandatory Dual-Layer Linking Standard

Every design document in `Obsidian/Amiga/Design/` must maintain two distinct, complementary layers of connectivity:


1. **Layer 1: Contextual Inline Navigational Links:**
   - Embedded directly into body paragraphs, data flow explanations, and register breakdowns at the precise moments concepts, components, or micro-steps are introduced or referenced.
   - Cross-link to related subsystem design docs (`[Agnus.md](Agnus.md)`, `[MemoryBus.md](MemoryBus.md)`), system rules (`[`performance-and-readability.md`](../../../.agents/rules/performance-and-readability.md)`), and concrete Rust implementation source files (`[`crates/m68000/src/state.rs`](../../../crates/m68000/src/state.rs)`).
   - Links should be natural, high-signal, and informative.

2. **Layer 2: Structural Referential Section at Bottom (`## Reference Documentation & Upstream Ground Truth`):**
   - Every design document must conclude with a dedicated reference section containing curated links to:
     - **Official Upstream Documentation:** Commodore Amiga Hardware Reference Manual (`../Reference/Hardware Reference Manual/...`), Motorola 68000 User Manuals (`../Reference/68000 User's Manual/...`), A500/A2000 Technical Reference Manual, and Amiga Guru Book.
     - **Silicon Reference Emulators:** Verified reference implementations in `ref_src/` (`../../../ref_src/vAmiga-4.5/...`, `../../../ref_src/WinUAE-6030/...`, `../../../ref_src/SingleStepTests-m68000/...`).
     - **Living Crate Source Files:** Direct links to the primary Rust module files implementing the subsystem.
   - **Mandatory Analytical Rationale:** Every item in the reference section must include a concise 1-sentence explanation of its architectural or hardware relationship (e.g. which registers, state machine, or bus phases it defines).

*Single-layer notes are strictly non-compliant*: omitting bottom references destroys scannability and hides primary sources; relegating all links to a bottom list produces disconnected, unlinked prose.

---

## 3. Inverted Pyramid Information Hierarchy & Structure

To ensure maximum signal and top-down cognitive clarity, all design specifications must adhere to the **Inverted Pyramid model** ([`information-hierarchy.md`](information-hierarchy.md)) and maintain a direct, hands-on practitioner tone ([`practitioner-voice-and-tone.md`](practitioner-voice-and-tone.md)):

1. **Top-Down Section Order:**
   - **Document Title & Scope Callout (Lines 1–30):** High-impact summary of subsystem responsibility, hardware chip revision, and note callout linking to master machine coordination and parent guidelines.
   - **Section 1: Hardware Scope & Subsystem Architecture:** Block diagram (Mermaid), signal pinouts, and system integration.
   - **Section 2: Module Decomposition & Rust Crate Structure:** Concrete crate layout under `crates/*` with module responsibilities.
   - **Section 3: Register Memory Map & Bitfields:** Exhaustive address table, access modes (R/W/Strobe), and bit definitions.
   - **Section 4+: Operational State Machines & Circuit Timings:** Cycle-exact micro-operations, Color Clock phases (CCK1/CCK2), DMA contention, and ALU operations.
   - **Pre-Final Section: Reset Defaults & Machine Coordination:** Hardware register initializations on cold/warm reset.
   - **Final Section: Reference Documentation & Upstream Ground Truth:** The Layer 2 curated structural reference list.

2. **Eliminating the "Bottom-Heavy Accumulation Trap":**
   - When integrating new findings, test results, user directives, or hardware quirks during development, **never default to appending them blindly to the bottom of the document**.
   - Integrate new insights into their proper architectural section (scope at top, registers in the memory map, operational quirks in state machines).

---

## 4. Strict Relative Path Depth & Zero Broken Links Policy

All links in `Obsidian/Amiga/Design/` must adhere to mathematically verified relative path depths:

| Target Destination | Relative Path Format from `Obsidian/Amiga/Design/` | Example |
| :--- | :--- | :--- |
| **Sibling Design Doc** | `[File.md](File.md)` or `[Title](File.md)` | `[Agnus.md](Agnus.md)` |
| **Hardware Reference Manual** | `[Title](../Reference/<Folder>/<File>.md)` | `[HRM Ch 2](../Reference/Hardware%20Reference%20Manual/02%20-%20Chapter%202%20-%20Coprocessor%20Hardware.md)` |
| **Workspace Crates** | `[`crates/<crate>/...`](../../../crates/<crate>/...)` | `[`crates/m68000/src/m68000.rs`](../../../crates/m68000/src/m68000.rs)` |
| **Operational Rules** | `[`rule.md`](../../../.agents/rules/<rule>.md)` | `[`docs-maintenance.md`](../../../.agents/rules/docs-maintenance.md)` |
| **Reference Source (`ref_src`)** | `[`ref_src/...`](../../../ref_src/<path>)` | `[`vAmiga Agnus`](../../../ref_src/vAmiga-4.5/Core/Chips/Agnus/Copper.cpp)` |
| **Root Docs (`AGENTS.md`, `ROADMAP.md`)**| `[Doc](../../../<Doc>.md)` | `[AGENTS.md](../../../AGENTS.md)` |

### Zero Broken Links Contract
- Every relative link pointing to an on-disk file must resolve to a valid file.
- Moving or refactoring any Rust source file or markdown document requires updating all referencing links immediately.
- Enforced automatically via `cargo test -p test_runner --test test_architecture_rules`.

---

## 5. Hub-and-Spoke Topology & Canonical Authorities

- **Central Canonical Hub:** [`General Architecture.md`](General%20Architecture.md) serves as the top-level index for the entire emulator architecture. It must catalog and contextually link downward to all subsystem design documents.
- **Lateral Subsystem Connectivity:** Subsystems that interact on the physical bus or via DMA (e.g. Agnus $\leftrightarrow$ MemoryBus, Paula $\leftrightarrow$ Floppy, CIA-A $\leftrightarrow$ Keyboard) must maintain reciprocal cross-links.
- **Small-World Graph Density:** Maintain high-signal semantic links (typically 6–15 high-signal links per document). Avoid superficial link sprawl.

