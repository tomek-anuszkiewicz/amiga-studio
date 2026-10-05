# Information Hierarchy and Instruction Budgets

- [AGENTS.md](../../AGENTS.md) contains shared obligations, machine invariants, completion requirements, and conditional rule routing. Keep full procedures and examples in skills or specifications.
- Repository budgets are 14,000 bytes for `AGENTS.md` and 23,000 bytes per rule. These are local policy limits, distinct from the client's configurable instruction-discovery budget; linked rules are read explicitly.
- Skill discovery uses each skill's name and description. Do not duplicate a skill catalog in `AGENTS.md`; retain explicit references when their use is mandatory.
- Rules define obligations, scope, and exceptions. Skills define execution steps and link conditional references, scripts, and templates. Mechanical checks belong in scripts and tests.
- Lead longer explanations with the decision and essential invariants, then explain context, mechanisms, evidence, and execution detail. Adapt structure to the document; short rules do not require six sections.
- Integrate new findings into the relevant argument. Preserve substantive mechanisms, caveats, evidence, and useful links while removing repeated wording.

The restructuring procedure and six-layer narrative model are in [author-methodology-doc](../skills/author-methodology-doc/SKILL.md). Apply [practitioner writing style](practitioner-voice-and-tone.md).
