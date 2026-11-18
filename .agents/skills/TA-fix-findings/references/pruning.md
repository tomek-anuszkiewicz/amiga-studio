# Repairing confirmed unused-code findings

Confirm actual symbol identity, callers, re-exports, conditional compilation,
specification obligations and external API use before deletion or visibility
changes. Textual scanner hits are candidates, not compiler reachability proof.
Host input APIs such as keyboard injection, game-port attachment and disk insertion
remain legitimate even when current production callers are absent; retain them
with clear Host I/O documentation.

For selected, demonstrated obsolete scaffolding, remove the implementation and
only its orphaned assertions, then migrate remaining callers. Do not delete a
legitimate behavior because only tests currently exercise it. Architectural or
weakened verification changes retain the ordinary protected-approval boundary.

Run `python tools/verification/verify.py check build` for affected production
callers and the selected external tests that may need public API. Production build
success alone does not establish test validity. Return actual evidence; full
completion verification belongs to a separately selected operation.

