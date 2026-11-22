# Preparing meaningful commits

The commit assignment covers only the selected changes and their staging.
Inspect their diff and preserve unrelated index entries. Propose which changes
belong together in a commit and write the proposed messages in English.
Include diary content only when it belongs to the selected work or the user
explicitly requests it. An ordinary commit needs no diary entry.

Report actual tests, coverage, reviews and known gaps. Missing or failed checks
are not passing evidence. Run additional checks or reviews only within the
assignment. A separate stage runs final tests and reviews on the integrated
change on master. Preparing a commit requires no four-lane reviews, frozen
trees, seals, publication receipts or reusable-evidence records.

## Coverage during development and review

For selected production changes, assess whether the tests meaningfully protect
the changed behavior. Apply the shared [testing criteria](../../../references/testing-criteria.md).
When the assignment includes a coverage measurement, use the existing verifier.
Report unmeasured areas separately from unresolved gaps in the behaviors that
tests cover. Missing evidence alone does not start measurement, audits or repairs
during commit preparation.
