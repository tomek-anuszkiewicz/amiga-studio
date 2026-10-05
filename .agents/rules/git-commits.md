# Git Commits & Immediate Atomic History Protocol

---

## 1. Core Mandate

- **Commit after every discrete task** before completing the turn. Never leave uncommitted changes across turns.
- **No blind bulk staging** (`git add -A`) when changes span distinct concerns. Inspect → Decompose → Stage targeted files → Verify.
- When changes have accumulated: run `git status` / `git diff --stat`, then group files into logical atomic units.

---

## 2. Atomic Decomposition

Stage separately by concern: `git add <file1> <file2>`

| Concern | Commit type |
|---|---|
| `Obsidian/Amiga/Design/*.md` | `docs(obsidian):` / `docs(design):` |
| `.agents/rules/`, `.agents/skills/`, `AGENTS.md` | `chore(rules):` / `docs(rules):` |
| `crates/<subsystem>/src/` | `feat/fix/refactor/perf(<subsystem>):` |
| `crates/test_runner/tests/` | `test(arch):` / `test(<subsystem>):` |

**Cohesive unit:** implementation, relevant tests, design updates, and the diary entry for the **same feature** belong in one atomic commit. Every separate atomic commit gets its own diary entry.

---

## 3. Conventional Commits Format

```text
<type>(<scope>): <concise imperative summary in English>

- <Concrete change: what was added, modified, or removed>
- <Technical rationale or trade-off>
- <Verification: test suites passed>
```

**Types:** `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `chore`
**Scopes:** `m68000`, `memory_bus`, `agnus`, `denise`, `paula`, `cia`, `gui`, `debugger`, `obsidian`, `arch`, `rules`

---

## 4. Pre-Commit Gate

```powershell
python tools/harness/pre_flight.py --quick
cargo test -p test_runner --test test_architecture_rules
```

> Every commit includes a new `DIARY.md` Section 10 entry describing its changes and verification. At minor roadmap points or milestones, compact settled entries per [diary-maintenance.md](diary-maintenance.md); synchronize design docs and prune the roadmap when their completion conditions apply.

---

## 5. On-Demand Review: `$audit-code-quality`

Run [`$audit-code-quality`](../skills/audit-code-quality/SKILL.md) ([manual diff checklist](../skills/audit-code-quality/references/manual-diff-checklist.md)) only when explicitly requested or before a major branch merge.
