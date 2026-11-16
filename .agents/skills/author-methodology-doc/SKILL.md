---
name: author-methodology-doc
description: Author, audit, or restructure narrative articles, methodology documents, essays, and retrospective devlogs (e.g. docs/how_this_emulator_was_written.md) using the top-down Inverted Pyramid hierarchy.
---

# Recipe: Authoring & Restructuring Narrative Documents (Inverted Pyramid Model)

This skill provides a systematic procedure for authoring, evaluating, and restructuring narrative articles, methodology papers, retrospective devlogs, and architectural case studies (such as [`docs/how_this_emulator_was_written.md`](../../../docs/how_this_emulator_was_written.md)) according to the **Inverted Pyramid model** (leading with core conclusions and architecture).

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

Readers start at the top and want the core takeaway immediately. Content must progress from central architectural decisions down to granular execution details:

The layers below guide narrative order; adapt their section boundaries to the
article rather than repeating an outline mechanically.

## 4. Layer-by-Layer Breakdown

| Layer | Contribution |
| --- | --- |
| 1. Core conclusion | Lead with the decisive result, architectural choice or mental model, normally within the opening 20-50 lines. Do not bury the thesis in an appendix. |
| 2. Strategic and human constraints | Explain the problem, stakes, bottlenecks and failed approaches that made the decision necessary. |
| 3. Architectural solution | Explain the mechanisms and patterns that address those constraints, using a diagram where it helps. |
| 4. Execution realities | Ground the solution in hardware timing, host execution, memory layout or compiler constraints supported by evidence. |
| 5. Tactical workflow | Give the relevant reproducible procedures, verification boundaries and operational guardrails, or link their authoritative owners. |
| 6. Synthesis and references | Connect conclusions and useful source relationships without repeating the article. |

---

## 5. Core Operating Principles & Anti-Trap Safeguards

### A. Eliminating the "Bottom-Heavy Accumulation Trap"
- During pair programming or document revision, new insights, corrections, or user revelations frequently emerge.
- **Mandatory Anti-Trap Rule:** Never default to simply appending new insights to the bottom of the document.
- Analyze where the new insight belongs hierarchically:
  - If it represents a **Core Thesis / Paradigm Shift** $\rightarrow$ Weave it into Layer 1 (opening 20–50 lines).
  - If it represents a **First-Step Methodology** (e.g. proof-of-concept verification) $\rightarrow$ Place it in Layer 3 (Phase 1 of Core Patterns).
  - If it represents an **Operational Guardrail or Command** $\rightarrow$ Place it in Layer 5 (Tactical Execution).

### B. Preserve Substantive Content
- Preserve mechanisms, evidence, caveats, formulas, timing tables, and useful diagrams when restructuring.
- Remove repeated wording when concision is requested. Move detailed examples to linked owning references when the task calls for that separation; verify they remain reachable.
- Do not discard unique technical reasoning merely to meet an arbitrary size target.

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
   - Verify that unique technical reasoning, evidence, caveats, formulas, and useful links remain present or reachable in explicitly moved references. Confirm that removed material is duplicated or within the authorized edit scope.

5. **Step 5: Return to the Authorized Workflow**
   - Follow [commit policy](../../rules/git-commits.md) for preparation and commit
     timing. A documentation edit or audit alone leaves changes unstaged and
     does not authorize a commit.
