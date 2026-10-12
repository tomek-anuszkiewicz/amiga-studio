---
title: "Custom Chip Register Ownership and Access Matrix"
aliases: ["Custom Chip Register Matrix", "Custom Register Map", "Hardware Register Catalog", "Chipset Register Reference"]
tags: ["amiga", "design", "registers", "agnus", "denise", "paula", "memory-map"]
category: "Design"
subsystem: "memory_bus"
status: "active"
created: 2026-09-14
updated: 2026-10-12
related: ["[General Architecture.md](General%20Architecture.md)", "[MemoryBus.md](MemoryBus.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
---

# Custom Chip Register Ownership and Access Matrix

This document defines the format for a future custom register matrix. It contains no register entries. Consult the [MemoryBus specification](MemoryBus.md) for address routing and the [cross-chip signal catalog](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md) for propagation between chips.

## Matrix format

Use one row per register offset. Keep the read and write identities separate: the same offset can expose different circuits, and the chips responsible for reading and writing can differ. Record unsupported access directions explicitly rather than assuming that a register is symmetric.

| Offset | Read Name (Chip) | Write Name (Chip) | Access Mode | Scope | Delay | Function & Silicon Quirks |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `<offset>` | `<read identity or no decoded read>` | `<write identity or no decoded write>` | `<mode>` | `<scope>` | `<timing>` | `<behavior and evidence>` |

The angle-bracketed row is a template, not a register entry.

| Column | Meaning |
| :--- | :--- |
| **Offset** | Even byte offset within the custom register block, written in hexadecimal. Use an individual offset for each row; explain mirrors or address aliases in the notes if relevant. |
| **Read Name (Chip)** | Name of the value or circuit selected by a read, followed by its physical owner: Agnus, Denise, or Paula. State when no register drives a read; do not invent a readable counterpart for a write register. |
| **Write Name (Chip)** | Name of the destination or action selected by a write, followed by its physical owner. State when no write is decoded. List multiple recipients if a write is distributed to more than one chip. |
| **Access Mode** | Classify the offset's access behavior: `RO` (read only), `WO` (write only), `RW` (read and write), `COR` (clear on read), or `STROBE` (access triggers an action). Explain any direction-specific combination or exception in the last column. |
| **Scope** | `Internal` when effects remain within the owning chip; `Cross-Chip` when a signal or state change reaches another chip. Name the propagation path in the last column or link to the signal catalog. |
| **Delay** | Time from the bus access to the observable effect, in CCKs, or `Live` when the value is sampled directly. Distinguish read and write delays, and chip-specific delays, when they differ. Use `Unknown` when timing has not been established. |
| **Function & Silicon Quirks** | Concise function, side effects, open-bus behavior, revision-specific behavior, and any evidence or uncertainty needed to interpret the row. |

## Reference Documentation & Upstream Ground Truth

- [Hardware Reference Manual, Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md): Register names and documented access directions.
- [Hardware Reference Manual, Appendix B](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md): Address-ordered register summary.
- [Platform Quirks and Invariants Catalog](Platform%20Quirks%20and%20Invariants%20Catalog.md): Hardware-specific exceptions and evidence boundaries.
