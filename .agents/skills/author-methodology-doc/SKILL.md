---
name: author-methodology-doc
description: Author, audit, or restructure narrative articles, methodology documents, essays, and retrospective devlogs (e.g. docs/how_this_emulator_was_written.md) using the top-down Inverted Pyramid hierarchy.
---

# Recipe: Authoring & Restructuring Narrative Documents (Inverted Pyramid Model)

This skill provides a systematic procedure for authoring, evaluating, and restructuring narrative articles, methodology papers, retrospective devlogs, and architectural case studies (such as [`docs/how_this_emulator_was_written.md`](../../../docs/how_this_emulator_was_written.md)) according to the **Inverted Pyramid model** and **top-down cognitive progression**.

---

## 1. Target Document Resolution

When executing this skill:
1. **Explicit Target Argument (`<TARGET_DOCUMENT>`):**
   - Extract the target file path passed by the user (e.g. `/author-methodology-doc docs/how_this_emulator_was_written.md` or `"apply skill to <path>"`).
2. **Implicit Fallback (No argument provided):**
   - If no document path was explicitly specified in the prompt:
     - Check the currently active / focused file from the session context (open IDE tab).
     - If still ambiguous or no candidate document is evident, stop and ask the user:
       *"Which document would you like to author or restructure using the Inverted Pyramid model?"*
3. **Verification:**
   - Verify that the target file exists before attempting edits (or confirm intent if creating a new file from scratch).

---

## 2. When to Trigger This Skill

- **Authoring:** Writing new retrospective articles, case studies, architectural post-mortems, or public devlogs.
- **Restructuring:** Auditing existing methodology documents (like [`docs/how_this_emulator_was_written.md`](../../../docs/how_this_emulator_was_written.md)) where key insights, core breakthroughs, or decision frameworks are buried late in the document.
- **Integrating New Findings:** Weaving substantive user feedback, new experimental conclusions, or philosophical breakthroughs into an existing document without falling into the "bottom-heavy accumulation trap".

---

## 3. The 6-Layer Inverted Pyramid Architecture

Readers absorb information in a top-down narrative. When opening a document, reader attention and cognitive energy are at their peak. Content must progress from high-impact paradigm shifts down to granular execution details:

```text
┌─────────────────────────────────────────────────────────────┐
│ 1. THE HOOK & CORE THESIS                                   │
│    The bold paradigm shift, economic inversion, or          │
│    decisive architectural conclusion (opening 20–50 lines). │
├─────────────────────────────────────────────────────────────┤
│ 2. STRATEGIC & HUMAN DIMENSIONS                             │
│    Root problems, human bottlenecks, systemic traps         │
│    (e.g., Unverified Generation, The Frankenstein Phase).   │
├─────────────────────────────────────────────────────────────┤
│ 3. CORE ARCHITECTURAL PATTERNS & SOLUTIONS                  │
│    The primary mechanisms that solve the dilemma            │
│    (e.g., Minimal Frame, Exploratory Pruning, Shadow-Twin). │
├─────────────────────────────────────────────────────────────┤
│ 4. SUBSTRATE & EXECUTION REALITIES                          │
│    Hardware realities, cache alignment, compiler            │
│    optimization, physical memory layout (DOD/SoA).          │
├─────────────────────────────────────────────────────────────┤
│ 5. TACTICAL EXECUTION & DEVELOPER WORKFLOWS                 │
│    Granular commit sequences, test harnesses, review        │
│    rules, and operational checklists.                       │
├─────────────────────────────────────────────────────────────┤
│ 6. SYNTHESIS & RELATIONSHIP TO KNOWLEDGE GRAPH              │
│    Summary principles, key takeaways, and curated links.    │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. Layer-by-Layer Breakdown

### Layer 1: The Hook & Core Thesis (Lines 1–50)
- **Objective:** Grab the reader immediately with the most transformative, decisive conclusion, non-intuitive paradigm shift, or empirical breakthrough.
- **Rule:** Never bury foundational conclusions, core decision matrixes, or defining mental models in trailing sections or appendices. Deliver high signal within the first 20–50 lines.
- *Example from [`docs/how_this_emulator_was_written.md`](../../../docs/how_this_emulator_was_written.md):* Opening with *"Zero Hand-Written Code (and Zero Rust Experience)"*—setting the radical premise upfront.

### Layer 2: Strategic & Human / Structural Dimensions
- **Objective:** Frame the stakes, friction points, and human or architectural bottlenecks that forced the change.
- **Elements:** Root causes of failure, systemic antipatterns (e.g. unverified generation, brittle prompt compounding), and the philosophical dilemma.

### Layer 3: Core Architectural Patterns & Solutions
- **Objective:** Present the overarching conceptual mechanisms that resolve the Layer 2 dilemma.
- **Elements:** The core mental model (e.g. *The "Minimal Frame" Rule*, *Exploratory Pruning*, *Decoupled Chip Ownership*). Include architectural diagrams (Mermaid) illustrating the pattern.

### Layer 4: Substrate & Execution Realities
- **Objective:** Ground the architectural pattern in physical reality and technical constraints.
- **Elements:** Host CPU execution models (superscalar pipelines, branch predictability, cache locality), guest silicon constraints (CCK clock phases, bus contention), or compiler optimization realities.

### Layer 5: Tactical Execution & Developer Workflows
- **Objective:** Provide concrete, reproducible steps, tooling, and day-to-day practices.
- **Elements:** The verification harness (e.g. silicon test vectors, architecture tests), automated gates, commit sequences, and operational recipes.

### Layer 6: Synthesis & Knowledge Graph Relationships
- **Objective:** Synthesize the overarching thesis into actionable takeaways and connect the note to the broader knowledge graph.
- **Elements:** Axiomatic summary bullets, upstream references, and dual-layer curated links.

---

## 5. Core Operating Principles & Anti-Trap Safeguards

### A. Eliminating the "Bottom-Heavy Accumulation Trap"
- During pair programming or document revision, new insights, corrections, or user revelations frequently emerge.
- **Mandatory Anti-Trap Rule:** Never default to simply appending new insights to the bottom of the document.
- Analyze where the new insight belongs hierarchically:
  - If it represents a **Core Thesis / Paradigm Shift** $\rightarrow$ Weave it into Layer 1 (opening 20–50 lines).
  - If it represents a **First-Step Methodology** (e.g. proof-of-concept verification) $\rightarrow$ Place it in Layer 3 (Phase 1 of Core Patterns).
  - If it represents an **Operational Guardrail or Command** $\rightarrow$ Place it in Layer 5 (Tactical Execution).

### B. 100% Content & Thought Preservation Standard
- Re-hierarchization must be **purely structural**.
- **Zero Dilution Policy:** Never discard, dilute, or summarize away substantive technical thoughts, code snippets, mathematical formulas, timing tables, or Mermaid diagrams during restructuring.
- Rearrange and polish the narrative flow while strictly preserving 100% of the conceptual substance.

---

## 6. Practical Step-by-Step Restructuring Workflow

When auditing or restructuring an existing document:

1. **Step 1: Outline Extraction & Mapping**
   - Read `<TARGET_DOCUMENT>` completely.
   - Extract all heading levels (`#`, `##`, `###`) and map the existing narrative flow.

2. **Step 2: Identify Buried Treasures**
   - Carefully inspect the **bottom 30%** of the document (appendices, trailing notes, final subsections).
   - Identify high-impact conclusions, profound architectural inversions, or prerequisite mental models that are languishing at the end.

3. **Step 3: Top-Down Restructuring**
   - Re-order sections following the 6-layer progression:
     $$\text{Hook} \longrightarrow \text{Strategic Stakes} \longrightarrow \text{Core Mechanisms} \longrightarrow \text{Substrate} \longrightarrow \text{Tactical Execution} \longrightarrow \text{Synthesis}$$
   - Weave identified "buried treasures" into their appropriate top-down layers.

4. **Step 4: Verify Substantive Completeness**
   - Run a comparative diff (`git diff` or before/after inspection).
   - Verify that every technical detail, formula, code block, diagram, and link from the original document is present and intact.

5. **Step 5: Atomic Git Commit**
   - Commit the restructured document with an atomic, intent-driven Conventional Commit:
     ```text
     refactor(docs): re-hierarchize [Document Title] for top-down inverted pyramid
     ```
