# Internal migrations

Apply to selected implementation, documentation updates and architecture review
involving internal APIs, schemas, configuration, generated formats or snapshots.
The [configuration specification](../../Obsidian/Amiga/Design/Configuration.md) and
[save format](../../Obsidian/Amiga/Design/Application/SaveState.md) define behavior
and ownership.

Internal APIs, schemas, configuration and generated formats have no default
backward-compatibility requirement. Update the workspace consumers together.
Change the affected callers, imports, fixtures and tests, then remove replaced
code, aliases, forwarders, re-exports, duplicate namespaces and legacy fallbacks
in the same change. Do not add migrations, adapters, deprecated interfaces or parallel
implementations for hypothetical consumers. Until first release, snapshots need
no backward compatibility. Preserve compatibility only for an explicit request or
a documented external contract.

Preserve external specifications and user data. Explain the effect and obtain
the user's direction before removing support for an external format or deleting
user data that cannot be recreated. Before completion, look for the replaced
symbols and mechanisms in active code and tests. Assess each remaining
reference and run the relevant checks. Before an authorized commit, also reconcile documentation, instructions and executable
examples. Identify unavailable capabilities in historical accounts and state
prerequisites for future work. An explicit request to preserve documentation
takes precedence. Report the resulting documentation debt. Detailed cohesion/visibility assessment belongs to
[architecture criteria](architecture-criteria.md).
