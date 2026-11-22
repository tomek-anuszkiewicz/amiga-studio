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

This note explains why the project uses these engineering practices. The
maintained requirements, including their limits, crate lists and verification
commands, are in the source documents.
[Rust best practices](../../../.agents/references/rust-best-practices.md),
[workspace layout](General%20Architecture.md#workspace-layout),
[architecture review criteria](../../../.agents/references/architecture-criteria.md),
[testing criteria](../../../.agents/references/testing-criteria.md)

## Keep execution paths concrete

With explicit functions and direct calls, a developer can follow a call in the
IDE and see which hardware phase it represents. Custom macros obscure that
connection and complicate compiler diagnostics. Const-generic instruction
handlers also make it harder to follow the path for a specific operand size and
addressing mode. The project uses specialized functions and static dispatch
where its specifications and practices require them.

## Preserve guest semantics on the host

Guest arithmetic wraps, and the emulator handles guest faults as emulated
exceptions. The emulator must preserve these guest semantics regardless of
Rust's debug overflow behavior. The project reserves its shared fatal helper
for failures of internal invariants. The host component that owns the machine
terminates a failed machine.
[Host ownership](Application/GUI.md#input-and-output)

Guest memory is big-endian. Explicit byte conversion shows where an operation
crosses that boundary. For some raw operations, including bitwise logic and
unchanged byte copies, reversing the byte order before or after the operation
gives the same result. The exception is limited to such operations. Host-endian
pointer casts and treating arithmetic values as raw byte streams are outside
that exception.

## Organize by responsibility

Keep the code for one responsibility together even when it uses several types.
The debugger keeps breakpoint definitions, condition evaluation and hit testing
together. Giving each type its own file would split that capability while
leaving the responsibility the same.
[Breakpoints](../../../crates/debugger/src/breakpoints.rs)

The architecture describes which subsystem owns each responsibility.
[Architecture](General%20Architecture.md)

Specifications and shared practices govern dependency permissions, required
module layout, constraints on frequently executed code and when verification
runs. Architecture review defines the detailed criteria for cohesion,
visibility, numeric naming and inlining. The configured checkers enforce the
mechanical limits. Each criterion still applies if it moves to another
document.

## References

- The architecture documents system boundaries and subsystem specifications.
  [General Architecture](General%20Architecture.md)
- The architecture checker enforces mechanical limits. The owning policies and
  behavioral tests still apply after the checker passes.
  [Architecture checker](../../../tools/verification/checks/check_architecture.py)
- The frontend guidelines describe host presentation and interaction constraints.
  [egui Guidelines](egui%20Guidelines.md)
