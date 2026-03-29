# Agent Instruction File Loading

How each supported coding agent discovers and loads project-level system instructions.

## PI (`@mariozechner/pi-coding-agent`)

**Source**: `node_modules/@mariozechner/pi-coding-agent/dist/core/resource-loader.js`

### Instruction files

| File | Location | Behavior |
|------|----------|----------|
| `AGENTS.md` or `CLAUDE.md` | Walk up from cwd to `/` | Appended as "Project Context" section. `AGENTS.md` checked first, `CLAUDE.md` as fallback. |
| `AGENTS.md` or `CLAUDE.md` | `~/.pi/agent/` | Global context, loaded before project files |
| `SYSTEM.md` | `{cwd}/.pi/SYSTEM.md` | **Replaces** the entire default system prompt |
| `SYSTEM.md` | `~/.pi/agent/SYSTEM.md` | Global system prompt override (fallback if no project-level one) |
| `APPEND_SYSTEM.md` | `{cwd}/.pi/APPEND_SYSTEM.md` | Appended to system prompt without replacing |
| `APPEND_SYSTEM.md` | `~/.pi/agent/APPEND_SYSTEM.md` | Global append (fallback) |

### Assembly order

1. Base system prompt (default or `SYSTEM.md` override)
2. `APPEND_SYSTEM.md` content
3. All `AGENTS.md`/`CLAUDE.md` files concatenated as "# Project Context"
4. Skills section
5. Current date + working directory

### Key behavior

- Walks the entire directory tree upward from cwd
- Context files are **always** injected, even with a custom `SYSTEM.md`
- `AGENTS.md` takes priority over `CLAUDE.md` in the same directory

---

## OpenCode (`opencode-ai`)

**Source**: Go binary, config at `internal/config/config.go`, prompt at `internal/llm/prompt/prompt.go`

### Instruction files

Default `contextPaths` (all relative to project root):

```
.github/copilot-instructions.md
.cursorrules
.cursor/rules/           (directory, recursive)
CLAUDE.md
CLAUDE.local.md
opencode.md
opencode.local.md
OpenCode.md
OpenCode.local.md
OPENCODE.md
OPENCODE.local.md
```

### Configuration

- `.opencode.json` in: `$HOME`, `$XDG_CONFIG_HOME/opencode/`, `$HOME/.config/opencode/`, or `./`
- `contextPaths` array is customizable in config
- Directories (paths ending with `/`) are walked recursively

### Assembly order

1. Base system prompt (coder or task agent)
2. `# Project-Specific Context` header
3. All discovered instruction files concatenated

### Key behavior

- Does **not** walk up the directory tree; only checks project root
- Reads many competing formats (Cursor rules, Copilot instructions, Claude, OpenCode)
- Case-insensitive deduplication
- Files processed in parallel

---

## Claude Code

**Source**: Claude Code internal behavior, documented conventions

### Instruction files

| File | Location | Behavior |
|------|----------|----------|
| `CLAUDE.md` | Project root (or `.claude/CLAUDE.md`) | Primary project instructions |
| `CLAUDE.md` | Parent directories (walk up) | Inherited context |
| `CLAUDE.md` | `~/.claude/CLAUDE.md` | User-level personal instructions |
| `CLAUDE.md` | `/etc/claude-code/CLAUDE.md` (Linux) | Managed/org-wide policy (cannot be excluded) |
| `.claude/rules/**/*.md` | Project root | Modular rule files, loaded unconditionally or by path glob |
| `~/.claude/rules/**/*.md` | Home dir | User-level rule files |

### Assembly order

1. Managed policy CLAUDE.md (highest priority, cannot be excluded)
2. Project CLAUDE.md
3. Parent directory CLAUDE.md files (walking up)
4. User-level CLAUDE.md
5. Rules files (path-scoped rules activate on matching globs)

### Key behavior

- Walks **up** from cwd, concatenates all found files
- All files are concatenated, not overridden
- Subdirectory CLAUDE.md files loaded on-demand when Claude accesses those dirs
- `@path/to/file` import syntax supported (max depth 5)
- Recommended: under 200 lines per file

---

## Codex (`@openai/codex`)

**Source**: Codex CLI (Rust binary), documented at developers.openai.com/codex

### Instruction files

| File | Location | Behavior |
|------|----------|----------|
| `AGENTS.override.md` | `~/.codex/` | Global override (highest priority) |
| `AGENTS.md` | `~/.codex/` | Global instructions |
| `AGENTS.override.md` | Git root down to cwd | Per-directory override |
| `AGENTS.md` | Git root down to cwd | Per-directory instructions |

### Configuration

- `~/.codex/config.toml`
- `project_doc_fallback_filenames` — alternative filenames to check
- `project_doc_max_bytes` — max size limit (default 32 KiB)

### Assembly order

1. Global file (`~/.codex/AGENTS.override.md` or `~/.codex/AGENTS.md`)
2. Git root `AGENTS.override.md` or `AGENTS.md`
3. Each subdirectory down to cwd, in order

Each file becomes a separate user-role message prefixed with:
```
# AGENTS.md instructions for <directory>
```

### Key behavior

- Walks **down** from git root to cwd (not up)
- `AGENTS.override.md` takes priority over `AGENTS.md` in the same dir
- 32 KiB default byte limit; discovery stops when limit reached
- Deeper files appear later in prompt (effectively higher priority for LLM attention)

---

## Common Ground

All four agents share these patterns:

1. **Project-root instruction file** — every agent loads at least one file from the project root
2. **Global/user-level config** — every agent supports a home-directory level file
3. **Concatenation** — instructions are appended to the system prompt, not replacing it
4. **Walk behavior varies** — PI walks up, Codex walks down from git root, OpenCode stays flat, Claude Code walks up + on-demand subdirs

---

## CLI Flags & Direct Injection (non-file approaches)

| Agent | CLI flags | Env vars |
|-------|-----------|----------|
| **PI** | `--system-prompt <text>`, `--append-system-prompt <text>` | None |
| **Claude Code** | `--system-prompt <text>`, `--system-prompt-file <path>`, `--append-system-prompt <text>`, `--append-system-prompt-file <path>` | None |
| **OpenCode** | None | None |
| **Codex** | None | None |

Only PI and Claude Code support direct injection via CLI flags. OpenCode and Codex are file-only.

---

## Per-Agent Injection Approaches (without modifying cwd)

### PI

| # | Approach | Mechanism | Touches cwd? | Clobber risk |
|---|----------|-----------|:---:|:---:|
| 1 | Global context file | Write `~/.pi/agent/AGENTS.md` in VM | No | None (VM home is fresh) |
| 2 | Global append file | Write `~/.pi/agent/APPEND_SYSTEM.md` in VM | No | None |
| 3 | CLI flag | Pass `--append-system-prompt <text>` when spawning pi-acp | No | None |
| 4 | Project-level system prompt | Write `{cwd}/.pi/SYSTEM.md` | Yes | High |

### Claude Code

| # | Approach | Mechanism | Touches cwd? | Clobber risk |
|---|----------|-----------|:---:|:---:|
| 1 | User-level CLAUDE.md | Write `~/.claude/CLAUDE.md` in VM | No | None (VM home is fresh) |
| 2 | User-level rules | Write `~/.claude/rules/agent-os.md` in VM | No | None |
| 3 | CLI flag | Pass `--append-system-prompt <text>` when spawning | No | None |
| 4 | CLI flag (file) | Pass `--append-system-prompt-file <path>` pointing to a file we write | No | None |

### OpenCode

| # | Approach | Mechanism | Touches cwd? | Clobber risk |
|---|----------|-----------|:---:|:---:|
| 1 | Global config with custom contextPaths | Write `$HOME/.opencode.json` with `contextPaths` including our instructions file | No | **See below** |
| 2 | Write instruction file to cwd | Write `CLAUDE.md` at project root (default contextPaths) | Yes | High |
| 3 | Write global config + instruction file | Write `$HOME/.config/opencode/.opencode.json` pointing to instruction file | No | **See below** |

**Validated findings for OpenCode:**
- **Absolute paths do NOT work** in contextPaths. Code does `filepath.Join(workDir, path)` which treats absolute paths as relative. `/home/user/.agent-os/instructions.md` becomes `{cwd}/home/user/.agent-os/instructions.md`.
- **Config arrays are REPLACED, not merged.** Viper's `MergeConfigMap()` does shallow merge — if a project `.opencode.json` has `contextPaths`, it completely overwrites the global config's `contextPaths`.
- **Project config wins over global.** Merge order: defaults → global config → project config (last write wins for arrays).
- **Env var `OPENCODE_CONTEXTPATHS`** exists but also replaces, doesn't append.
- **No `additionalContextPaths` or similar field** exists.

**Consequence**: If we write a global config with a custom contextPath, and the user's mounted project has its own `.opencode.json` with contextPaths, ours gets silently dropped. And since absolute paths don't work, we can't even point to a file outside the project root.

**Best option for OpenCode:**
- **Env var `OPENCODE_CONTEXTPATHS`** — set at spawn time. We construct the value by taking the default contextPaths list, appending our instructions file path, and passing the combined list. This replaces whatever config-level contextPaths exist, but since we include all the defaults, user's standard files (CLAUDE.md, opencode.md, etc.) are still discovered. The instructions file itself is written to a path relative to cwd (e.g., `.agent-os/instructions.md`).
- If the user has a project `.opencode.json` with custom contextPaths, those get overridden by the env var. Acceptable tradeoff — the env var includes all defaults plus ours.

### Codex

| # | Approach | Mechanism | Touches cwd? | Clobber risk |
|---|----------|-----------|:---:|:---:|
| 1 | Global instructions file | Write `~/.codex/AGENTS.md` in VM | No | None (VM home is fresh) |
| 2 | Global override file | Write `~/.codex/AGENTS.override.md` in VM | No | None |
| 3 | Config fallback filenames | Set `project_doc_fallback_filenames` in `~/.codex/config.toml` + write file to cwd | Touches cwd (namespaced) | Low |
| 4 | `CODEX_HOME` env var | Set `CODEX_HOME` to custom dir with our AGENTS.md | No | Loses user config |
| 5 | `instructions` config field | Pass `-c instructions="..."` at spawn | No | **Replaces** built-in system instructions |
| 6 | `developer_instructions` config field | Pass `-c developer_instructions="..."` at spawn | No | None — additive developer role message |

**Validated findings for Codex (corrected):**
- **`project_doc_fallback_filenames` DOES support paths with `/`** (subdirectories). Rust `PathBuf::join()` handles this correctly. So `.agent-os/instructions.md` works — it will look for that relative path at each directory in the git-root-to-cwd walk.
- **`instructions` config field** — sets system instructions but **replaces** the built-in model instructions. Not suitable.
- **`developer_instructions` config field** — injected as a `developer` role message. **Additive**, does not replace anything. Can be set via `-c developer_instructions="..."` at spawn time.
- **No env vars** for custom instruction paths (only `CODEX_HOME` to redirect the entire home dir).
- **`experimental_instructions_file`** / `model_instructions_file` — replaces built-in model instructions. Not suitable.
- **Symlinks don't work** — Codex doesn't follow symlinks to AGENTS.md.
- **`~/.codex/AGENTS.md` is always loaded** regardless of project config.

**Best options for Codex:**
- **#6 (`developer_instructions`)** — cleanest, pass at spawn time via `-c`, additive, no filesystem writes
- **#3 (`project_doc_fallback_filenames`)** — write config to `~/.codex/config.toml` + write `.agent-os/instructions.md` to cwd. Namespaced directory avoids user file collisions, but still touches cwd.
- **#1 (`~/.codex/AGENTS.md`)** — simple, no clobber in fresh VM, always loaded

---

## Selected Approaches

| Agent | Mechanism | Touches cwd? | Clobber risk |
|-------|-----------|:---:|:---:|
| **PI** | `--append-system-prompt <text>` CLI flag | No | None |
| **Claude Code** | `--append-system-prompt <text>` CLI flag | No | None |
| **OpenCode** | `OPENCODE_CONTEXTPATHS` env var (defaults + our path) + write `.agent-os/instructions.md` to cwd | Namespaced dir only | Low — overrides project contextPaths config but includes all defaults |
| **Codex** | `-c developer_instructions="..."` CLI flag | No | None — additive developer role message |

### Notes

- PI and Claude Code are cleanest: direct CLI flag injection, zero filesystem writes.
- Codex `developer_instructions` is injected as a developer role message — additive, no replacement. Passed via the `-c` config override flag at spawn time.
- OpenCode is the only agent that requires a file write. We write to `.agent-os/instructions.md` (namespaced hidden directory, very unlikely to collide with user files). The env var ensures OpenCode discovers it even if the user has custom contextPaths.
- All approaches are disabled when `skipOsInstructions: true` is passed to `createSession()`.
