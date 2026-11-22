# Mixed Game Scene

The GUI and the native benchmark both run `program.bin`, a standalone 68000
program that exercises the CPU, Chip RAM, and the display and audio DMA engines
together. It runs without Kickstart, a disk image, or an operating system.

The 320 x 256 PAL display shows moving columns of colored sprite emblems over
scrolling bitplanes, with palette changes along the raster. The scene starts at
raster line 40, leaving 14 lines above and 15 below in the current non-interlaced
PAL image. Copper bars also cover the surrounding border. The Copper writes the
palette during horizontal blanking to keep each bar aligned. The native example
reports which parts of the machine are active and whether runs reproduce the same
state. Visual fidelity remains exploratory. Comparison with a physical Amiga and
compatibility with real games require separate evidence.

## Run in the GUI

Start the GUI from the repository root.

```powershell
cargo run --release -p gui --bin amiga-studio -- --game --load crates/machine_loop/tests/programs/display/mixed_scene/program.bin --addr 1000
```

Press **Run** and wait approximately eight emulated PAL fields for initialization
before judging the image. The program then loops indefinitely. In an already
running GUI, reset first. Load `program.bin` at `$001000`, set the PC through the
loader, and select Run. The full scene requires PAL. NTSC runs the workload with
the bottom of the scene clipped because its fields are shorter.

The program initializes its own stack and interrupt vectors and controls all
custom registers. The scene runs without adding host keyboard and mouse mappings
or speaker output. The guest runs autonomously in the GUI. The benchmark runs
the same guest and supplies synthetic connector events.

The following command captures the program offscreen in the GUI after 64 fields.

```powershell
python tools/verification/verify.py runner gui -- --scenario mixed_scene --output .agent/tmp/mixed-scene/gui.png
```

## Measure or Inspect Activity

```powershell
cargo run --release -p machine_loop --example mixed_scene -- audit --output .agent/tmp/mixed-scene/audit
python tools/verification/run_mixed_scene.py --output .agent/tmp/mixed-scene/bench
```

Use `--ntsc` for NTSC. Set the measurement interval with `--warmup N`, `--fields N`,
and `--series N`. The `--help` flag explains the defaults. Both modes start with
32 warmup fields by default. Audit then observes one series of 32 fields. Bench
measures three series of 120 fields. Both modes count fields when the Agnus
raster wraps, as in `step_frame`, and report the actual CCK count. Host repaints
are outside the field count.

Before each timed series, the runner restores the same complete state saved
after warmup.
During the series, it replays identical input events. All series must end in the
same complete machine state. The timer covers normal emulation, pixel generation,
deterministic input delivery, and draining digital audio after each line. Loading,
state capture and restore, comparison, reporting, and image export happen outside
the timer. The timing also excludes GUI work, texture conversion and upload,
pacing, speakers, and the detailed activity observer.

Select an output directory to receive JSON results and a final full-frame PPM
image. With no output directory, results go only to the console. When comparing
revisions, record the source changes, compiler and build settings, and executable
hash. Use the same workload, video standard, warmup, and field count. Compare guest
progress as well as elapsed time. An approved performance baseline and a timing
pass/fail threshold remain outside the scope of these runs.

Audit reports completed instructions, DMA owner selections, Blitter busy CCKs,
sprite output, collisions, keyboard edges, and nonzero stereo samples. The CPU
owner category includes unclaimed bus opportunities, so it cannot give an exact
count of CPU memory transfers. Guest counters track frame updates, computation
loops, blits started, keyboard bytes, and CIA and audio interrupts. Each repeated
blit start requires the previous blit to complete. To demonstrate activity, use
observed events or counters, since register enable bits alone are insufficient.

## Workload

- Five low-resolution bitplanes use a 64-byte row stride with fine/coarse scroll.
  Standard data fetch starts after sprite 7's DMA slots. An earlier wide-fetch
  pilot starved that sprite and was corrected in the guest program.
- Eight 96-line sprites move, overlap and collide with each other and playfields.
  The last pair uses attachment. All sprite streams are reloaded every field.
- Copper reloads display pointers and performs three palette writes during
  horizontal blanking on lines 26 through 311, covering the visible PAL border.
  An extra wait at the end of line 255 handles the vertical comparison wrap.
  Its transfers compete through ordinary arbitration.
- The Blitter repeatedly combines three source planes into the fifth plane with
  four-channel area operations. Normal Blitter priority allows CPU progress.
- CPU arithmetic, multiplication, rotating/XOR operations and Chip RAM stores
  continue between Blitter launches and interrupts. There is no STOP idle loop.
- Four audio DMA channels play fixed sample streams at different periods. The
  runner consumes digital samples each line without resampling or sound playback.
- CIA timers run, and guest handlers process VBlank, audio and CIA-A interrupts.
  Keyboard bytes travel through the modeled serial pins and guest acknowledgments.
- The host moves the mouse every 16 lines, changes joystick contacts/buttons at
  field boundaries and alternates a key press/release every eight fields.
  It delivers those events through device APIs without injecting JOY/SDR register
  values. Emulated coordinates alone determine when input arrives.
- Disk DMA remains disabled. Busy engines have internal phases when they work or
  wait, so the scene can exercise them while leaving some bus opportunities free.

## Rebuild and Observe

Use the assembler provisioned by `tools/bootstrap/assembler-manifest.json`.
The following command uses its pinned Windows release.

```powershell
.\tools\m68k-asm\v2.4.6\x86_64-pc-windows-msvc\m68k-asm.exe crates/machine_loop/tests/programs/display/mixed_scene/program.as --origin 0x1000 -o crates/machine_loop/tests/programs/display/mixed_scene/program.bin
```

Use the native example described above to observe the scene manually and measure
FPS. The automated execution and assertion tests for this graphical program were
removed. The scene and its bootstrap remain guest code.

`host.rs` supports the native example. The scene stores state and counters at
`$8000`, Copper data at `$10000`, planes at `$20000..$33fff`, sprites at
`$38000..$3bfff`, audio at `$3e000`, and CPU scratch data at `$50000`. The
supervisor stack starts at `$7f000`. See `program.as` for exact register values
and the header and data formats derived from HRM chapters 2-8.
