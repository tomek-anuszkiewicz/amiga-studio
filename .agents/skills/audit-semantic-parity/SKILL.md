---
name: audit-semantic-parity
description: At minor roadmap point or major milestone completion, audit modified subsystems and their interactions against approved specifications; identify missing implementation, compatible documentation gaps and conflicts.
---

# Recipe: Inference-Driven Bidirectional Semantic Parity Auditor

This skill provides an inference-driven, qualitative architectural audit that evaluates whether living design specifications in `Obsidian/Amiga/Design/` genuinely, bidirectionally correspond to active Rust implementation in `crates/`.

---

## Completion Trigger and Deterministic Checks

Mandatory at completion of a minor roadmap point or major milestone: review the
modified subsystems before declaring the stage complete. This obligation is
owned here; ordinary commits alone do not require this semantic review.
Explicit subsystem audits remain available on request.

Review the modified subsystems and their relevant interactions across the
completed stage, including code unchanged by the final commit. This scope is
broader than the pre-commit review of an individual patch. Record the reviewed
snapshot, requirements and evidence; the same audit may satisfy
[verify-spec-compliance](../verify-spec-compliance/SKILL.md) for the completion
commit when it covers that skill's selected scope. Do not repeat the same
comparison unless relevant inputs change.

Both comparison directions follow
[spec-compliance.md](../../rules/spec-compliance.md): approved requirements
govern code, and significant compatible implementation details may enrich the
documentation. Examine approval evidence for changed requirements; agreement
between final code and documents alone does not establish authorization.

Start with the workspace-wide deterministic checks, retaining their full scope:

```powershell
python .agents/skills/audit-semantic-parity/scripts/audit_semantic_sync.py --semantic-sync
```

Record failures and then assess their meaning alongside the subsystem review.
A passing script does not establish full semantic parity. Recheck static evidence
if remediation changes its inputs before closure.

## 1. The Core Mandate: Beyond Static Heuristics

Deterministic static analysis can verify configured register offsets, memory ranges and method names. However, **static scripts cannot judge semantic comprehension**:
- A script cannot determine whether a design specification is an exhaustive, cycle-exact hardware document or merely a 10-line high-level stub.
- A script cannot detect **Blind Spots**: subtle silicon behaviors, edge cases, wait-state logic, or register bitfields implemented in Rust that the documentation never mentions.
- A script cannot detect **Ghost Features & Hallucinations**: timing models, register modes, or hardware features asserted in Markdown that the Rust code never actually simulates.

This skill leverages deep LLM reasoning and inference to evaluate true bidirectional parity.

Assess completeness across the owning specification and its linked descriptions.
Shared mechanisms or instruction semantics documented there are not blind spots
merely because an archetype section does not repeat it. Apply the scope rules in
[docs-maintenance.md](../../rules/docs-maintenance.md#living-design-specifications).
Use signatures and fields to locate significant mechanisms, not as a requirement
to document every helper or implementation detail. Missing descriptions are
enrichment candidates only when the behavior is compatible with approved
requirements and within the authorized scope; classify contradictions and
unauthorized extensions separately.

---

## 2. The 3-Vector Bidirectional Evaluation Model

Every audit assesses a paired target `(crates/<crate>/src/, Obsidian/Amiga/Design/<Spec>.md)` across three distinct vectors:

```mermaid
flowchart TD
    subgraph Vectors ["The 3 Evaluation Vectors"]
        V1["Vector 1: Forward Parity (Code -> Docs)\nBlind Spot & Undocumented Logic Detector"]
        V2["Vector 2: Reverse Parity (Docs -> Code)\nGhost Feature & Spec Drift Detector"]
        V3["Vector 3: Silicon Depth & Rigor Score\nTechnical Completeness & Inverted Pyramid"]
    end
```

### Vector 1: Forward Parity (Code $\to$ Docs — The "Blind Spot" Detector)
- **Question:** *What is implemented in the Rust codebase that is missing, under-specified, or only trivially mentioned in the specification?*
- **Audit Targets:**
  - Public functions, trait implementations, and companion helpers.
  - State struct fields, internal counters, phase latches, and buffers.
  - Hardware edge cases (e.g. unaligned accesses, contention wait states, reset states).
  - Register bitmasks, write strobe side effects, and pipeline propagation delays.

### Vector 2: Reverse Parity (Docs $\to$ Code — The "Ghost Feature" Detector)
- **Question:** *What is asserted, specified, or described in the design document that does not actually exist or is bypassed in the Rust codebase?*
- **Audit Targets:**
  - Speculative draft snippets or obsolete pseudo-code lingering from earlier development.
  - Features claimed in prose (e.g. 9-bit serial parity checking, custom bus arbitration) that are simplified or omitted in code.
  - Register ownership or access rights claimed in text that contradict memory bus router routing.

### Vector 3: Silicon Depth & Architectural Rigor
- **Question:** *Does this document read like a professional Commodore/Motorola hardware manual with mechanical sympathy, or is it a generic, shallow summary?*
- **Assessment:** Identify which relevant register definitions, bitfields, phases, memory ranges, and edge cases are covered, missing, or contradicted. Support the assessment with concrete references; do not assign a subjective percentage without a defined denominator.

---

## 3. Subsystem Mapping Matrix

When auditing a subsystem, inspect its corresponding Rust crate and Obsidian design specification:

| Subsystem Domain | Primary Rust Crates | Authoritative Design Spec |
| :--- | :--- | :--- |
| **Interrupts** | `crates/interrupts` | `Obsidian/Amiga/Design/Interrupts.md` |
| **Paula Coordinator** | `crates/paula` | `Obsidian/Amiga/Design/Paula.md` |
| **Audio Engine** | `crates/audio` | `Obsidian/Amiga/Design/Audio.md` |
| **Floppy Controller** | `crates/floppy` | `Obsidian/Amiga/Design/Floppy.md` |
| **Serial Port** | `crates/paula` (`src/serial.rs`) | `Obsidian/Amiga/Design/Paula.md` |
| **Agnus Coordinator** | `crates/agnus` | `Obsidian/Amiga/Design/Agnus.md` |
| **Copper Coprocessor**| `crates/copper` | `Obsidian/Amiga/Design/Copper.md` |
| **Blitter Engine** | `crates/blitter` | `Obsidian/Amiga/Design/Blitter.md` |
| **DMA Controller** | `crates/dma` | `Obsidian/Amiga/Design/DMA.md` |
| **Denise Display** | `crates/denise` | `Obsidian/Amiga/Design/Denise.md` |
| **Hardware Sprites** | `crates/sprites` | `Obsidian/Amiga/Design/Sprites.md` |
| **Frame Buffer** | `crates/frame_builder` | `Obsidian/Amiga/Design/Frame Buffer.md` |
| **Memory Bus & Gary** | `crates/memory_bus` | `Obsidian/Amiga/Design/MemoryBus.md` |
| **CIA Timers & Ports**| `crates/cia` | `Obsidian/Amiga/Design/CIA.md` |
| **M68000 CPU Core** | `crates/cpu` | `Obsidian/Amiga/Design/CPU Motorola M68000.md` |
| **Motherboard Loop** | `crates/machine_loop` | `Obsidian/Amiga/Design/Machine Loop.md` |

---

## Invocation & Target Selection

Invoke `$audit-semantic-parity <subsystem>` using the mapping above (`m68000`
maps to `crates/cpu`). With no target, use `git status` and `git diff --stat` to
select recently modified subsystems; if none are modified, inspect a primary
coordinator (`paula` or `agnus`) and state the selected scope.

## 4. Step-by-Step Execution Protocol

When executing an audit:

1. **Step 1: Extract Code Signatures:**
   - Query Graphify first, then read the identified definitions in the crate root and key submodules.
   - Note all public structs, fields, constants, bitmasks, and hardware action methods (`step_cck`, `write_*`, `read_*`, `poll_*`).
2. **Step 2: Extract Specification Claims:**
   - Read `Obsidian/Amiga/Design/<Spec>.md`.
   - Map all section headings, register tables, timing assertions, and architectural claims.
3. **Step 3: Execute Bidirectional Comparison:**
   - Cross-examine each code entity against document sections.
   - Cross-examine each document assertion against live Rust code.
4. **Step 4: Emit Standardized Parity Report:**
   - Render the report using the standard template in Section 5.
5. **Step 5: Propose Actionable Remediation:**
   - Provide concrete Markdown additions or code adjustments to achieve parity.
   - For `NEEDS_DOCS_ENRICHMENT`, add only significant compatible details under
     [spec-compliance.md](../../rules/spec-compliance.md#compatible-documentation-enrichment). Use
     [obsidian-vault-linking](../obsidian-vault-linking/SKILL.md) for links and metadata.
   - For `NEEDS_CODE_ALIGNMENT`, implement already approved requirements and
     verify relevant behavior within the authorized scope. Obtain direction
     before expanding that scope; do not request the same requirement approval again.
   - For `SPEC_DIVERGENCE`, present the requirement, contrary evidence and options
     before changing requirements or accepting divergent behavior, per
     [spec-compliance.md](../../rules/spec-compliance.md).
   - Do not declare stage completion while significant documentation gaps,
     unresolved conflicts or required implementation/verification gaps remain.
     After remediation, reassess affected findings on the final intended snapshot.

---

## 5. Standardized Semantic Parity Report Template

```markdown
# Semantic Parity Audit: [Subsystem Name]

**Evaluated Scope:**
- **Snapshot and Approval Evidence:** [revision or intended patch; approved requirement changes]
- **Code:** `crates/<crate>/src/`
- **Specification:** `Obsidian/Amiga/Design/<Spec>.md`
- **Evidence Coverage:** [assessed dimensions and limitations]
- **Verdict:** [PASS | NEEDS_DOCS_ENRICHMENT | NEEDS_CODE_ALIGNMENT | SPEC_DIVERGENCE]

---

### 1. Forward Parity: Blind Spots (In Code, Missing in Docs)
- 🔴 **[Symbol / Feature / Quirk]:** [Description of what the code implements and why its absence from docs harms fidelity].

### 2. Reverse Parity: Ghost Features (In Docs, Missing in Code)
- 🟡 **[Claim / Assertion / Mode]:** [Description of what the spec claims that is unsimulated or contradicted in code].

### 3. Engineering Depth & Rigor Assessment
- **Register & Bitfield Completeness:** [Exhaustive | Partial | Shallow]
- **Clock Phase & Timing Fidelity:** [Cycle-Exact | Qualitative | Missing]
- **Inverted Pyramid Structure:** [Compliant | Non-Compliant]

### 4. Actionable Remediation Plan
- [Concrete step 1: exact section to add or refine in markdown]
- [Concrete step 2: draft code to prune or align]
```
