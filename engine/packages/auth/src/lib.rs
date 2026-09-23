//! Operational authentication for administrator tokens and Rivet JWTs.

mod check;
mod scope;

pub use check::{
	AuthenticatedCredential, CheckInput, CredentialKind, JwtCredential, RequestAuthState,
	authenticate, check,
};
pub use rivet_auth_policy::errors;
pub use rivet_auth_policy::{
	AccessRequest, EffectiveAuthority, Grant, OperationKind, OwnedGrant, ResourceKind, Scope,
	can_delegate, grant_allows, is_authorized,
};
pub use scope::{AccessNamespaceScope, TargetScope};
