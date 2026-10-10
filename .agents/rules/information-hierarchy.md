# Information Hierarchy and Concise Instructions

- [AGENTS.md](../../AGENTS.md) contains shared obligations, machine invariants, completion requirements, and conditional rule routing. Keep full procedures and examples in skills or specifications.
- Keep `AGENTS.md` and rule files concise and relevant. Remove duplicated instructions and move detailed procedures and conditional examples into their owning skills or references. Preserve essential obligations and routing; do not add arbitrary byte ceilings.
- Skill discovery uses each skill's name and description. The skill files are the source of truth; no separately synchronized catalog is required. Retain explicit references when their use is mandatory.
- Rules define obligations, scope, and exceptions. Skills define execution steps and link conditional references, scripts, and templates. Mechanical checks belong in scripts and tests.
- Lead longer explanations with the decision and essential invariants, then explain context, mechanisms, evidence, and execution detail. Adapt structure to the document; short rules do not require six sections.
- Integrate new findings into the relevant argument. Preserve substantive mechanisms, caveats, evidence, and useful links while removing repeated wording.

The restructuring procedure and six-layer narrative model are in [author-methodology-doc](../skills/author-methodology-doc/SKILL.md). Apply [practitioner writing style](practitioner-voice-and-tone.md).
