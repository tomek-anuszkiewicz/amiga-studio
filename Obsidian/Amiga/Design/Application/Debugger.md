---
title: "Amiga 500 Debugger"
aliases: ["Debugger", "Disassembler", "Inspection Engine"]
tags: ["amiga", "design", "debugger", "disassembler", "tracing"]
category: "Design"
subsystem: "debugger"
status: "active"
created: 2026-09-05
updated: 2026-10-12
related: ["[General Architecture.md](../General%20Architecture.md)", "[Machine Loop.md](../Machine%20Loop.md)", "[GUI.md](GUI.md)", "[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)"]
tracked_paths:
  - "crates/debugger"
---
# Amiga 500 Debugger

The debugger lets a developer pause the emulated machine, inspect its state, and follow a small M68000 program as it executes. A headless debugging session owns the [machine](../Machine%20Loop.md) and its execution controls; the [Developer Studio GUI](GUI.md) presents the registers, memory, disassembly, breakpoints, and history. The debugger reads machine state without advancing emulation during inspection.

## Controlling execution

Execution can run continuously, pause, advance by one M68000 instruction, or advance by one Color Clock step. Continuous execution stops before an instruction whose program counter matches an enabled breakpoint, or when the CPU halts or stops. Instruction stepping executes the next instruction even when a breakpoint is set at its address. The GUI exposes run/pause and both stepping controls; see the [Debugger Guide](../../../../docs/debugger.md) for shortcuts.

## Inspecting code and memory

The register view shows the current CPU state. The memory viewer lets the developer navigate to an address, inspect bytes in hex and ASCII, search memory, and edit bytes. Disassembly decodes words from the selected address into M68000 instructions, highlights the current instruction, and supports navigation through surrounding code. An instruction can also be replaced in place with assembled text when the replacement has the same byte length. Inspection reads do not cause guest bus side effects.

## Stopping at a location

A breakpoint can be added from a disassembly row or the breakpoint panel. It targets a program-counter address and can optionally require a CPU register condition; the panel can enable, disable, or remove it. The UI also lets developers configure and highlight read/write watchpoint ranges in memory. Watchpoint matching exists in the debugger backend, but guest memory accesses do not currently invoke it to pause execution.

## Following earlier execution

The execution trace records recent instruction addresses, opcodes, disassembly, CPU state, and Color Clock positions for inspection after a run or step. A separate, optional history records CPU snapshots before instructions and allows the developer to browse backward and forward through those recorded states. History recording starts disabled. Browsing history restores CPU state for inspection; it does not rewind memory or the rest of the machine.

## References

- [Debugger and session source](../../../../crates/debugger/src/session.rs): Machine ownership, execution control, trace recording, and history navigation.
- [Disassembler source](../../../../crates/disassembler/src/disassembler.rs): M68000 instruction decoding used by the debugger.
- [Developer Studio GUI](GUI.md): Placement of the debugger views in the application.
- [CPU specification](../CPU%20Motorola%20M68000.md): Meaning of the CPU state and instruction execution being inspected.
