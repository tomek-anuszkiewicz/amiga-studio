# Clean-Break Refactoring

Internal APIs are maintained together in this workspace. Refactor them completely rather than preserving obsolete interfaces for hypothetical consumers.

- Update affected callers, imports, tests, fixtures, and documentation within the requested change.
- Delete replaced implementations, compatibility aliases, forwarding wrappers, obsolete re-exports, duplicate namespaces, and legacy fallback paths unless the user requires compatibility.
- For external contracts or existing user data, including saved snapshots, explain the impact and obtain direction before removing old-format support.
- Before completion, search the workspace for replaced symbols and mechanisms, review each remaining reference, and run the required checks and relevant tests.
