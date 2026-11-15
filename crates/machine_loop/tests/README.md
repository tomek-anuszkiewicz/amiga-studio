# Native disk boot checks

The ordinary machine suites use synthetic ROMs or independently expected raw
words. `test_floppy_media_dma` uses an inserted standard ADF and production drive,
Paula, DMAL, Agnus and memory paths; it supplies no host read pulses.

`test_native_disk_boot` contains two explicitly ignored asset-dependent checks.
They must be invoked with `--ignored`: an ordinary test run does not execute or
certify them. Missing private inputs fail the selected check. No ROM is included.
Set `AMIGA_KICKSTART_ROM` to an unmodified 256 KiB Kickstart 1.3 rev 34.5 image.
The test checks its version, checksum and real reset entry. From the repository root:

```powershell
$env:AMIGA_KICKSTART_ROM = '<private-rom-path>'
cargo test -p machine_loop --release --test test_native_disk_boot cross1_boots -- --ignored --nocapture

$env:AMIGA_WORKBENCH_ADF = '<private-workbench-adf-path>'
cargo test -p machine_loop --release --test test_native_disk_boot workbench13 -- --ignored --nocapture
```

The first check reads the original `ref_src/vAmigaTS/Agnus/Copper/cross/cross1/cross1.adf`.
Its custom bootblock reads 1,024 payload bytes at disk offset 1,024 to `$70000`
and jumps there. Source `include/ministartup.i` opens `graphics.library`, calls
LoadView and WaitTOF, installs its VBI and calls MAIN. Binary offsets 1,462 and
1,556 identify MAIN's continuing loop and Copper list. The test compares those
bytes after loading and checks all 83 COLOR00 operations in list order, actual
committed palette changes and healthy CPU execution over multiple full fields.
Expected data comes from the original binary/source, never an emulator capture.
Two clean starts must agree; a 30-second no-disk control must lack the signature.
The positive budget is 30 emulated seconds plus 0.1 second of post-entry observation,
within that run. Tests execute unthrottled; host time is not guest time.

The separate Workbench check runs for 60 emulated seconds with write protection.
It inspects the executed bootblock, `dos.library` and named Workbench task through
read-only guest state. ExecBase lists use the layout in
`ref_src/vAmiga/Core/Misc/OSDebugger/OSDebuggerTypes.h` (ThisTask 276, LibList 378,
TaskReady 406 and TaskWait 420; Node name pointer 10). It saves a PPM frame under
`.agent/tmp/native-disk/`. Passing this check establishes those startup stages,
not a correct desktop image. Inspect the frame independently before claiming
screen/window correctness. Broad image verification remains Roadmap 2.6.

## Recorded input identity and evidence

Task `NATIVE-DISK-BRING-UP-20261115`, initial repository revision
`e1ed94601d4aac2e5f675997b2baca7241dc4fd2`, uses standard PAL 1 MB A500:
OCS Agnus 8371, Denise 8362, Paula 8364, 512 KiB Chip RAM and 512 KiB Slow RAM
at `$C00000`, with no Fast RAM. All runs start with zeroed RAM and real ROM vectors.

| Input | SHA-256 |
| --- | --- |
| Kickstart 1.3 rev 34.5, 262,144 bytes | `ee05862d8102a08436ac4056da7d549db31625c7d47b24dfb7b3c9a5c113ca53` |
| Original cross1 ADF, 901,120 bytes | `133a0a489ccb241fe4011e5e1f6d9d5641dd31c4e8c05620290ee57a0769838a` |
| Supplied Workbench 1.3 ADF, 901,120 bytes | `097d332a6443be8339ef686aea49be578988e6f0131eb6e6d36a47f246c9cae8` |

Both bootblocks and the ROM have end-around checksum `$FFFFFFFF`. The Workbench
image contains an OFS volume `Workbench1.3`; its actual `s/Startup-Sequence` runs
SetPatch, FastMemFirst, BindDrivers, SetClock and FF, starts `s/StartupII`, waits
for its signal, then runs SetMap, LoadWB delay and EndCLI. StartupII installs
resident commands and RAM assignments and signals the first process with `break 1 C`.
No companion disk or replacement startup was supplied.

The original implementation never reached cross1's body within 30 seconds.
After autonomous streaming, both clean runs observed overlay removal at CCK
1,179,707, the first disk transfer at 13,628,637, bootblock execution at 14,580,595,
payload startup at 14,598,254 and body entry at 14,752,752 (about 4.16 seconds).
The following 0.1 second included five field boundaries and 70,054 loop retirements;
complete Copper sequences and committed colors matched the original list.
The no-disk control ran 106,406,850 CCK without that signature.

The Workbench attempt observed bootblock execution at CCK 14,590,039,
DOS at 14,896,939 and the Workbench task at 204,655,554, within the 212,813,700-CCK
budget. The saved frame was visibly corrupted; a correct desktop was not confirmed.
This is evidence of disk/filesystem and selected OS startup, not universal AmigaDOS
support. Media writes and host image persistence remain unverified.
