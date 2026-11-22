# PowerShell result criteria

Architecture review checks that the script follows the intended invocation
contract and nearby project conventions. Check for typed parameters and
explicit descriptions of defaults and data effects. Check that any inferred
action is unambiguous and that modifiers trigger no unrelated work, even when
the caller selects the action explicitly.
With no arguments, tools that need an action or input print usage and succeed.
Tools with a complete single-purpose default run that default. The usage path
must work before missing mandatory parameters can prompt or stop the call.
The no-argument path covers usage without extra help switches or aliases.
Invalid input reports the specific problem and relevant usage, then exits with
a nonzero status.

Console usage explains what parameters do, their defaults, their constraints
and any changes to data. Include executable examples of ordinary invocations
and meaningful option combinations. Align the output using the actual option
names and keep it readable without color. Keep one implementation responsible
for usage and avoid repeating the manual in comment help. Replace labels such
as "force mode" with an explanation of what happens to the data.

Inspect quoted paths, Join-Path composition, LiteralPath use and task-specific
variables. Keep file operations in one PowerShell shell. Before a recursive
move or deletion, check that each resolved absolute target stays within the
intended directory. Make sure the script propagates failure status from
external tools.

The tests reviewer checks the parser results and whether the declared
parameters match the usage text. The reviewer also looks for evidence of
representative no-argument, valid and invalid calls made safely within scope.
The parser can inspect a script without running a default that changes data. Authorization to read usage
covers help inspection alone. Use existing tool tests when they already
supply the required evidence. Do not require another run to reproduce evidence
that is already available.
