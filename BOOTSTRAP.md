# BOOTSTRAP: Autonomous Amiga Emulator Reconstruction & Clean-Room Pipeline

This document defines the strategic recipe and minimal instructional framework required to execute **Section 8 of [DIARY.md](DIARY.md)** (*"The Ultimate Goal: The Clean-Room Re-Generation Experiment"*): enabling an autonomous AI agent to independently rebuild the cycle-exact Amiga 500 emulator from scratch.

---

## 1. Automated Ingestion, Downloading & Preprocessing Pipeline
- [ ] Centralize web links for all required external assets: documentation, test suites, and any indispensable external sources.
- [ ] Create specialized agent workflows or scripts to autonomously fetch resources from network links on demand (avoiding storing copyrighted materials or heavyweight files directly in repo).
- [ ] Implement automated preprocessing stages:
  - [ ] Convert online documentation into clean, readable Markdown (`.md`), vector diagrams (`.svg`), and images (`.png`).
  - [ ] Unpack and curate CPU test suites (e.g. extracting SingleStepTests JSON archives, pruning redundant test files).
  - [ ] Execute pre-compilation or generation steps on external tools/sources where required (e.g. compiling tools to generate CPU opcode tables, precomputing BLEP synthesis tables, or assembling test binaries).

- [ ] If specific external sources are genuinely required during development or generation, explicitly document them along with precise fetch, compilation, and integration instructions.

## 2. Prompt & Skill System Refinement (Autonomous Emulator Generation)
- [ ] System goal: build an agent system that can construct the Amiga emulator autonomously from prompts, skills, and documentation.
- [ ] Refine and finalize all prompts: skills (`.agents/skills/`), rules (`.agents/rules/`), workflows (`.agents/workflows/`), and IDE configurations.
- [ ] **Iterative Documentation Skill Refinement & Ground-Truth Calibration (`pdf-to-markdown`):**
  - Refine and harden the documentation conversion skill (`.agents/skills/pdf-to-markdown/`) using primary hardware manuals as testbeds:
    - Commodore *Amiga Hardware Reference Manual* (Agnus, Denise, Paula, CIAs, custom chipset registers).
    - Motorola *M68000 User's Manual / Programmer's Reference Manual* (CPU architecture, instructions, bus cycles, exception processing).
  - Iteratively run the skill against these source PDFs, evaluating the generated output against our existing reference notes (`Obsidian/Amiga/Reference/`).
  - Continuously tune conversion prompts, table stitching, figure/diagram extraction, and layout heuristics until the agent-generated documentation achieves parity in structure, completeness, and technical fidelity with our existing curated documentation.


## 3. Documentation De-duplication & Knowledge Pruning
- [ ] Remove redundant knowledge from current design documentation (`Obsidian/Amiga/Design/`).
- [ ] Strip out information that is already documented in official Amiga reference books and can be retrieved directly from primary manuals.
- [ ] Keep only architectural decisions, emulator state models, timing specifications, and documented bug fixes.

## 4. Reference Code Clean-Room Wipe & Regeneration Experiment
- [ ] Remove reference emulator sources (`ref_src/`), keeping strictly:
  - SingleStepTests (M68000 test vectors).
  - vAmigaTS (Amiga hardware test suite disks).
- [ ] Restrict the agent from looking at reference emulator sources, requiring it to learn and verify against the test suites.
- [ ] **Iterative Closed-Loop Calibration (Design Documentation vs. Generated Code Parity):**
  - Autonomous regeneration is not a naive single-shot process (wiping the repository and expecting an instant, perfect emulator). It operates as a disciplined, iterative feedback loop:
    - Prompt the agent to generate modules and subsystems strictly from the curated design specifications (`Obsidian/Amiga/Design/`).
    - Verify whether the generated sources and test suites achieve parity with our proven baseline:
      - 100% passing test suites (SingleStepTests, DMA contention, architecture rules, headless GUI tests).
      - Perfect compatibility and numerical parity with golden benchmark result baselines (`m68k_benchmark_baseline.csv`, `m68k_benchmark_baseline.json`, cryptographic SHA256 anti-tamper hashes).
    - Iteratively refine, sharpen, and calibrate the design documentation—clarifying state invariants, cycle phase models, and edge cases—until the documentation reliably guides the agent to generate code identical in quality, structure, and behavior to our existing codebase.
- [ ] **Pilot Subsystem Testbed: BLEP Synthesis & Analog Audio Filter Generator Experiment:**
  - Create targeted design documentation modeling the physical Amiga 500 audio output circuitry: operational amplifier (op-amp) stages, low-pass filter networks (passive RC and active Sallen-Key low-pass filter), dynamic CIA-A LED filter switching, and raw component values (resistors, capacitors).
  - Provide mathematical requirements for band-limited step (BLEP) anti-aliasing audio synthesis without including precomputed lookup tables or reference DSP code.
  - Instruct the agent to independently implement the BLEP generator, calculate filter responses, and generate the Paula audio filtering and table synthesis code.
  - Iterate on the design documentation and prompt specifications until the generated BLEP tables and audio filter implementation achieve parity with our verified baseline (`blep_tables.rs` and Paula audio engine).
- [ ] **Clean-Room Wipe & Autonomous Regeneration Trial:**
  - With prompts, skills, and design documentation fully calibrated in the loop, wipe the written emulator source code and autonomously regenerate the emulator from scratch using only the refined prompts, skills, calibrated documentation, and test suites.



## 5. Differential Testing & Final Cleanup
- [ ] Write instructions / evaluate side-by-side differential execution against WinUAE (referenced in roadmap).
- [ ] Delete `ROADMAP.md` at the end of the project once all milestones and this pipeline are established.
- [ ] Further minimize documentation volume, eliminating any redundant public knowledge that an agent can discover online.
