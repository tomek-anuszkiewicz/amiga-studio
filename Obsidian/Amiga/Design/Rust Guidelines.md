---
title: "Amiga 500 Rust Engineering Guidelines & Best Practices"
aliases: ["Rust Guidelines", "Engineering Guidelines", "Rust Best Practices"]
tags: ["amiga", "design", "rust", "guidelines", "standards"]
category: "Design"
subsystem: "general"
status: "active"
created: 2026-09-10
updated: 2026-11-17
related: ["[General Architecture.md](General%20Architecture.md)", "[egui Guidelines.md](egui%20Guidelines.md)", "[PowerShell tooling skill](../../../.agents/skills/TA-write-powershell/SKILL.md)"]
---

# Rust Engineering Rationale

The maintained engineering obligations live in the [Rust best practices](../../../.agents/references/rust-best-practices.md),
[workspace layout](General%20Architecture.md#workspace-layout),
[architecture review criteria](../../../.agents/references/architecture-criteria.md) and
[testing criteria](../../../.agents/references/testing-criteria.md). This note retains
why those choices matter rather than maintaining a second copy of their limits,
crate lists or verification commands.

## Keep execution paths concrete

Explicit functions and direct calls preserve IDE navigation and make a hardware
phase visible at its call site. Custom macros obscure that relationship and
complicate compiler diagnostics; const-generic instruction handlers also hide the
concrete size/addressing path. The project instead uses specialized functions
and static dispatch where required by the owning specifications and practices.

## Preserve guest semantics on the host

Guest arithmetic wraps, and guest faults remain emulated exceptions rather than
host failures. Rust's debug overflow behavior therefore cannot define the guest
machine. The shared fatal helper is reserved for internal invariant violations;
[the host owner](Application/GUI.md#input-and-output) terminates a failed machine.

Guest memory is big-endian. Explicit byte conversion makes that boundary visible.
Some raw operations commute with byte reversal, including bitwise logic and
unchanged byte copies; that exception does not authorize host-endian pointer
casts or treating arithmetic values as raw byte streams.

## Organize by responsibility

Cohesive behavior belongs together even when it needs several types. For example,
[breakpoints](../../../crates/debugger/src/breakpoints.rs) combine definitions,
condition evaluation and hit testing. Splitting each type into its own file would
separate one capability without creating an independent responsibility.

The [architecture](General%20Architecture.md) explains subsystem ownership.
Specifications and shared practices own dependency permissions, required module layout, hot-path
constraints and verification triggers. Architecture review owns the detailed
cohesion, visibility, numeric naming and inlining criteria. Mechanical limits
remain with their configured checkers; moving a criterion does not waive it.

## References

- [General Architecture](General%20Architecture.md): System boundaries and subsystem specifications.
- [Architecture checker](../../../tools/verification/checks/check_architecture.py): Mechanical enforcement; a passing check does not replace the owning policies or behavioral tests.
- [egui Guidelines](egui%20Guidelines.md): Host presentation and interaction constraints.
