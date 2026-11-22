# Repairing confirmed unused-code findings

Before deleting code or changing its visibility, identify the exact symbol
and check its callers, re-exports, conditional compilation, specification
obligations and external API use. Use text scanner hits to identify candidates.
Confirm reachability with evidence beyond the text match.
Retain host input APIs such as keyboard injection, game-port attachment and
disk insertion even when they have no current production callers. Keep their
Host I/O documentation clear.

Once you establish that the selected scaffolding is obsolete, remove its
implementation and any assertions left without a purpose by that removal.
Migrate any remaining callers. Retain legitimate behavior even when only tests
exercise it. Follow the ordinary approval rules for protected architecture
changes or weaker verification.

Run `python tools/verification/verify.py check build` for affected production
callers and the selected external tests that may need public API. Production build
success alone leaves test validity open. Report the evidence from the checks
you ran. Full completion verification belongs to a separately selected operation.
