# Reviewed SingleStep vector overrides

`corrections.json` contains approved corrections to specific upstream records.
It is tracked independently of the ignored, reproducible `ref_src/` tree. The
shared loader embeds this catalog at compile time, so adding a catalog entry
requires rebuilding the tests. SingleStep and Cartesian DMA use the same loader.

Each entry identifies a suite filename and an exact test name, protects the whole
original record with SHA-256, and replaces only explicitly listed `final` fields.
The digest is computed from the parsed supported vector serialized as compact
JSON with recursively sorted object keys; arrays retain their order. Source
whitespace and object property order do not affect identity. Additional fields
outside the supported schema do not participate. Plain JSON and gzip inputs use
the same identity and catalog entry.

The loader fails if the target is missing, ambiguous or changed, including when
upstream fixes the original expectation. Revisit and remove the obsolete entry
after reviewing such a change. Overrides apply before filtering and sampling.
The normal comparator still executes each selected vector and checks registers,
listed RAM, instruction clocks and timed bus transactions. No case is skipped or
counted as passed without execution. SingleStep JSON results list the executed
override names; the Markdown summary includes their count. DMA logs the number
of corrected vectors selected by its sampler.

## ASL.b records 1583 and 1761

Both records encode `$E502`, `ASL.b #2,D2`, but expect the upper 24 bits of D2 to
change. Byte operations preserve those bits. The user approved the overrides on
2026-10-11 after comparing the instruction behavior and independent MAME data.

| Record | Original expected D2 / SR | Corrected expected D2 / SR |
| --- | --- | --- |
| 1583 | `$2E5E4304` / `$2713` | `$CDFB7FF8` / `$270A` |
| 1761 | `$6461D390` / `$271B` | `$417C7EF4` / `$271B` |

The [upstream report](https://github.com/SingleStepTests/680x0/issues/4) proposes
these same changes. The independent [MAME-generated ASL.b dataset](https://github.com/SingleStepTests/m68000/blob/64b253116a3de04aaac4346c43680960dc9b67e5/v1/ASL.b.json.bin)
contains analogous `ASL.b #2,D7` records: 625 maps `$A370CDBE` to `$A370CDF8`,
CCR `$0A`; 695 maps `$104DA87D` to `$104DA8F4`, CCR `$1B`. Both log a four-clock
prefetch read followed by six internal clocks. These use different registers and
full states; they are independent implementation evidence, not identical test
copies or physical hardware captures. Initial states, RAM, prefetch, transactions
and clock totals remain the upstream values in both corrected records.

## Adding or retiring a correction

Investigate the reference conflict, record supporting evidence and obtain user
approval under [spec-compliance.md](../../../../../.agents/rules/spec-compliance.md).
Then add the exact record identity, corrected fields, reason and evidence to
`corrections.json`. A digest alone does not justify a change. Run the affected
full-input instruction suite and the loader contract tests. Keep the reference
files untouched and report overrides separately from unmodified inputs.
