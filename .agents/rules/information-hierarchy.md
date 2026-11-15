# Document and Instruction Ownership

- [AGENTS.md](../../AGENTS.md) contains shared obligations, machine invariants, completion requirements, and routing to applicable rules.
- Rules define obligations, scope, and exceptions. Skills own execution procedures and link supporting references, scripts, and templates. Scripts and tests own mechanical checks.
- `docs/` explains the project and provides starting points for users and developers. Keep useful launch or verification examples there; link detailed operational procedures and command options to the owning tool/test README or agent skill/reference. Change history belongs in the diary.
- Keep each instruction or procedure in one authoritative location and link to it elsewhere. Keep `AGENTS.md` and rules concise without arbitrary size limits; preserve essential obligations and required procedure links when moving detail.
- Discover skills by name/description; link mandatory procedures without a duplicate catalog.
- Changes to `AGENTS.md`, rules, skills, role instructions or verification requirements
  must assess their effect on the reviewer criteria in the
  [four review lanes](../skills/code-review/SKILL.md). Update affected checklists and
  routing in the same change. Keep reviewer checklists concise and independently
  usable; upstream rules remain authoritative. Unaffected lanes need no cosmetic edit.
