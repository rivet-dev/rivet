/// Prelude module for easy importing of common API types and handler methods
pub use crate::{ApiCtx, ApiError, GlobalApiCtx};

// Error types
pub use crate::errors::{
	ApiForbidden, ApiInternalError, ApiInvalidToken, ApiNotFound, ApiUnauthorized,
};

// HTTP method handlers
pub use crate::router::ApiRouter;
pub use crate::wrappers::{bin, delete, get, patch, post, put};

// Common types
pub use anyhow::Result;
pub use axum::{
	Router,
	extract::{Extension, Path, Query},
	response::{IntoResponse, Json},
};
pub use serde::{Deserialize, Serialize};
