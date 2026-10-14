---
title: "Machine Loop"
aliases: ["A500 Machine Loop", "Main Machine Loop", "machine_loop"]
tags: ["amiga", "design", "machine-loop", "coordination"]
category: "Design"
subsystem: "machine_loop"
status: "active"
created: 2026-08-31
updated: 2026-10-12
related: ["[General Architecture.md](General%20Architecture.md)", "[MemoryBus.md](MemoryBus.md)", "[DMA.md](DMA.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)"]
tracked_paths:
  - "crates/machine_loop"
---
# Machine Loop

[`A500Machine`](../../../crates/machine_loop/src/machine_loop.rs) is the top-level owner and clock coordinator of the emulated Amiga 500. Its main loop advances the machine one Color Clock (CCK) at a time. It also carries outputs between components that do not directly own or call one another: bus data, interrupt requests, reset lines, peripheral pins, and other modeled hardware signals. The loop determines when those connections are sampled and delivered; each component implements its own internal behavior.

The machine owns the CPU, physical memory, Agnus, Denise, Paula, both CIAs, the RTC, floppy controller, keyboard, and game ports. Its `cck` field counts executed Color Clock steps. [`MemoryBus`](MemoryBus.md) is a temporary router over these owned components for CPU accesses and Copper custom-register writes. [Agnus](Agnus.md) determines Chip RAM contention and DMA activity; the slot schedule and arbitration belong to [DMA.md](DMA.md), not to this coordinator.

## One Color Clock

`A500Machine::step_cck()` is the common path used by the stepping APIs. It performs these operations in order:

1. Increment the wrapping `cck` counter and call `step_subsystems_cck()`.
2. If the keyboard asserts its reset line, perform a warm reset and return `false` without stepping the CPU in that call.
3. Otherwise, record the CPU's current `RESET` output, create a `MemoryBus` over the machine components, and advance the CPU by one CCK phase.
4. If that CPU phase newly asserted `RESET`, reset external devices while preserving the CPU's registers and program counter.

The return value says whether the CPU completed an instruction during this step. The non-CPU components advance even when the CPU is waiting or stopped, subject to the keyboard reset path above. `step_cycles`, `step_instruction`, `step_line`, and `step_frame` all repeat `step_cck()` until their respective count, CPU instruction, Agnus line transition, or Agnus frame transition condition is met. These are ways to drive the same machine loop, not separate execution models.

## Connections advanced within the step

`step_subsystems_cck()` transfers the currently modeled signals in a fixed order:

1. It presents the disk DMA request, derived from Paula's active disk DMA and a word available in the floppy controller, to Agnus. Agnus then advances its beam, Copper, Blitter, and DMA against Chip RAM. The loop sends a pending Copper register write through `MemoryBus`, latches a bitplane DMA word into Denise, and handles an available disk read DMA slot by moving a floppy word into Chip RAM. It copies Agnus's Chip RAM blocked state to physical memory for bus accesses. Agnus's Blitter completion and vertical blank outputs request Paula interrupts; vertical blank also ticks CIA-A's time-of-day clock.
2. It takes Agnus's beam position for Denise's video step. At horizontal position zero, it ticks CIA-B's time-of-day clock.
3. It advances Paula and the floppy controller. Floppy sync and completed disk blocks request Paula interrupts. An audio channel restart reloads the corresponding Agnus DMA pointer and requests that channel's Paula interrupt.
4. It advances keyboard transmission into CIA-A, then both CIAs. Pending CIA-A and CIA-B interrupt outputs feed Paula's INT2 and INT6 inputs, respectively.
5. It advances the RTC. A CIA-A overlay transition changes whether low memory maps to Chip RAM or Kickstart ROM. It then samples peripheral connections and copies Paula's pending interrupt level to the CPU's `ipl` input.

The peripheral sampling step carries CIA-B drive controls to the floppy controller, floppy status to CIA-A, a floppy byte to Paula, game-port fire inputs to CIA-A, digital position counters to Denise, and potentiometer values to Paula. The machine also samples these connections after construction and resets. See the [cross-chip signal catalog](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md) for the broader signal inventory; the order above describes what this implementation actually dispatches per CCK.

## Lifecycle around the loop

Construction creates the components from `A500Config`, resets the CPU against physical memory, samples peripheral inputs, and initializes the CPU interrupt level from Paula. A cold `reset()` resets physical memory, the master counter, chips, and devices before resetting the CPU. `reset_warm()` preserves RAM through the physical-memory warm-reset path and restarts the CPU and devices with the low-memory overlay restored. A CPU `RESET` instruction takes a narrower path: `reset_external_devices()` re-engages the Kickstart overlay and resets external chips and devices without clearing RAM or resetting CPU execution state. These are distinct operations in [`machine_loop.rs`](../../../crates/machine_loop/src/machine_loop.rs).

The machine also owns whole-machine snapshots: `save_state()` captures the clock, configuration, CPU, memory, chips, and peripherals, together with a frame-buffer screenshot; `load_state()` checks the snapshot version, restores those components, and samples peripheral pins again. Snapshot format and persistence are described in [SaveState.md](Application/SaveState.md). Video buffer production belongs to [Denise.md](Denise.md) and [Frame Buffer.md](Frame%20Buffer.md).

## Reference Documentation & Upstream Ground Truth

- [`machine_loop.rs`](../../../crates/machine_loop/src/machine_loop.rs): Source of component ownership, CCK ordering, signal dispatch, resets, and stepping APIs described here.
- [`save_state.rs`](../../../crates/machine_loop/src/save_state.rs): Defines the machine snapshot and versioned serialization contract.
- [General Architecture.md](General%20Architecture.md): Places the coordinator within the machine's component structure.
- [MemoryBus.md](MemoryBus.md): Describes address routing and bus access between the CPU and connected devices.
- [DMA.md](DMA.md): Owns the Agnus transfer schedule and Chip RAM arbitration details.
- [Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md): Records the wider set of modeled inter-chip connections.
