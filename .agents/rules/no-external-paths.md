# Path Privacy and Portability

- Never hardcode or commit private host paths in repository code, configuration, scripts, or documentation. Use repository-relative paths, environment variables, CLI arguments, or generic placeholders such as `<repo_path>`.
- Using an already authorized local path supplied through untracked configuration or a CLI argument does not require another permission request. Keep its value out of committed artifacts and unnecessary output.
- If required configuration is missing, ask for the missing value or preferred configuration mechanism; do not invent, embed, or expose a personal host path.
