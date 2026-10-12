---
title: "Amiga 500 Testing Strategy and Quality Assurance"
aliases: ["Testing Strategy", "QA Strategy", "Four-Tier Verification Architecture"]
tags: ["amiga", "design", "testing", "architecture", "qa"]
category: "Design"
subsystem: "testing"
status: "active"
created: 2026-09-14
updated: 2026-10-12
related: ["[General Architecture.md](../General%20Architecture.md)", "[CPU SingleStepTests.md](CPU%20SingleStepTests.md)", "[CPU Instruction Benchmarking.md](../Performance/CPU%20Instruction%20Benchmarking.md)", "[Machine Loop.md](../Machine%20Loop.md)", "[Platform Quirks and Invariants Catalog.md](../Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
tracked_paths:
  - "crates/test_runner/tests"
  - "tools/verification"
  - "tools/bootstrap/tests"
  - "tools/bootstrap/conversion/tests"
  - "tools/bootstrap/pdf-to-markdown/tests"
  - "tools/amiga-rag-mcp-server/tests"
---

# Amiga 500 Testing Strategy and Quality Assurance

> [!NOTE]
> Project-wide engineering constraints, Rust coding guidelines, and constitutional rules are defined in [AGENTS.md](../../../../AGENTS.md) and [.agents/rules/unit-testing-policy.md](../../../../.agents/rules/unit-testing-policy.md).

---

## Completion entry point

Completion uses [verify.py](../../../../tools/verification/verify.py) with
`commit` for ordinary crate tests with coverage, quality gates, all available
standard harness suites, all Python tool contracts and the full English audit.
Standard harness selection excludes the aggregate `test_singlestep` target.
Transaction checker regressions, SingleStep override contracts and the bounded
[JSON mutation pilot](../../../../crates/test_runner/tests/test_singlestep_mutations.rs)
are standard suites. The pilot checks the verifier using selected silicon cases;
it does not run the full external suite. If no
standard suites remain, it schedules no Cargo test command. Python contracts
are discovered separately for bootstrap, shared conversion, PDF conversion,
the RAG MCP adapter and verification tooling.

The separate `milestone` profile always runs full SingleStepTests
and the full English audit, including on a clean tree. It does not repeat
commit checks. Stage closure requires both commands once for the same snapshot.
Standalone `test runner singlestep` selects full silicon verification for explicit diagnosis or user requests. This synthetic suite uses
TestMemoryBus rather than the production machine.

The [selection implementation](../../../../tools/verification/commands.py)
owns execution profiles; the [test execution reference](../../../../tools/verification/README.md#completion-verification)
describes commands and environment settings. Staging is optional, and completion
evidence must match the final committed source. Ordinary `test python` runs only
contracts affected by changed tool inputs or shared dependencies; explicit
directory selection runs that suite regardless of changed paths.

Scoped design review, vault checks and workflow maintenance use their owning
skills. Milestone semantic review follows [audit-semantic-parity](../../../../.agents/skills/audit-semantic-parity/SKILL.md).
English audit scope and vocabulary approval follow the [language policy](../../../../.agents/rules/language-policy.md).
The aggregate SingleStep test finalizes reports inside Rust after all selected groups finish;
vAmigaTS captures require a future execution entry point. Manual tier commands below are diagnostics,
not additional completion obligations.

## 1. Architectural Purpose & Testing Pyramid

The four tiers describe different evidence boundaries. Ordinary crate and integration tests, external CPU vectors and ordinary CPU wait checks are available. Whole-machine capture execution is deferred until a new entry point exists. Passing one tier establishes only the properties its fixtures and assertions check. Runtime code and external test suites remain separate:

```mermaid
graph TD
    classDef t1 fill:#1e3a5f,stroke:#4f9da6,stroke-width:2px,color:#ffffff;
    classDef t2 fill:#2d1b4e,stroke:#9d4edd,stroke-width:2px,color:#ffffff;
    classDef t3 fill:#3b1e36,stroke:#f72585,stroke-width:2px,color:#ffffff;

    subgraph Tier1["Tier 1: Isolated Unit Tests (L1 — Fast, Microsecond Scope)"]
        U1["Existing Crate Suites<br/><code>crates/cpu/tests/</code><br/><code>crates/config/tests/</code>"]:::t1
        U2["Existing Module Suites<br/><code>crates/disassembler/tests/</code><br/><code>crates/physical_memory/tests/</code>"]:::t1
        U3["Isolated Behavior<br/>Only where retained tests assert it"]:::t1
    end

    subgraph Tier2["Tier 2: Headless Multi-Crate Integration Tests (L2 — Subsystem Orchestration)"]
        I1["Machine Loop Integration<br/><code>crates/machine_loop/tests/</code><br/>(DMA contention, interrupts, reset, save states)"]:::t2
        I2["Debugger & Tooling Orchestration<br/><code>crates/debugger/tests/</code><br/>(Binary execution, temporal rewind, stepping)"]:::t2
        I3["Headless Developer Studio UI<br/><code>crates/gui/tests/test_interactions.rs</code><br/>(Full egui frame passes, keyboard, hex editing)"]:::t2
        I4["Motherboard Bus Routing<br/><code>crates/memory_bus/tests/</code><br/>(Address dispatch & register wiring)"]:::t2
    end

    subgraph Tier3["Tier 3: CPU Silicon (L3)"]
        V1["Silicon SingleStepTests<br/><code>crates/test_runner/tests/test_singlestep.rs</code>"]:::t3
    end

    V3["Tier 4: vAmigaTS Whole-Machine Captures<br/><code>Execution entry point unavailable</code>"]:::t3
    A1["Repository Architecture Gate<br/><code>tools/verification/checks/check_architecture.py</code>"]:::t1
    Tier1 --> Tier2
    Tier2 --> Tier3
    Tier3 --> V3
```

---

## 2. Unit, Integration, and CPU/Bus Verification

Ordinary CPU tests cover phase-specific waiting, stable retries, counted delay,
and transfer side effects through PhysicalMemory and a small recording wrapper.
They do not enumerate silicon vectors or DMA schedules; production arbitration
is tested separately. See [CPU wait coverage](CPU%20SingleStepTests.md#8-ordinary-cpu-wait-state-tests).

### Tier 1: Isolated Unit Tests (L1 — Fast, Unit Scope)
- **Scope**: Single crate, single module in complete isolation. Zero multi-chip machine state.
- **Execution Speed**: Depends on the remaining selected crate suites.
- **Execution Command**: `python tools/verification/verify.py test rust unit`
- **Responsibilities**:
  - Bitwise operations, register reads/writes, wrapping arithmetic, and state transitions.
  - Disassembler opcode parsing, effective address modes, and Fibonacci branch heuristics ([`crates/disassembler/tests/`](../../../../crates/disassembler/tests)).
  - CPU instruction, memory, wait-state, and exception behavior ([`crates/cpu/tests/`](../../../../crates/cpu/tests)).
  - Configuration and physical memory behavior ([`crates/config/tests/`](../../../../crates/config/tests), [`crates/physical_memory/tests/`](../../../../crates/physical_memory/tests)).

The removed isolated chip and peripheral suites no longer establish their former register, algorithm, or boundary assertions. Production-machine integration tests cover only their stated cross-chip cases. [Roadmap step 3.3](../../../../ROADMAP.md) plans new HRM-derived subsystem tests.

### Tier 2: Headless Multi-Crate Integration Tests (L2 — Subsystem Orchestration)
- **Scope**: Multi-crate interaction, bus routing, and system loop orchestration.
- **Execution Speed**: 3 to 15 seconds.
- **Execution Command**: `python tools/verification/verify.py test rust integration`
- **Primary Integration Hubs**:
  1. **Machine Loop Nexus ([`crates/machine_loop/tests/`](../../../../crates/machine_loop/tests))**:
     - `test_machine_loop.rs`: Top-level A500 state machine loop step progression, monotonic CCK clock progression, CPU bus wait-states.
     - `test_copper_machine_integration.rs`: Copper list execution, raster beam WAIT synchronization, palette mutation, and Level 3 IRQ.
     - `test_blitter_machine_integration.rs`: Blitter 2D memory operations (copy, fill), Chip RAM mutation, and Level 3 `_BLITINT` signaling.
     - `test_denise_palette_sprite_integration.rs`: Full 32-color palette batch writes, sprite channel vertical comparators, and data arming.
     - `test_audio_machine_integration.rs`: Paula audio DMA playback streaming from Chip RAM, period clock division, and Level 4 `AUDxDSR` interrupt propagation.
     - `test_dma_switching_and_signals_integration.rs`: Dynamic `DMACON` bitwise SET/CLR across all chips, electronic signal propagation delays, and unmapped open bus reads.
     - `test_dma_contention.rs`: Agnus DMA scheduler contending with CPU bus accesses.
     - `test_interrupt_pipeline.rs`: Level 1-6 interrupt prioritization between Paula, Agnus, CIA-A, CIA-B and CPU IPL.
     - `test_register_propagation.rs`: CPU writing custom registers with CCK electronic delay pipelines.
     - `test_cia_keyboard_integration.rs`: Physical keyboard transmission -> CIA-A SDR -> Level 2 interrupt to CPU.
     - `test_action_dispatch.rs`: Register writes dispatching strongly-typed actions across custom chips.
     - `test_reset.rs`: Cold, warm, and external CPU RESET instruction handling.
     - `test_rtc.rs`: Real-time clock BCD decoding and cycle-exact stepping.
      - `test_save_state.rs`: Complete machine state serialization and restoration roundtrip.

#### The 4-Question Integration Checklist (When Tier 2 Tests are Mandatory)

Whenever any custom chip, coprocessor, peripheral device, or interrupt line is added, modified, or refactored:
1. **Control & Strobe Mutation:** Does changing register bits (e.g. `DMACON`, `INTENA`, `COPCON`) gate or trigger execution?
2. **Autonomous Memory Transfer:** Does the subsystem read or write Chip RAM over multiple CCK cycles (Copper, Blitter, Denise, Audio, Floppy)?
3. **Cross-Chip Signal & Interrupt Routing:** Does the subsystem raise an interrupt line, latch status, or send electronic signals to other chips (Paula -> CPU, Agnus -> Denise, CIAs -> CPU)?
4. **Hardware Contention & Wait States:** Does the subsystem compete for Chip RAM slots or stall the CPU?

If any answer is **YES**, identify integration protection for that coordination. Reuse or extend an existing production fixture; add a test only for a demonstrated gap. A module change does not by itself require another test file. Expected results and runtime observation interfaces follow [unit-testing-policy.md](../../../../.agents/rules/unit-testing-policy.md).

#### The 4 Standardized Integration Test Archetypes

1. **Archetype A (Autonomous Progress):** Setup registers/memory -> trigger strobe -> step $N$ CCKs -> assert mutated Chip RAM buffer.
2. **Archetype B (Signal Escalation & CPU Autovector):** Trigger event -> step CCKs -> assert CPU IPL escalation and PC vector branch.
3. **Archetype C (DMA Gatekeeping):** Step with DMA bit set (assert progress) -> step with bit or master `DMAEN` cleared (assert zero progress).
4. **Archetype D (Contention & Concurrency):** Run active Chip RAM transfer -> assert CPU wait states on Chip RAM alongside zero wait states in Fast RAM.

#### Custom Chip Whole-Machine Verification Matrix

| Custom Chip | Primary Integration Test File | Behavior Exercised by Integration Fixtures |
| :--- | :--- | :--- |
| **Copper** | `test_copper_machine_integration.rs`, `test_copper_control_flow_integration.rs` | List execution, beam `WAIT`, `MOVE` to `COLOR00`, DMACON enable/disable, Level 3 IRQ, `SKIP` condition, `CDANG` danger mode |
| **Blitter** | `test_blitter_machine_integration.rs`, `test_blitter_nasty_contention_integration.rs` | 2D copy & fill in Chip RAM, `BLTSIZE` start, Level 3 `_BLITINT` assertion, CPU bus lockout, Blitter Nasty vs Fast RAM immunity |
| **Denise** | `test_denise_palette_sprite_integration.rs`, `test_denise_bitplane_integration.rs` | 32-color batch mutation, sprite vertical window, `SPRxDATA` arming, DMA toggling, 1-plane bitplane fetch, display window clipping |
| **Paula** | `test_audio_machine_integration.rs`, `test_interrupt_pipeline_integration.rs` | Audio DMA playback, period scaling, sample buffer generation, Level 4 `AUD0DSR`, multi-interrupt IPL 1-6 arbitration, INTENA masking |
| **DMA Arbiter** | `test_dma_switching_and_signals_integration.rs` | Bitwise SET/CLR in `DMACON`, master `DMAEN` global halt, 1-2 CCK electronic delays, open bus `$FFFF` |
| **CIAs** | `test_cia_keyboard_integration.rs`, `test_cia_machine_integration.rs` | Keyboard serial shift to CIA-A `SDR`, Timer A/B underflow Level 2/6 IRQs, horizontal TOD (CIA-B), vertical TOD 50 Hz tick (CIA-A) |


  2. **Debugger & Tooling Nexus ([`crates/debugger/tests/`](../../../../crates/debugger/tests))**:
     - `test_debugger.rs` & `test_stepping_and_session.rs`: Debugger session stepping `machine_loop` and CPU core.
     - `test_sample_binaries.rs`: Injecting compiled M68000 test binaries into RAM and executing instructions.
     - `test_temporal_and_trace.rs`: Time-travel rewind buffer tracking full machine state history.
  3. **Headless Developer Studio UI ([`crates/gui/tests/`](../../../../crates/gui/tests))**:
     - `test_interactions.rs`: Headless UI interaction tests simulating mouse clicks, keyboard shortcuts, memory edits, time-travel scrubbing, view switching.
     - `test_gui.rs` & `test_persistence.rs`: Immediate-mode UI layout, theme loading, dock splits, and window state.
  4. **Motherboard Bus Routing ([`crates/memory_bus/tests/`](../../../../crates/memory_bus/tests))**:
     - `test_router.rs` & `test_register_wiring.rs`: Routing 24-bit physical addresses across chips and storage.

### Tier 3: Silicon Ground Truth & Verification Harness (L3 — Physical Vectors)
- **Scope**: CPU state, instruction clocks and timed callbacks against external 68000 vectors. Ordinary crate tests own focused waiting invariants.
- **Execution Commands**: `python tools/verification/verify.py test runner singlestep` for explicit diagnosis; automatic full-suite selection occurs at milestones.
- **Key Verification Suites**:
  - **Tom Harte SingleStepTests ([`crates/test_runner/tests/test_singlestep.rs`](../../../../crates/test_runner/tests/test_singlestep.rs))**: Every executed vector compares state, instruction clocks and timed bus callbacks. Sampling changes input count only; abstract callback and state-comparison limits remain explicit (see [CPU SingleStepTests.md](CPU%20SingleStepTests.md#12-input-scope-and-complete-verification)).

Repository architecture checks are a separate gate, implemented in
[`tools/verification/checks/check_architecture.py`](../../../../tools/verification/checks/check_architecture.py).
`verify.py check architecture [case FILTER]` runs source and repository inspection
without compiling Rust. Commit verification always includes all 17 controls.
The compiled opcode descriptor coverage check remains in
[`crates/cpu/tests/test_visibility.rs`](../../../../crates/cpu/tests/test_visibility.rs)
and runs with ordinary CPU tests.

---

## 3. Tier 4: Whole-Machine Hardware Capture Verification

vAmigaTS reference assets remain available, but the former runner, injection,
script execution, viewport extraction and comparison commands have been removed.
Neither completion profile executes captures or chipset throughput measurements.
The old capture path compared a rendered 716 x 285 RGB24 viewport with hardware
reference images after running the selected program for scripted frames. A future
workflow must establish executable workload setup and a validated comparison path
before those assets can certify current whole-machine output.

A requested integration sprint uses the available production machine tests.
The [integration-test-sprint skill](../../../../.agents/skills/integration-test-sprint/SKILL.md)
owns discovery, failure clustering, scoped repair and final validation. Its capture
branch requires an executable entry point first. A passing crate or CPU suite
cannot substitute for that missing hardware comparison.

## 4. Diagnose Mechanisms Before Consolidating Fixes

A timing offset or local condition that makes one test pass can conceal the
wrong bus phase, signal propagation or state transition. Reproduce an isolated
failure and establish its mechanism before scaling a fix. Investigate a shared
cause when the evidence connects failures; unrelated defects need not collapse
into one hardware law. Preserve justified state-machine conditions while removing
temporary compensating patches.

Use [synthesize-test-fixes](../../../../.agents/skills/synthesize-test-fixes/SKILL.md)
when a requested review or accumulated exploratory changes need synthesis. Its
symptom, patch location and applied mechanism matrix exposes whether changes
share an upstream cause. This procedure does not add a mandatory review campaign
after every edit. Scope, reproduction and reference conflicts follow
[structural-root-cause.md](../../../../.agents/rules/structural-root-cause.md),
[repro-first.md](../../../../.agents/rules/repro-first.md) and
[spec-compliance.md](../../../../.agents/rules/spec-compliance.md).

---

## 5. Select Evidence for the Changed Behavior

Select existing checks for the actual change. The following categories describe available evidence and remaining prerequisites; they are not a sequence to execute after every edit. Commit and milestone profiles use the triggers above.

| Level | Verification Tier | Primary Tool / Command | Pass Criteria |
| :---: | :--- | :--- | :--- |
| **L1** | **Crate Behavior** | `python tools/verification/verify.py test rust crate <crate>` | Selected behavior and failure-mode assertions pass |
| **L2** | **Production Integration** | `python tools/verification/verify.py test rust crate machine_loop` | Existing or extended fixtures protect the affected coordination |
| **L3** | **CPU Vectors and Synthetic Contention** | `python tools/verification/verify.py test runner silicon` | Selected vectors pass state, clocks and timed bus comparisons; limits reported |
| **L4** | **Whole-Machine Captures** | Unavailable; needs a future execution and comparison entry point | Hardware workload and expected capture comparison |
| **Separate gate** | **Architecture and Quality** | Relevant `check` command; full profile before an authorized commit | Static constraints pass; source review remains necessary |

---

## 6. Structural Conventions & Invariants

1. **Dedicated External `tests/` Directory**:
   - Zero inline tests in `crates/*/src/` (`#[cfg(test)] mod tests` is strictly forbidden).
   - Keeps runtime production code clean, uncluttered, and $\le 800$ lines.
2. **Canonical Test Naming (`test_<name>.rs`)**:
   - In Cargo workspaces, every `.rs` file directly inside `crates/<crate>/tests/` compiles into an independent test binary.
   - All test files must strictly start with `test_` (e.g. `test_align.rs`, `test_singlestep.rs`).
   - Shared non-test helper modules must reside inside subdirectories (e.g. `tests/common/mod.rs`).
3. **Existing Module Suite Layout**:
   - The architecture gate retains required module-suite filenames for selected crates that still have isolated tests. Their presence does not prove behavioral coverage or require a new test file for every added submodule. Current examples:
     - `crates/disassembler`: `test_align.rs`, `test_alu.rs`, `test_branch.rs`, `test_data.rs`, `test_ea.rs`, `test_disassembler_facade.rs`.
     - `crates/config`: `test_config.rs`, `test_mutation.rs`.
     - `crates/physical_memory`: `test_address_bus.rs`, `test_map.rs`, `test_physical_memory.rs`, `test_presets.rs`.
4. **Behavior Coverage**:
   - Existing external test suites must contain an executable test. Crates without isolated suites depend on relevant integration fixtures until new domain tests are authored. Assess the behavior and failure modes actually checked; no assertion-count quota applies.

---

## 7. Automated Verification Tooling

| Tool / Script | Purpose | Enforcement Layer |
| :--- | :--- | :--- |
| [`tools/verification/verify.py`](../../../../tools/verification/verify.py) | Canonical commit/milestone quality gate and diagnostic frontend (`test rust`, `test python`, `test runner`, `check`, `runner`). | Explicit agent/developer command & CI |
| [`tools/verification/checks/check_architecture.py`](../../../../tools/verification/checks/check_architecture.py) | Python checks validating source constraints, test naming and retained required suite files; passing them does not prove behavioral coverage. | `verify.py check architecture` & commit verification gate |
| [`tools/verification/checks/audit_code_quality.py`](../../../../tools/verification/checks/audit_code_quality.py) | Checks dead code, test-only code and accessor conventions. Observed-caller visibility suggestions are advisory because external public API use is outside the repository scan. | `verify.py check code` & commit verification gate |
| [`$test-runner`](../../../../.agents/skills/test-runner/SKILL.md) | Standardized execution runner across tiers, writing snapshots to `.agent/tmp/singlestep/` and diffing regressions. | Codex skill |
| [`$integration-test-sprint`](../../../../.agents/skills/integration-test-sprint/SKILL.md) | Orchestrates the 4-iteration cascading verification sweep and failure cluster triage. | Codex skill |
| [`$synthesize-test-fixes`](../../../../.agents/skills/synthesize-test-fixes/SKILL.md) | Audits recent test patches, builds the 3-column diagnostic matrix, and synthesizes root causes into upstream crates. | Codex skill |

---

## 8. Related Architecture Specifications

- [General Architecture.md](../General%20Architecture.md): Amiga 500 system block diagram and crate dependencies.
- [Machine Loop.md](../Machine%20Loop.md): Machine loop cycle stepping, reset sequencing, and interrupt arbitration.
- [CPU SingleStepTests.md](CPU%20SingleStepTests.md): Verification against physical silicon instruction test vectors.
- [CPU Instruction Benchmarking.md](../Performance/CPU%20Instruction%20Benchmarking.md): Host performance measurement framework and retained comparison baselines.
- [Platform Quirks and Invariants Catalog.md](../Platform%20Quirks%20and%20Invariants%20Catalog.md): Centralized master catalog of hardware idiosyncrasies and anti-tamper invariants.
