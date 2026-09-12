---
trigger: always_on
description: Strict discipline against synthetic linguistic attractors, high-register jargon monoculture, and misplaced hardware buzzwords.
---

# Vocabulary & Linguistic Attractor Discipline Rule

This rule establishes strict safeguards against synthetic linguistic attractors (*gravity wells*), academic jargon monoculture, and misplaced hardware buzzwords across all agent communications, design specifications, documentation, and source code.

---

## 1. The Phenomenon: LLM Bias & Autoregressive RAG Echo

Every major LLM agentic architecture (Gemini, Claude, GPT) exhibits systemic stylistic biases:
- **Intellectualizing & High-Register Jargon ("Mądrkowanie"):** Models naturally drift toward complex, pretentious academic terms when plain, precise software engineering language is far superior.
- **Autoregressive RAG Echo (The Self-Amplifying Gravity Well):** When a high-concept term (e.g. *epistemic*, *zero-friction trap*, *testing oracle*) is introduced in an initial document or prompt, subsequent retrieval passes (RAG, AST search, or file reads) ingest that term as canonical repository vocabulary. The model then echoes and amplifies it across new summaries, rules, and notes, creating an overwhelming monoculture.

To ensure the repository remains grounded, concise, and clean, all agent interactions must actively resist these attractors.

---

## 2. Quarantined Attractor Vocabulary & Grounded Replacements

The following synthetic phrases and high-register terms are **strictly quarantined** across the codebase:

| Quarantined Term / Attractor | Rationale for Quarantine | Approved Grounded Replacements |
| :--- | :--- | :--- |
| **`epistemic`** *(e.g. epistemic debt, epistemic anchor)* | Pretentious academic jargon replacing simple cognitive and software concepts. | *knowledge drift*, *context*, *cognitive load*, *architectural understanding*, *authoritative reference* |
| **`teleological`** | Unnecessary philosophical jargon for design intent. | *purpose-driven*, *goal-oriented*, *design intent*, *subsystem scope* |
| **`zero-friction trap`** / **`zero cognitive friction`** | Overused rhetorical catchphrase creating a self-amplifying attractor. | *unverified code generation*, *false sense of velocity*, *unanchored edits*, *intuitive*, *straightforward* |
| **`testing oracle`** / **`the oracle`** | Theatrical jargon for test verification. | *test verification reference*, *ground truth vector*, *hardware test capture*, *reference harness* |
| **`The Invariance Invariant`** / **Inflated Invariant Monoculture** | Using "invariant" as a universal answer for routine conventions or rules. | *machine system invariants*, *state assertions*, *architectural consistency*, *mandatory constraints* |
| **`L1i density`** / **`L1 cache footprint` in docs** | Misplaced physical hardware jargon leaked into pure markdown files. | *compact*, *lean*, *cohesive*, *concise* |
| **`mechanical sympathy` in headings / slogans** | Overused catchphrase slapped into titles, headings, and role descriptions (*"Guardian of Mechanical Sympathy"*, *"Mechanical Sympathy Invariant"*). Allowed only in grounded low-level substrate execution contexts. | *host hardware efficiency*, *physical execution reality*, *low-level systems comprehension*, *hardware-aligned execution*, *host pipeline optimization* |

---

## 3. Domain Containment & Hardware Term Discipline

- **Physical Substrate vs Pure Documentation:** Host CPU cache physics (`L1`, `L2`, `L3`, `cache line`, `branch predictor`) are real technical concerns for low-level memory banks, static dispatch tables, and execution profiling.
- **Prohibition of Leakage:** Never leak microarchitectural cache jargon into pure markdown specifications, skill instructions, or operating rules. Documentation and rules do not run on silicon caches—keep documentation language focused on clarity, structure, and readability.
- **Prohibition of Sloganization in Headings:** Never use catchphrases like "Mechanical Sympathy" in Markdown section headings (`#`, `##`, `###`, `####`). Use precise systems engineering titles (`Host Hardware Efficiency`, `Physical Execution Reality`, `Host Pipeline Optimization`).

---

## 4. Mitigating Autoregressive RAG & In-Context Echo

When researching via `rag_search` or reading existing project notes:
1. **Extract Concepts, Not Stylistic Mannerisms:** Absorb the technical facts, cycle timings, and circuit specifications without mimicking rhetorical flourishes or buzzwords found in older text.
2. **Standard Software Engineering Vocabulary:** Always express solutions in universally recognized software engineering idioms (`separation of concerns`, `linear execution`, `state isolation`, `test coverage`, `bounded time-slicing`).

---

## 5. Automated Verification & CI Enforcement

This rule is enforced by two automated layers:
1. **Autonomous Python Linter:** Packaged in [`attractor-discipline`](../skills/attractor-discipline/SKILL.md) skill at `.agents/skills/attractor-discipline/scripts/lint_attractors.py` (accessible via root wrapper `python scripts/lint_attractors.py`). Scans all `.md` and `.rs` files and supports `--fix`.
2. **Automated Architecture Test:** `test_zero_synthetic_attractors` in `crates/test_runner/tests/test_architecture_rules.rs` (runs on every `cargo test` and pre-commit check).