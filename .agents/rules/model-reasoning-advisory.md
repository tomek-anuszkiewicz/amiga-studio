# Dynamic Model & Reasoning Effort Advisory Rule

## 1. Rule Mandate
The agent actively monitors the active model and reasoning effort level (e.g. `Medium` vs `High` thinking) from session metadata and proactively advises the user when a switch is recommended based on the nature and complexity of the current task.

Because model selection is controlled directly by the user in the IDE UI, the agent cannot change the model automatically. Instead, it must issue a concise, clear hint/recommendation.

## 2. Advisory Criteria

### A. When to Recommend `High` (e.g. Gemini 3.8 Flash High / Pro High):
Recommend switching to `High` whenever the task involves deep architectural analysis, cycle-exact timing intricacies, or complex hardware circuit modeling:
- **CPU Micro-Architecture & Pipeline**: Designing or refactoring the M68000 micro-operation state machine, prefetch queue (`IR`/`IRC`) progression, or bus cycle phases (`CCK1`/`CCK2`).
- **Single-Step Test Failures & Edge Cases**: Diagnosing subtle timing discrepancies in `SingleStepTests`, complex address error exception frames, stack order quirks, or atypical ALU/CCR behavior (e.g. `ABCD`/`SBCD`/`NBCD`, multi-cycle division/multiplication, `MOVEM`).
- **Bus Contention & Chip Synchronization**: Designing memory bus arbitration, wait-state mechanics (`MemoryBusResult::Wait`), or cycle stealing between Agnus (Copper/Blitter), DMA channels, and CPU.
- **Root-Cause Architectural Debugging**: Investigating regressions or hardware circuit race conditions that require deep multi-step reasoning.

### B. When to Recommend `Medium` (or lower reasoning budget):
Recommend switching back to `Medium` whenever the task is repetitive, mechanical, or execution-heavy, where high reasoning latency offers diminishing returns:
- **Routine Instruction Implementation**: Adding opcodes following established patterns and recipes (`add-m68k-instruction`) once the core execution framework is proven.
- **Mechanical Code Refactoring**: Splitting files exceeding 800 lines into submodules, organizing imports, adding inlining attributes (`#[inline]`).
- **Documentation & Test Execution**: Running test suites, formatting Obsidian documentation, updating roadmaps, or writing boilerplate unit tests.

## 3. Hint Presentation Format
When an advisory trigger is met and the active model is suboptimal for the task, include a prominent callout at the beginning of the response (right below the audio transcript if voice input was used):

```markdown
> 💡 **Rekomendacja modelu:** Do obecnego zadania ([krótkie uzasadnienie, np. debugowanie kolejki prefetch w SingleStepTests]) zalecane jest przełączenie na model o wyższym poziomie myślenia (**High** / **Pro**), aby zapewnić precyzyjną analizę sekwencji cykl po cyklu.
```

Or for reverting to Medium:
```markdown
> 💡 **Rekomendacja modelu:** Wchodzimy w etap powtarzalnej implementacji opcodów według szablonu – możesz bezpiecznie przełączyć na **Medium**, co znacząco skróci czas odpowiedzi i oszczędzi limity tokenów.
```
