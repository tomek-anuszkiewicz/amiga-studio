# Architecture result criteria

This reference owns manual architecture and code-quality standards. Apply it to
the assigned source and affected interactions; do not turn a review into repairs
or repeat canonical Clippy/architecture checks. Shared constraints remain in the
[Rust best practices](rust-best-practices.md) and
[workspace layout](../../Obsidian/Amiga/Design/General%20Architecture.md#workspace-layout).

## Named numeric values

- Production logic uses semantic names or existing semantic types for register
  flags, masks, shifts, encoded modes, hardware addresses, timing values and domain
  limits, even when used once. Comments and hexadecimal/binary spelling are not
  semantic names. Lookup tables do not exempt encoded hardware values.
- Name the meaning, not the value: `MASK_CRB_INPUT_MODE`, not `MASK_03`. Reuse
  authoritative definitions in the nearest owning module. Equal numbers with
  different meanings need not share one constant.
- Prefix addresses, offsets and selectors with `REGISTER_`; individual flags and
  bit positions with `BIT_`; field/group selection, clearing and preservation masks
  with `MASK_`. Use these prefixes even inside a dedicated namespace, for example
  `REGISTER_CRA`, `BIT_CONTROL_START`, `BIT_CRB_INPUT_MODE_SHIFT` and
  `MASK_CRB_INPUT_MODE`. Encoded modes, timing values and limits keep their own
  semantic names; choose by meaning, not the numeric value.
- Clear operational literals remain valid: zero comparisons, unit increments,
  ordinary indexing and explicit byte/word width truncation. A register flag or
  encoded mode valued `0` or `1` is not covered by this exception.
- Test inputs and expected values may use literal hardware encodings to verify
  the contract independently. Never replace an independent expected encoding with
  the implementation constant in a way that hides an incorrect definition.

## Cohesion and complete migrations

Organize production Rust files by behavioral responsibility, not one struct per
file. Follow the affected implementation block's organization and separate
independent responsibilities even below the size limit. Serializable state may be
separate from heavy simulation when coherent. Tests remain outside `src/`.

Below 600 lines is the usual cohesive range; 600–800 lines requires responsibility
review. The checker enforces 800 lines unless a registered exception applies.
Static dispatch/LUT tables, indivisible linear instruction decoders and hardware
state machines may justify exceptions. Adding or removing an exception requires
[protected-verification approval](protected-verification.md#protected-verification),
including stale exceptions for deleted files or files reduced to 800 lines or fewer.
Assess touched modules; do not expand the task to unrelated large files. Source
limits do not apply to documentation or reference manuals.

Inspect affected callers, exports and tests for replaced mechanisms, stale aliases,
forwarders, wrappers, duplicate definitions and hypothetical compatibility paths.
Apply the [internal migration contract](internal-migrations.md).
Prefer simpler local mechanisms; unnecessary abstractions or duplicate ownership
need a concrete requirement, not a speculative future consumer.

## Visibility, reachability and accessors

Treat textual scanner results as candidates. Confirm symbol identity, production
and test callers, re-exports, conditional compilation, external API obligations
and specification requirements before calling code dead. `pub` in a library can
hide reachability warnings; crate-only users normally need `pub(crate)`, file-only
users private items, and internal submodules should not be unnecessarily public.

Test-only references do not establish obsolete code. Retain intentional Host I/O
and debugger boundaries, such as input injection and disk insertion, and document
their purpose. Propose removal only for confirmed superseded scaffolding and its
orphaned tests. A successful production build alone does not check integration-test
callers. Review reports confirmed issues and rejected scanner candidates separately.

- Ordinary getters match the field name without `get_`, using `const fn` where
  supported and returning the value or a borrow for non-Copy state.
- Boolean getters use `is_`, or natural existing `has_` / `can_` prefixes. Do not
  duplicate a field's existing `is_` prefix.
- Setters use `set_<field>` and `const fn` where supported.
- Buffer/sequence views use borrowed slices, not `&Vec<T>` / `&mut Vec<T>`;
  string inputs use `&str`, not `&String`. Assess actual semantics before applying
  scanner suggestions.

## Readability, inlining and mechanical evidence

Assess meaningful names, responsibilities and control flow without an operator
count. Preserve lazy short-circuiting, evaluation order, side effects and hardware
timing. Keep branches traceable to registers, signals and clock phases; concise
hardware rationale belongs beside non-obvious silicon states.

Use `inline(always)` for ultra-hot ALU/CCR helpers and CPU register accessors,
`inline` for small public accessors and cross-crate forwarders, and `inline(never)`
for cold exception/trap handlers. Do not force-inline large control-flow functions
(roughly over 15–20 lines) or indirect dispatch-table targets. Preserve configured
ALU/CCR/exception checks; performance conclusions require comparable measurement.

The compiler, configured Clippy permissions and architecture checker own their
mechanical acceptance. Inspect actual coverage and semantic gaps rather than
assuming a check name proves full enforcement. Preserve the fatal-helper exception,
host threading permissions, macro/const-generic prohibitions, external test layout,
path privacy and current dependency/line-limit data. No passing gate grants approval
to change protected policy. Consume existing diagnostics without rerunning them
solely for this assessment.
