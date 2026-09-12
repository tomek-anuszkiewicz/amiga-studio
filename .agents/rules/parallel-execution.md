---
trigger: model_decision
description: Parallel execution, targeted sub-suite testing, and asynchronous background task management.
---

# Parallel Execution & Asynchronous Task Optimization Rule

This rule governs how the agent executes tests, validation suites, and long-running operations to maximize execution velocity, eliminate unnecessary blocking latency, and leverage background parallelism.

---

## 1. Targeted Sub-Suite Execution in the Inner Loop

1. **Iterative Development Rule**:
   - When implementing, debugging, or refactoring a specific instruction, addressing mode, or subsystem, **never run the full test suite (~300,000 SingleStepTest cases) on every incremental code change**.
   - Always run the **targeted test filter** matching the opcode or module under active development:
     ```powershell
     cargo test -p test_runner --test test_singlestep -- <opcode_filter>
     ```
     *(e.g., `cargo test -p test_runner --test test_singlestep -- muls_mulu` or `cargo test -p m68000 -- test_ea_`)*.
   - Targeted tests execute in milliseconds, allowing rapid iterative cycles.

2. **Full Workspace Sweeps Reserved for Milestones**:
   - Full test sweeps (`cargo test`, `$env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep`, and `test_dma_cartesian`) are strictly reserved for the final verification phase of a task or milestone.

---

## 2. Asynchronous Background Task Execution

1. **Non-Blocking Execution for Heavy Operations**:
   - Heavy tasks (compilations, full SingleStepTests, cartesian DMA verification, RAG indexing) should be dispatched as asynchronous background tasks using `run_command` with low `WaitMsBeforeAsync` (e.g. 500ms).
   - Once dispatched, the agent should proceed with non-blocking tasks—such as formulating the Definition of Done checklist, inspecting git diffs, drafting walkthrough notes, or updating design documentation.

2. **Parallel Task Dispatching**:
   - When multiple independent checks are required (e.g., formatting check, architectural rules test, cartesian DMA test), launch them concurrently rather than waiting sequentially for each command:
     - Task 1: `cargo fmt --all -- --check`
     - Task 2: `cargo test -p test_runner --test test_architecture_rules`
     - Task 3: `cargo test -p test_runner --test test_dma_cartesian`
   - Alternatively, execute compound commands in a single pipeline:
     ```powershell
     cargo fmt --all -- --check; cargo test -p test_runner --test test_architecture_rules
     ```

3. **Reactive Wakeup (Zero Polling)**:
   - Antigravity's message broker automatically wakes up the agent when any background task completes.
   - The agent must **never busy-poll** or loop on `manage_task(Action='status')`. Simply proceed with other work or yield turn to let the platform notify upon completion.

---

## 3. Subagent & Multi-Agent Parallelism

1. **Specialized Workflows via Slash Commands**:
   - When initiating comprehensive audits, use the `/code-review` workflow to inspect git diffs, architecture compliance, and documentation pruning in a clean, focused context.
   - For long-running or autonomous goal-directed implementations, suggest `/goal` so the platform orchestrates thorough multi-step execution.

2. **Multi-Session Task Separation**:
   - For complex tasks requiring heavy research alongside active coding, recommend or utilize parallel conversation panels:
     - **Session A (Hardware Research / RAG)**: Queries Amiga hardware reference manuals (`amiga_rag`) and inspects circuit diagrams.
     - **Session B (Core Implementation)**: Authors instruction micro-steps and specialized handlers in `crates/m68000/`.
     - **Session C (Verification & QA)**: Runs exhaustive test runners and monitors test outputs.

---

## 4. The Subagent Delegation & Return Contract Standard

To eliminate main conversation token exhaustion while guaranteeing zero context amnesia:
1. **Isolated Subagent Execution**:
   - Heavy tasks (rendering offscreen PNGs, analyzing multi-MB test vectors, scanning 28 markdown specs, batch PDF transcription) must execute in isolated child subagents.
2. **Model Tier Assignment**:
   - **`Gemini Pro High`**: Reserved strictly for adversarial compliance reviews ([`code-review`](../skills/code-review/SKILL.md)) and cycle-exact microcode trace diagnosis ([`m68k-singlestep-test`](../skills/m68k-singlestep-test/SKILL.md)).
   - **`Gemini Flash High (Vision)`**: Perceptual layout inspection and self-healing ([`egui-vision-debugger`](../skills/egui-vision-debugger/SKILL.md)) and manual transcription ([`pdf-to-markdown`](../skills/pdf-to-markdown/SKILL.md)).
   - **`Gemini Flash Medium / Low`**: Fast text transformations, code scaffolding, git merges, and spec synchronizations.
   - **Main Agent**: Direct 1-shot perceptions ([`capture-gui-screenshot`](../skills/capture-gui-screenshot/SKILL.md)) and core pair-programming.
3. **Mandatory Return Contract**:
   - Every subagent must conclude its task by returning a strictly structured Return Contract (tables of symbol relocations, failing cycle coordinates, decision logs, or final image paths).
4. **The Multimodal Handshake**:
   - Subagents performing visual tasks must return the file path of the final verified image. The parent Main Agent immediately calls `view_file` on that path and embeds it in chat artifacts, ensuring both user and agent retain visual ground truth.
