# Amiga 500 Emulator: Development Roadmap

## Step 1: Manual Conversion and Initial Knowledge Base

Author reviewed book recipes once, then bootstrap Markdown and searchable assets from those recipes and matching source PDFs. Tools: [PDF converter](tools/bootstrap/pdf-to-markdown/README.md), [HTML converter](tools/bootstrap/html-to-markdown/README.md), and [bootstrap dispatcher](tools/bootstrap/bootstrap_documentation.ps1).

- **1.1: Evaluate SVG Extraction for PDF Graphics**
  - Try extracting selected graphics directly from source PDF regions as SVG instead of rendering PNG crops. Check whether vector paths, text, and embedded images remain faithful to the source; a raster image wrapped in SVG does not count as vector extraction.
  - **Gate:** Compare representative SVG outputs with the source PDF and current PNG crops for completeness, appearance, and usable bounds. Record which graphics can be preserved as SVG and which still require PNG because the source is raster or the extraction is incomplete.

- **1.2: Author Table, Image, and ASCII-Art Descriptions**
  - Generate descriptions during recipe authoring from one prepared group and its source context. Store accepted descriptions as explicit recipe content so reconstruction does not call a model again. Preserve HTML table spans, graphics and text-fenced ASCII art.
  - Render descriptions directly below the associated element in closed, collapsed `<details><summary>...</summary>...</details>` blocks. Describe source-visible labels, values and spatial/logical/timing relationships; table descriptions retain a GFM fallback with cell order and span semantics.
  - Generate each image's `<asset filename>.txt` sidecar from the same stored description for RAG ingestion, rather than maintaining an independent transcription.
  - Index representative generated content and test search queries separately for descriptions inside collapsed `<details>` blocks and for image `.txt` sidecars. Compare which terms and relationships are retrieved, result ranking, source attribution and duplicate results when both formats contain the same description.
  - **Gate:** User-reviewed samples establish faithful content and placement; repeated reconstruction produces matching descriptions/sidecars without duplicated blocks or new inference. Record RAG queries and returned source fragments for both description formats, including any retrieval gaps before broad indexing.

- **1.3: Final Chapter Proofreading and Code Formatting**
  - Once chapters are assembled, format code listings and add appropriate language tags for syntax highlighting without changing code, fixed-width text or ASCII-art alignment. Check uncertain words against the source PDF and correct supported OCR and typographical errors while preserving the author's wording.
  - Refine the existing [chapter proofreading prompt](tools/bootstrap/book-to-markdown/prompts/proofreading.md) for a model-assisted final pass that aligns formatting across the completed chapters. Keep source ambiguities visible for review rather than allowing the model to invent corrections.
  - **Gate:** Establish the final pass on a completed chapter by comparing it with the source PDF and rendered Markdown, then review the resulting changes across all completed chapters. Confirm source-supported word corrections, code formatting and highlighting, and preservation of tables, figures, links and technical values.

- **1.4: Define and Verify Portable Book Recipes (BOOK-RECIPE-PORTABLE)**
  - Define the published current-format `book.json` boundary: reviewed order, type/role, visibility, chapter organization, heading depth, table/cell structure and spans, source selections, graphic regions and explicit authored corrections/descriptions. Publish recipes and source identification; PDFs and generated bundles remain local.
  - Keep source-derived text as ordered references to physical PDF pages, extracted block IDs and Unicode character ranges with exclusive ends. Keep Markdown/HTML syntax and explicit replacements as authored recipe data. Verify source attribution where repeated equal text makes character matching ambiguous; rectangles select candidates rather than defining exact text selections.
  - Define extraction order, coordinate frame, DPI and rotation rules, including disconnected/multi-page content. Reconstruct graphics from PDF regions instead of requiring local `assets/` files. Record source SHA-256 in `source.json` and the authoring OCR/extraction profile separately from element data.
  - **Gate:** A reviewed recipe reproduces selected text, headings, table cells/spans, code spacing, math, corrections and graphics from the matching PDF without Stage 03/05/07 inference artifacts or copied generated assets. Establish the supported cross-platform reconstruction boundary; version pins alone do not prove byte-identical PDFs or matching text extraction.

- **1.5: Support Multiple Source PDFs in One Book Recipe**
  - Allow one logical book's `book.json` to reference several catalogued source PDFs. Identify the exact PDF for each source-derived text or graphic selection and keep its page, block, character range and region mappings specific to that PDF; different scans or editions must not be assumed to share coordinates or text extraction.
  - Support both assembly from multiple PDFs and reconstruction from any selected supported variant when the required content exists in that variant. Record explicit source-specific mappings and unresolved differences rather than silently substituting content from another PDF.
  - **Gate:** With a representative book that has multiple source PDFs, rebuild a reviewed fragment using each supported variant and a fragment assembled from more than one PDF. Check the output against the selected sources and reject an unknown PDF hash or a missing source mapping.

- **1.6: Implement Recipe-Based Markdown Bootstrap (BOOK-RECIPE-BOOTSTRAP)**
  - Add a standalone bootstrap command that obtains catalogued source PDFs, checks all 64 SHA-256 characters against each source's `source.json`, prepares native/OCR text with the recipe's authoring profile and extracts positioned text using the defined conventions. A directory suffix or an arbitrary existing OCR PDF is insufficient source/profile identity.
  - Execute reviewed `book.json` recipes to rebuild chapter Markdown, the generated TOC, source graphics, descriptions and sidecars. Resolve authored chapter titles/filenames and preserve explicit formatting and corrections. Do not repeat model inference, classification or description generation.
  - Target the current format only. Reconstruction/import may overwrite generated content; do not add backward compatibility, migrations or artifact-rescue code. Local editor acceptance does not itself publish a portable recipe or establish end-to-end reproducibility.
  - **Gate:** From a clean output directory and a reviewed recipe/source record, obtain the selected source PDF or PDFs and rebuild a user-selected book fragment without existing preparation directories, editor bundles or model calls. Demonstrate source/hash/profile mismatch handling and repeatable text/structure/assets under the supported extraction environment before expanding the run.

- **1.7: Reconstruct Reviewed Manuals and Verify RAG Ingestion**
  - After approving recipe authoring and standalone reconstruction, prepare reviewed recipes for the selected manuals and rebuild Markdown/assets through the recipe bootstrap. Check generated chapter links, source selections, tables, graphics and description sidecars before indexing.
  - Use canonical `rag_qdrant` ingestion into `projects_docs` with source `amiga`. Verify fresh and incremental indexing, hashing, chunk fidelity, embeddings and description retrieval. Project search covers `amiga` and `devnotes`; the owning DevNotes project remains responsible for indexing `devnotes`.
  - **Gate:** Reconstructed reviewed artifacts and fresh/incremental retrieval evidence follow the [RAG setup and indexing workflow](.agents/references/developer-setup.md#5-domain-hardware-knowledge-local-vector-rag-amiga-rag). Unchanged input does not trigger unnecessary reindexing; changed content is reflected in retrieved source fragments and image descriptions.

- **1.8: Bootstrap Optional FPGA Chipset References**
  - Add optional source selections for [MiniMig](https://github.com/cristianoag/minimig) and the [Amiga Replacement Project](https://github.com/nonarkitten/amiga_replacement_project) under `ref_src/`. Pin each downloaded source to a recorded upstream revision and keep the default test-source bootstrap independent of these research references.
  - **Gate:** Focused bootstrap checks verify selection, pinned source identity, repeat runs, and completeness for both references; the source inventory documents their local paths and revisions.

- **1.9: Include Measured Web Evidence in Hardware Research**
  - Check whether the [hardware researcher](.codex/agents/hardware_researcher.toml) can use web search when local manuals leave a hardware question open. If available, add focused `site:techtravels.org <hardware question>` searches and inspection of relevant results to the owning role or [domain rule](.agents/rules/amiga-rag.md). Otherwise route the search through an agent with web access or evaluate a scoped search tool. The existing [Amiga RAG MCP adapter](tools/amiga-rag-mcp-server/README.md) searches local indexed documents; extend it only if the selected route requires a new capability.
  - **Gate:** A representative Paula/`DMAL` question finds a relevant Techtravels.org page, cites the page and its supporting passage, and distinguishes measured signals from reconstructed circuitry without treating web results as indexed manual evidence.

## Step 2: CPU Benchmarking and Profiling

- **2.1: Rebuild CPU Instruction Benchmarks and Comparison Reports (`CPU-BENCH-01`)**
  - Derive an inventory of implemented legal instruction forms from the CPU decoder, excluding decode-exception entries. Cover each instruction, size, permitted addressing mode, direction, and materially distinct execution path with representative registers; add A7 cases where stack-pointer behavior differs. Give each case a stable ID and workload fingerprint covering its actual opcode, extension fixture, path, inputs, and recipe. Report included, excluded, and unsupported forms with reasons, and estimate full-run duration before executing it; the existing 108 rows do not establish full coverage.
  - Build deterministic, terminating workloads, aiming for about 700 target executions per block where safe and using separate constructions for control flow, exceptions, large encodings, and stateful operations. Verify the emitted target/helper sequence, actual counts, path, final PC/state, memory bounds, and observed CPU clocks before accepting a timing row. Preserve failed or wrong-path cases as diagnostic records, never as valid performance samples.
  - Inject and reset outside the timer; measure only the host CPU stepping loop through the intended exit, including its control overhead and any executed guest helpers. Warm up, pilot a series of about 100 ms, then hold its execution count fixed for four measured series. Retain raw durations and actual counts, report median/min/max/dispersion, and use bounded, recorded retries for unstable cases. Keep CPU clocks and CCK distinct and resolve the documented unit conflict through [specification compliance](.agents/rules/spec-compliance.md).
  - Report raw elapsed time per validated target execution as the primary composite cost. Include a same-session NOP control; isolate helper cost only when a matched paired workload validates subtraction. Record operands and paths for data-dependent instructions. Persist versioned JSON/CSV and a readable report under `.agent/tmp/benchmarks/`, with source and dirty-patch identity, harness/toolchain/build/host settings, pinning outcome, units, controls, retries, and anomalies.
  - Generate machine-readable case comparisons and analyses of comparable addressing-mode increments, byte/word/long scaling, complex/system workloads, and host time versus observed CPU clocks. Match cases by executed encoding, fixture, path, and workload, not label or row order; show missing or incompatible cases. Attribute a speed change to CPU source only after a controlled old/new rerun with the same harness and comparable host conditions. Treat the retained baseline without source provenance as historical context.
  - **Gate:** A full decoder-derived inventory and candidate run account for every included case, omission, failure, measured series, dispersion, actual runtime, and source/harness identity; JSON/CSV agree and comparisons exclude invalid or incomparable cells. Focused tests protect demonstrated workload, termination, counting, timer-boundary, and serialization failures. Supersede the old generated report only after validating its replacement. Present any tracked baseline retirement or replacement, with checksums and limitations, for a separate explicit reference decision; a generated report does not change the protected baseline.

- **2.2: Sampling and Counter Profiling Feasibility**
  - Try `cargo flamegraph` on headless execution; use the [external profiling workflow](.agents/skills/profile-external/SKILL.md) and `samply` when host constraints require it. Evaluate `cargo-profiler`/Callgrind/Cachegrind or `perf` as complementary tools and compare available hot-symbol results.
  - **Gate:** Record a working manual invocation, post-processing, baseline throughput, and available flamegraph output in `Obsidian/Amiga/Design/Performance/CPU Instruction Benchmarking.md`; document host/WASM limitations and a suitable alternative platform for unavailable tools.

## Step 3: HRM-Aligned Custom Chipset Reconstruction

- **3.1: Verify Custom Register Paths and Physical Signal Routing**
  - Inventory every custom-register and CIA-A/CIA-B address decoded by `memory_bus.rs` against the register definitions and [ownership matrix](Obsidian/Amiga/Design/Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md). For each writable register, trace CPU and Copper access where applicable through byte/word decoding, all intended recipients, delayed mutation staging and commit where applicable, and the receiving chip's own state or explicit action. Account for multi-chip fan-out, set/clear semantics, and strobes without requiring a stored field where hardware behavior does not call for one; repair missing routes and missing commit effects for queued writes.
  - For each readable register, verify that live and debug reads use the correct chip state or computed value, including read side effects, CIA selection and lane behavior, documented dummy addresses, and the modeled open-bus fallback.
  - Compare A500 wiring and the [cross-chip signals catalog](Obsidian/Amiga/Design/Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md) with `memory_bus.rs` and `machine_loop.rs`. Trace register-bus strobes, DMA requests and grants, interrupts, sync, reset, and peripheral pins from source output through machine routing to recipient input at the required CCK phase; resolve missing, reversed, or direct peer connections against hardware evidence.
  - **Gate:** Record a register-by-register route, recipient, commit/state or action, and read-result inventory, distinguishing implemented behavior from a decoder branch or queued mutation. Focused chip, bus, and machine integration tests cover each distinct register behavior and signal path, including exceptions and multi-chip writes. No mapped write silently disappears or remains queued without its intended effect; identify hardware behavior that remains unproven rather than inventing it.

- **3.2: Model Old Gary Address Decoding on the A500 (`MEM-GARY-01`)**
  - In every supported A500 preset, route banks `$A0-$BF` to the CIA decoder. A13/A12 are active-low selects: `00` selects both CIAs, `01` CIA-B, `10` CIA-A, and `11` neither. A11..A8 select the register; A7..A1 mirror it. `$BFC000` is one address in the `$BFC000-$BFCFFF` selected block, and the same selection pattern repeats in other mapped banks. CIA register selector `B` remains unconnected.
  - With both CIAs selected, an even byte read uses CIA-B and an odd byte read CIA-A; a word read puts CIA-B in the high byte and CIA-A in the low byte. A byte write sends the same value to every selected CIA regardless of parity; a word write sends the high byte to selected CIA-B and the low byte to selected CIA-A. Pass full register identities to the existing CIA recipients and preserve CIA-A Port A overlay effects on every alias. Retain the documented fixed placeholder for an inactive read lane rather than assuming a physical data-bus latch.
  - Route `$DE` and `$DF` through the same custom-register decoder, including its low-offset mirror, byte/word behavior, delayed mutations, side effects, and side-effect-free debug reads. Preserve installed Slow RAM and RTC precedence; evaluate other model-dependent mirrors separately. Update [MemoryBus](Obsidian/Amiga/Design/MemoryBus.md), [Physical Memory](Obsidian/Amiga/Design/Physical%20Memory.md), [CIA](Obsidian/Amiga/Design/CIA.md), and the [Platform Quirks and Invariants Catalog](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md) together with the code. Recheck the HRM wording; WinUAE `cia.cpp`/`memory.cpp` and `ref_src/vAmiga-4.5/Core/Components/Memory/Memory.cpp` corroborate the decode but do not prove physical A500 wiring without schematic or measured evidence.
  - **Gate:** First reproduce failures against the original router and bank table. Focused `memory_bus` and `physical_memory` tests cover dual selection, byte/word lanes and writes, nonzero register selectors, low-bit and `$A0-$BE` aliases, inactive selection, overlay effects, `$DE` reads/writes/debug reads, and Fast/Slow RAM and RTC boundaries. Then pass adjacent bus and machine integration checks.

- **3.4: Strategic Clean-Slate Reset of Custom Chips & Machine Loop (Baseline HRM Spec Alignment)**
  - **Objective:** Cleanse and reset `crates/machine_loop` and all specialized custom chip subsystems (`agnus`, `denise`, `paula`, `cia`, `copper`, `blitter`, `floppy`) outside the CPU and physical memory to pure HRM architectural specifications, purging accumulated legacy scaffolding.
  - **Actionable Scope:**
    - Cleanse register definitions and reset states to align strictly with the official Commodore Amiga Hardware Reference Manual (HRM).
    - Establish an "idealistic", pure architectural baseline with zero ad-hoc hacks.
    - **Decision required before implementation:** The proposed removal/deferment of timing and hardware quirks conflicts with the current design invariants. Review it against [specification compliance](.agents/rules/spec-compliance.md) and the [Platform Quirks Catalog](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md); preserve documented behavior unless an explicit specification change is approved.
  - **Verification Gate:**
    - `python tools/verification/verify.py check build`
    - `python tools/verification/verify.py check architecture`

- **3.5: Signal Propagation Documentation Audit & Hardening**
  - **Objective:** Ensure that all agent governance files (rules, skills, workflows, design docs) accurately and completely describe the two signal propagation mechanisms so any future agent has zero ambiguity about how to implement them correctly.
  - **Actionable Scope:**
    - **Register-write propagation:** Verify that `hardware-bus-topology.md`, relevant design specs, and `MachineLoop` documentation clearly describe what happens clock-phase by clock-phase when a CPU or DMA write lands in a custom chip register (e.g. `COLOR00`, `BPLCON0`, `INTENA`). Update or author missing sections.
    - **Special inter-chip signal lines:** Verify documentation for DMA request/grant (`RGA` bus), interrupt lines (`_IPL0`–`_IPL2`, `_INTREQ`/`_INTENA` propagation through Paula → CIA → CPU), and any other active signal paths (e.g. blitter busy, disk DMA, copper WAIT/SKIP). Ensure polling methods and timing are documented with cycle-phase precision.
    - Sync any divergence between documentation and the actual `MachineLoop` poll-and-route implementation.
  - **Verification Gate:**
    - Run the scoped specification review and [vault link/frontmatter audit](.agents/skills/obsidian-vault-linking/SKILL.md); the milestone command does not execute documentation maintenance. At point closure, run commit and milestone profiles at their existing triggers.
    - The intended commit changes were reviewed against documents selected by `tracked_paths`; real corrections are included, and accurate documents remain untouched.

- **3.6: HRM-Aligned Subsystem Test Suite Authoring**
  - **Objective:** Write a clean, authoritative test suite for each custom chip subsystem derived strictly from the Amiga Hardware Reference Manual — not from source code inference. Tests must be simple, register-behavioral, and free from cycle-exact timing complexity.
  - **Actionable Scope:**
    - One test file per subsystem: `test_agnus.rs`, `test_denise.rs`, `test_paula.rs`, `test_cia.rs`, `test_copper.rs`, `test_blitter.rs`.
    - Each test: write a register value → advance minimum required clocks → assert observable output or state. No multi-chip timing chains in L1 tests.
    - Simple inter-subsystem interaction tests (L2): focus on the cleanest signal paths — e.g. CIA timer overflow → Paula `_INT` → CPU IPL change. Prefer to write inter-chip tests and any required implementation changes during horizontal blanking (hblank return) when bus is idle, to minimize contention complexity.
    - Do **not** target vAmigaTS or silicon-level cycle accuracy in this step — that is Step 3.10 and Step 5.
  - **Verification Gate:**
    - `python tools/verification/verify.py commit`
    - `python tools/verification/verify.py check architecture`
    - Tests verify subsystem behavior, boundary conditions, and relevant failure modes per unit-testing-policy; no assertion-count quota applies.

- **3.7: Lockstep Differential Tracer vs. vAmiga (Contingency)**
  - **Trigger:** Only if HRM-aligned tests (3.6) fail to isolate a regression — i.e. tests pass but behavior diverges from reference in ways not yet covered by the test suite.
  - **Objective:** Build a cycle-exact co-simulation harness that runs the same ROM/ADF through both this emulator and vAmiga in lockstep, dumps a full machine state snapshot at every CCK boundary, and reports the first divergence point with a structured diff.
  - **Actionable Scope:**
    - Add a `StepTracer` trait (or feature-gated callback) to `MachineLoop`: at each `step_cck()`, serialize `CpuState` + all chip register banks into a compact binary or JSON snapshot. All state structs already implement `serde::Serialize` — snapshot is essentially `serde_json::to_string(&machine.snapshot())`.
    - Wire vAmiga's existing headless state-dump mode (`ref_src/vAmiga`) to emit equivalent snapshots at the same CCK boundaries.
    - Harness (`tools/verification/lockstep_diff.py`): load both snapshot streams, walk them in parallel, and stop at the first CCK where any field diverges — output: `CCK #N | field | expected (vAmiga) | actual (ours)`.
    - Keep tracer behind a feature flag (`--features tracer`) so zero overhead in production builds.
  - **Verification Gate:**
    - Tracer successfully identifies the CCK and field of a known injected regression (synthetic test).
    - `python tools/verification/verify.py test rust crate test_runner suite test_lockstep_tracer` (smoke test against a trivial ROM loop).

- **3.8: Kickstart ROM & Floppy Subsystem Bring-Up for Native Program Execution**

  - **Objective:** Operationalize the authentic floppy disk subsystem and Kickstart ROM overlay bootloader sequence to load and execute genuine Amiga programs from disk images (`.adf`).
  - **Actionable Scope:**
    - Implement `DF0:` drive mechanics, MFM track deserializer, and DMA track streaming.
    - Wire Kickstart ROM overlay boot sequence ($000000 / $FC0000).
    - Establish end-to-end capability to boot ADF disk images as the foundational prerequisite for native test harnesses.
  - **Verification Gate:**
    - Unit tests in `crates/floppy/tests/`
    - Machine loop boot integration tests in `crates/machine_loop/tests/`

- **3.9: vAmigaTS Native Disk-Based Verification & Self-Testing**
  - **Objective:** Execute native Amiga disk-based test suites validating drive control, floppy DMA, and CPU coordination.
  - **Actionable Scope:**
    - Execute disk-based vAmigaTS tests verifying that the emulator can run native Amiga software to test itself.
    - Leverage unthrottled headless execution (running as fast as the host CPU permits without wall-clock rate limiting) for maximum throughput.
  - **Verification Gate:**
    - Automated test run reports for disk-based test suites passing with zero unexpected halts.

- **3.10: Principled, Rule-Compliant Custom Chip & Register Verification**
  - **Objective:** Advance custom chips and registers systematically from clean baseline specifications to fully validated cycle-exact implementations.
  - **Actionable Scope:**
    - Implement and calibrate custom chip features following the substrate-first causality chain (Layer 0 $\to$ Layer 4).
    - Strictly adhere to defect resolution (`.agents/rules/structural-root-cause.md`) and 4-tier integration archetypes (`.agents/rules/unit-testing-policy.md`).
  - **Verification Gate:**
    - `python tools/verification/verify.py test rust crate machine_loop`
    - `python tools/verification/verify.py test rust integration`

---

## Step 4: Developer Studio GUI & Diagnostic Tooling Hardening (Ongoing Companion Track)

- **4.1: Continuous Testing & Refinement of Developer Studio (Debugger View)**
  - **Objective:** Actively test, harden, and refine the Developer Studio GUI (`crates/gui`) under live emulation across all display tiers and interactive workflows.
  - **Actionable Scope:**
    - Test interactive panels under live stepping: Disassembly infinite stream browsing, in-place instruction patching, Memory Hex selection and row-wrapping keyboard navigation, register/CCR editing, Microcode Inspector step progression, and breakpoint/watchpoint triggers.
    - Verify layout stability, margin geometry, scrollbar ergonomics, and responsive display tiers (`LayoutTier::FullHdWide`, `LayoutTier::StandardDesktop`, `LayoutTier::Compact`).
    - Test upcoming Save State management (`State` menu, in-memory quick slots 1–5, `F6`/`F9` shortcuts, and native file dialogs).
  - **Verification Gate:**
    - `python tools/verification/verify.py test rust crate gui suite test_interactions` (all headless egui interaction tests passing).

- **4.2: Developer Studio & Debugger Keyboard Shortcuts Documentation & Discoverability**
  - **Objective:** Complete audit, documentation, and user-facing surfacing of all implemented keyboard shortcuts across Developer Studio and Game View modes, ensuring zero hidden keys.
  - **Actionable Scope:**
    - Implement a dedicated in-app Keyboard Shortcuts Reference Modal (`?` / `F1` / Top Menu Help > Shortcuts).
    - Enrich button hover tooltips with contextual shortcut cues.
    - Surface all shortcuts: View Modes (`F12`/`F2`, `Escape`), Stepping Controls (`F5`/`Space`, `F10`, `Shift + F10`, `F11`), Tooling & Machine Controls (`F8`, `Alt + T`, `Ctrl + R`, `Ctrl + O`), Save States (`F6`, `F9`, `Ctrl + S`, `Ctrl + L`), Display Zoom (`Ctrl + +/=`, `Ctrl + -`, `Ctrl + 0`), and Hex/Disassembly editing navigation.
  - **Verification Gate:**
    - Headless integration test in `crates/gui/tests/` verifying modal dialog trigger and shortcut handling.

---

## Step 5: vAmigaTS Silicon Verification Sub-Suites & Contention Track (After Chipset Bring-Up)

- **5.1: Test Suite Categorization, Filtering & Deferred Execution Matrix (609 Deferred Tests)**
  - **Objective:** Enforce strict gating and categorization for tests requiring hardware beyond the baseline A500 OCS configuration.
  - **Actionable Scope:**
    - Maintain and audit formal gate categories: FPU Coprocessors (206 tests deferred to the scope decision in Step 11.2), ECS & AGA Silicon (112 tests deferred to Steps 11.1 and 11.2), 68010 CPU Architecture (91 tests deferred to Step 11.2), AmigaOS Floppy MFM Bootblock (7 tests deferred to Step 9.1), and Non-Visual Register Assertions (193 tests deferred to Step 5.2.6).
    - A future capture entry point must classify supported and deferred tests before executing baseline OCS suites; the previous runner has been removed.
  - **Verification Gate:**
    - Demonstrate supported/deferred selection through the future capture entry point before executing its baseline; no current Cargo suite provides this gate.

- **5.2: Targeted Subsystem Verification Sub-Suites Execution Track (1,468 Active Baseline Tests)**
  - **Objective:** Progressively execute and pass the 1,468 active baseline vAmigaTS tests in strict substrate-first causal order (Layer 0 $\to$ Layer 4). Re-establish the reporting format when this verification track resumes.
  - **Actionable Scope:**
    - **Sub-Suite 5.2.1: Agnus Master DMA Contention & Bus Arbitration (Layer 0 Substrate, 225 tests):** `Agnus/` (`DMACON/`, `BplDma/`, `DIW/`, `DDF/`, `bususage/`), verifying Color Clock phase allocation (odd/even), refresh cycles, CPU wait-state generation, and Blitter Nasty (`BLTPRI`).
    - **Sub-Suite 5.2.2: Agnus Blitter & Copper Coprocessor Engines (Layer 1 Coprocessors, 364 tests):** `Agnus/Blitter/` (250 tests: `line/`, `fill/`, `sblit/`, `timing/`, `bbusy/`, `bltint/`) and `Agnus/Copper/` (114 tests: `Wait/`, `Skip/`, `coptim/`, `coprace/`, `copvbl/`), verifying autonomous bus master operations, 256-minterm ALU, barrel shifts, modulos, and Copper 4-CCK timing.
    - **Sub-Suite 5.2.3: Denise Video, Bitplanes & Sprites (Layer 2 Video Serializer, 210 tests):** `Denise/` (`Registers/`, `Modes/`, `DIW/`, `Sprites/`), verifying pixel serialization, palette translation, display window clipping, and sprite multiplexing.
    - **Sub-Suite 5.2.4: Paula Audio & Floppy Subsystem (Layer 3 Peripherals, 107 tests):** `Paula/` (`Audio/`, `Interrupts/basicint/`), verifying PCM sample streaming, period clock division, and Level 1–4 interrupt requests.
    - **Sub-Suite 5.2.5: Complex CIA-A / CIA-B & Timers (Layer 3 Peripherals):** Timers A & B, TOD clock 50/60 Hz synchronization, serial shift register (SDR), and port handshake lines.
    - **Sub-Suite 5.2.6: Mainboard, Memory & Peripheral Assertions (Layer 4 System, 58 visual + 193 non-visual tests):** `Mainboard/`, `Memory/`, `Misc/`, verifying address decoding, port registers, and RAM expansion configurations.
    - **Sub-Suite 5.2.7: M68000 CPU Silicon Pipeline (Layer 4 System Integration, 503 tests):** `CPU/` (ALU, bitwise, shifts, exceptions, traps, IPL autovectors).
  - **Verification Gate:**
    - Restore a capture-runner entry point and verify each selected vAmigaTS category against its hardware reference captures.

- **5.3: Dynamic Agnus DMA Slot Arbitration Optimization (Deferred Performance Track)**
  - **Objective:** Optimize the dynamic per-CCK DMA slot priority evaluation into a clean, pre-computed scanline slot lookup table (`[DmaSlot; 227]`).
  - **Actionable Scope:**
    - Localize optimization strictly within the Agnus DMA scheduler (`crates/dma/` / `crates/agnus/src/dma/`).
    - Preserve code simplicity and readability, with zero complex asynchronous event wheels or timing skips.
    - Once a representative whole-machine workload is executable, compare fresh throughput and external sampling profiles before and after the optimization; CPU-only benchmarks do not measure Agnus scheduling.
  - **Verification Gate:**
    - Zero regression across all active baseline OCS vAmigaTS suites.
    - Run DMA, Agnus and production memory_bus regression tests, including contention and pending CPU-request arbitration; CPU-only waiting tests do not validate scheduler changes.

---

## Step 6: Custom Chipset Debugger & Deep Architectural Observability (Developer Studio Extension)

- **6.1: Custom Chipset Registers & Mutation Delay Pipeline Inspector**
  - **Objective:** Implement dedicated register inspection docks/tabs in Developer Studio (`crates/gui`) with live delta highlighting and mutation delay observability.
  - **Actionable Scope:**
    - Dedicated docks for Agnus, Denise, Paula, CIAs (A & B), and RTC.
    - Format values in hex/binary with electric cyan change/delta highlighting.
    - Bitfield breakdown widgets and interactive tooltips for control/status registers (`DMACON`, `INTENA`, `INTREQ`, `BPLCON0`-`BPLCON2`, `ADKCON`, `CRA`/`CRB`).
    - Visualize staged pipeline register latches taking effect across Color Clock phases ($CCK1 \to CCK2$).
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying register dock rendering and delta tracking.

- **6.2: Agnus DMA Slot Scheduler & Real-Time Bus Allocation Visualizer**
  - **Objective:** Build a horizontal scanline timeline visualizer displaying the 227 Color Clock slots per scanline (PAL) / 226 slots (NTSC).
  - **Actionable Scope:**
    - Fixed allocations: DRAM refresh (CCK 0..3), Disk DMA (CCK 4), Audio DMA 0–3 (CCK 5..8), Sprite pairs 0–7 (CCK 12..27).
    - Dynamic allocations: Bitplane DMA (BPL 1–6), Blitter DMA (channels A, B, C, D), and CPU bus slots.
    - Live beam position cursor tracking horizontal ($HPOS$) and vertical ($VPOS$) raster coordinates.
    - Bus contention & wait-state indicator visually flagging cycles where CPU or Copper are stalled (BLTPRI / Blitter Nasty).
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying timeline visualizer rendering.

- **6.3: Copper Coprocessor Inspector & Real-Time Execution Tracker**
  - **Objective:** Implement a dedicated Copper list disassembler panel decoding instruction streams directly from Chip RAM with real-time execution tracking.
  - **Actionable Scope:**
    - Decode instruction streams (`MOVE`, `WAIT`, `SKIP`) from Chip RAM pointers (`COP1LC`, `COP2LC`, `COPJMP1`, `COPJMP2`).
    - Real-time execution pointer tracking: highlight executing Copper instruction, pending wait condition ($VPOS$/$HPOS$ comparison), and CDANG danger mode status.
    - Visual correlation with raster beam: highlight beam position where Copper interrupts or register modifications trigger palette swaps or Blitter dispatches.
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying Copper disassembler panel.

- **6.4: Internal Chipset State Machines & Deep Diagnostics**
  - **Objective:** Surface live diagnostic panels for internal chipset state machines in `crates/gui`.
  - **Actionable Scope:**
    - *Blitter Engine Diagnostics:* Visual representation of active channels (A, B, C, D), 256-minterm truth table visualization ($LF$ code decomposition), shift/mask register stages, and Bresenham line-drawing state counters.
    - *Denise Video & Sprite Pipeline:* Live inspection of bitplane serializers, dual-playfield priority layers, 8 hardware sprite registers, and collision latches (`CLXDAT`/`CLXCON`).
    - *Paula Multi-Engine Status:* Audio channel frequency, length, volume, BLEP table synthesis status, floppy MFM bit-stream buffers/sync-word detector (`$4489`), and serial UART FIFO.
    - *CIA Timers & Port Observability:* Live countdown of Timers A & B, TOD clock sub-second counters, serial shift register (SDR) status, and I/O port pin states.
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying state machine diagnostic views.

---

## Step 7: Host I/O Peripherals, Audio Playback & Controller Hub

- **7.1: Host Audio Playback & CRT Presentation Shaders**
  - **Objective:** Operationalize host real-time audio output streaming and authentic CRT display presentation.
  - **Actionable Scope:**
    - Audio sink ring buffer decoupled from host audio playback (`cpal` / Web Audio) with dynamic resampling and ring buffer underflow/overflow protection.
    - GPU post-processing shaders for authentic CRT TV presentation (scanlines, shadow mask, curvature, phosphor bloom).
  - **Verification Gate:**
    - Audio ring buffer unit tests in `crates/audio/tests/`.
    - Headless shader initialization and rendering verification.

- **7.2: Host Input Subsystem, Game Controller Mapping & Port Hub**
  - **Objective:** Implement comprehensive host-to-Amiga keyboard translation, physical mouse capture, gamepad ingestion, and dual game port switching (`crates/keyboard`, `crates/mouse`, `crates/joystick`, `crates/game_ports`, `crates/gui`).
  - **Actionable Scope:**
    - **Host Keyboard Mapping:** Translation table from host keyboard events (`winit::keyboard::KeyCode` / `egui::Key`) to raw Amiga scancodes, qualifiers (Amiga keys, Alt, Ctrl, CapsLock, Help), `Ctrl + Left Amiga + Right Amiga` reset combo, and Keyboard-as-Joystick mapping.
    - **Dual Game Port Hub:** Slot assignment for Port 1 & Port 2 (Mouse, Joystick, CD32 Pad, Keyboard-Joystick, Disconnected), simultaneous dual-mouse and dual-joystick modes, hardware button routing (CIA-A `PRA`, Paula `POTGOR`), and CD32 7-button shift register protocol.
    - **Physical Host Mouse Capture:** Viewport mouse grab/lock mode, relative motion delta scaling into Amiga 8-bit wrap-around quadrature counters ($0..255$), and sensitivity tuning.
    - **Physical Gamepad & Joystick Ingestion:** Host gamepad enumeration via `gilrs` (native desktop) and HTML5 Gamepad API (WASM), analog deadzones, and button mapping.
    - **Developer Studio Input UI:** Interactive "Game Ports & Input" tab/dock displaying port status, device dropdowns, and live switch/button indicators.
  - **Verification Gate:**
    - Unit tests in `crates/keyboard/tests/`, `crates/mouse/tests/`, `crates/joystick/tests/`, and `crates/game_ports/tests/`.
    - Headless UI input tests in `crates/gui/tests/`.

---

## Step 8: Dedicated Player GUI & Frontend Experience

- **8.1: Hardware Configuration & Kickstart ROM Selector**
  - **Objective:** Deliver user-friendly Amiga hardware profile selection and Kickstart ROM management.
  - **Actionable Scope:**
    - Profile selector: Basic A500 512 KB, Classic A500 1 MB (Recommended), Expanded A500 4 MB.
    - Reset warning confirmation modal informing user that changing hardware configuration requires a cold machine reset.
    - Kickstart ROM manager: file picker for ROM images (1.2, 1.3, custom) with automatic checksum validation (CRC32/SHA-256).
  - **Verification Gate:**
    - Headless UI tests in `crates/gui/tests/` for profile selection and ROM validation modals.

- **8.2: Multi-Drive Floppy Disk Manager (`DF0:` – `DF3:`)**
  - **Objective:** Visual floppy drive slot manager with live drive activity indicators and disk insertion controls.
  - **Actionable Scope:**
    - Drive slot manager displaying primary internal drive `DF0:` and optional external drives (`DF1:`–`DF3:`).
    - Per-drive enable/disable toggles to mount or disconnect external floppy drives on the fly.
    - ADF file picker per drive with quick insert, eject, and write-protect latch controls.
    - Visual drive activity indicators and drive stepping audio feedback.
  - **Verification Gate:**
    - Unit tests for multi-drive management and headless UI interaction tests.

- **8.3: Visual Save State Manager (Screenshots, Timestamps & Custom Labels)**
  - **Objective:** Provide an interactive visual save/load state overlay and slot manager with screenshot previews and configuration guards.
  - **Actionable Scope:**
    - Visual snapshot cards containing embedded thumbnail screenshots captured at the moment of saving.
    - Formatted creation date and time timestamps and user-defined state labels/descriptions.
    - Configuration integrity guard verifying matching hardware profiles before restoring state to prevent guest crashes.
    - Preserve the active CPU execution path when restoring a snapshot during a redirected branch or exception; opcode-only slice reconstruction does not establish this. Gate with interrupted execution and identical continuation after serialization.
  - **Verification Gate:**
    - Save state serialization unit tests and UI state manager tests in `crates/gui/tests/`.

---

## Step 9: Real-World Amiga Workloads, Host Cache Profiling & Pipeline Optimization (Post-Boot)

- **9.1: Deferred Full-OS & MFM Floppy vAmigaTS Test Suites Verification Gate**
  - **Objective:** Verify physical floppy track MFM streaming and full Kickstart 1.3 bootstrap test suites.
  - **Actionable Scope:**
    - Execute the 9 non-standard test ADFs using physical floppy MFM track streaming via `crates/floppy`.
    - Unfilter and pass all vAmigaTS tests requiring full Kickstart 1.3 bootstrap (`dos.library`, `intuition.library`, and filesystem calls).
  - **Verification Gate:**
    - vAmigaTS test runner passing all MFM and OS-level tests.

- **9.2: End-to-End Bootable ADF Integration Testing**
  - **Objective:** Boot and execute genuine Amiga benchmarks and diagnostic suites directly from floppy disk images (`python tools/verification/verify.py test rust crate test_runner suite test_boot_adf`).
  - **Actionable Scope:**
    - Load and execute `AmigaTestKit.adf`, `SysInfo.adf`, and Dhrystone on the authentic Kickstart / Amiga chipset stack.
    - Leverage address guards, breakpoint traps, and instruction bounds for parameterized termination.
    - Assert functional correctness, numerical determinism, and measure baseline throughput (MIPS, host CPU cycle cost per emulated CCK).
  - **Verification Gate:**
    - `python tools/verification/verify.py test rust crate test_runner suite test_boot_adf` passing cleanly.

- **9.3: Real-Workload Host Cache Miss & Footprint Impact**
  - **Objective:** Profile host L1i/L1d cache misses, Last Level Cache (LLC) misses, and branch mispredictions during continuous execution of genuine Amiga software.
  - **Actionable Scope:**
    - Profile host PMU counters across the 65,536-entry static dispatch table during real workloads.
    - Quantify the empirical performance impact of CPU core memory footprint on non-repetitive workloads.
  - **Verification Gate:**
    - Documented PMU profiling metrics and analysis in `Obsidian/Amiga/Design/`.

- **9.4: CPU Core Memory Footprint Audit & Compaction Assessment**
  - **Objective:** Audit the exact memory footprint of the CPU core and assess compaction strategies.
  - **Actionable Scope:**
    - Audit host `.rodata` footprint (sizeof(OpcodeDescriptor) * 65,536, microcode arrays) and runtime state footprint (`Cpu`, `CpuState`, `CpuMicroState`).
    - Investigate whether bit-packing or deduplication yields measurable cache-miss reductions, or whether flat layout maximizes throughput.
    - Refactor slow handlers using host efficiency principles without custom macros or const-generics.
  - **Verification Gate:**
    - Memory footprint audit report and SingleStepTests passing with zero regressions.

- **9.5: Cross-Architecture & Mobile Performance Projections**
  - **Objective:** Extrapolate measured throughput to target mobile and constrained environments (e.g. mobile WebAssembly, ARM mobile).
  - **Actionable Scope:**
    - Model CPU overhead margins to ensure sustained 50 Hz (PAL) / 60 Hz (NTSC) cycle-exact emulation with full chipset contention and rendering.
  - **Verification Gate:**
    - Documented performance analysis projections in `Obsidian/Amiga/Design/`.

- **9.6: Fidelity & Regression Validation Gate**
  - **Objective:** Final regression certification across all optimization and refactoring phases.
  - **Actionable Scope:**
    - Ensure every optimized handler retains 100% cycle-exact Color Clock fidelity.
    - Pass the full exhaustive SingleStepTests suite ($env:SINGLESTEP_FULL = "1") and full workspace unit/integration test suites.
  - **Verification Gate:**
    - `python tools/verification/verify.py test runner singlestep full`
    - `cargo test --workspace`

---

## Step 10: Documentation-to-Code Regeneration Experiment

- **10.1: Isolated Documentation-to-Code Regeneration Trial**
  - In an isolated checkout, remove one selected subsystem and regenerate it from curated specs, rules, and test vectors. Compare behavior and source with the preserved original; use failures to refine documentation and instructions.
  - **Gate:** The regenerated subsystem passes its unit and integration suites without regressions.

## Step 11: Later Hardware and Diagnostic Extensions

- **11.1: ECS and Later A500 Models**
  - Add A500 Rev 6A 1 MB Chip RAM with Agnus 8372A, then A500 Plus with ECS Agnus/Denise, Productivity modes, Kickstart 2.04, and battery-backed RTC.
  - **Gate:** Enable and verify the ECS suites deferred by Step 5.1, including `_ecs.raw`, `_plus.raw`, SuperHires, and expanded Agnus registers.

- **11.2: AGA, Later CPUs, and Optional Coprocessors**
  - Add A1200 support with 68EC020, 2 MB Chip RAM, Alice/Lisa, eight bitplanes, and a 24-bit palette. Treat 68010 and FPU support as separate scope decisions tied to their deferred suites.
  - **Gate:** Enable and verify the relevant later-CPU, AGA, and selected coprocessor suites from Step 5.1 when their hardware prerequisites exist.

- **11.3: Peripheral Extensions**
  - Add a parallel-port four-player joystick adapter, analog potentiometer sampling through POT0DAT/POT1DAT, and light-pen/gun beam-position latching.
  - **Gate:** Device behavior and integration tests verify the selected extension against its hardware specification.

- **11.4: Resource Extractor and Reverse Engineering Assistant**
  - Extract bitplanes, sprites, palettes, and audio from guest memory; annotate assets, addresses, and game phases with optional LLM assistance and map them in the 24-bit address space.
  - **Gate:** Asset extraction unit tests in `crates/debugger/tests/` verify output and source-address provenance.
