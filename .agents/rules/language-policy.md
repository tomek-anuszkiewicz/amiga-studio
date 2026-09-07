# Language Policy Rule: Strict English for Responses, Plans, Artifacts & Code

## 1. Core Mandate
- **User Input Flexibility**: The user may converse, submit prompts, or provide voice recordings in Polish or English.
- **Mandatory English Output**:
  - **Conversational Responses**: The agent must **always generate its answers and explanations in English**, regardless of whether the user speaks or writes in Polish.
  - **Planning Documents & Artifacts**: All task plans (`implementation_plan.md`), walkthroughs (`walkthrough.md`), architectural reports, and scratch design documents must be written strictly in English.
  - **Source Code & Documentation**: All Rust/C/assembly code, identifiers, types, variables, functions, comments, docstrings, commit messages, and PR summaries must strictly be in English.

## 2. Voice Recording Exception (Rule 7 Echo)
- When the user uploads an audio recording, the **Mandatory Spoken Input Echo** at the very beginning of the response must transcribe what the user actually said in their spoken language (e.g. Polish):
  ```markdown
  > 🎙️ **Rozpoznana treść wiadomości / Recognized Spoken Input:** "[Exact spoken Polish transcription]"
  ```
- Everything following the transcription header (model advisory, explanations, execution steps, tool descriptions, and user prompts) must immediately and strictly continue in English.
