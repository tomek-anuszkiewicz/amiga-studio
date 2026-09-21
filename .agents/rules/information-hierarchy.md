---
trigger: model_decision
description: Information hierarchy, inverted pyramid structure, constitutional AGENTS.md size ceiling (<= 14,000 bytes), and non-redundancy policy. Load on major roadmap milestones and doc restructuring tasks.
---

# Information Hierarchy, Inverted Pyramid & Document Limits Rule

This rule governs how information, architecture guidelines, specifications, and central constitutions are structured, prioritized, and bounded across this repository to eliminate bottom-heavy bloat, duplicate sprawl, and context truncation.

---

## 1. Constitutional Role of AGENTS.md & Size Limits

[`AGENTS.md`](../../AGENTS.md) is the central architectural constitution for the emulator. It is injected into every agent prompt session. Its responsibilities are strictly limited to:

1. **Section 1 (Rules Index):** Concise, 1-line pointers linking to modularized operating rules under `.agents/rules/*.md`.
2. **Section 2 (Core Architectural Principles):** Target platforms, color clock execution model (`CCK1`/`CCK2`), decoupled ownership without circular references, and hardware circuit simulation.
3. **Section 3 (Machine Systems Invariants):** Guest vs host Big-Endianness, zero panics on guest code, wrapping arithmetic, zero runtime heap allocations in hot paths, and pointers to modular subsystem guidelines.
4. **Section 4 (Definition of Done):** Lean checklist pointing to rules and workflows.
5. **Section 5 (Knowledge Base):** Pointers to specifications, manuals, and test vectors.

### Strict Prohibition of Content Duplication
- **Zero Rule Re-Implementation:** `AGENTS.md` must **never duplicate** verbose operational rules, full skill procedures, code-level examples, or extensive checklists that are already modularized under `.agents/rules/*.md`.
- **Reference by Pointer Only:** Whenever a new operational rule is added, add only a single 1-line bullet in Section 1 pointing to the respective `.agents/rules/<rule>.md` file. Never paste the rule's body into `AGENTS.md`.

### Byte Budget & Automated CI Enforcement
- **Strict Size Ceiling:** `AGENTS.md` must strictly remain $\le 14,000$ bytes on disk.
- **Rationale:** Antigravity silently truncates rule files exceeding ~24,000 bytes. Keeping `AGENTS.md` under 14,000 bytes prevents prompt bloat, conserves context token budget, and eliminates truncation risk.
- **Automated Verification Gate:** Enforced on every build via `check_agents_md()` in `tools/harness/pre_flight.py` and `test_rule_files_size_limit_and_truncation_safety` in `crates/test_runner/tests/test_architecture_rules.rs`.

---

## 2. The Inverted Pyramid Model (Specifications & Notes)

Whenever creating, modifying, updating, or refactoring notes and architectural documentation in this repository, the agent must strictly structure content according to the **Inverted Pyramid model**. Readers start at the top and want the decisive conclusions and architecture immediately, rather than having them buried at the bottom.

### The Inverted Pyramid Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│ 1. CORE TAKEAWAYS & ARCHITECTURAL DECISIONS                 │
│    The decisive conclusions, core invariants, or            │
│    primary design choices (opening 20–50 lines).            │
├─────────────────────────────────────────────────────────────┤
│ 2. PRACTICAL CONTEXT & SYSTEMIC TRAPS                       │
│    Root problems, design trade-offs, and failure traps      │
│    (e.g., Unverified Generation, The Frankenstein Phase).   │
├─────────────────────────────────────────────────────────────┤
│ 3. CORE ARCHITECTURAL PATTERNS & SOLUTIONS                  │
│    The primary mechanisms that solve the dilemma            │
│    (e.g., Minimal Frame, Exploratory Pruning, Shadow-Twin). │
├─────────────────────────────────────────────────────────────┤
│ 4. HARDWARE REALITIES & MECHANICAL SYMPATHY                 │
│    Hardware constraints, bus timing (CCK1/CCK2),            │
│    cache line efficiency, and physical memory layout.       │
├─────────────────────────────────────────────────────────────┤
│ 5. TACTICAL EXECUTION & DEVELOPER WORKFLOWS                 │
│    Granular commit sequences, test harnesses, review        │
│    rules, and implementation recipes.                       │
├─────────────────────────────────────────────────────────────┤
│ 6. SUMMARY & KNOWLEDGE GRAPH RELATIONSHIPS                  │
│    Summary principles, parent hubs, and curated links.      │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Core Hierarchy & Non-Redundancy Principles

1. **Lead with Core Decisions & Key Invariants**:
   - The opening 20–50 lines of every note must deliver the most decisive conclusions and architectural choices.
   - Never bury foundational conclusions, decision matrices, or defining models at the bottom of a document. Give the reader the architectural answer upfront.

2. **Top-Down Information Hierarchy**:
   Structure notes following a consistent descent from architecture to implementation:
   - **Layer 1: Core Decisions & Architecture** (Core takeaways, central design dilemmas, and definitive outcomes).
   - **Layer 2: Practical Context & Failure Modes** (Root problems, trade-offs, and failure traps).
   - **Layer 3: Core Architectural Patterns & Solutions** (Primary mechanisms solving the problem, e.g. Minimal Frame, Shadow-Twin, Explicit Code).
   - **Layer 4: Hardware Realities & Mechanical Sympathy** (Hardware constraints, CPU cache behavior, branch predictability, physical bus cycles).
   - **Layer 5: Tactical Execution & Developer Workflows** (Granular commit sequences, test harnesses, and operational checklists).
   - **Layer 6: Summary & Related Documentation** (Summary takeaways, parent hubs, and curated reference links).

3. **Eliminating the "Bottom-Heavy Accumulation Trap"**:
   - When integrating user feedback, corrections, or newly discovered hardware quirks, **never default to simply appending them to the bottom of the document**.
   - Analyze where the new insight belongs hierarchically:
     - If it represents a **Core Architectural Decision** $\rightarrow$ weave it into the opening 20–50 lines.
     - If it represents a **First-Step Pattern** $\rightarrow$ place it at the beginning of the technical methodology.
     - If it represents an **Operational Detail** (e.g. commit rules or CLI flags) $\rightarrow$ place it in downstream execution sections.
   - Never append critical concepts as an afterthought.

4. **100% Content & Thought Preservation Standard**:
   - Re-hierarchization must be purely structural. **Never discard or dilute existing substantive thoughts, technical nuances, code blocks, ASCII diagrams, or mathematical formulas**.
   - Rearrange, re-order, and polish the narrative flow while preserving 100% of the conceptual substance.

---

## 4. Practical Note Auditing & Restructuring Workflow

When reviewing or refactoring an existing note for information hierarchy:

1. **Step 1: Outline Extraction**: Read the file and extract all headings (`#`, `##`, `###`). Map the conceptual progression.
2. **Step 2: Identify Buried Treasures**: Scan the bottom 30% of the document. Identify critical failure modes, prerequisite workflows, or key insights languishing in appendices or trailing sections.
3. **Step 3: Restructure Top-Down**: Re-order sections so the reader encounters the highest-value concepts first:
   $\text{Core Decisions} \rightarrow \text{Architecture} \rightarrow \text{Mechanisms} \rightarrow \text{Hardware Realities} \rightarrow \text{Workflows} \rightarrow \text{References}$.
4. **Step 4: Verify Substantive Completeness**: Perform a diff to confirm zero insights, formulas, code snippets, or links were lost.
5. **Step 5: Atomic Git Commit**: Commit the restructuring with a descriptive, intent-driven message:
   `refactor(structure): re-hierarchize [Note Title] for top-down clarity`.
