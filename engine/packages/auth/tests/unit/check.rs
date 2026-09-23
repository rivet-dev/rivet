use std::sync::atomic::{AtomicUsize, Ordering};

use rivet_auth_jwt::{Claims, DecodedToken, KeyId, VerifiedJwt};

use super::*;

fn credential(kid_byte: u8) -> AuthenticatedCredential {
	AuthenticatedCredential::Jwt(JwtCredential {
		verified: VerifiedJwt {
			token: DecodedToken {
				claims: Claims {
					rivet_ver: 1,
					iss: "test".into(),
					aud: "rivet-api".into(),
					sub: None,
					iat: 1,
					exp: 2,
					jti: "test".into(),
					rivet_ns: "test".into(),
					rivet_grants: "test".into(),
				},
				namespace_id: Id::new_v1(1),
				grants: Vec::new(),
			},
			kid: KeyId::from_bytes([kid_byte; 16]),
			authorization_deadline: 3,
		},
		authority: EffectiveAuthority::new(Vec::new()),
	})
}

#[tokio::test]
async fn same_token_is_authenticated_once() {
	let state = RequestAuthState::default();
	let calls = AtomicUsize::new(0);
	for _ in 0..2 {
		state
			.credential_for_token_with("jwt-a", || async {
				calls.fetch_add(1, Ordering::Relaxed);
				Ok(credential(1))
			})
			.await
			.unwrap();
	}
	assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn different_token_never_reuses_cached_credential() {
	let state = RequestAuthState::default();
	let first = state
		.credential_for_token_with("jwt-a", || async { Ok(credential(1)) })
		.await
		.unwrap();
	let second = state
		.credential_for_token_with("jwt-b", || async { Ok(credential(2)) })
		.await
		.unwrap();
	assert_ne!(
		first.require_jwt().unwrap().verified().kid,
		second.require_jwt().unwrap().verified().kid,
	);
}

#[tokio::test]
async fn concurrent_authentication_is_single_flight() {
	let state = RequestAuthState::default();
	let calls = Arc::new(AtomicUsize::new(0));
	let first_calls = calls.clone();
	let second_calls = calls.clone();
	let first = state.credential_for_token_with("jwt-a", || async move {
		first_calls.fetch_add(1, Ordering::Relaxed);
		tokio::task::yield_now().await;
		Ok(credential(1))
	});
	let second = state.credential_for_token_with("jwt-a", || async move {
		second_calls.fetch_add(1, Ordering::Relaxed);
		Ok(credential(2))
	});
	let (first, second) = tokio::join!(first, second);
	assert_eq!(
		first.unwrap().require_jwt().unwrap().verified().kid,
		second.unwrap().require_jwt().unwrap().verified().kid,
	);
	assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[test]
fn jwt_cannot_mint_another_token() {
	assert!(credential(1).require_admin_token().is_err());
}

#[test]
fn administrator_token_is_not_a_jwt() {
	assert!(AuthenticatedCredential::AdminToken.require_jwt().is_err());
}
