# Clean-Break Refactoring

Internal APIs are maintained together in this workspace. Refactor them completely rather than preserving obsolete interfaces for hypothetical consumers. Do not infer a compatibility requirement from existing code, tests, documentation, or possible consumers.

- Update affected callers, imports, tests, and fixtures within the requested change. Review code-driven documentation changes before an authorized commit, unless documentation work was explicitly requested earlier.
- Migrate affected internal callers and delete replaced implementations, compatibility aliases, forwarding wrappers, obsolete re-exports, duplicate namespaces, and legacy fallback paths in the same change. Preserve compatibility only when the user explicitly requests it or a documented external contract requires it.
- For external contracts or existing user data, including saved snapshots, explain the impact and obtain direction before removing old-format support.
- Before completion, search active code and tests for replaced symbols and mechanisms, review each remaining reference, and run the required checks and relevant tests. Review documentation and instruction references before an authorized commit.
- Before an authorized commit, retire active command examples, skill procedures and ownership mappings for a removed mechanism. Retained historical descriptions must state that the capability is unavailable; future plans must identify its prerequisites rather than present retired commands as executable. An explicit user request to preserve documentation takes precedence; report the resulting debt.
