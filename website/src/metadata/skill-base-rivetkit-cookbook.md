# RivetKit Guide

Before you implement anything:

1. Ensure the `rivetkit` skill is installed.
   - If it is already installed, do not reinstall it.
   - If it is not installed, install it non-interactively and only for the current agent:
     - `npx --yes skills add rivet-dev/skills --skill rivetkit --agent <agent> -y`
     - Replace `<agent>` with the current agent (e.g. `claude-code`, `cursor`, `windsurf`).
2. Read `rivetkit/SKILL.md` first (and any relevant references).

If `rivetkit` is not installed, instruct the user to install it, then stop and wait before proceeding.

## Error Handling Policy

- Prefer fail-fast behavior by default.
- Avoid `try/catch` unless absolutely needed for recovery, cleanup, or adding context.
- Never swallow errors. If a `catch` exists, handle the error explicitly, at minimum by logging it.

<!-- CONTENT -->
