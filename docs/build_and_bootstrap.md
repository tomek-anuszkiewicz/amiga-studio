# Building & Bootstrapping Guide

This guide details the compilation process, toolchain prerequisites, and the optional multi-tier bootstrapping suite for the Amiga 500 emulator.

---

## 1. Zero-Setup Build & Execution

A freshly cloned repository is **100% self-contained for compilation and execution** using the standard stable Rust toolchain ([rustup.rs](https://rustup.rs)). No bootstrapping, external downloads, or database services are required to build and run the emulator:

```powershell
# Build entire workspace (debug profile):
cargo build

# Build optimized native release binary for GUI:
cargo build --release -p gui
```

---

## 2. Optional Bootstrapping (`tools/bootstrap/bootstrap.ps1`)

Bootstrapping is **strictly optional** and only needed for specialized development tasks:

| Mode | Switch | When Needed | What It Provisions |
| :--- | :--- | :--- | :--- |
| **Verification Testbed** | `-Sources` | Running exhaustive single-step M68000 suites and DMA contention stress tests | Provisions Tom Harte physical silicon test vectors (auto-decompressing `.gz`/`.zip` archives in `ref_src/SingleStepTests-680x0/`, 124 suites), verifies vAmiga/vAmigaTS reference suites, and diagnostic disks |
| **Code Knowledge Graph** | `-Graphify` | Codebase structural navigation, call hierarchy, and symbol dependency analysis | AST-level code knowledge graph (`graphify-out/`), mapping crates, structs, functions, and cross-module relationships |
| **Documentation & RAG** | `-Rag` | AI agent pair-programming, hardware research, architecture design | Local Qdrant vector database (`http://localhost:6333`), indexing Commodore HRM, 68000 PRMs, technical specs, and design specs |
| **Reference Scans** | `-Documentation` | External reference scans and manual archives | Provisions raw reference manuals and PDF scans into `temp/` |
| **Full Setup** | `-All` | Complete initial development setup | Provisions all primary components (sources -> Graphify AST -> RAG documentation) |

### Bootstrapper Commands

```powershell
# Hardware test vectors verification setup:
.\tools\bootstrap\bootstrap.ps1 -Sources

# Code AST knowledge graph setup:
.\tools\bootstrap\bootstrap.ps1 -Graphify

# Documentation & AI pair-programming setup:
.\tools\bootstrap\bootstrap.ps1 -Rag

# External reference manuals and scans:
.\tools\bootstrap\bootstrap.ps1 -Documentation

# Complete setup (sources -> Graphify AST -> RAG docs):
.\tools\bootstrap\bootstrap.ps1 -All
```
