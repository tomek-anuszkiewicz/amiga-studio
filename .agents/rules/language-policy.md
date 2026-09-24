---
trigger: model_decision
description: Reply in the user's language while keeping repository artifacts and commit messages in English.
---

# Language Policy Rule: Match the User's Language in Conversation

## 1. Core Mandate
- **User Input Flexibility**: The user may converse, submit prompts, or provide voice recordings in Polish or English.
- **Conversation Language**:
  - **Conversational Responses**: Answer and explain in the language used by the user in the current request. Continue in that language throughout the response unless the user asks to switch.
- **English Repository Content**:
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
- After the transcription header, continue the conversation in the language used by the user in that request. Repository artifacts and code remain in English.

## 3. Automated Milestone Enforcement (`check_polish.py`)
- **Minor Roadmap Point Gate**: Automated batch scanning (`python tools/harness/check_polish.py --git`) is executed during **Minor Roadmap Point Milestone Gates** (`pre_flight.py --milestone`). Routine micro-commits do not run this scanner to prevent commit friction.
- **Diacritics-Independent Detection**: Powered by `lingua-language-detector` and `tools/harness/check_polish.py`. Evaluates text using statistical n-gram models and lexical dictionaries, identifying Polish words even when written without diacritics ("ogonki").
- **Verification Commands**:
  - Run CLI check on any target file: `python tools/harness/check_polish.py [path]`.
  - Run check across staged git files: `python tools/harness/check_polish.py --git`.

