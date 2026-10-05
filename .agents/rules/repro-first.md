# Reproduce Defects Before Repair

- Before changing production behavior to fix a defect, identify or author a focused regression test and confirm that it fails on the original code for the expected reason. An existing isolated failing test is sufficient.
- Repair the demonstrated mechanism within the requested scope, then confirm the reproduction passes and run adjacent domain regressions.
- Keep the regression protection after the fix; never delete or disable it merely to hide a failure. Rust test placement follows [unit-testing-policy.md](unit-testing-policy.md).
- Apply [structural-root-cause.md](structural-root-cause.md) and [spec-compliance.md](spec-compliance.md); never adjust goldens or timing offsets to manufacture a pass.
- Use the focused reproduction procedure in [test-runner](../skills/test-runner/SKILL.md). For CPU silicon divergences, use [m68k-singlestep-test](../skills/m68k-singlestep-test/SKILL.md).

Completion requires the domain and per-commit checks in [AGENTS.md](../../AGENTS.md); report actual results and remaining gaps.
