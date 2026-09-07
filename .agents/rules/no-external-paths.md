# Strict Path Privacy & Isolation Rule (Zero External Paths)

- **Never use external paths**: Never write, hardcode, or commit paths outside the project workspace (such as private directories, personal drives, `D:\...`, `/home/...`, Google Drive, user folders) into any code, configuration, scripts, or documentation within the Amiga project.
- **Privacy & Portability**: Hardcoded external paths leak private user information and break project reproducibility across environments.
- **Use Placeholders in Documentation**: Always use generic placeholders in examples, templates, and documentation (e.g. `<PATH_TO_VAULT>`, `<PATH_TO_CACHE_DIR>`, `<repo_path>`).
- **Ask Before Resolving External Paths**: If an external path appears to be needed for configuration, tooling, or runtime, **you must stop and ask the user how to solve it** (e.g., via `.env` variables, CLI arguments, or relative paths) rather than assuming, embedding, or exposing external host paths.
