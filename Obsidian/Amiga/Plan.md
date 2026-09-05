
This plan describe what should I do to reach goal to Amiga emulator.

## BLEPGEN

Rewrite it to Rest. Compare Blep curves.

## A500

### We need to start with CPU

Write a cpu related code
Need to go cycle by cycle
Write for one instruction from each group
Run tests agains it from SingleStepTests
Review architecture
Run agents to write rest of cpu code, use SingleStepTests in loop
Refine agentic loop with skills, mcp, subagents, especially for review
Run tests agains Winuae test
Run tests agains vamiga test
Run tests againt  other(?) test
Note: we need to look for lock, so we cannot access memory

I saw some comments under other cpu implementations about special cases. 
Not sure if winuae and SingleStepTests should cover them.
But we can try to run some investigation with agent if our cpu logic need to be refined futher

use egui + light webasm front

allow to prepare webasm bundles

stack
prefetch after reset

## Debugger

- disasm - should be done in react front
- breakpoints - on memory address instruction, on read/write, condition
- debug steo
- historical data navigation
- memory view with explaination
	- search, dump range, copy/paste, clear, set etc
- registry view with explaination
- use egui + light webasm front

## Resource extractor

Extract graph and audio, describe it with llm help
note address, and "phase/disk"
show it also on memory map, current sprites, current blitter etc

## Decompiler

Use resource extractor to give meaningful names to variables
Use LLM to decompile code (any language)
Allow to debug asm/decompiled code in sync
## Hardware Roadmap & Milestones

- **Phase 1 (Immediate Focus):** Standard **Amiga 500 (Rev 5 / Rev 6a OCS)**
  - 68000 CPU cycle-exact
  - 512 KB Chip RAM + optional 512 KB Trapdoor Slow RAM
  - OCS Agnus (8370/8371) & OCS Denise (8362)
  - Standard dual game ports (Mouse on Port 1, Joystick on Port 2)
  - 1x Floppy Drive (DF0) with ADF byte slice injection

- **Phase 2 (Later Models):**
  - **A500 Rev 6A / OCS 1MB:** 1MB Chip RAM configuration with Agnus 8372A in OCS mode.
  - **A500+ (Plus):** Full ECS chipset (Agnus 8372A 1MB, Denise 8373), Kickstart 2.04, battery-backed RTC.
  - **A1200:** AGA chipset (Alice, Lisa), 68EC020 CPU (32-bit), 2MB Chip RAM, 24-bit palette.

- **Peripheral Enhancements (Later):**
  - **4-Joystick Adapter:** Parallel port 4-player adapter (e.g. for games like Super Skidmarks, Dynablaster).
  - **Analog Joysticks:** Proportional analog stick reading via Paula/Denise pot pins (`POT0DAT`/`POT1DAT`).
## A1200

Later

## Other models

Later