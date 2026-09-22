---
name: describe-diagram-assets
description: Bootstrap visual-asset handling for source-faithful HTML or PDF Markdown conversions, including RAG sidecars and collapsible text fallbacks.
---

# Describe Diagram Assets

Use this skill once at the beginning of a source-faithful HTML- or PDF-to-Markdown conversion that contains visual assets. It decides which visuals matter, makes retained information searchable, and records how the active Gemini workflow hands work to an agent. Do not use it for routine emulator-code diagrams or for repeated per-page conversion work.

Read [`asset-descriptions.md`](../../rules/asset-descriptions.md) first. That rule is the authoritative representation and verification contract.

## Bootstrap Outcome

Produce a concise bootstrap record in the conversion workspace or its approved plan. It must identify:

- the current PDF and HTML Gemini stages that process tables or graphics, including their automatic outputs and their manual task or review handoffs;
- whether each visual class adds information beyond surrounding extracted text;
- the chosen output form for HTML tables, retained images, diagrams, and ASCII art; and
- a representative verification that sidecars and collapsed descriptions were written to the rendered Markdown, not merely returned by a model call.

Inspect the active workflow implementation rather than assuming that a configured prompt, task directory, or documented stage persists its result. Treat missing, unusable, or ambiguous automation as a manual handoff and state the exact artifact the agent must create or review.

## Description Quality

Write source-grounded descriptions that make the visual independently retrievable: purpose, named elements, significant labels or values, and the relevant spatial, logical, or temporal relationships. A caption or label list is insufficient. Do not infer facts that cannot be established from the visual and its source context.

For every retained image, create `<asset filename>.txt` beside the asset and place the same description in a closed Markdown `<details>` block after the embed. For retained ASCII art, preserve the art in a `text` fence and follow it with the equivalent collapsed description. For retained HTML tables, keep the source-faithful HTML and provide an exact GFM table fallback in a closed `<details>` block.

## Completion Check

This skill is complete only after a representative output proves the intended files and Markdown blocks exist and agree. Run it again only if the source assets or conversion contract changes.
