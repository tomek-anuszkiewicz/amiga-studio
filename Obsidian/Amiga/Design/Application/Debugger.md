---
title: "Amiga 500 Debugger"
aliases: ["Debugger", "Disassembler", "Inspection Engine"]
tags: ["amiga", "design", "debugger", "disassembler", "tracing"]
category: "Design"
subsystem: "debugger"
status: "active"
created: 2026-09-05
updated: 2026-11-15
related: ["[General Architecture.md](../General%20Architecture.md)", "[Machine Loop.md](../Machine%20Loop.md)", "[GUI.md](GUI.md)", "[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)"]
tracked_paths:
  - "crates/debugger"
  - "crates/disassembler"
---
# Amiga 500 Debugger

Use the debugger to pause the emulated machine, inspect its state, and follow a
small M68000 program as it executes. The debugging session owns the machine
and its execution controls and can run without a window. Inspection reads the
machine's state without advancing emulation. [machine](../Machine%20Loop.md)

The GUI presents the session's registers, memory, disassembly, breakpoints,
and history. [Developer Studio GUI](GUI.md)

## Controlling execution

Run advances execution continuously. An ordinary pause finishes the current
video field, or stops immediately if the field is already complete. The
debugger uses Denise's frame counter to detect the boundary, which can occur
partway through a CPU instruction. Step Frame advances to the next completed
field through the same bounded execution path as Run and respects breakpoints.
An enabled breakpoint stops continuous execution before the instruction at its
address. A CPU halt also stops execution. Both stops take precedence over
completing the field.

Run and Step Frame keep chip clocks, video and interrupts advancing during CPU
STOP. While the CPU waits, the debugger records no instructions and cannot
trigger a breakpoint at the following instruction until the CPU resumes. An
explicit instruction step executes the next M68000 instruction even if a
breakpoint is set at its address. A Color Clock step advances one Color Clock.

The host runtime applies the GUI's selected speed and paces execution against
wall-clock time. The headless session advances emulated CCKs. A motherboard
keyboard reset ends the current slice, clears field-boundary and instruction
tracking, and lets the host owner begin a new input generation. Shortcuts are
listed in the [Debugger Guide](../../../../docs/debugger.md).

## Inspecting code and memory

When Game execution starts, the session disables debugger instrumentation.
The same machine CCK loop continues without breakpoint checks, instruction
disassembly, trace recording, recording of historical CPU states, CPU cloning,
memory diff capture or debugger counter updates. On return, the debugger
synchronizes inspection state with the current machine and leaves the
unrecorded interval absent from its history. Both views keep the machine
running during CPU STOP. Manual save/load remains explicit and independent of
history recording.

The register view shows the current CPU state. In the memory viewer, a
developer can jump to an address, read bytes in hex and ASCII, search memory,
and edit bytes. Inspection reads have no guest bus side effects.

The disassembly listing decodes words at the selected address as M68000
instructions and highlights the current instruction. The developer can browse
surrounding code. The decoder recognizes MOVEP and NBCD and checks whether
effective addresses are legal for the ALU, data, and control-flow forms it
covers. A rejected encoding appears as `DATA.W` and consumes one word,
keeping the listing aligned with the input stream.

The developer can replace an instruction in place by entering assembly that
produces the same number of bytes. Editing the PC, patching an instruction, or
restoring a historical CPU state restarts instruction tracking. On the next
Run, the debugger checks the breakpoint before executing the instruction.

## Stopping at a location

Add a breakpoint from a disassembly row or from the breakpoint panel. The
breakpoint targets a program-counter address and can require a CPU register
condition. Use the panel to enable, disable, or remove it. Developers can
configure and highlight read/write watchpoint ranges in memory. Stopping at
those watchpoints remains incomplete. [debugger work](../../../../ROADMAP.md#debugger-execution-control)

## Following earlier execution

During explicit instruction, CCK and frame steps, the execution trace records
recent instruction addresses, opcodes, disassembly, CPU state and Color Clock
positions. Developers can enable history to record CPU snapshots before those
instructions. They can then browse the recordings in either direction. Selecting
a snapshot restores the CPU state for inspection only. Memory and the rest of
the machine keep their current state. History starts disabled.

During continuous Run, the debugger checks enabled PC breakpoints and their
conditions against the current CPU state without disassembling instructions,
writing trace entries, recording snapshots for history, repeatedly cloning
CPU state or capturing memory diffs. An ordinary pause keeps this policy until
the field boundary. On Stop, existing entries remain available and the
unrecorded interval remains absent from the record.

Run, Game and explicit stepping use the same bounded CCK execution. The
session accumulates retired instruction counts at slice boundaries and
publishes them after stopping, independently of any recorded entries. When
execution stops, the debugger synchronizes its clocks and memory comparison
baselines with the current machine. When returning from Game, instrumentation
stays disconnected until the pending pause completes.

## References

- [Debugger and session source](../../../../crates/debugger/src/session.rs): Machine ownership, execution control, trace recording, and history navigation.
- [Disassembler source](../../../../crates/disassembler/src/disassembler.rs): M68000 instruction decoding used by the debugger.
- [Developer Studio GUI](GUI.md): Placement of the debugger views in the application.
- [CPU specification](../CPU%20Motorola%20M68000.md): Meaning of the CPU state and instruction execution being inspected.
