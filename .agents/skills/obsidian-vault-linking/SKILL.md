---
name: obsidian-vault-linking
description: Audit and repair YAML frontmatter properties, dual-layer linking, and relative path integrity in Obsidian specs.
---

# Recipe: Obsidian Properties & Dual-Layer Vault Linking

This skill defines the complete operational checklist and formatting template for maintaining knowledge graph integrity and dual-layer linking across [Obsidian/Amiga/Design/](../../../Obsidian/Amiga/Design/) per [docs-maintenance.md](../../rules/docs-maintenance.md#vault-metadata-and-link-integrity).

---

## 1. When to Trigger This Skill

- **Mandatory Trigger:** Whenever any note under `Obsidian/Amiga/Design/` is created, modified, or reorganized.
- **Scope:** All 28+ architecture specifications, hardware reference notes, and developer workflow guidelines.

---

## 2. Step-by-Step Linking & Properties Procedure

### Step 1: Evaluate and Update Obsidian Properties (YAML Frontmatter)
Ensure line 1 contains an active, up-to-date YAML frontmatter block:
```yaml
---
title: "<Full Subsystem / Document Title>"
aliases: ["<Short Code>", "<Chip Revision>", "<Alternate Name>"]
tags: ["amiga", "design", "<subsystem-tag>"]
category: "Design"
subsystem: "<subsystem_name>" # agnus, denise, paula, m68000, memory_bus, cia, gui, debugger
status: "active" # active | completed | draft
created: YYYY-MM-DD
updated: YYYY-MM-DD
related: ["[SiblingDoc.md](SiblingDoc.md)"]
---
```
**Audit Check:**
- If substantive content changed, is `updated` set to today (`YYYY-MM-DD`)? A review without content corrections must leave dates unchanged.
- Does `status` reflect current implementation maturity?
- Are `related` links formatted as valid relative markdown links?

### Step 2: Weave Contextual Inline Links (Layer 1)
- Contextually embed links directly into prose, block diagrams, and register breakdowns where components are first introduced.
- Cross-link to related subsystem design docs (`[Agnus.md](Agnus.md)`), system rules (`[`performance-and-readability.md`](../../../.agents/rules/performance-and-readability.md)`), and Rust source files (`[`crates/cpu/src/state.rs`](../../../crates/cpu/src/state.rs)`).
- Keep links natural, high-signal, and informative.

### Step 3: Audit Structural Reference Section (Layer 2)
Every design specification must conclude with a dedicated reference section:
```markdown
## Reference Documentation & Upstream Ground Truth

- **Commodore Amiga Hardware Reference Manual**: [Chapter X - Title](../Reference/Hardware%20Reference%20Manual/XX%20-%20Chapter%20X%20-%20Title.md)
  - *Analytical Rationale*: Defines register bitfields, timing diagrams, and DMA slot assignments.
- **Motorola M68000 Programmer's Reference Manual**: [Section X](../Reference/68000%20User's%20Manual/XX%20-%20Section%20X.md)
  - *Analytical Rationale*: Ground truth for microcode execution phases and condition code updates.
- **Silicon Reference Implementations**:
  - [vAmiga Subsystem](../../../ref_src/vAmiga-4.5/Core/Chips/...): Verified behavioral reference for cycle timing.
  - [WinUAE Subsystem](../../../ref_src/WinUAE-6030/...): Ground truth for hardware quirks and edge cases.
- **Living Crate Source Code**:
  - [`crates/<crate>/src/<module>.rs`](../../../crates/<crate>/src/<module>.rs): Primary Rust implementation.
```
**Audit Check:**
- Does every entry include a concise 1-sentence analytical rationale?

### Step 4: Verify Path Depth & Zero Broken Links
Scan all design notes recursively, including organizational subdirectories.
Resolve each relative link from the parent directory of its source document.
The following examples apply to a note directly in the Design root:
- Sibling Design Doc: `[File.md](File.md)`
- Reference Manual: `[Title](../Reference/<Folder>/<File>.md)`
- Workspace Crates: `[`crates/<crate>/...`](../../../crates/<crate>/...)`
- Operational Rules: `[`rule.md`](../../../.agents/rules/<rule>.md)`
- Reference Sources: `[`ref_src/...`](../../../ref_src/<path>)`

For a note in `Design/Application/`, `Design/Verification/`, or
`Design/Performance/`, add one `../` when linking to the repository or Reference
roots. Link sibling notes using their actual relative position; for example,
`[MemoryBus.md](../MemoryBus.md)` or `[CPU SingleStepTests.md](../Verification/CPU%20SingleStepTests.md)`.

Run automated verification:
```powershell
python .agents/skills/obsidian-vault-linking/scripts/audit_vault.py --vault-links --frontmatter
```
Ensure both checks complete with zero issues.

---

## 5. Standard Audit Report Format

```markdown
### 🔗 Obsidian Vault Link Audit Report
- **Documents Audited:** 28 design specifications
- **Audit Status:** [ALL PASS | REPAIRED]
- **Broken Links Fixed:**
  | Document | Broken Link Target | Corrected Relative Path |
  | :--- | :--- | :--- |
  | `Obsidian/Amiga/Design/...` | `[Wrong.md]` | `[Correct.md](Correct.md)` |
- **Script Result:** <actual link/frontmatter results; report missing checks>.
```
