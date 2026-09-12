# BOOTSTRAP: Autonomous Amiga Emulator Reconstruction & Clean-Room Pipeline

This document defines the strategic recipe and minimal instructional framework required to execute **Section 8 of [DIARY.md](DIARY.md)** (*"The Ultimate Goal: The Clean-Room Re-Generation Experiment"*): enabling an autonomous AI agent to independently rebuild the cycle-exact Amiga 500 emulator from scratch.

---

## 1. Automated Ingestion, Downloading & Preprocessing Pipeline
- [ ] Centralize web links for all required external assets: documentation, test suites, and any indispensable external sources.
- [ ] Create specialized agent workflows or scripts to autonomously fetch resources from network links on demand (avoiding storing copyrighted materials or heavyweight files directly in repo).
- [ ] Implement automated preprocessing stages:
  - [ ] Convert online documentation into clean, readable Markdown (`.md`), vector diagrams (`.svg`), and images (`.png`).
  - [ ] Unpack and curate CPU test suites (e.g. extracting SingleStepTests JSON archives, pruning redundant test files).
  - [ ] Execute pre-compilation or generation steps on external tools/sources where required (e.g. compiling tools to generate CPU opcode tables or test binaries).
- [ ] If specific external sources are genuinely required during development or generation, explicitly document them along with precise fetch, compilation, and integration instructions.

## 2. Prompt & Skill System Refinement (Autonomous Emulator Generation)
- [ ] System goal: build an agent system that can construct the Amiga emulator autonomously from prompts, skills, and documentation.
- [ ] Refine and finalize all prompts: skills (`.agents/skills/`), rules (`.agents/rules/`), workflows (`.agents/workflows/`), and IDE configurations.

## 3. Documentation De-duplication & Knowledge Pruning
- [ ] Remove redundant knowledge from current design documentation (`Obsidian/Amiga/Design/`).
- [ ] Strip out information that is already documented in official Amiga reference books and can be retrieved directly from primary manuals.
- [ ] Keep only architectural decisions, emulator state models, timing specifications, and documented bug fixes.

## 4. Reference Code Clean-Room Wipe & Regeneration Experiment
- [ ] Remove reference emulator sources (`ref_src/`), keeping strictly:
  - SingleStepTests (M68000 test vectors).
  - vAmigaTS (Amiga hardware test suite disks).
- [ ] Restrict the agent from looking at reference emulator sources, requiring it to learn and verify against the test suites.
- [ ] Experiment: wipe the written emulator source code and attempt to regenerate the entire emulator from scratch using only the refined prompts, skills, curated documentation, and test suites to see if it is possible.

## 5. Differential Testing & Final Cleanup
- [ ] Write instructions / evaluate side-by-side differential execution against WinUAE (referenced in roadmap).
- [ ] Delete `ROADMAP.md` at the end of the project once all milestones and this pipeline are established.
- [ ] Further minimize documentation volume, eliminating any redundant public knowledge that an agent can discover online.
