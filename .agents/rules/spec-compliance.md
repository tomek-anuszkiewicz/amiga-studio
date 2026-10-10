# Specification Compliance and Reference Artifacts

Design specifications and [AGENTS.md](../../AGENTS.md) govern implementation. Never silently contradict or bypass documented behavior, timing, memory semantics, or architectural boundaries.

- When a requirement, test expectation, or reference emulator conflicts with the specification, stop the conflicting change and present the current rule, contrary evidence, and concrete options.
- Obtain an explicit recorded user decision before deviating or amending the specification. Otherwise implement the documented behavior.

## Reference Artifact Integrity

- Reference traces, CSV baselines, cycle totals, and silicon vectors are protected references. Never change them merely to make a failing test pass.
- A justified reference correction requires root-cause investigation, presentation of the evidence and proposed change, and explicit user approval. Hardware-derived references also require precise hardware evidence.
- For hardware/regression review, consult the [Platform Quirks and Invariants Catalog](../../Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md) and relevant validated test evidence.
