# Mixed Game Scene

A standalone 68000 program keeps CPU, Chip RAM and the display/audio DMA engines
working together. The same `program.bin` runs in the GUI and in the native
benchmark. It needs no Kickstart, disk image or operating system.

The display contains moving columns of colored sprite emblems over scrolling
bitplanes and raster palette changes. Visual fidelity is still exploratory.
The integration test checks activity and reproducibility; it does not certify
the image against a physical Amiga or establish compatibility with real games.

## Run in the GUI

From the repository root:

```powershell
cargo run --release -p gui --bin amiga-studio -- --game --load crates/machine_loop/tests/programs/display/mixed_scene/program.bin --addr 1000
```

Press **Run**. Initialization takes approximately eight emulated PAL fields;
allow it to finish before judging the image. The program loops indefinitely.
To load into an already running GUI, reset first, then load `program.bin` at
`$001000`, set the PC through the loader, and Run. PAL and NTSC are supported.

The program initializes its stack and interrupt vectors and controls all custom
registers itself. Host keyboard/mouse mappings and speaker output are not added
by this scene. The benchmark supplies synthetic connector events; the GUI runs
the same autonomous guest without those extra events.

An offscreen GUI capture of the program after 64 fields is available through:

```powershell
python tools/verification/verify.py runner gui -- --scenario mixed_scene --output .agent/tmp/mixed-scene/gui.png
```

## Measure or Inspect Activity

```powershell
cargo run --release -p machine_loop --example mixed_scene -- audit --output .agent/tmp/mixed-scene/audit
cargo run --release -p machine_loop --example mixed_scene -- bench --output .agent/tmp/mixed-scene/bench
```

Use `--ntsc` for NTSC. `--warmup N`, `--fields N`, and `--series N` select the
measurement interval; `--help` explains defaults. Audit uses one series. Both
modes run 32 warmup fields by default; audit observes the next 32, while bench
measures three series of 120. Fields are counted by the normal Agnus raster wrap,
as in `step_frame`, with the actual CCK count reported. They are not host repaints.

Each timed series restores the same complete warmed state and replays identical
input events. Loading, state capture/restore, comparison, reporting and image
export are outside the timer. All series must end in the same complete machine
state. Timed execution includes normal emulation, pixel generation, deterministic
input delivery and draining digital audio after each line. It excludes GUI,
texture conversion/upload, pacing, speakers and the detailed activity observer.

JSON results and a final full-frame PPM image go to the selected output directory.
No output directory means console results only. Record the source changes,
compiler/build settings and executable hash when comparing revisions. Keep
workload, video standard, warmup and field count identical; compare guest progress
as well as elapsed time. These runs create no approved performance baseline and
impose no timing pass/fail threshold.

Audit reports instructions completed, DMA owner selections, Blitter busy CCKs,
sprite output, collisions, keyboard edges and nonzero stereo samples. The CPU
owner category includes unclaimed opportunities: it is not an exact count of CPU
memory transfers. Guest counters separately record frame updates, computation
loops, blits started, keyboard bytes and CIA/audio interrupts. Repeated blit
starts require the preceding blit to complete. Register enable bits alone do not
demonstrate activity.

## Workload

- Five low-resolution bitplanes use a 64-byte row stride with fine/coarse scroll.
  Standard data fetch starts after sprite 7's DMA slots. An earlier wide-fetch
  pilot starved that sprite and was corrected in the guest program.
- Eight 96-line sprites move, overlap and collide with each other and playfields.
  The last pair uses attachment. All sprite streams are reloaded every field.
- Copper reloads display pointers and performs three palette writes on each of
  200 visible lines. Its waits and transfers compete through ordinary arbitration.
- The Blitter repeatedly combines three source planes into the fifth plane with
  four-channel area operations. Normal Blitter priority allows CPU progress.
- CPU arithmetic, multiplication, rotating/XOR operations and Chip RAM stores
  continue between Blitter launches and interrupts. There is no STOP idle loop.
- Four audio DMA channels play fixed sample streams at different periods. The
  runner consumes digital samples each line; it does not resample or play sound.
- CIA timers run, and guest handlers process VBlank, audio and CIA-A interrupts.
  Keyboard bytes travel through the modeled serial pins and guest acknowledgments.
- The host moves the mouse every 16 lines, changes joystick contacts/buttons at
  field boundaries and alternates a key press/release every eight fields.
  It uses device APIs, not injected JOY/SDR register values. Input scheduling
  depends on emulated coordinates, never wall time.
- Disk DMA remains disabled. Busy engines still have internal work/wait phases;
  a useful scene does not require every bus opportunity to be occupied.

## Rebuild and Check

Use the assembler provisioned by `tools/bootstrap/assembler-manifest.json`.
For its pinned Windows release:

```powershell
.\tools\m68k-asm\v2.4.6\x86_64-pc-windows-msvc\m68k-asm.exe crates/machine_loop/tests/programs/display/mixed_scene/program.as --origin 0x1000 -o crates/machine_loop/tests/programs/display/mixed_scene/program.bin
python tools/verification/verify.py test rust crate machine_loop suite test_display_program
```

The focused test runs PAL and NTSC, checks simultaneous activity and changing
video/input, then restores the initial state and compares the entire result of
the normal runner with the instrumented audit. It contains no timing assertions
or golden image hashes. The scene and its bootstrap are guest code; no emulator
timing or rendering behavior was changed to accommodate it.

`host.rs` is shared by the integration test and the native example. Scene state
and counters live at `$8000`, Copper at `$10000`, planes at `$20000..$33fff`,
sprites at `$38000..$3bfff`, audio at `$3e000`, CPU scratch at `$50000`, and the
supervisor stack starts at `$7f000`. See `program.as` for exact register values
and the header/data formats derived from HRM chapters 2-8.
