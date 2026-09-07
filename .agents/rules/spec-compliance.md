# Specification Compliance & Divergence Escalation Rule (Zero Silent Spec Violations)

Design specifications under `Obsidian/Amiga/Design/` and guidelines in `AGENTS.md` are the authoritative ground truth for this project.

## 1. Zero Unilateral Divergence
- Agents must **never silently implement code that contradicts or bypasses existing design specifications or rules**.
- Examples of prohibited silent divergences:
  - Returning `$00` instead of `$FF` on unmapped memory reads.
  - Silently skipping or altering bus contention and wait states.
  - Violating endianness rules without explicit permission.
  - Silently changing data structures or architectural boundaries.

## 2. Mandatory Conflict Detection & Escalation
Whenever an implementation requirement, external test harness expectation (e.g. SingleStepTests flat memory model assumptions), or reference emulator quirk conflicts with the documented specification:
- **You MUST STOP immediately and present the conflict to the USER before modifying code.**
- Clearly outline:
  1. **Current Specification:** What the existing design document / hardware rule states.
  2. **Conflicting Expectation:** What the external test suite or scenario expects.
  3. **Proposed Options:** Concrete architectural solutions (e.g., configurable parameters, test adapters, or formal specification amendments).

## 3. Explicit User Decision Required
No code may deviate from existing documentation without an explicit, recorded decision by the user. Either:
- The design documentation is officially modified with user approval, OR
- The code strictly adheres to the specification.
