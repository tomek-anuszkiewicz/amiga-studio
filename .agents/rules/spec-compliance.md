# Specification Compliance and Golden References

Design specifications and [AGENTS.md](../../AGENTS.md) govern implementation. Never silently contradict or bypass documented behavior, timing, memory semantics, or architectural boundaries.

- When a requirement, test expectation, or reference emulator conflicts with the specification, stop the conflicting change and present the current rule, contrary evidence, and concrete options.
- Obtain an explicit recorded user decision before deviating or amending the specification. Otherwise implement the documented behavior.

## Golden Test Vector & Hash Invariance (Anti-Tamper Rule)

- Golden hashes, checksums, cycle totals, and reference vectors are protected baselines. Never change them merely to make a failing test pass.
- A justified golden correction requires root-cause investigation, precise hardware evidence, presentation of the proposed change, and explicit user approval before updating constants.
- For hardware/regression review, consult the [Platform Quirks and Invariants Catalog](../../Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md) and relevant [vAmigaTS Verification Scorecard](../../Obsidian/Amiga/Design/vAmigaTS%20Verification%20Scorecard.md).
