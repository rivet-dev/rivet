use rivet_auth::{OperationKind, OwnedGrant, ResourceKind, Scope};
use rivet_auth_jwt::{Claims, DecodedToken, KeyId, VerifiedJwt};
use rivet_util::Id;

use super::*;

#[test]
fn expiration_uses_requested_duration() {
	assert_eq!(bound_expiration(1_000, 60).unwrap(), 61_000);
}

#[test]
fn inspection_returns_only_safe_verified_metadata() {
	let namespace_id = Id::new_v1(1);
	let actor_id = Id::new_v1(1);
	let response = inspect_response(&VerifiedJwt {
		token: DecodedToken {
			claims: Claims {
				rivet_ver: rivet_auth_jwt::CLAIMS_VERSION,
				iss: "https://api.rivet.dev".into(),
				aud: "rivet-api".into(),
				sub: Some("user-123".into()),
				iat: 100,
				exp: 200,
				jti: "not-returned".into(),
				rivet_ns: namespace_id.to_string(),
				rivet_grants: "not-returned".into(),
			},
			namespace_id,
			grants: vec![OwnedGrant {
				namespace: Scope::Id(namespace_id),
				resource: ResourceKind::Actor,
				target: Scope::Id(actor_id),
				operations: vec![OperationKind::Read],
			}],
		},
		kid: KeyId::from_bytes([7; 16]),
		authorization_deadline: 230,
	})
	.unwrap();

	assert_eq!(response.namespace_id, namespace_id);
	assert_eq!(response.subject.as_deref(), Some("user-123"));
	assert_eq!(response.issued_ts, 100_000);
	assert_eq!(response.expires_ts, 200_000);
	assert_eq!(response.grants.len(), 1);
	assert_eq!(
		response.grants[0].resource,
		rivet_api_types::auth::tokens::Resource::Actor
	);
	assert_eq!(
		response.grants[0].target,
		rivet_api_types::auth::tokens::TargetScope::Id(actor_id)
	);
	assert_eq!(response.grants[0].operations, vec![OperationKind::Read]);
}
