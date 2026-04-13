# CLAUDE.md

## Important Domain Information

**ALWAYS use `rivet.dev` - NEVER use `rivet.gg`**

**Never claim "zero cold start" or "no cold start" for agentOS or Rivet Actors.** Always say "near-zero cold start" or specify the actual latency (e.g. "~6 ms cold start"). Cold starts are real, just very small.

- API endpoint: `https://api.rivet.dev`
- Cloud API endpoint: `https://cloud-api.rivet.dev`
- Dashboard: `https://hub.rivet.dev`
- Documentation: `https://rivet.dev/docs`

The `rivet.gg` domain is deprecated and should never be used in this codebase.

**Use "sandbox mounting" when referring to the agentOS sandbox integration.** Do not use "sandbox extension" or "sandbox escalation." The feature mounts a sandbox as a filesystem inside the VM.

**ALWAYS use `github.com/rivet-dev/rivet` - NEVER use `rivet-dev/rivetkit` or `rivet-gg/*`**

**Never modify an existing published `*.bare` runner protocol version unless explicitly asked to do so.**

- Add a new versioned schema instead, then migrate `versioned.rs` and related compatibility code to bridge old versions forward.
- When bumping the protocol version, update `PROTOCOL_MK2_VERSION` in `engine/packages/runner-protocol/src/lib.rs` and `PROTOCOL_VERSION` in `rivetkit-typescript/packages/engine-runner/src/mod.ts` together. Both must match the latest schema version.

## Commands

### Build Commands
```bash
# Build all packages in the workspace
cargo build

# Build a specific package
cargo build -p package-name

# Build with release optimizations
cargo build --release
```

### Test Commands
```bash
# Run all tests in the workspace
cargo test

# Run tests for a specific package
cargo test -p package-name

# Run a specific test
cargo test test_name

# Run tests with output displayed
cargo test -- --nocapture
```

### Development Commands
```bash
# Format code (enforced by pre-commit hooks)
# cargo fmt
# DO NOT RUN CARGO FMT AUTOMATICALLY (note for humans: we need to run cargo fmt when everything is merged together and make sure lefthook is working)

# Run linter and fix issues
./scripts/cargo/fix.sh

# Check for linting issues
cargo clippy -- -W warnings
```

- Ensure lefthook is installed and enabled for git hooks (`lefthook install`).

### Docker Development Environment
```bash
# Start the development environment with all services
cd self-host/compose/dev
docker-compose up -d
```

### Git Commands
```bash
# Use conventional commits with a single-line commit message, no co-author
git commit -m "chore(my-pkg): foo bar"
```

- We use Graphite for stacked PRs. Diff against the parent branch (`gt ls` to see the stack), not `main`.
- To revert a file to the version before this branch's changes, checkout from the first child branch (below in the stack), not from `main` or the parent. Child branches contain the pre-this-branch state of files modified by branches further down the stack.

**Never push to `main` unless explicitly specified by the user.**

## Dependency Management

### pnpm Workspace
- Use pnpm for all npm-related commands. We're using a pnpm workspace.

### TypeScript Concurrency
- Use `antiox` for TypeScript concurrency primitives instead of ad hoc Promise queues, custom channel wrappers, or event-emitter based coordination.
- Prefer the Tokio-shaped APIs from `antiox` for concurrency needs. For example, use `antiox/sync/mpsc` for `tx` and `rx` channels, `antiox/task` for spawning tasks, and the matching sync and time modules as needed.
- Treat `antiox` as the default choice for any TypeScript concurrency work because it mirrors Rust and Tokio APIs used elsewhere in the codebase.

### RivetKit Type Build Troubleshooting
- If `rivetkit` type or DTS builds fail with missing `@rivetkit/*` declarations, run `pnpm build -F rivetkit` from repo root (Turbo build path) before changing TypeScript `paths`.
- Do not add temporary `@rivetkit/*` path aliases in `rivetkit-typescript/packages/rivetkit/tsconfig.json` to work around stale or missing built declarations.

### RivetKit Test Fixtures
- Keep RivetKit test fixtures scoped to the engine-only runtime.
- Prefer targeted integration tests under `rivetkit-typescript/packages/rivetkit/tests/` over shared multi-driver matrices.

### SQLite Package
- RivetKit SQLite runtime is native-only. Use `@rivetkit/rivetkit-native` and do not add `@rivetkit/sqlite`, `@rivetkit/sqlite-vfs`, or other WebAssembly SQLite fallbacks.

### RivetKit Package Resolutions
- The root `/package.json` contains `resolutions` that map RivetKit packages to local workspace versions:

```json
{
  "resolutions": {
    "rivetkit": "workspace:*",
    "@rivetkit/react": "workspace:*",
    "@rivetkit/workflow-engine": "workspace:*",
    // ... other @rivetkit/* packages
  }
}
```

- Use `*` as the dependency version when adding RivetKit packages to `/examples/`, because root resolutions map them to local workspace packages:

```json
{
  "dependencies": {
    "rivetkit": "*",
    "@rivetkit/react": "*"
  }
}
```

- Add new internal `@rivetkit/*` packages to root `resolutions` with `"workspace:*"` if missing, and prefer re-exporting internal packages (for example `@rivetkit/workflow-engine`) from `rivetkit` subpaths like `rivetkit/workflow` instead of direct dependencies.

### Dynamic Import Pattern
- For runtime-only dependencies, use dynamic loading so bundlers do not eagerly include them.
- Build the module specifier from string parts (for example with `["pkg", "name"].join("-")` or `["@scope", "pkg"].join("/")`) instead of a single string literal.
- Prefer this pattern for modules like `@rivetkit/rivetkit-native/wrapper`, `sandboxed-node`, and `isolated-vm`.
- If loading by resolved file path, resolve first and then import via `pathToFileURL(...).href`.

### Fail-By-Default Runtime Behavior
- Avoid silent no-ops for required runtime behavior.
- Do not use optional chaining for required lifecycle and bridge operations (for example sleep, destroy, alarm dispatch, ack, and websocket dispatch paths).
- If a capability is required, validate it and throw an explicit error with actionable context instead of returning early.
- Optional chaining is acceptable only for best-effort diagnostics and cleanup paths (for example logging hooks and dispose/release cleanup).

### Rust Dependencies

## Documentation

- If you need to look at the documentation for a package, visit `https://docs.rs/{package-name}`. For example, serde docs live at https://docs.rs/serde/
- When adding new docs pages, update `website/src/sitemap/mod.ts` so the page appears in the sidebar.
- When changing actor/runtime limits or behavior that affects documented limits (for example KV, queue, SQLite, WebSocket, HTTP, or timeouts), update `website/src/content/docs/actors/limits.mdx` in the same change.

## Code Blocks in Docs

- All TypeScript code blocks in docs are typechecked during the website build. They must be valid, compilable TypeScript.
- Use `<CodeGroup workspace>` only when showing multiple related files together (e.g., `actors.ts` + `client.ts`). For a single file, use a standalone fenced code block.
- Code blocks are extracted and typechecked via `website/src/integrations/typecheck-code-blocks.ts`. Add `@nocheck` to the code fence to skip typechecking for a block.

## Content Frontmatter

### Docs (`website/src/content/docs/**/*.mdx`)

- Required frontmatter fields:

- `title` (string)
- `description` (string)
- `skill` (boolean)

### Blog + Changelog (`website/src/content/posts/**/page.mdx`)

- Required frontmatter fields:

- `title` (string)
- `description` (string)
- `author` (enum: `nathan-flurry`, `nicholas-kissel`, `forest-anderson`)
- `published` (date string)
- `category` (enum: `changelog`, `monthly-update`, `launch-week`, `technical`, `guide`, `frogs`)

- Optional frontmatter fields:

- `keywords` (string array)

## Examples

- All example READMEs in `/examples/` should follow the format defined in `.claude/resources/EXAMPLE_TEMPLATE.md`.

## Agent Working Directory

All agent working files live in `.agent/` at the repo root.

- **Specs**: `.agent/specs/` -- design specs and interface definitions for planned work.
- **Research**: `.agent/research/` -- research documents on external systems, prior art, and design analysis.
- **Todo**: `.agent/todo/*.md` -- deferred work items with context on what needs to be done and why.
- **Notes**: `.agent/notes/` -- general notes and tracking.

When the user asks to track something in a note, store it in `.agent/notes/` by default. When something is identified as "do later", add it to `.agent/todo/`. Design documents and interface specs go in `.agent/specs/`.
- When the user asks to update any `CLAUDE.md`, add one-line bullet points only, or add a new section containing one-line bullet points.

## Architecture

### Monorepo Structure
- This is a Rust workspace-based monorepo for Rivet with the following key packages and components:

- **Core Engine** (`packages/core/engine/`) - Main orchestration service that coordinates all operations
- **Workflow Engine** (`packages/common/gasoline/`) - Handles complex multi-step operations with reliability and observability
- **Pegboard** (`packages/core/pegboard/`) - Actor/server lifecycle management system
- **Common Packages** (`/packages/common/`) - Foundation utilities, database connections, caching, metrics, logging, health checks, workflow engine core
- **Core Packages** (`/packages/core/`) - Main engine executable, Pegboard actor orchestration, workflow workers
- **Shared Libraries** (`shared/{language}/{package}/`) - Libraries shared between the engine and rivetkit (e.g., `shared/typescript/virtual-websocket/`)
- **Service Infrastructure** - Distributed services communicate via NATS messaging with service discovery

### Engine Runner Parity
- Keep `engine/sdks/typescript/runner` and `engine/sdks/rust/engine-runner` at feature parity.
- Any behavior, protocol handling, or test coverage added to one runner should be mirrored in the other runner in the same change whenever possible.
- When parity cannot be completed in the same change, explicitly document the gap and add a follow-up task.

### Trust Boundaries
- Treat `client <-> engine` as untrusted.
- Treat `envoy <-> pegboard-envoy` as untrusted.
- Treat traffic inside the engine over `nats`, `fdb`, and other internal backends as trusted.
- Treat `gateway`, `api`, `pegboard-envoy`, `nats`, `fdb`, and similar engine-internal services as one trusted internal boundary once traffic is inside the engine.
- Validate and authorize all client-originated data at the engine edge before it reaches trusted internal systems.
- Validate and authorize all envoy-originated data at `pegboard-envoy` before it reaches trusted internal systems.

### Important Patterns

**Error Handling**
- Custom error system at `packages/common/error/`
- Uses derive macros with struct-based error definitions

- Use this pattern for custom errors:

```rust
use rivet_error::*;
use serde::{Serialize, Deserialize};

// Simple error without metadata
#[derive(RivetError)]
#[error("auth", "invalid_token", "The provided authentication token is invalid")]
struct AuthInvalidToken;

// Error with metadata
#[derive(RivetError, Serialize, Deserialize)]
#[error(
    "api",
    "rate_limited",
    "Rate limit exceeded",
    "Rate limit exceeded. Limit: {limit}, resets at: {reset_at}"
)]
struct ApiRateLimited {
    limit: u32,
    reset_at: i64,
}

// Use errors in code
let error = AuthInvalidToken.build();
let error_with_meta = ApiRateLimited { limit: 100, reset_at: 1234567890 }.build();
```

- Key points:
- Use `#[derive(RivetError)]` on struct definitions
- Use `#[error(group, code, description)]` or `#[error(group, code, description, formatted_message)]` attribute
- Group errors by module/domain (e.g., "auth", "actor", "namespace")
- Add `Serialize, Deserialize` derives for errors with metadata fields
- Always return anyhow errors from failable functions
- For example: `fn foo() -> Result<i64> { /* ... */ }`
- Do not glob import (`::*`) from anyhow. Instead, import individual types and traits
- Prefer anyhow's `.context()` over `anyhow!` macro

**Rust Dependency Management**
- When adding a dependency, check for a workspace dependency in Cargo.toml
- If available, use the workspace dependency (e.g., `anyhow.workspace = true`)
- If you need to add a dependency and can't find it in the Cargo.toml of the workspace, add it to the workspace dependencies in Cargo.toml (`[workspace.dependencies]`) and then add it to the package you need with `{dependency}.workspace = true`

**Native SQLite & KV Channel**
- RivetKit SQLite is served by `@rivetkit/rivetkit-native`. Do not reintroduce SQLite-over-KV or WebAssembly SQLite paths in the TypeScript runtime.
- The Rust KV-backed SQLite implementation still lives in `rivetkit-typescript/packages/sqlite-native/src/`. When changing its on-disk or KV layout, update the internal data-channel spec in the same change.
- Full spec: `docs-internal/engine/NATIVE_SQLITE_DATA_CHANNEL.md`

**Inspector HTTP API**
- When updating the WebSocket inspector (`rivetkit-typescript/packages/rivetkit/src/inspector/`), also update the HTTP inspector endpoints in `rivetkit-typescript/packages/rivetkit/src/actor/router.ts`. The HTTP API mirrors the WebSocket inspector for agent-based debugging.
- When adding or modifying inspector endpoints, also update the relevant RivetKit tests in `rivetkit-typescript/packages/rivetkit/tests/` to cover all inspector HTTP endpoints.
- When adding or modifying inspector endpoints, also update the documentation in `website/src/metadata/skill-base-rivetkit.md` and `website/src/content/docs/actors/debugging.mdx` to keep them in sync.

**Database Usage**
- UniversalDB for distributed state storage
- ClickHouse for analytics and time-series data
- Connection pooling through `packages/common/pools/`

**Performance**
- Never use `Mutex<HashMap<...>>` or `RwLock<HashMap<...>>`.
- Use `scc::HashMap` (preferred), `moka::Cache` (for TTL/bounded), or `DashMap` for concurrent maps.
- Use `scc::HashSet` instead of `Mutex<HashSet<...>>` for concurrent sets.
- `scc` async methods do not hold locks across `.await` points. Use `entry_async` for atomic read-then-write.

### Code Style
- Hard tabs for Rust formatting (see `rustfmt.toml`)
- Follow existing patterns in neighboring files
- Always check existing imports and dependencies before adding new ones
- **Always add imports at the top of the file inside of inline within the function.**

## Naming Conventions

- Data structures often include:

- `id` (uuid)
- `name` (machine-readable name, must be valid DNS subdomain, convention is using kebab case)
- `description` (human-readable, if applicable)

## Implementation Details

### Data Storage Conventions
- Use UUID (v4) for generating unique identifiers
- Store dates as i64 epoch timestamps in milliseconds for precise time tracking

### Timestamp Naming Conventions
- When storing timestamps, name them *_at with past tense verb. For example, created_at, destroyed_at.

## Logging Patterns

### Structured Logging
- Use tracing for logging. Never use `eprintln!` or `println!` for logging in Rust code. Always use tracing macros (`tracing::info!`, `tracing::warn!`, `tracing::error!`, etc.).
- Do not format parameters into the main message, instead use tracing's structured logging.
  - For example, instead of `tracing::info!("foo {x}")`, do `tracing::info!(?x, "foo")`
- Log messages should be lowercase unless mentioning specific code symbols. For example, `tracing::info!("inserted UserRow")` instead of `tracing::info!("Inserted UserRow")`

## Configuration Management

### Docker Development Configuration
- Do not make changes to self-host/compose/dev* configs. Instead, edit the template in self-host/compose/template/ and rerun (cd self-host/compose/template && pnpm start). This will regenerate the docker compose config for you.

## Development Warnings

- Do not run ./scripts/cargo/fix.sh. Do not format the code yourself.
- When adding or changing any version value in the repo, verify `scripts/publish/src/lib/version.ts` (`bumpPackageJsons` for package.json files, `updateSourceFiles` for Cargo.toml + examples) updates that location so release bumps cannot leave stale versions behind.

## Testing Guidelines
- **Never use `vi.mock`, `jest.mock`, or module-level mocking.** Write tests against real infrastructure (Docker containers, real databases, real filesystems). For LLM calls, use `@copilotkit/llmock` to run a mock LLM server. For protocol-level test doubles (e.g., ACP adapters), write hand-written scripts that run as real processes. If you need callback tracking, `vi.fn()` for simple callbacks is acceptable.
- When running tests, always pipe the test to a file in /tmp/ then grep it in a second step. You can grep test logs multiple times to search for different log lines.
- For RivetKit TypeScript tests, run from `rivetkit-typescript/packages/rivetkit` and use `pnpm test <filter>` with `-t` to narrow to specific suites. For example: `pnpm test driver-file-system -t ".*Actor KV.*"`.
- When RivetKit tests need a local engine instance, start the RocksDB engine in the background with `./scripts/run/engine-rocksdb.sh >/tmp/rivet-engine-startup.log 2>&1 &`.
- For frontend testing, use the `agent-browser` skill to interact with and test web UIs in examples. This allows automated browser-based testing of frontend applications.
- If you modify frontend UI, automatically use the Agent Browser CLI to take updated screenshots and post them to the PR with a short comment before wrapping up the task.

## Optimizations

- Never build a new reqwest client from scratch. Use `rivet_pools::reqwest::client().await?` to access an existing reqwest client instance.

## Documentation

- When talking about "Rivet Actors" make sure to capitalize "Rivet Actor" as a proper noun and lowercase "actor" as a generic noun

### Documentation Sync
- Ensure corresponding documentation is updated when making engine or RivetKit changes:
- **Limits changes** (e.g., max message sizes, timeouts): Update `website/src/content/docs/actors/limits.mdx`
- **Config changes** (e.g., new config options in `engine/packages/config/`): Update `website/src/content/docs/self-hosting/configuration.mdx`
- **RivetKit config changes** (e.g., `rivetkit-typescript/packages/rivetkit/src/registry/config/index.ts` or `rivetkit-typescript/packages/rivetkit/src/actor/config.ts`): Update `website/src/content/docs/actors/limits.mdx` if they affect limits/timeouts
- **Actor error changes**: When adding, removing, or modifying variants in `ActorError` (`engine/packages/types/src/actor/error.rs`) or `RunnerPoolError`, update `website/src/content/docs/actors/troubleshooting.mdx` to keep the Error Reference in sync. Each error should document the dashboard message (from `frontend/src/components/actors/actor-status-label.tsx`) and the API JSON shape.
- **Actor status changes**: When modifying status derivation logic in `frontend/src/components/actors/queries/index.ts` or adding new statuses, update `website/src/content/docs/actors/statuses.mdx` and the corresponding tests in `frontend/src/components/actors/queries/index.test.ts`.
- **Kubernetes manifest changes**: When modifying k8s manifests in `self-host/k8s/engine/`, update `website/src/content/docs/self-hosting/kubernetes.mdx`, `self-host/k8s/README.md`, and `scripts/run/k8s/engine.sh` if file names or deployment steps change.
- **Landing page changes**: When updating the landing page (`website/src/pages/index.astro` and its section components in `website/src/components/marketing/sections/`), update `README.md` to reflect the same headlines, features, benchmarks, and talking points where applicable.
- **Sandbox provider changes**: When adding, removing, or modifying sandbox providers in `rivetkit-typescript/packages/rivetkit/src/sandbox/providers/`, update `website/src/content/docs/actors/sandbox.mdx` to keep provider documentation, option tables, and custom provider guidance in sync.

### CLAUDE.md conventions

- When adding entries to any CLAUDE.md file, keep them concise. Ideally a single bullet point or minimal bullet points. Do not write paragraphs.

### Comments

- Write comments as normal, complete sentences. Avoid fragmented structures with parentheticals and dashes like `// Spawn engine (if configured) - regardless of start kind`. Instead, write `// Spawn the engine if configured`. Especially avoid dashes (hyphens are OK).
- Do not use em dashes (—). Use periods to separate sentences instead.
- Documenting deltas is not important or useful. A developer who has never worked on the project will not gain extra information if you add a comment stating that something was removed or changed because they don't know what was there before. The only time you would be adding a comment for something NOT being there is if its unintuitive for why its not there in the first place.

### Examples

- When adding new examples, or updating existing ones, ensure that the user also modified the vercel equivalent, if applicable. This ensures parity between local and vercel examples. In order to generate vercel example, run `./scripts/vercel-examples/generate-vercel-examples.ts ` after making changes to examples.
- To skip Vercel generation for a specific example, add `"skipVercel": true` to the `template` object in the example's `package.json`.

#### Common Vercel Example Errors

- You may see type-check errors like the following after regenerating Vercel examples:
```
error TS2688: Cannot find type definition file for 'vite/client'.
```
- You may also see `node_modules missing` warnings; fix this by running `pnpm install` before type checks because regenerated examples need dependencies reinstalled.
