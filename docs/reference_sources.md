# External Reference Sources, Testbeds & Repositories Guide

This document catalogs all external reference source trees, physical hardware silicon test suites, and diagnostic media located under `ref_src/` and `tools/`. It specifies their upstream origins, pinned release versions, directory contents, and how they are provisioned and verified by the automated bootstrapper.

---

## 1. External Sources Overview & Pinned Versions

The emulator core validates execution against physical hardware silicon vectors, reference C++ emulators, and golden Amiga test suites. Because these test assets contain multi-gigabyte datasets (~6.5 GB uncompressed), they reside outside Git history in `ref_src/` and `tools/`, managed by automated provisioning scripts and shared across worktrees via zero-cost NTFS directory junctions.

### Pinned Upstream Sources Summary

| Repository / Asset | Local Path | Upstream Repository & URL | Pinned Version / Release | Primary Role in Emulator |
| :--- | :--- | :--- | :--- | :--- |
| **Tom Harte SingleStepTests** | `ref_src/SingleStepTests-680x0/` | [SingleStepTests/680x0](https://github.com/SingleStepTests/680x0) | **Format v1** (`68000/v1/`), 124 opcode suites | Exhaustive M68000 instruction-level verification captured from physical silicon pins (~1M test vectors) |
| **vAmiga C++ Emulator** | `ref_src/vAmiga-4.5/` (or `ref_src/vAmiga/`) | [dirkwhoffmann/vAmiga](https://github.com/dirkwhoffmann/vAmiga) | **Release v4.5** | Clean-room C++ reference implementation for custom chipset timing, bus cycles, and register behavior |
| **vAmiga Test Suite (vAmigaTS)** | `ref_src/vAmigaTS/` | [dirkwhoffmann/vAmigaTS](https://github.com/dirkwhoffmann/vAmigaTS) | **`master`** (2,077 test directories) | Whole-machine integration testbed with ADF disk images and golden RGB24 viewport captures from real Amigas |
| **Amiga Test Kit** | `tools/AmigaTestKit/AmigaTestKit.adf` | [keirf/amiga-stuff](https://github.com/keirf/amiga-stuff) | **Release v1.20+** (`AmigaTestKit.adf`) | Bootable diagnostic floppy disk for end-to-end machine loop validation and peripheral stress testing |

---

## 2. Directory Contents & Upstream Details

### A. Tom Harte Single-Step Silicon Vectors (`ref_src/SingleStepTests-680x0/`)
- **Upstream Repository:** `https://github.com/SingleStepTests/680x0`
- **Maintainer:** Tom Harte
- **Pinned Version:** Format version 1 (`68000/v1/`).
- **Directory Layout:**
  ```text
  ref_src/SingleStepTests-680x0/
  ├── 68000/
  │   └── v1/
  │       ├── NOP.json
  │       ├── ADD.B.json
  │       ├── MOVE.W.json
  │       └── ... (124 instruction test files, ~6.5 GB uncompressed)
  ├── map/
  └── README.md
  ```
- **Contents & Format:**
  - 124 per-instruction test files in JSON format.
  - Each file contains thousands of individual test cases captured directly from the pins of a physical Motorola 68000 microprocessor.
  - Each test defines the complete processor state before execution (`initial`: registers `D0`-`D7`, `A0`-`A7`, `PC`, `SR`, prefetch queue `IR`/`IRC`, and memory RAM slices), the cycle-by-cycle bus activity (`bus_events`), and the final verified state (`final`).
- **Test Harness Integration:**
  - Executed via `cargo test -p test_runner --test test_singlestep`.
  - Supports quick filtering (`cargo test -p test_runner --test test_singlestep test_nop`), full multi-suite sweeps (`$env:SINGLESTEP_FULL = "1"`), and sample size limits (`$env:SINGLESTEP_LIMIT = "500"`).

### B. Dirk W. Hoffmann vAmiga Reference Emulator (`ref_src/vAmiga-4.5/`)
- **Upstream Repository:** `https://github.com/dirkwhoffmann/vAmiga`
- **Maintainer:** Dirk W. Hoffmann
- **Pinned Version:** **v4.5** (directory can be named `ref_src/vAmiga-4.5` or `ref_src/vAmiga`).
- **Directory Layout:**
  ```text
  ref_src/vAmiga-4.5/
  ├── Core/
  │   ├── Components/
  │   │   ├── Agnus/      (Blitter, Copper, DMA channel allocation)
  │   │   ├── Denise/     (Bitplanes, sprites, color burst DAC)
  │   │   ├── Paula/      (Audio DMA, Floppy disk controller, UART)
  │   │   ├── CIA/        (MOS 8520 complex interface adapters)
  │   │   └── Gary/       (Bus address decoding and RAM gating)
  │   └── VAmiga.cpp
  ├── GUI/
  └── README.md
  ```
- **Contents & Role:**
  - Clean C++ implementation of the Amiga 500 hardware architecture.
  - Used as an authoritative reference model for cross-chip latency, register side-effects, strobe decoding, and DMA slot conflict arbitration.
  - Indexed by Graphify (`graphify update .`) to allow semantic AST queries and symbol comparisons between Rust emulator crates and the C++ reference.

### C. vAmiga Test Suite (`ref_src/vAmigaTS/`)
- **Upstream Repository:** `https://github.com/dirkwhoffmann/vAmigaTS`
- **Maintainer:** Dirk W. Hoffmann
- **Pinned Version:** `master` tracking the 2,077 test case suite.
- **Directory Layout:**
  ```text
  ref_src/vAmigaTS/
  ├── Agnus/         (Blitter line mode, Copper timing, DMA contention)
  ├── CIA/           (Timer A/B countdown, ICR interrupts, TOD latching)
  ├── CPU/           (Addressing modes, CCR flags, exception vectors)
  ├── Denise/        (Dual playfield, sprite demultiplexing, border blanking)
  ├── FPU/           (MC68881/MC68882 math coprocessor tests — deferred)
  ├── Mainboard/     (Gary memory mapping, open-bus floating reads)
  ├── Paula/         (Audio period timing, floppy DMA sync words)
  └── README.md
  ```
- **Contents & Role:**
  - Contains 2,077 regression tests compiled into `.adf` disk images alongside golden reference screenshots captured from physical Amiga machines.
  - Evaluated via `cargo run -p test_runner -- vamiga` against the 1,468 active Phase 1 Baseline OCS tests (see `Obsidian/Amiga/Design/vAmigaTS Verification Scorecard.md`).

### D. Keir Fraser Amiga Test Kit (`tools/AmigaTestKit/`)
- **Upstream Repository:** `https://github.com/keirf/amiga-stuff`
- **Maintainer:** Keir Fraser
- **Pinned Version:** Release v1.20+ (`AmigaTestKit.adf`).
- **Target File:** `tools/AmigaTestKit/AmigaTestKit.adf`
- **Contents & Role:**
  - Authoritative bootable diagnostic disk for real Amiga hardware.
  - Tests Chip RAM and Fast RAM bit errors, CIA timer accuracy, audio channels, and keyboard scancodes.
  - Utilized in integration testing via `cargo test -p test_runner --test test_boot_adf`.

---

## 3. Automated Provisioning (`tools/bootstrap/bootstrap_sources.ps1`)

The external sources bootstrapper automates the retrieval, extraction, decompression, and layout verification of these components:

### Running the Bootstrapper

Invoke via the central bootstrap coordinator:
```powershell
# Provision external test suites and hardware vectors:
.\tools\bootstrap\bootstrap.ps1 -Sources
```

Or run the dedicated sources bootstrapper directly:
```powershell
.\tools\bootstrap\bootstrap_sources.ps1
```

### Automation Lifecycle Steps
1. **Zip Archive Expansion:** Automatically scans `ref_src/SingleStepTests-680x0/` for `.zip` archives and unpacks them into place.
2. **Gzip Decompression:** Scans for `.json.gz` or `.gz` compressed test archives and decompresses them into native `.json` files using .NET `GZipStream` (zero external dependencies).
3. **Directory Canonicalization:** Migrates any loose `.json` test suites from `68000/` into the canonical `68000/v1/` directory.
4. **Presence Validation:** Verifies that `SingleStepTests-680x0` contains all 124 test suites, checks for `tools/AmigaTestKit/AmigaTestKit.adf`, and validates `ref_src/vAmiga` and `ref_src/vAmigaTS`.
5. **Immediate Smoke Verification:** Automatically runs a quick smoke check (`cargo test -p test_runner --test test_singlestep test_nop`) to ensure the test runner harness is functional.

---

## 4. Git Worktree Isolation & Zero-Cost NTFS Junctions

Because `ref_src/` contains upwards of 6.5 GB of test vectors, checking these files into Git history or copying them across worktrees would create massive repository bloat.

### Git Ignore Boundary (`.gitignore`)
- `ref_src/` and `tools/AmigaTestKit/` are excluded from Git commits via `.gitignore`.
- They represent preserved external assets that should never be deleted.

### Sharing Across Worktrees (`tools/git/worktree.ps1`)
When creating isolated Git worktrees for parallel agent development or feature branches:
```powershell
# Create a new isolated worktree with automatically linked external assets:
.\tools\git\worktree.ps1 add <branch-name>
```
The worktree script creates zero-cost NTFS directory junctions pointing back to the main repository's `ref_src/` and `tools/AmigaTestKit/`:
- **Speed:** Instant ($< 5\text{ ms}$).
- **Disk Footprint:** Zero additional bytes ($0\text{ MB}$).
- **Shared Access:** All worktrees immediately share the decompressed test vectors and reference sources without redundant downloads.
