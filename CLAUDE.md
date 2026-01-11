# CLAUDE.md

## Important Domain Information

**ALWAYS use `rivet.dev` - NEVER use `rivet.gg`**

- API endpoint: `https://api.rivet.dev`
- Cloud API endpoint: `https://cloud-api.rivet.dev`
- Dashboard: `https://hub.rivet.dev`
- Documentation: `https://rivet.dev/docs`

The `rivet.gg` domain is deprecated and should never be used in this codebase.

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

### Docker Development Environment
```bash
# Start the development environment with all services
cd docker/dev
docker-compose up -d
```

### Git Commands
```bash
# When committing changes, use Graphite CLI with conventional commits
# Always use a single-line commit message, no co-author
gt c -m "chore(my-pkg): foo bar"
```

**Never push to `main` unless explicitly specified by the user.**

## Graphite CLI Commands
```bash
# Modify a Graphite PR
gt m
```

## Dependency Management

### pnpm Workspace
- Use pnpm for all npm-related commands. We're using a pnpm workspace.

### SQLite Package
- Use `@rivetkit/sqlite` for SQLite WebAssembly support.
- Do not use the legacy upstream package directly. `@rivetkit/sqlite` is the maintained fork used in this repository and is sourced from `rivet-dev/wa-sqlite`.

### RivetKit Package Resolutions
The root `/package.json` contains `resolutions` that map RivetKit packages to their local workspace versions:

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

When adding RivetKit dependencies to examples in `/examples/`, use `*` as the version. The root resolutions will automatically resolve these to the local workspace packages:

```json
{
  "dependencies": {
    "rivetkit": "*",
    "@rivetkit/react": "*"
  }
}
```

If you need to add a new `@rivetkit/*` package that isn't already in the root resolutions, add it to the `resolutions` object in `/package.json` with `"workspace:*"` as the value. Internal packages like `@rivetkit/workflow-engine` should be re-exported from `rivetkit` subpaths (e.g., `rivetkit/workflow`) rather than added as direct dependencies.

### Rust Dependencies

## Documentation

- If you need to look at the documentation for a package, visit `https://docs.rs/{package-name}`. For example, serde docs live at https://docs.rs/serde/
- When adding new docs pages, update `website/src/sitemap/mod.ts` so the page appears in the sidebar.

## Content Frontmatter

### Docs (`website/src/content/docs/**/*.mdx`)

Required frontmatter fields:

- `title` (string)
- `description` (string)
- `skill` (boolean)

### Blog + Changelog (`website/src/content/posts/**/page.mdx`)

Required frontmatter fields:

- `title` (string)
- `description` (string)
- `author` (enum: `nathan-flurry`, `nicholas-kissel`, `forest-anderson`)
- `published` (date string)
- `category` (enum: `changelog`, `monthly-update`, `launch-week`, `technical`, `guide`, `frogs`)

Optional frontmatter fields:

- `keywords` (string array)

## Examples

All example READMEs in `/examples/` should follow the format defined in `.claude/resources/EXAMPLE_TEMPLATE.md`.

## Architecture

### Monorepo Structure
This is a Rust workspace-based monorepo for Rivet. Key packages and components:

- **Core Engine** (`packages/core/engine/`) - Main orchestration service that coordinates all operations
- **Workflow Engine** (`packages/common/gasoline/`) - Handles complex multi-step operations with reliability and observability
- **Pegboard** (`packages/core/pegboard/`) - Actor/server lifecycle management system
- **Common Packages** (`/packages/common/`) - Foundation utilities, database connections, caching, metrics, logging, health checks, workflow engine core
- **Core Packages** (`/packages/core/`) - Main engine executable, Pegboard actor orchestration, workflow workers
- **Shared Libraries** (`shared/{language}/{package}/`) - Libraries shared between the engine and rivetkit (e.g., `shared/typescript/virtual-websocket/`)
- **Service Infrastructure** - Distributed services communicate via NATS messaging with service discovery

### Important Patterns

**Error Handling**
- Custom error system at `packages/common/error/`
- Uses derive macros with struct-based error definitions

To use custom errors:

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

Key points:
- Use `#[derive(RivetError)]` on struct definitions
- Use `#[error(group, code, description)]` or `#[error(group, code, description, formatted_message)]` attribute
- Group errors by module/domain (e.g., "auth", "actor", "namespace")
- Add `Serialize, Deserialize` derives for errors with metadata fields
- Always return anyhow errors from failable functions
	- For example: `fn foo() -> Result<i64> { /* ... */ }`
- Do not glob import (`::*`) from anyhow. Instead, import individual types and traits

**Rust Dependency Management**
- When adding a dependency, check for a workspace dependency in Cargo.toml
- If available, use the workspace dependency (e.g., `anyhow.workspace = true`)
- If you need to add a dependency and can't find it in the Cargo.toml of the workspace, add it to the workspace dependencies in Cargo.toml (`[workspace.dependencies]`) and then add it to the package you need with `{dependency}.workspace = true`

**Inspector HTTP API**
- When updating the WebSocket inspector (`rivetkit-typescript/packages/rivetkit/src/inspector/`), also update the HTTP inspector endpoints in `rivetkit-typescript/packages/rivetkit/src/actor/router.ts`. The HTTP API mirrors the WebSocket inspector for agent-based debugging.
- When adding or modifying inspector endpoints, also update the driver test at `rivetkit-typescript/packages/rivetkit/src/driver-test-suite/tests/actor-inspector.ts` to cover all inspector HTTP endpoints.
- When adding or modifying inspector endpoints, also update the documentation in `website/src/metadata/skill-base-rivetkit.md` and `website/src/content/docs/actors/debugging.mdx` to keep them in sync.

**Database Usage**
- UniversalDB for distributed state storage
- ClickHouse for analytics and time-series data
- Connection pooling through `packages/common/pools/`

### Code Style
- Hard tabs for Rust formatting (see `rustfmt.toml`)
- Follow existing patterns in neighboring files
- Always check existing imports and dependencies before adding new ones
- **Always add imports at the top of the file inside of inline within the function.**

## Naming Conventions

Data structures often include:

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
- Use tracing for logging. Do not format parameters into the main message, instead use tracing's structured logging. 
  - For example, instead of `tracing::info!("foo {x}")`, do `tracing::info!(?x, "foo")`
- Log messages should be lowercase unless mentioning specific code symbols. For example, `tracing::info!("inserted UserRow")` instead of `tracing::info!("Inserted UserRow")`

## Configuration Management

### Docker Development Configuration
- Do not make changes to docker/dev* configs. Instead, edit the template in docker/template/ and rerun (cd docker/template && pnpm start). This will regenerate the docker compose config for you.

## Development Warnings

- Do not run ./scripts/cargo/fix.sh. Do not format the code yourself.

## Testing Guidelines
- When running tests, always pipe the test to a file in /tmp/ then grep it in a second step. You can grep test logs multiple times to search for different log lines.
- For RivetKit TypeScript tests, run from `rivetkit-typescript/packages/rivetkit` and use `pnpm test <filter>` with `-t` to narrow to specific suites. For example: `pnpm test driver-file-system -t ".*Actor KV.*"`.
- For frontend testing, use the `agent-browser` skill to interact with and test web UIs in examples. This allows automated browser-based testing of frontend applications.

## Optimizations

- Never build a new reqwest client from scratch. Use `rivet_pools::reqwest::client().await?` to access an existing reqwest client instance.

## Documentation

- When talking about "Rivet Actors" make sure to capitalize "Rivet Actor" as a proper noun and lowercase "actor" as a generic noun

### Documentation Sync
When making changes to the engine or RivetKit, ensure the corresponding documentation is updated:
- **Limits changes** (e.g., max message sizes, timeouts): Update `website/src/content/docs/actors/limits.mdx`
- **Config changes** (e.g., new config options in `engine/packages/config/`): Update `website/src/content/docs/self-hosting/configuration.mdx`
- **RivetKit config changes** (e.g., `rivetkit-typescript/packages/rivetkit/src/registry/config/index.ts` or `rivetkit-typescript/packages/rivetkit/src/actor/config.ts`): Update `website/src/content/docs/actors/limits.mdx` if they affect limits/timeouts

### Comments

- Write comments as normal, complete sentences. Avoid fragmented structures with parentheticals and dashes like `// Spawn engine (if configured) - regardless of start kind`. Instead, write `// Spawn the engine if configured`. Especially avoid dashes (hyphens are OK).
- Documenting deltas is not important or useful. A developer who has never worked on the project will not gain extra information if you add a comment stating that something was removed or changed because they don't know what was there before. The only time you would be adding a comment for something NOT being there is if its unintuitive for why its not there in the first place.

### Examples

- When adding new examples, or updating existing ones, ensure that the user also modified the vercel equivalent, if applicable. This ensures parity between local and vercel examples. In order to generate vercel example, run `./scripts/vercel-examples/generate-vercel-examples.ts ` after making changes to examples.
- To skip Vercel generation for a specific example, add `"skipVercel": true` to the `template` object in the example's `package.json`.

#### Common Vercel Example Errors

After regenerating Vercel examples, you may see type check errors like:
```
error TS2688: Cannot find type definition file for 'vite/client'.
```
with warnings about `node_modules missing`. This happens because the regenerated examples need their dependencies reinstalled. Fix by running `pnpm install` before running type checks.
