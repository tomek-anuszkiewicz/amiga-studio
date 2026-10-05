---
name: compact-diary
description: Compact and synthesize older chronological entries in DIARY.md into concise high-level architectural digests.
---

# Recipe: Engineering Diary Maintenance & Milestone Compaction (`DIARY.md`)

This skill defines the standardized procedure for logging milestone completions and compacting historical engineering entries in [DIARY.md](../../../DIARY.md) (Section 10).

---

## 1. Milestone Engineering Diary Logging Protocol

Whenever an agent finishes a minor roadmap point (e.g. Step 1.1, 1.2, 2.1) or major architectural milestone:
- **Mandatory Milestone Trigger:** Append a detailed narrative entry to [`DIARY.md`](../../../DIARY.md) under Section 10 (Living Chronological Engineering Log). Routine micro-commits must not add diary entries.
- **Deterministic Logging Tool (`tools/harness/log_diary.py`):**
  Always use the deterministic CLI tool to eliminate prompt bloat:
  ```powershell
  python tools/harness/log_diary.py \
    --title "<Title>" \
    --subsystems "<crates/..., rules/...>" \
    --changes "<bullet 1>; <bullet 2>" \
    --rationale "<rationale>" \
    --results "<test verification>"
  ```
- **Standard Entry Structure:**
  Every log entry under Section 10 must systematically document:
  1. **Affected Subsystems:** Crates, modules, rules, or design notes modified.
  2. **What Was Changed (The Concrete Reality):** Specific code modifications, data structures, algorithms, or mechanics introduced or refactored.
  3. **Why It Was Done & Architectural Rationale:** The problem statement, edge cases discovered, user directives, and trade-offs behind the solution.
  4. **Verification & Test Results:** Specific test suites executed and verified (e.g. `pre_flight.py --milestone`, SingleStepTests, formatting checks).

---

## 2. When to Trigger Historical Diary Compaction

- **Triggers:**
  - Completion of a minor roadmap point (e.g. Step 1.1, 1.2, 1.3, 1.4) or major milestone in [ROADMAP.md](../../../ROADMAP.md).
  - Whenever Section 10 grows unwieldy (e.g. exceeds operational threshold of ~150 KB or > 1,500 lines).
- **Goal:** Prevent token bloat and maintain scannability while permanently safeguarding the architectural rationale, problem-solving insights, and evolutionary context of the emulator.


---

## 3. Core Guiding Philosophy: High-Signal Wisdom vs. Low-Level Noise

Compacting `DIARY.md` is an intelligent, selective synthesis — **not an arbitrary truncation, deletion, or flattening**:

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

### E. Anti-Steamroller Invariant (Zero Milestone Merging)
- **Prohibition of Over-Compression:** Never collapse multiple distinct major engineering milestones into a single generic bucket.
- Compaction synthesizes commit-level noise and micro-diffs *within* each milestone sprint, but **permanently preserves every substantive milestone as its own dedicated Milestone Digest**.
- Major domains (e.g. Custom Chipset silicon calibrations, whole-machine verification suites, PDF document ingestion pipelines, worktree storage isolation, compiler hardening, and RAG architecture) must never be blended together or discarded.

---

## 4. Compaction Strategy & Structure

1. **Active / Recent Milestone Protection:**
   - Keep all entries belonging to the currently active or most recently completed roadmap point in **full granular chronological detail** (exact files touched, what was changed, technical rationale, and test suites).
2. **Older Milestones Consolidation (Milestone Digests):**
   - Synthesize older, settled milestones into cohesive, high-signal **Milestone Digests** adhering strictly to the 5 canonical fields:
     ```markdown
     ### [YYYY-MM-DD HH:MM CEST] — Milestone Digest: <Theme>
     - **Timestamp & Context**: Date range and evolutionary context of the milestone.
     - **Affected Subsystems**: Crates, modules, tools, and design specifications.
     - **What Was Changed (The Concrete Reality)**: Specific algorithms, data structures, and mechanics established.
     - **Architectural Rationale & Trade-Offs**: Why this solution was chosen, dilemmas resolved, and alternatives rejected.
     - **Verification & Invariants**: Specific test suites, golden hashes, and proof of correctness.
     ```

---

## 5. Step-by-Step Execution Workflow

1. **Analyze Current Diary Structure:**
   - Record the current line count and byte size.
   - Read [DIARY.md](../../../DIARY.md) Section 10 to identify the boundary between settled past milestones and active/recent work.
   - Catalog all distinct major milestone themes present in the uncompacted block.
2. **Draft Individual Milestone Digests:**
   - Author a separate, dedicated digest for each distinct architectural milestone, applying the guiding philosophy and anti-steamroller invariant above.
3. **Replace Older Entries with the Digests:**
   - Replace the verbose multi-entry block with the chronologically ordered Milestone Digests.
4. **Audit and Verify:**
   - Verify that no architectural rationale, hardware quirk resolution, or major subsystem milestone was lost.
   - Run `python tools/harness/pre_flight.py --quick` and `cargo test -p test_runner --test test_architecture_rules`.
5. **Log Compaction Action:**
   - Append a brief chronological note in `DIARY.md` recording that historical milestone entries were compacted per this skill.



## 6. Output Contract
Conclude with the standardized summary report:
```markdown
### 📔 Diary Compaction Summary
- **Target Milestones Compacted:** `<milestone_name_and_dates>`
- **Active Entries Retained:** <count> granular entries
- **Original Size:** <lines_before> lines (<kb_before> KB)
- **Compacted Size:** <lines_after> lines (<kb_after> KB)
- **Size Reduction:** <percent>% by bytes; label any token estimate explicitly
- **Key Architectural Digests Preserved:** <bullet list of digest sections>
- **Verification:** `pre_flight.py` (PASS)
```
