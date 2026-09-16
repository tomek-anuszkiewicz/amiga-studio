---
trigger: always_on
description: Mandatory structural root-cause resolution; strict prohibition of local symptom patches (pixel nudging, ad-hoc regexes, special-case branches).
---

# Structural Root-Cause Resolution Rule (Zero Local Symptom Patches)

This rule governs all defect resolution, bug fixes, test corrections, and user-requested modifications across the entire repository (emulation core, test suites, and tooling pipelines).

---

## 1. The Core Mandate: Structural Solutions Over Symptom Masking

Whenever addressing any defect, test divergence (e.g. against vAmiga or golden vectors), or user-reported anomaly:
- **Never apply surface-level, local tweaks to silence the immediate symptom.**
- Always step back, analyze the data lifecycle or physical hardware timing upstream, and implement a **structural solution** that resolves the root cause for the entire class of problems.

---

## 2. Strictly Prohibited Anti-Patterns

### A. Coordinate & Timing Nudging ("One Pixel Left, One Pixel Right")
- **Forbidden:** Blindly adjusting an offset, beam coordinate, loop counter, or cycle delay by $\pm 1$ or $\pm 2$ just to make a visual test or comparison pass.
- **Why:** In cycle-exact emulation, an off-by-one pixel or clock cycle is almost never an isolated constant error. It is a symptom of phase misalignment (e.g. CCK1 vs CCK2), pipeline latency, DMA slot contention, or trigger timing. Nudging the coordinate masks the bug and breaks adjacent video modes or cycle fidelity.
- **Required:** Trace the physical silicon clock phase or signal origin to understand *why* the event fired early or late.

### B. Ad-Hoc Regexes & String Replacement Hacks
- **Forbidden:** Adding hardcoded word replacements (`re.sub(r"HARDW\s+ARE", "HARDWARE")`), string splits, or ad-hoc character stripping to fix an OCR typo or formatting flaw.
- **Why:** Scanned text contains hundreds of unpredictable split words and OCR artifacts. Hardcoded regexes are brittle, unmaintainable, and fail to scale across hundreds of pages.
- **Required:** Use general, robust mechanisms (e.g. LLM proofreading streams, general language tokenizers) and fix pipeline ordering so clean data is produced upstream.

### C. Isolated Special-Casing
- **Forbidden:** Adding special-case `if` branches for a single instruction opcode, specific register address, or single file name to bypass standard handling.
- **Required:** Modify the underlying state machine, table generator, or data model to handle the condition generically.

---

## 3. The "Assume Systematic Scope" Invariant

When an anomaly is observed or reported (e.g. *"chapter title has a space in the middle"*, *"sprite is shifted by one pixel"*):
1. **Assume It Is NOT an Isolated One-Off:**
   - If one chapter title has broken spacing, other titles and headings likely suffer from the same OCR segmentation flaw.
   - If one raster line has delayed DMA, the arbitration or beam counter logic affects all raster lines.
2. **Never Patch Just the Specific Reported Instance:**
   - Do not write code that specifically checks for the exact word, register, or page that was pointed out.
   - Generalize the observation: what systematic defect in the upstream pipeline allowed this state to occur?

---

## 4. Upstream Data Lifecycle & Pipeline Order Analysis

Before writing any fix, trace the data lifecycle backwards:
```mermaid
flowchart LR
    A["Upstream Source / Input"] --> B["Intermediate Processing / Stages"]
    B --> C["Serialization / State Update"]
    C --> D["Point of Observation (Symptom)"]
```
- **Ask:** *Why did dirty, unaligned, or erroneous state reach the point of observation?*
- **Fix Upstream:** Fix the issue at the earliest possible stage in the lifecycle, or reorder the stages so that preconditions are met before dependent operations run (e.g. proofreading streams and manifests *before* emitting filenames and markdown, rather than patching files after emission).

---

## 5. Review Checklist for Corrections

Before submitting any bug fix or adjustment:
- [ ] Is this change free of arbitrary coordinate/cycle nudges ($\pm 1$ offsets without silicon timing proof)?
- [ ] Is this change free of hardcoded ad-hoc string/word substitutions or regex patches?
- [ ] Does this fix resolve the entire class of similar errors, rather than just the single reported instance?
- [ ] Has the root cause been addressed upstream in the data lifecycle or hardware state machine?
- [ ] Would this solution work cleanly across 400 pages or 100,000 frames without manual intervention?
