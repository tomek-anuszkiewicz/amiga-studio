# Mixed Game Scene

`program.bin` exercises the 68000, Chip RAM and the display and audio DMA
engines together. The application boots that same program from
[`mixed_scene.adf`](mixed_scene.adf). The native benchmark loads the program
directly into Chip RAM and runs without firmware.

The 320 x 256 PAL display shows moving columns of colored sprite emblems over
scrolling bitplanes, with palette changes along the raster. The scene starts at
raster line 40, leaving 14 lines above and 15 below in the current non-interlaced
PAL image. Copper bars also cover the surrounding border. The Copper writes the
palette during horizontal blanking to keep each bar aligned. The native example
reports which parts of the machine are active and whether runs reproduce the same
state. Visual fidelity remains exploratory. Comparison with a physical Amiga and
compatibility with real games require separate evidence.

## Run in the GUI

Open **Manual Setup**, select a 256 KiB Kickstart 1.3 ROM from your own
collection and insert `mixed_scene.adf` in **DF0**. Use a PAL configuration
with at least 512 KiB Chip RAM. Click **Start**. Kickstart reads the disk
before the guest initializes the display. The moving sprites and scrolling
background then run continuously. An NTSC session clips the bottom.

The guest initializes its own stack and interrupt vectors and drives the
custom chips. In the GUI the scene runs without host input. The benchmark
adds synthetic connector events and drains digital audio.

To capture the guest image without starting the application or supplying
firmware, run the headless GUI inspector. It seeds a local session with the
raw program and captures the image after 64 fields.

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

### Run the FPS regression cases

From the repository root, supply your private 256 KiB Kickstart 1.3 ROM.

```powershell
python tools/verification/run_mixed_scene_fps.py --kickstart-rom "<path to Kickstart 1.3 ROM>"
```

The command builds release targets and runs the cases serially. It uses Python
with Playwright and the PowerShell, Trunk and nightly toolchain required by
[the browser build](../../tools/web.ps1). Use `--browser-executable PATH` to
select a local Chromium, Chrome or Edge executable. The default uses
Playwright's Chromium. Keep the desktop Game window or browser tab in the
foreground through warmup and timing. If the desktop Game window opens
in the background, bring it to the foreground before the launch times out.
Supply `--native-refresh-hz` with the actual monitor refresh rate if the
runner cannot query it.

| Case | What the command counts |
| --- | --- |
| FPS-01 | Completed emulated fields in the uncapped native machine loop |
| FPS-02 | Completed emulated fields in the uncapped machine loop in a browser worker |
| FPS-03 | Completed emulated fields in desktop Game at 4× |
| FPS-04 | Completed emulated fields in browser Game at 4× |

The uncapped cases embed `program.bin`, replay deterministic connector input
and drain digital audio after each line. They must produce matching guest
progress and emulated clock counts. Each Game case runs three fresh sessions
using Manual Setup with the external ROM and `mixed_scene.adf` in DF0.
Each session confirms guest progress and uses the ordinary Faster command
to reach 4×. Once the owner confirms 4×, the benchmark lets at least 32
more fields complete. It divides the change in the owner's completed-field
count by the actual elapsed time at the first sampling opportunity at or
after ten seconds. Host waits count toward elapsed time. The screen continues
to show the latest completed image at its normal presentation rate.

A valid Game sample of 190 FPS or higher fails that case before reference
comparison, even if its median is lower. The warning identifies the case,
sample and rate because it is approaching the 4× PAL playback limit of
about 200 FPS. Measuring higher throughput requires a decision about raising
the application's playback speed limit.

The command reports the sample counts, elapsed times and rates, the median
FPS, reference FPS, signed percentage change and verdict. It saves the
schema-2 report to `.agent/tmp/mixed-scene-fps/result.json` by default.
The report also records the Game field counts at warmup start and both
timing boundaries. Use `--output PATH` for a separate disposable result.
The command reads
`benchmarks/mixed_scene_fps_reference.json` and succeeds when all four cases
pass. Missing ROM input leaves the Game rows unverified. A missing or
incompatible reference leaves a measured row unverified.

The reference is a JSON object with `schema` set to `2` and `cases` keyed
by `FPS-01` through `FPS-04`. Each case records its approved
`reference_fps`, fractional `tolerance` (0.05 for 5%) and `compatibility`
settings from the result. Game compatibility includes the completed-field
metric and 4× speed, plus the boot mode and ROM and ADF SHA-256 digests.
The runner supplies the ROM at runtime and puts only its digest in the
result. The unrounded median passes from `reference_fps × (1 − tolerance)`
through `reference_fps × (1 + tolerance)`, including both endpoints.

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

## Rebuild the ADF

Use the assembler provisioned by `tools/bootstrap/assembler-manifest.json`.
From the repository root, build a temporary ADF and compare it with the
checked-in image. These commands use the pinned Windows release.

```powershell
$scene = 'tests/mixed_scene'
$assembler = 'tools/m68k-asm/v2.4.6/x86_64-pc-windows-msvc/m68k-asm.exe'
New-Item -ItemType Directory -Force .agent/tmp/mixed-scene | Out-Null
python "$scene/build_adf.py" --assembler $assembler --output .agent/tmp/mixed-scene/mixed_scene.adf
Get-FileHash "$scene/mixed_scene.adf", .agent/tmp/mixed-scene/mixed_scene.adf
```

The packager assembles [`boot.as`](boot.as), writes the checksum across the
first two sectors and copies the 1,268-byte `program.bin` to disk offset
`$500`. It fills the rest of the 880 KiB image with zeroes. Kickstart gives
the boot block a trackdisk request. The boot block reads the next three
sectors into reserved Chip RAM at `$74000` and returns a continuation. After
Kickstart closes the device, that continuation enters supervisor mode,
copies the program to `$1000` and jumps there. The disk contains the boot
loader and the scene, with no filesystem or firmware.
[Amiga floppy boot process](https://wiki.amigaos.net/wiki/Amiga_Floppy_Boot_Process_and_Physical_Layout)

To reassemble the original payload for comparison, build it at its existing
origin and compare its hash with `program.bin`.

```powershell
& $assembler "$scene/program.as" --origin 0x1000 -o .agent/tmp/mixed-scene/program.bin
Get-FileHash "$scene/program.bin", .agent/tmp/mixed-scene/program.bin
```

`host.rs` supports both machine-loop benchmarks. The scene stores state and
counters at `$8000`, Copper data at `$10000`, planes at `$20000..$33fff`, sprites at
`$38000..$3bfff`, audio at `$3e000`, and CPU scratch data at `$50000`. The
supervisor stack starts at `$7f000`. See `program.as` for exact register values
and the header and data formats derived from HRM chapters 2-8.
