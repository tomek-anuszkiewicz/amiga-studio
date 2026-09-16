---
trigger: model_decision
description: Inverted pyramid structure, top-down cognitive progression, and elimination of bottom-heavy accumulation across notes and documentation.
---

# Information Hierarchy & Inverted Pyramid Rule

Whenever creating, modifying, updating, or refactoring notes and architectural documentation in this repository, the agent must strictly structure content according to the **Inverted Pyramid model** and **top-down cognitive hierarchy**. This ensures that the most valuable, transformative, and decisive insights lead the document rather than being buried at the bottom.

---

## Core Architecture: The Inverted Pyramid

Readers absorb information in a top-down narrative. When opening a note, reader attention and cognitive energy are at their peak. Content must progress from high-impact paradigm shifts down to granular execution details:

```text
┌─────────────────────────────────────────────────────────────┐
│ 1. THE HOOK & CORE THESIS                                   │
│    The bold paradigm shift, economic inversion, or          │
│    decisive architectural conclusion (opening 20–50 lines). │
├─────────────────────────────────────────────────────────────┤
│ 2. STRATEGIC & PRACTICAL DIMENSIONS                         │
│    Root problems, human bottlenecks, and systemic traps     │
│    (e.g., Unverified Generation, The Frankenstein Phase).   │
├─────────────────────────────────────────────────────────────┤
│ 3. CORE ARCHITECTURAL PATTERNS & SOLUTIONS                  │
│    The primary mechanisms that solve the dilemma            │
│    (e.g., Minimal Frame, Exploratory Pruning, Shadow-Twin). │
├─────────────────────────────────────────────────────────────┤
│ 4. SUBSTRATE & MECHANICAL SYMPATHY                          │
│    Hardware realities, cache lines (L1i vs D-cache),        │
│    compiler optimization, memory layout (DOD/SoA).          │
├─────────────────────────────────────────────────────────────┤
│ 5. TACTICAL EXECUTION & DEVELOPER WORKFLOWS                 │
│    Granular commit sequences, review rules, operational     │
│    checklists, and implementation recipes.                  │
├─────────────────────────────────────────────────────────────┤
│ 6. SYNTHESIS & RELATIONSHIP TO THE KNOWLEDGE GRAPH          │
│    Summary principles and dual-layer curated wikilinks.     │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Operating Principles

1. **Lead with the High-Impact Hook & Core Thesis**:
   - The opening 20–50 lines of every note must deliver the most decisive conclusion, architectural shift, or conceptual breakthrough.
   - Never bury foundational conclusions, core decision matrixes, or defining mental models at the bottom of a document. Engage the reader immediately with high-signal value.

2. **Top-Down Cognitive Progression**:
   Structure notes following a consistent, logical descent through the 6-layer cognitive hierarchy shown above:
   - **Layer 1: The Hook & Core Thesis** (Core takeaways, the central architectural dilemma, definitive outcome).
   - **Layer 2: Strategic & Practical Dimensions** (Root problems, developer bottlenecks, failure traps).
   - **Layer 3: Core Architectural Patterns & Solutions** (Primary mechanisms solving the dilemma, e.g. Minimal Frame, Shadow-Twin, Explicit Code).
   - **Layer 4: Substrate & Mechanical Sympathy** (Hardware realities, CPU cache line behavior, branch predictability, physical memory layout).
   - **Layer 5: Tactical Execution & Developer Workflows** (Granular commit sequences, test harnesses, operational checklists).
   - **Layer 6: Synthesis & Knowledge Graph Relationships** (Summary axioms, parent hubs, and dual-layer curated links).

3. **Eliminating the "Bottom-Heavy Accumulation Trap"**:
   - When integrating user feedback, corrections, or newly introduced insights during conversations, **never default to simply appending them to the bottom of the document**.
   - Analyze where the new insight belongs hierarchically:
     - If it represents a **Core Thesis / Paradigm Shift** $\rightarrow$ weave it into the opening 20–50 lines.
     - If it represents a **First-Step Pattern** $\rightarrow$ place it at the beginning of the technical methodology.
     - If it represents an **Operational Detail** (e.g. commit rules or CLI flags) $\rightarrow$ place it in downstream tactical execution sections.
   - Never append critical concepts as an afterthought.

4. **100% Content & Thought Preservation Standard**:
   - Re-hierarchization must be purely structural. **Never discard or dilute existing substantive thoughts, technical nuances, code blocks, ASCII diagrams, or mathematical formulas**.
   - Rearrange, re-order, and polish the narrative flow while preserving 100% of the conceptual substance.

---

## Practical Note Auditing & Restructuring Workflow

When reviewing or refactoring an existing note for information hierarchy:

1. **Step 1: Outline Extraction**: Read the file and extract all headings (`#`, `##`, `###`). Map the conceptual progression.
2. **Step 2: Identify Buried Treasures**: Scan the bottom 30% of the document. Identify critical failure modes, prerequisite workflows, or golden mental models that are languishing in appendices or trailing sections.
3. **Step 3: Restructure Top-Down**: Re-order sections to encounter the highest-signal concepts first:
   $\text{Hook} \rightarrow \text{Strategic Stakes} \rightarrow \text{Core Mechanisms} \rightarrow \text{Mechanical Sympathy} \rightarrow \text{Tactical Rules} \rightarrow \text{Graph}$.
4. **Step 4: Verify Substantive Completeness**: Perform a diff to confirm zero insights, formulas, code snippets, or links were lost.
5. **Step 5: Atomic Git Commit**: Commit the restructuring with a descriptive, intent-driven message:
   `refactor(structure): re-hierarchize [Note Title] for top-down reader engagement`.
