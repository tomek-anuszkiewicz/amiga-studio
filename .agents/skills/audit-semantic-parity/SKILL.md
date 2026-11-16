---
name: audit-semantic-parity
description: At minor roadmap point or major milestone completion, audit modified subsystems and their interactions against approved specifications; identify missing implementation, valuable knowledge gaps and conflicts.
---

# Bidirectional Semantic Parity Review

Review agreement between approved design contracts and active implementation,
and preserve knowledge under
[Documentation Value](../../rules/docs-maintenance.md#documentation-value).

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
govern code, and the documentation-value rule governs enrichment and pruning.
Examine approval evidence for changed requirements; agreement
between final code and documents alone does not establish authorization.

Start with the workspace-wide deterministic checks, retaining their full scope:

```powershell
python .agents/skills/audit-semantic-parity/scripts/audit_semantic_sync.py --semantic-sync
```

Record failures and then assess their meaning alongside the subsystem review.
A passing script does not establish full semantic parity. Recheck static evidence
if remediation changes its inputs before closure.

## 1. Review Meaning and Evidence

Static checks establish only their configured comparisons. Manual review must
assess the meaning, evidence and authority of remaining claims. Apply
[Documentation Value](../../rules/docs-maintenance.md#documentation-value), including
its hardware-source comparison and enrichment procedure, across the owning
specification and its linked explanations. Use code signatures and fields to
locate relevant behavior; their absence from prose alone is not a defect.

Classify missing knowledge separately from unsupported claims, unmet requirements
and unauthorized extensions. Retain source-backed hardware explanations even
when explicit in code, and do not infer historical rationale from implementation.

---

## 2. Three Review Perspectives

| Perspective | Question and evidence |
| --- | --- |
| Code to documentation | Which source-confirmed hardware behavior or other valuable knowledge is missing? Inspect affected behavior and its sources; do not create a catalog of functions, fields or call order. |
| Documentation to code | Which requirements are unmet or descriptive claims unsupported? Trace timing, register access, subsystem interactions and other reviewed contracts to implementation and tests. |
| Hardware evidence and rationale | Are hardware explanations supported and traceable, and are modeling choices and established decisions distinguished from observations? Identify concrete missing evidence or lost knowledge rather than scoring prose length or resemblance to a manual. |

---

## 3. Subsystem Mapping Matrix

When auditing a subsystem, inspect its corresponding Rust crate or internal module and Obsidian design specification:

| Subsystem Domain | Primary Rust Sources | Authoritative Design Spec |
| :--- | :--- | :--- |
| **Interrupts** | `crates/paula/src/interrupts.rs` | `Obsidian/Amiga/Design/Interrupts.md` |
| **Paula Coordinator** | `crates/paula` | `Obsidian/Amiga/Design/Paula.md` |
| **Audio Engine** | `crates/paula/src/audio.rs` | `Obsidian/Amiga/Design/Audio.md` |
| **Floppy Controller** | `crates/floppy` | `Obsidian/Amiga/Design/Floppy.md` |
| **Serial Port** | `crates/paula` (`src/serial.rs`) | `Obsidian/Amiga/Design/Paula.md` |
| **Agnus Coordinator** | `crates/agnus` | `Obsidian/Amiga/Design/Agnus.md` |
| **Copper Coprocessor**| `crates/agnus/src/copper.rs` | `Obsidian/Amiga/Design/Copper.md` |
| **Blitter Engine** | `crates/agnus/src/blitter.rs` | `Obsidian/Amiga/Design/Blitter.md` |
| **DMA Controller** | `crates/agnus/src/dma.rs` | `Obsidian/Amiga/Design/DMA.md` |
| **Denise Display** | `crates/denise` | `Obsidian/Amiga/Design/Denise.md` |
| **Hardware Sprites** | `crates/denise/src/sprites.rs` | `Obsidian/Amiga/Design/Sprites.md` |
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
   - Query the `project` Graphify graph first (reference comparisons query `reference` separately), then read the identified definitions in the crate root and key submodules.
   - Use relevant structs, fields, constants and hardware action methods to locate the reviewed contracts and interactions; do not build an exhaustive symbol inventory.
2. **Step 2: Extract Specification Claims:**
   - Read `Obsidian/Amiga/Design/<Spec>.md`.
   - Map all section headings, register tables, timing assertions, and architectural claims.
3. **Step 3: Execute Bidirectional Comparison:**
   - Compare affected behavior and hardware sources against retained knowledge under the documentation-value rule.
   - Cross-examine each document assertion against live Rust code.
4. **Step 4: Emit Standardized Parity Report:**
   - Render the report using the standard template in Section 5.
5. **Step 5: Propose Actionable Remediation:**
   - Provide concrete documentation corrections or code adjustments within the authorized scope.
   - For `NEEDS_DOCS_ENRICHMENT`, apply
     [Documentation Value](../../rules/docs-maintenance.md#documentation-value) and
     [spec-compliance.md](../../rules/spec-compliance.md#compatible-documentation-enrichment). Use
     [obsidian-vault-linking](../obsidian-vault-linking/SKILL.md) for links and metadata.
   - For `NEEDS_CODE_ALIGNMENT`, implement already approved requirements and
     verify relevant behavior within the authorized scope. Obtain direction
     before expanding that scope; do not request the same requirement approval again.
   - For `SPEC_DIVERGENCE`, present the requirement, contrary evidence and options
     before changing requirements or accepting divergent behavior, per
     [spec-compliance.md](../../rules/spec-compliance.md).
   - Do not declare stage completion while valuable knowledge gaps,
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

### 1. Missing Valuable Knowledge
- [Behavior or established decision; source evidence; why its absence matters; proposed owning section].

### 2. Unsupported Claims and Unmet Requirements
- [Exact claim or requirement; affected implementation and tests; contrary evidence; approval status].

### 3. Hardware Evidence and Rationale
- [Source-confirmed behavior retained or added; modeling assumptions; established rationale; unresolved evidence and review limits].

### 4. Actionable Remediation Plan
- [Concrete step 1: exact section to add or refine in markdown]
- [Concrete step 2: draft code to prune or align]
```
