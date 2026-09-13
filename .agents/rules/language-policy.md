---
trigger: model_decision
description: Strict English output policy for all agent responses, plans, artifacts, and code.
---

# Language Policy Rule: Strict English for Responses, Plans, Artifacts & Code

## 1. Core Mandate
- **User Input Flexibility**: The user may converse, submit prompts, or provide voice recordings in Polish or English.
- **Mandatory English Output**:
  - **Conversational Responses**: The agent must **always generate its answers and explanations in English**, regardless of whether the user speaks or writes in Polish.
  - **Planning Documents & Artifacts**: All task plans (`implementation_plan.md`), walkthroughs (`walkthrough.md`), architectural reports, and scratch design documents must be written strictly in English.
  - **Source Code & Documentation**: All Rust/C/assembly code, identifiers, types, variables, functions, comments, docstrings, commit messages, and PR summaries must strictly be in English.

### 1.1 Concept Translation Mandate (Zero Raw Prompt Echoing)
- **Mandatory Translation Before Coding**: When the user describes an algorithm, feature, or architectural metaphor in Polish (e.g. *"przeczekać burzę"*, *"pętla opóźniająca"*, *"szyna danych"*), the agent **must always translate the concept into idiomatic English** (e.g. *"Wait Out the Storm"*, *"delay loop"*, *"data bus"*) before writing it into code.
- **Literal Quoting Prohibited**: Quoting Polish prompt phrases or metaphors in code or comments—even parenthetically, in quotation marks, or as explanatory aliases—is strictly forbidden across all repository source files (`crates/*`, `tests/*`).

## 2. Voice Recording Exception (Rule 7 Echo)
- When the user uploads an audio recording, the **Mandatory Spoken Input Echo** at the very beginning of the response must transcribe what the user actually said in their spoken language (e.g. Polish):
  ```markdown
  > 🎙️ **Transcribed User Voice Input:** "[Exact spoken user transcription]"
  ```
- Everything following the transcription header (model advisory, explanations, execution steps, tool descriptions, and user prompts) must immediately and strictly continue in English.

## 3. Automated Enforcement & Pre-Tool Hook (`language-policy-guard`)
- **Lifecycle Pre-Tool Hook**: Active in `.agents/hooks.json` under `language-policy-guard`. It intercepts file modification tools (`write_to_file`, `replace_file_content`, `multi_replace_file_content`) and blocks execution (`decision: deny`) if Polish vocabulary or phrases are introduced in content.
- **Diacritics-Independent Detection**: Powered by `lingua-language-detector` and `tools/check_polish.py`. Evaluates text using statistical n-gram models and lexical dictionaries, identifying Polish words even when written without diacritics ("ogonki").
- **Verification Commands**:
  - Run CLI check on any target file: `python tools/check_polish.py [path]` (defaults to `language-policy.md` when omitted).
  - Run check across staged git files: `python tools/check_polish.py --git` (enforced via `.git/hooks/pre-commit`).

