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

## 2. Automated Provisioning (`tools/bootstrap/bootstrap_sources.ps1`)

The external sources bootstrapper automates the retrieval, extraction, decompression, and layout verification of these components:

### Running the Bootstrapper

Invoke via the central bootstrap coordinator:
```powershell
# Provision external test suites and hardware vectors:
.\tools\bootstrap\bootstrap.ps1 -Sources
```

### Automation Lifecycle Steps
1. **Zip Archive Expansion:** Automatically scans `ref_src/SingleStepTests-680x0/` for `.zip` archives and unpacks them into place.
2. **Gzip Decompression:** Scans for `.json.gz` or `.gz` compressed test archives and decompresses them into native `.json` files using .NET `GZipStream` (zero external dependencies).
3. **Directory Canonicalization:** Migrates any loose `.json` test suites from `68000/` into the canonical `68000/v1/` directory.
4. **Presence Validation:** Verifies that `SingleStepTests-680x0` contains all 124 test suites, checks for `tools/AmigaTestKit/AmigaTestKit.adf`, and validates `ref_src/vAmiga` and `ref_src/vAmigaTS`.
5. **Immediate Smoke Verification:** Automatically runs a quick smoke check (`cargo test -p test_runner --test test_singlestep test_nop`) to ensure the test runner harness is functional.

---

## 3. Git Worktree Isolation & Zero-Cost NTFS Junctions

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
