---
name: textbook-pdf-markdown
description: Support the current temporary textbook effort and its PDF-to-Markdown preparation, chapter generation, recipes, prompts, and documentation in this repository's bootstrap tools.
---

# Temporary Textbook and PDF-to-Markdown Focus

Treat the textbook and the tooling needed to produce it as the current development priority. This is a temporary authoring and converter-development focus, not an instruction to launch conversion automatically.

## Work from the current book task

- Identify the requested deliverable: prepared PDF data, chapter Markdown, a recipe, a prompt, proofreading, a description, or supporting documentation. Read the relevant existing material before changing it.
- Read the [conversion development contract](../../../tools/bootstrap/reference-conversion-contract.md#development-workflow) when changing or running conversion. Read [PDF preparation](../../../tools/bootstrap/pdf-to-markdown/README.md) or [book authoring](../../../tools/bootstrap/book-to-markdown/README.md) as applicable. Verify current implementation before describing a planned capability as available.
- Keep preparation and book generation responsibilities consistent with their documented handoff. Reuse existing transport, cache, and source metadata when the task needs them.
- Preserve the original source and distinguish source-faithful conversion from explicitly requested editorial additions. Keep claims about results limited to the actual artifacts and runs inspected.
- Implement requested code, configuration, prompt, and documentation changes. Run a conversion only for the source fragment and stages requested by the user; report execution results and output locations. The user evaluates conversion quality. Follow the existing [converter testing scope](../../rules/unit-testing-policy.md#bootstrap-converter-scope).

Follow explicit user changes of scope. When this temporary priority ends, update the [bootstrap guidance](../../../tools/bootstrap/AGENTS.md) and retire or revise this skill together.
