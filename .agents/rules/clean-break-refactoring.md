# Clean-Break Refactoring

Internal APIs are maintained together in this workspace. Refactor them completely rather than preserving obsolete interfaces for hypothetical consumers.

- Update all affected callers, imports, tests, fixtures, and documentation in the same change. Stay within the requested scope.
- Delete replaced implementations. Do not retain compatibility aliases, forwarding wrappers, obsolete re-exports, duplicate namespaces, legacy fallback paths, or deprecated interfaces unless the user explicitly requires compatibility.
- If a change affects an external contract or existing user data, including saved snapshots, explain the impact and obtain direction before removing support for the old format.
- Before completion, use `rg` across the workspace to find remaining references to replaced symbols and mechanisms, review each match, and run the required checks and relevant tests.
