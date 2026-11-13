---
title: "Amiga 500 Debugger"
aliases: ["Debugger", "Disassembler", "Inspection Engine"]
tags: ["amiga", "design", "debugger", "disassembler", "tracing"]
category: "Design"
subsystem: "debugger"
status: "active"
created: 2026-09-05
updated: 2026-11-13
related: ["[General Architecture.md](../General%20Architecture.md)", "[Machine Loop.md](../Machine%20Loop.md)", "[GUI.md](GUI.md)", "[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)"]
tracked_paths:
  - "crates/debugger"
  - "crates/disassembler"
---
# Amiga 500 Debugger

The debugger lets a developer pause the emulated machine, inspect its state, and follow a small M68000 program as it executes. A headless debugging session owns the [machine](../Machine%20Loop.md) and its execution controls; the [Developer Studio GUI](GUI.md) presents the registers, memory, disassembly, breakpoints, and history. The debugger reads machine state without advancing emulation during inspection.

## Controlling execution

Execution can run continuously, pause, advance by one M68000 instruction or Color Clock, or advance to the next completed video field with Step Frame. Ordinary pause completes the current field; pausing at an already completed field does not advance another one. Field completion follows Denise's frame counter, so stopping can occur within a CPU instruction. Frame stepping uses the same bounded execution path as Run and respects breakpoints. Continuous execution stops before an instruction whose program counter matches an enabled breakpoint, or when the CPU halts or stops; these debugger stops take precedence over completing the field. Instruction stepping executes the next instruction even when a breakpoint is set at its address. The GUI owns wall-clock pacing and speed selection; the headless session only advances emulated CCKs. See the [Debugger Guide](../../../../docs/debugger.md) for shortcuts.

## Inspecting code and memory

Game execution disables debugger instrumentation at the session boundary. It advances the same machine CCK loop without breakpoint checks, instruction disassembly, trace or temporal recording, CPU cloning, memory diff capture or debugger counter updates. Switching back synchronizes inspection state; it does not reconstruct the unrecorded interval. CPU STOP does not pause Game execution: chips continue until an interrupt can resume the CPU. Manual save/load remains explicit and independent of history recording.

The register view shows the current CPU state. The memory viewer lets the developer navigate to an address, inspect bytes in hex and ASCII, search memory, and edit bytes. Disassembly decodes words from the selected address into M68000 instructions, highlights the current instruction, and supports navigation through surrounding code. It recognizes MOVEP and NBCD and checks effective-address legality for the covered ALU, data, and control-flow forms. Rejected encodings are displayed as `DATA.W` and consume one word so inspection remains aligned with the input stream. An instruction can also be replaced in place with assembled text when the replacement has the same byte length. Replacing execution through PC editing, instruction patching or historical CPU restoration restarts instruction tracking, so the next Run checks its breakpoint before executing. Inspection reads do not cause guest bus side effects.

## Stopping at a location

A breakpoint can be added from a disassembly row or the breakpoint panel. It targets a program-counter address and can optionally require a CPU register condition; the panel can enable, disable, or remove it. The UI also lets developers configure and highlight read/write watchpoint ranges in memory. Watchpoint-triggered stops remain incomplete; see the [debugger work](../../../../ROADMAP.md#debugger-execution-control).

## Following earlier execution

The execution trace records recent instruction addresses, opcodes, disassembly, CPU state, and Color Clock positions for inspection after a run or step. A separate, optional history records CPU snapshots before instructions and allows the developer to browse backward and forward through those recorded states. History recording starts disabled. Browsing history restores CPU state for inspection; it does not rewind memory or the rest of the machine.

## References

- [Debugger and session source](../../../../crates/debugger/src/session.rs): Machine ownership, execution control, trace recording, and history navigation.
- [Disassembler source](../../../../crates/disassembler/src/disassembler.rs): M68000 instruction decoding used by the debugger.
- [Developer Studio GUI](GUI.md): Placement of the debugger views in the application.
- [CPU specification](../CPU%20Motorola%20M68000.md): Meaning of the CPU state and instruction execution being inspected.
