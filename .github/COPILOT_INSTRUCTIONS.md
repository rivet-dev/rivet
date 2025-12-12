# Copilot Instructions for Rivet

## Overview

Rivet is a Rust workspace-based monorepo for a distributed actor/server orchestration system. This guide provides essential context for working with the codebase.

## Monorepo Structure

Key packages and components:

- **Core Engine** (`engine/packages/engine/`) - Main orchestration service that coordinates all operations
- **Workflow Engine** (`engine/packages/gasoline/`) - Handles complex multi-step operations with reliability and observability
- **Pegboard** (`engine/packages/pegboard/`) - Actor/server lifecycle management system
- **Common Packages** (`engine/packages/`) - Foundation utilities, database connections, caching, metrics, logging, health checks, workflow engine core
- **Service Infrastructure** - Distributed services communicate via NATS messaging with service discovery

## Build Commands

```bash
# Build all packages in the workspace
cargo build

# Build a specific package
cargo build -p package-name

# Build with release optimizations
cargo build --release
```

## Test Commands

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

## Development Commands

```bash
# Run linter and fix issues
./scripts/cargo/fix.sh

# Check for linting issues
cargo clippy -- -W warnings
```

**Note**: Do not run `cargo fmt` automatically. Code formatting is handled by pre-commit hooks (lefthook).

## Error Handling

Custom error system at `engine/packages/error/` using derive macros with struct-based error definitions.

Example usage:

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

Key error handling principles:
- Use `#[derive(RivetError)]` on struct definitions
- Use `#[error(group, code, description)]` or `#[error(group, code, description, formatted_message)]` attribute
- Group errors by module/domain (e.g., "auth", "actor", "namespace")
- Add `Serialize, Deserialize` derives for errors with metadata fields
- Always return anyhow errors from failable functions: `fn foo() -> Result<i64> { /* ... */ }`
- Do not glob import (`::*`) from anyhow. Import individual types and traits instead

## Dependency Management

- When adding a dependency, check for a workspace dependency in `Cargo.toml`
- If available, use the workspace dependency (e.g., `anyhow.workspace = true`)
- If adding a new dependency not in the workspace, add it to `[workspace.dependencies]` in root `Cargo.toml`, then reference it with `{dependency}.workspace = true`
- Use pnpm for all npm-related commands (we use a pnpm workspace)

## Database Usage

- **UniversalDB** for distributed state storage
- **ClickHouse** for analytics and time-series data
- Connection pooling through `engine/packages/pools/`

## Code Style

- Hard tabs for Rust formatting (see `rustfmt.toml`)
- Follow existing patterns in neighboring files
- Always check existing imports and dependencies before adding new ones
- **Always add imports at the top of the file, not inline within functions**

## Naming Conventions

Data structures often include:
- `id` (uuid)
- `name` (machine-readable name, must be valid DNS subdomain, use kebab-case)
- `description` (human-readable, if applicable)

Timestamp naming:
- Use `*_at` with past tense verbs (e.g., `created_at`, `destroyed_at`)

## Data Storage

- Use UUID (v4) for generating unique identifiers
- Store dates as i64 epoch timestamps in milliseconds for precise time tracking

## Logging Patterns

Use tracing for structured logging:

```rust
// Good: structured logging
tracing::info!(?x, "foo");

// Bad: formatting into message
tracing::info!("foo {x}");
```

- Log messages should be lowercase unless mentioning specific code symbols
- Example: `tracing::info!("inserted UserRow")` instead of `tracing::info!("Inserted UserRow")`

## Testing Guidelines

- When running tests, pipe output to a file in `/tmp/` then grep in a second step
- You can grep test logs multiple times to search for different log lines

## Configuration Management

- Do not make changes to `engine/docker/dev*` configs directly
- Instead, edit the template in `engine/docker/template/` and rerun `(cd engine/docker/template && pnpm start)` to regenerate

## Optimizations

- Never build a new reqwest client from scratch
- Use `rivet_pools::reqwest::client().await?` to access an existing reqwest client instance

## Documentation

- When talking about "Rivet Actors", capitalize as a proper noun
- Use lowercase "actor" as a generic noun
- For Rust package documentation, visit `https://docs.rs/{package-name}`

## Examples

All example READMEs in `/examples/` should follow a consistent format. Refer to the example template in the repository for the standard structure.

## Git Workflow

When committing changes, use Graphite CLI with conventional commits:
```bash
gt c -m "chore(my-pkg): foo bar"
```

## Docker Development Environment

```bash
# Start the development environment with all services
cd engine/docker/dev
docker-compose up -d
```
