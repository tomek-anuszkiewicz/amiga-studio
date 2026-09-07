# Audio Input Transcription & Spoken Prompt Confirmation Rule

## 1. Rule Mandate
Whenever the user communicates using a voice recording (an audio file attached to `<USER_REQUEST>` or prompt):
- The agent **MUST ALWAYS** begin its response by displaying a clean, textual transcription of what the user said.
- The transcription should be lightly cleaned up and reformatted for readability (fixing minor colloquial stutters or punctuation while strictly preserving the user's exact meaning and intent).

## 2. Standard Output Format
Present the transcription at the very top of the response using a blockquote or callout:

```markdown
> 🎙️ **Rozpoznana treść wiadomości:**
> _"[Przeformatowana, czytelna transkrypcja wypowiedzi użytkownika]"_

---
```

## 3. Rationale
The chat UI does not automatically render speech-to-text transcripts for uploaded audio clips. Echoing the transcript provides the user with immediate visual certainty that their intent, technical terms, and instructions were recognized accurately before reviewing the agent's work.
