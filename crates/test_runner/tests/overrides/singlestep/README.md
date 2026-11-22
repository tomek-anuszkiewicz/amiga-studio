# Reviewed SingleStep vector overrides

`corrections.json` lists approved corrections to specific upstream test records.
Git tracks this catalog separately from the ignored, reproducible `ref_src/`
tree. SingleStep uses the shared loader to read test vectors and apply the
catalog. The loader embeds the catalog at compile time, so adding an entry
requires rebuilding the tests.

Each entry identifies a suite by filename and a test by its exact name. It guards
the whole original record with a SHA-256 digest and replaces only the `final`
fields named in the entry. To compute the digest, the loader parses the fields in
the supported vector schema and serializes them as compact JSON. It sorts the
keys of every object and preserves the order of each array. Whitespace, source
property order, and fields outside the supported schema have no effect on the
identity. The same identity and entry apply to plain JSON and gzip inputs.

The loader fails if the suite has no test with the specified name or has more
than one. For a unique name match, the loader checks the stored digest and fails
if the record changed, including when upstream fixes the original expectation.
Review that upstream change, then revisit and remove the obsolete entry.
The loader applies overrides before filtering and
sampling. Each selected vector still runs through the normal comparator, which
checks registers, listed RAM, instruction clocks, and timed bus transactions.
Every selected case must execute to count as passed. SingleStep lists the
overrides it executed in its JSON results and gives their count in the Markdown
summary. DMA logs how many corrected vectors its sampler selects.

## ASL.b records 1583 and 1761

Both records encode `$E502`, `ASL.b #2,D2`, and expect a change in the upper 24
bits of D2. Byte operations preserve those bits. The user approved the overrides
on 2026-10-11 after comparing the instruction behavior with independent MAME
data.

| Record | Original expected D2 / SR | Corrected expected D2 / SR |
| --- | --- | --- |
| 1583 | `$2E5E4304` / `$2713` | `$CDFB7FF8` / `$270A` |
| 1761 | `$6461D390` / `$271B` | `$417C7EF4` / `$271B` |

The upstream issue proposes the same corrections. See the
[upstream report](https://github.com/SingleStepTests/680x0/issues/4).
The independent [MAME-generated ASL.b dataset](https://github.com/SingleStepTests/m68000/blob/64b253116a3de04aaac4346c43680960dc9b67e5/v1/ASL.b.json.bin)
contains similar `ASL.b #2,D7` cases. Record 625 maps `$A370CDBE` to `$A370CDF8`,
with CCR `$0A`. Record 695 maps `$104DA87D` to `$104DA8F4`, with CCR `$1B`. Both
record a four-clock prefetch read followed by six internal clocks. The MAME cases
use different registers and full states. They support the correction through
similar cases from an independent implementation. The evidence here covers those
cases, with no identical test copies or captures from physical hardware. Both
corrected records retain the upstream initial states, RAM, prefetch,
transactions, and clock totals.

## Adding or retiring a correction

Investigate the conflict in the reference, record supporting evidence, and obtain
user approval under [spec-compliance.md](../../../../../.agents/references/spec-compliance.md).
Then add the entry to `corrections.json` with the exact record identity, its
digest, the corrected fields, the reason, and the evidence. The digest identifies
the original record. The reason and supporting evidence justify the correction.
Run the affected full-input instruction suite and the loader contract tests.
Preserve the reference files and report overrides separately from unmodified
inputs.
