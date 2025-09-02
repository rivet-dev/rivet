# CLAUDE.md

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

# When adding a new package to the workspace
deno run -A scripts/cargo/update_workspace.ts
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
gt c -m "chore(my-pkg): foo bar"
```

## Graphite CLI Commands
```bash
# Modify a Graphite PR
gt m
```

## Dependency Management
- Use pnpm for all npm-related commands. We're using a pnpm workspace.

## Documentation

- If you need to look at the documentation for a package, visit `https://docs.rs/{package-name}`. For example, serde docs live at https://docs.rs/serde/

## Architecture

### Monorepo Structure
This is a Rust workspace-based monorepo for Rivet. Key packages and components:

- **Core Engine** (`packages/core/engine/`) - Main orchestration service that coordinates all operations
- **Workflow Engine** (`packages/common/gasoline/`) - Handles complex multi-step operations with reliability and observability
- **Pegboard** (`packages/core/pegboard/`) - Actor/server lifecycle management system
- **Common Packages** (`/packages/common/`) - Foundation utilities, database connections, caching, metrics, logging, health checks, workflow engine core
- **Core Packages** (`/packages/core/`) - Main engine executable, Pegboard actor orchestration, workflow workers
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
- Import anyhow using `use anyhow::*` instead of importing individual types

**Dependency Management**
- When adding a dependency, check for a workspace dependency in Cargo.toml
- If available, use the workspace dependency (e.g., `anyhow.workspace = true`)
- If you need to add a dependency and can't find it in the Cargo.toml of the workspace, add it to the workspace dependencies in Cargo.toml (`[workspace.dependencies]`) and then add it to the package you need with `{dependency}.workspace = true`

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

## Optimizations

- Never build a new reqwest client from scratch. Use `rivet_pools::reqwest::client().await?` to access an existing reqwest client instance.
