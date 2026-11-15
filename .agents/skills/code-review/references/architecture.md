# Architecture and Code Quality Reviewer

Own implementation constraints and authoritative definitions. Use the common
contract in [code-review](../SKILL.md#assigned-reviewer-common-contract).

- For production Rust, inspect changed numeric literals explicitly. Hardware
  addresses, flags, masks, shifts, encodings, timing values and limits need semantic
  names even when used once. Hexadecimal spelling or a comment is not a name.
- Reuse existing owning definitions rather than copying their values. Search the
  relevant owner when a changed encoding may already exist. Apply the documented
  ordinary-index/zero/increment and independent-test-encoding exceptions.
- Check required `REGISTER_`, `BIT_` and `MASK_` prefixes by meaning. A lookup table
  does not implicitly exempt encoded hardware values from the numeric-value rule.
- Check crate boundaries, visibility, re-exports, cohesion, duplicate mechanisms,
  internal API migrations, hot-path constraints and prohibited constructs when affected.
- Architectural checks, dependency permissions and enforcement changes require
  explicit approval for the concrete change; a passing checker is not approval.
- Leave runtime assertions and coverage accounting to their lanes. A passing lint
  or previous review does not establish that manual rules were checked.

Read the applicable [Rust rules](../../../rules/rust-best-practices.md),
[workspace rules](../../../rules/workspace-structure-and-reexports.md) and
[scope rules](../../../rules/structural-root-cause.md). These remain authoritative.
