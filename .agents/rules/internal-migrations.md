# Internal migrations

Read before changing internal APIs, schemas, configuration, generated formats or snapshots. The [configuration specification](../../Obsidian/Amiga/Design/Configuration.md) and [save format](../../Obsidian/Amiga/Design/Application/SaveState.md) define their behavior and ownership.

Internal APIs, schemas, configuration and generated formats have no default
backward-compatibility requirement. Workspace consumers are maintained together;
update affected callers, imports, fixtures and tests, then remove replaced code,
aliases, forwarders, re-exports, duplicate namespaces and legacy fallbacks in the
same change. Do not add migrations, adapters, deprecated interfaces or parallel
implementations for hypothetical consumers. Until first release, snapshots need
no backward compatibility. Preserve compatibility only for an explicit request or
a documented external contract.

Preserve external specifications and user data. Explain impact and obtain direction
before removing external-format support or deleting user data that cannot be
recreated. Before completion, search active code/tests for replaced symbols and
mechanisms, assess each remaining reference and run relevant checks. Before an
authorized commit, also reconcile documentation, instructions and executable
examples: history must identify unavailable capabilities; future work must state
prerequisites. An explicit documentation-preservation request takes precedence;
report that debt. Detailed cohesion/visibility assessment belongs to
[architecture criteria](../references/architecture-criteria.md).
