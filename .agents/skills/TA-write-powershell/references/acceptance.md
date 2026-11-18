# PowerShell result criteria

Architecture review checks the intended invocation contract and nearby project
conventions: typed parameters, unambiguous action inference, explicit defaults and
data effects, and no unrelated action caused by a modifier. Action/input tools
print usage and succeed without arguments; complete single-purpose defaults run.
Missing mandatory parameter binding must not intercept the intended usage path.
No extra help switches or aliases are needed. Invalid input reports its concrete
problem and relevant usage with nonzero status.

Console usage explains parameter effects, defaults, constraints and data changes,
with executable ordinary and meaningful combined examples. Alignment derives
from actual option names and remains readable without color. Keep one owning
implementation; comment help must not duplicate the manual. Phrases such as
"force mode" do not describe a data-changing effect.

Inspect quoted paths, Join-Path composition, LiteralPath use and task-specific
variables. File operations stay in one PowerShell shell, with resolved absolute
target containment checked before recursive moves/deletions. External tool
failure status must propagate.

Tests/evidence review checks parser evidence, agreement between declarations and
usage, and safely scoped representative no-argument, valid and invalid invocations.
The parser can inspect a script without running its mutating default; merely
reading usage never authorizes unrelated default work. Existing tool tests may
supply this evidence. Do not require a second redundant execution.
