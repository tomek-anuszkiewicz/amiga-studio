# Documentation maintenance

Reader-facing `docs/` explains the project and useful entry points. Operational
procedures stay with their tools/skills; update incoming links when moving them.
[Information hierarchy](information-hierarchy.md) owns instruction responsibility.

## Documentation Value

Preserve actual hardware behavior confirmed by authoritative sources, even when it
is easy to read from code: it remains an independent implementation contract.
Preserve knowledge whose discovery/explanation would otherwise require substantial
work: established intent, decisions, consequential trade-offs, external evidence and
non-obvious constraints.

For other implementation descriptions, ask whether the reader can establish the
fact quickly and directly from code. If so, omit it unless it adds independent
value. Remove straightforward code narration, consolidate duplicates at their
owner, and retain valuable parts of mixed passages. Missing entity descriptions
alone are not defects. There is no size target or deletion quota.

Approved requirements retain intended behavior independently of current code.
Before deleting potentially unique knowledge, inspect relevant decisions, history,
tests or sources. Preserve minimum intent and evidence; distinguish established
rationale from inference and leave unknown reasons unknown.

### Hardware Explanations and Evidence

The owning specification explains hardware behavior with links to manual sections,
book passages or authoritative observations. Scoped review compares affected
behavior with those sources. Source-confirmed behavior missing its explanation
requires compatible enrichment at the owning specification, even when code makes
it obvious. Report-only review identifies that work; the caller assigns
[TA-sync-docs](../skills/TA-sync-docs/SKILL.md) before readiness where required.

Code agreement alone does not establish hardware correctness. Distinguish hardware
facts from modeling choices, assumptions and unresolved evidence. Report contrary
behavior, code, specification and source passages under
[specification compliance](spec-compliance.md); never remove a requirement to match
incorrect code. Preserve source books, imported references, conversion prompts and
generated artifacts. This editorial policy applies to project-authored explanations,
not rewriting their sources or executable instructions.

## Living Design Specifications

Read affected requirements before implementation. At preparation, scoped
[TA-review-spec-compliance](../skills/TA-review-spec-compliance/SKILL.md) uses frontmatter
`tracked_paths`, including rename/delete endpoints and changed requirements whose
code is unchanged. Report unmapped changes and unresolved differences; broader
matching semantic review may satisfy overlapping scope. Compatible enrichment and
selected alignment occur through writer operations, never by an assessor silently
editing requirements. Actual milestone closure needs
[TA-review-semantic-parity](../skills/TA-review-semantic-parity/SKILL.md).

Keep accurate specifications and dates unchanged. Substantive edits update `updated`
and relevant status/tags/dependencies; review-only or mechanical metadata removal
does not justify a date-only change. Preserve approved intent, neighboring structure
and links to implementation. Keep gaps/known defects as brief truthful caveats to
the owning [roadmap](../../ROADMAP.md); remaining diagnostics/acceptance work stays
there. Reuse shared specification owners; update General Architecture dependencies
when crate relationships change. Specifications and references have no line ceiling.

## Vault Metadata and Link Integrity

Authored Design notes keep line-one frontmatter with the required title, aliases,
tags, category, subsystem, status, dates and related links. The schema/templates and
full audit procedure belong to
[TA-audit-vault-links](../skills/TA-audit-vault-links/SKILL.md); selected repairs belong
to TA-sync-docs. Keep contextual inline links plus a curated final reference section
with concise rationale, including relevant manuals, models and living Rust modules.
Preserve relative links on moves and meaningful reciprocal dependencies through
[General Architecture](../../Obsidian/Amiga/Design/General%20Architecture.md), without quotas.
Adapt sections to the subsystem.

The final document set requires the full vault audit during preparation or an
explicit audit/repair assignment. Reuse actually overlapping canonical frontmatter
evidence; a structural scan does not establish semantic link quality. Preserve
reference-note metadata while editing; raw conversion has its nested instructions.

## Reference Book Metadata

Every Markdown file in a book directory under `Obsidian/Amiga/Reference/` has
line-one frontmatter containing `title`, `book`, `chapter` and `tags`. `book` names
the source title or composite publication identifier; `chapter` names its section
or unit, including contents/front matter. Replace `manual` with `book` without
changing the value. Tags include `reference`, `book`, `chapter`, retained relevant
tags and content-supported topics; reuse lowercase hyphenated tags and include
`amiga` for Amiga content. Do not infer topics from the book title alone.

The canonical gate checks every local vault Markdown file, including ignored books.
If metadata is missing, report affected paths and proposed frontmatter for each and
obtain approval before editing those source notes. The verifier does not repair them.
Structural acceptance does not replace the topic-selection review.
