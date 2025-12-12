# Copilot Instructions for Rivet

## Overview

Rivet is a monorepo containing multiple projects:
- **Engine** - Rust-based distributed actor/server orchestration system
- **Rivetkit** - SDK packages for multiple languages (TypeScript, Rust, Python)
- **Frontend** - React/Vite TypeScript application for the Rivet dashboard
- **Website** - Next.js documentation and marketing site

This guide provides essential context for working with each part of the codebase.

---

# Rust Engine (`engine/`)

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

- Log messages should use lowercase for descriptive text, but keep code symbols (like struct names) capitalized
- Example: `tracing::info!("inserted UserRow")` (lowercase verb, capitalized struct name) instead of `tracing::info!("Inserted UserRow")` (both capitalized)

## Testing Guidelines

- When running tests, pipe output to a file in `/tmp/` then grep in a second step
- You can grep test logs multiple times to search for different log lines

## Configuration Management

### Docker Configuration
- Do not edit `engine/docker/dev*` config files directly
- Instead, edit the template in `engine/docker/template/` and rerun `(cd engine/docker/template && pnpm start)` to regenerate the configs

## Docker Development Environment

To start the development environment (using the generated configs):

```bash
cd engine/docker/dev
docker-compose up -d
```

## Optimizations

- Never build a new reqwest client from scratch
- Use `rivet_pools::reqwest::client().await?` to access an existing reqwest client instance

## Documentation

- When talking about "Rivet Actors", capitalize as a proper noun
- Use lowercase "actor" as a generic noun
- For Rust package documentation, visit `https://docs.rs/{package-name}`

## Examples

All example READMEs in `/examples/` should follow a consistent format. Refer to `.claude/resources/EXAMPLE_TEMPLATE.md` in the repository for the standard structure.

## Git Workflow

When committing changes, use Graphite CLI with conventional commits:
```bash
gt c -m "chore(my-pkg): foo bar"
```

---

# Rivetkit SDK Packages

Rivetkit provides SDK packages for multiple languages located in `rivetkit-*` directories:
- **TypeScript** (`rivetkit-typescript/`) - Client SDK, React hooks, Next.js integration, Cloudflare Workers adapter
- **Rust** (`rivetkit-rust/`) - Client SDK for Rust applications
- **Python** (`rivetkit-python/`) - Client SDK for Python applications

## TypeScript Packages

Located in `rivetkit-typescript/packages/`:
- `rivetkit` - Core TypeScript/JavaScript SDK
- `react` - React hooks and components
- `next-js` - Next.js integration
- `cloudflare-workers` - Cloudflare Workers adapter
- `db` - Database utilities
- `framework-base` - Base framework utilities

### Build Commands

```bash
# From rivetkit-typescript directory
pnpm build

# Build specific package
pnpm --filter @rivetkit/react build
```

### Development

- Follow TypeScript best practices
- Use pnpm for package management
- Maintain type safety across all packages
- Follow existing patterns for API design

## Rust SDK

Located in `rivetkit-rust/packages/client/`:
- Client SDK for Rust applications
- Follows Rust best practices and patterns from engine

### Build Commands

```bash
# From rivetkit-rust directory
cargo build

# Run tests
cargo test
```

---

# Frontend (`frontend/`)

React/Vite TypeScript application for the Rivet dashboard with multiple configurations:
- **Inspector** - Actor inspection interface
- **Engine** - Engine management interface
- **Cloud** - Cloud dashboard interface

## Technology Stack

- **React** - UI framework
- **Vite** - Build tool and dev server
- **TypeScript** - Type-safe JavaScript
- **Tailwind CSS** - Utility-first CSS
- **Radix UI** - Accessible component primitives
- **Clerk** - Authentication
- **CodeMirror** - Code editing components

## Build Commands

```bash
# Development (runs all three apps)
pnpm dev

# Development for specific app
pnpm dev:inspector
pnpm dev:engine
pnpm dev:cloud

# Production builds
pnpm build:inspector
pnpm build:engine
pnpm build:cloud

# Type checking
pnpm ts-check
```

## Development Guidelines

- Follow React best practices and hooks patterns
- Maintain type safety with TypeScript
- Use existing UI components from `@radix-ui` before creating new ones
- Follow Tailwind CSS utility patterns
- Keep components modular and reusable
- Use proper error boundaries

## Code Style

- Use TypeScript for all new files
- Follow existing component structure patterns
- Use functional components with hooks
- Prefer composition over inheritance
- Keep components focused and single-purpose

---

# Website (`website/`)

Next.js-based documentation and marketing site.

## Technology Stack

- **Next.js** - React framework with SSR/SSG
- **TypeScript** - Type-safe JavaScript
- **MDX** - Markdown with JSX for content
- **Tailwind CSS** - Utility-first CSS
- **Giscus** - GitHub discussions-based comments

## Build Commands

```bash
# Development
pnpm dev

# Production build
pnpm build

# Linting
pnpm lint

# Code formatting
pnpm format
```

## Content Generation

The website has several content generation scripts:

```bash
# Generate navigation structure
pnpm gen:navigation

# Generate examples from repository
pnpm gen:examples

# Generate markdown and LLM content
pnpm gen:markdown

# Generate README files
pnpm gen:readme

# Generate TypeDoc API documentation
pnpm gen:typedoc

# Run all generators
pnpm gen
```

## Development Guidelines

- **Avoid `"use client"`** for components with text content for SEO and performance
- If client-side functionality is needed:
  - Add a `useEffect` hook, or
  - Move client-side code to a separate client-only component
- Follow Next.js best practices for SSR/SSG
- Use MDX for documentation content
- Maintain consistent documentation structure
- Ensure all links are valid (use `pnpm lint` to check)

## Content Structure

- Documentation lives in `src/app/(docs)/` 
- Blog posts and articles follow specific formats
- Use frontmatter for metadata
- Follow existing patterns for new content pages

---

# General Guidelines

## Git Workflow

When committing changes, use Graphite CLI with conventional commits:
```bash
gt c -m "chore(my-pkg): foo bar"
```
