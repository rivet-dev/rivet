# RivetError system

Full reference for the `rivet_error::RivetError` derive system. The custom error system lives at `packages/common/error/`.

## Derive pattern

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

## Conventions

- Use `#[derive(RivetError)]` on struct definitions.
- Use `#[error(group, code, description)]` or `#[error(group, code, description, formatted_message)]` attribute.
- Group errors by module/domain (e.g., `"auth"`, `"actor"`, `"namespace"`).
- Add `Serialize, Deserialize` derives for errors with metadata fields.

## Generated artifacts

- `RivetError` derives in `rivetkit-core` generate JSON artifacts under `rivetkit-rust/engine/artifacts/errors/`. Commit new generated files together with new error codes.

## anyhow usage

- Always return anyhow errors from failable functions. Example: `fn foo() -> Result<i64> { /* ... */ }`.
- Do not glob import (`::*`) from anyhow. Import individual types and traits.
- Prefer anyhow's `.context()` over the `anyhow!` macro.
