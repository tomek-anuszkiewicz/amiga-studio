---
name: compact-diary
description: >-
  Use this skill when completing major roadmap milestones in ROADMAP.md to compact and synthesize older chronological entries in DIARY.md into concise, high-level architectural digests without losing evolutionary rationale, key decisions, or verified invariants.
---

# Recipe: Compacting DIARY.md Upon Major Milestone Completion

This skill defines the standardized procedure for auditing, synthesizing, and compacting historical engineering entries in [DIARY.md](../../../DIARY.md) (Section 10) after reaching major roadmap milestones.

---

## 1. When to Trigger This Skill

- **Mandatory Trigger:** Completion of a major milestone in [ROADMAP.md](../../../ROADMAP.md) (e.g. Step 1: Standalone CPU Program Execution, Step 2: CPU Footprint Audit, Phase 1 Custom Chipset milestones).
- **Goal:** Prevent token bloat and maintain scannability while permanently safeguarding the architectural rationale, problem-solving insights, and evolutionary context of the emulator.

---

## 2. Core Guiding Philosophy: High-Signal Wisdom vs. Low-Level Noise

Compacting `DIARY.md` is an intelligent, selective synthesis — **not an arbitrary truncation or deletion**:

### A. Omit Low-Level Code Details ("Nie powinniśmy mieć detali")
- **Eliminate Micro-Diffs:** Do not retain blow-by-blow accounts of individual line changes, temporary variables, syntax refactorings, or mechanical formatting edits.
- The precise code history is already preserved in Git commits. `DIARY.md` must not duplicate the Git diff.

### B. Capture Architectural Dilemmas & Resolutions
- Focus on **structural architectural challenges**:
  - What design roadblocks, cycle contention race conditions, or hardware specification ambiguities were encountered?
  - How were they resolved (e.g. 2-clock micro-step slicing, dual staging registers, open-bus floating pull-up)?
  - Why was a specific architectural direction selected over alternative approaches?

### C. Document Agent-Human Collaboration Dynamics
- Record the **meta-engineering process** of working with the AI agent:
  - What subtle bugs or edge cases did the agent struggle with (e.g. address error stack alignment, prefetch queue priming)?
  - What institutional safeguards, test harnesses, or automated architectural rules were devised to permanently prevent those issues from recurring?

### D. Preserve Non-Obvious Lessons for the Future ("Ważne, nieoczywiste informacje")
- Preserve insights that cannot be easily reconstructed by simply reading the source code:
  - Undocumented hardware quirks discovered through hardware manuals or reference emulators.
  - Performance bottlenecks where intuition failed (e.g. host branch predictor dynamics vs. microcode dispatch layout).
  - Hard-won domain knowledge to guide future development phases.

---

## 3. Compaction Strategy & Structure

1. **Active / Recent Milestone Protection:**
   - Keep all entries belonging to the currently active or most recently completed milestone in **full granular chronological detail** (exact files touched, what was changed, technical rationale, and test suites).
2. **Older Milestones Consolidation (Milestone Digests):**
   - Synthesize older, settled milestones into a cohesive **Milestone Digest**:
     - **Milestone Scope & Date Range:** e.g. `[Phase 1 Baseline CPU: Batches 1.1–1.11] (2026-08-15 to 2026-09-05)`.
     - **Architectural Breakthroughs:** High-level summary of core mechanics established (e.g. 2-clock micro-step slicing, Cartesian DMA contention engine, static dispatch table).
     - **Key Decisions & Non-Obvious Trade-Offs:** Consolidate decisions explaining *why* the code is written the way it is.
     - **Agent Collaboration & Root-Cause Lessons:** Major bugs caught and institutional prevention mechanisms established.
     - **Verified Invariants:** Record final verification gates (e.g. 100% SingleStepTests pass, 0 broken links in Obsidian).

---

## 4. Step-by-Step Execution Workflow

1. **Analyze Current Diary Structure:**
   - Read [DIARY.md](../../../DIARY.md) Section 10 to identify the boundary between settled past milestones and active/recent work.
2. **Draft the Milestone Digest:**
   - Distill the overarching architectural and collaboration lessons, applying the guiding philosophy above.
3. **Replace Older Entries with the Digest:**
   - Replace the verbose multi-entry block of the completed milestone with the structured Milestone Digest.
4. **Audit and Verify:**
   - Verify that no architectural rationale or hardware bug fix explanation was lost.
   - Run `cargo fmt --all -- --check` and `cargo test -p test_runner --test test_architecture_rules`.
5. **Log Compaction Action:**
   - Append a brief chronological note in `DIARY.md` recording that historical milestone entries were compacted per this skill.
