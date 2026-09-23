//! Authorization data types and pure policy evaluation.

mod authorize;
pub mod errors;
mod types;

pub use authorize::{can_delegate, grant_allows, is_authorized};
pub use types::{
	AccessRequest, EffectiveAuthority, Grant, OperationKind, OwnedGrant, ResourceKind, Scope,
};
