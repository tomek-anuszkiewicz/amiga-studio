# Amiga 500 Main Machine Loop & Subsystem Coordination

> [!NOTE]
> Ownership architecture and module decoupling principles are defined in [AGENTS.md](../../../AGENTS.md) and [General Architecture.md](General%20Architecture.md).

---

## 1. Machine Struct (`A500`)

The top-level `A500` struct owns and orchestrates all components without circular references:
- `cpu`: Motorola 68000 core
- `memory_bus`: 24-bit address decoder, RAM buffers, and Kickstart ROM storage
- `cycle_counter`: Master 64-bit Color Clock counter (CCK)
- `agnus`, `denise`, `paula`: Custom chipsets
- `cia_a`, `cia_b`: MOS 8520 Complex Interface Adapters

---

## 2. Stepping Capabilities & Master Clock Loop

The main loop provides three levels of stepping granularity:

1. **Single CCK Step (`step_cck`)**:
   - Advance `cycle_counter` by 1 CCK (~280 ns PAL / ~279 ns NTSC).
   - Clock custom chips (Agnus beam counter/Copper/Blitter, Denise pixel pipeline, Paula audio).
   - Clock CIAs (decrement Timer A & B, update TOD).
   - Clock CPU bus phase (**CCK1** or **CCK2**):
     - CPU queries `memory_bus.read_phase1()`, `read_phase2()`, `write_phase1()`, or `write_phase2()`.
     - If `MemoryBusResult::Blocked`, CPU holds current micro-step and inserts a wait state.
   - Run interrupt arbitration loop.
2. **Cycle Count Step (`step_cycles(cck_count: u64)`)**:
   - Executes a designated number of Color Clocks in a loop.
3. **Full Video Frame Step (`step_frame`)**:
   - Executes until Denise / Agnus completes a full vertical frame (VBlank transition).

---

## 3. Interrupt Arbitration Pipeline

On each CCK step, the main loop coordinates interrupt requests across chips:

```mermaid
flowchart TD
    PAULA["Paula (Levels 1, 3, 4, 5)"] -->|Pending Request Lines| MAIN_LOOP["A500 Main Loop"]
    CIAA["CIA-A (Level 2 PORTS)"] -->|Active Line| MAIN_LOOP
    CIAB["CIA-B (Level 6 EXTER)"] -->|Active Line| MAIN_LOOP

    MAIN_LOOP -->|Calculate Highest Unmasked Priority| RESOLVE["Resolve IPL (0..6)"]
    RESOLVE -->|cpu.set_ipl(level)| CPU["Motorola 68000 CPU"]
```

1. **Query Sources:**
   - **Paula:** Level 1 (`TBE`, `DSKBLK`, `SOFT`), Level 3 (`VERTB`, `BLIT`, `COPER`), Level 4 (`AUD0-3`), Level 5 (`RBF`, `DSKSYN`).
   - **CIA-A:** Level 2 (`PORTS`).
   - **CIA-B:** Level 6 (`EXTER`).
2. **Resolve Level:** Calculate the highest pending, unmasked interrupt priority level ($IPL \in 1..6$, or $0$ if none).
3. **Drive CPU Lines:** Call `cpu.set_ipl(resolved_level)`. The CPU samples `ipl` at the instruction microcode boundary against `SR` interrupt mask bits.

---

## 4. Reset Flows: Cold vs. Warm Reset

On real Amiga hardware, **all resets start CPU execution from address `$000000` via the Kickstart low-memory overlay (`_OVL`)**:

### 4.1 Cold / Hard Reset (`reset_cold`)
1. **MemoryBus:** Call `memory_bus.reset_cold()`. Zeroes all physical Chip RAM and Fast RAM buffers (`$00`) and engages low-memory overlay (`map_kickstart_to_low_memory()`).
2. **Specialized Chips:** Apply chip reset defaults:
   - Agnus: `DMACON = $0000` (DMA disabled), Copper halted.
   - Paula: `INTENA = $0000`, `INTREQ = $0000`, audio volumes set to `0`.
   - CIAs: `DDRA/B = $00`, `ICR = $00`, timers stopped.
3. **CPU:** Apply CPU reset:
   - `SR` set to `$2700` ($S=1, T=0, I=7$).
   - Read 32-bit initial `SSP` from `$000000` (routed to Kickstart ROM).
   - Read 32-bit initial `PC` from `$000004` (routed to Kickstart ROM).
   - Prime prefetch queue (`IR` and `IRC`).
4. **Execution:** CPU executes Kickstart entry point. Since RAM was zeroed, memory checksum checks fail, and Kickstart runs full cold initialization.

### 4.2 Warm Reset (`reset_warm`)
1. **MemoryBus:** Call `memory_bus.reset_warm()`. Leaves RAM contents completely intact and re-engages low-memory overlay (`map_kickstart_to_low_memory()`).
2. **Specialized Chips:** Apply chip reset lines (disable DMA, mask interrupts, mute audio, reset CIA port latches), keeping register states and memory undisturbed.
3. **CPU:**
   - Re-initialize `SR = $2700`.
   - Reload initial `SSP` from `$000000` and initial `PC` from `$000004`.
   - Prime prefetch queue.
4. **Kickstart Detection:**
   - The CPU starts execution at the Kickstart ROM entry point.
   - Kickstart code scans RAM for magic resident signatures (`KickTagPtr`, checksum, ExecBase pointers).
   - Because RAM was preserved, the checksum succeeds, and Kickstart executes a warm reboot (preserving resident modules, alert state, and fast-booting).

---

## 5. Host Interfaces & Persistence

- **Video Frame Retrieval:** Returns current frame buffer slice (`&[u32]` ARGB, $720 \times 576$ max PAL).
- **Audio Sample Retrieval:** Decouples stereo audio ring buffers (`&[i16]`).
- **Save State:** Serializes complete machine state via [SaveState.md](SaveState.md).