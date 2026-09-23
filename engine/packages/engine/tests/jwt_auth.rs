use std::{future::Future, time::Duration};

use reqwest::{Client, Response, StatusCode};
use serde_json::{Value, json};

#[path = "common/api/mod.rs"]
mod api;
#[path = "common/ctx.rs"]
mod ctx;

const ADMIN_TOKEN: &str = "jwt-test-admin";

#[test]
fn issues_and_inspects_jwts_from_a_non_writer_datacenter() {
	run(
		ctx::TestOpts::new(2)
			.with_auth_admin_token(ADMIN_TOKEN)
			.with_timeout(90),
		|ctx| async move {
			let namespace = format!("jwt-test-{}", rand::random::<u16>());
			let created = api::peer::namespaces_create(
				ctx.leader_dc().api_peer_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: namespace.clone(),
					display_name: "JWT Test".into(),
				},
			)
			.await
			.expect("failed to create JWT test namespace");

			let client = Client::new();
			let follower_base = format!("http://127.0.0.1:{}", ctx.get_dc(2).guard_port());
			let issue_body = json!({
				"namespace": namespace,
				"duration": 60,
				"subject": "end-user-123",
				"grants": [{
					"resource": "actor",
					"target": "any",
					"operations": ["list"]
				}]
			});

			let issued = retry_success(|| {
				client
					.post(format!("{follower_base}/auth/tokens"))
					.bearer_auth(ADMIN_TOKEN)
					.json(&issue_body)
			})
			.await
			.json::<rivet_api_types::auth::tokens::CreateResponse>()
			.await
			.expect("failed to decode JWT issuance response");

			let inspected = client
				.get(format!("{follower_base}/auth/tokens/inspect"))
				.bearer_auth(&issued.token)
				.send()
				.await
				.expect("failed to inspect JWT");
			assert_eq!(inspected.status(), StatusCode::OK);
			let inspected = inspected
				.json::<rivet_api_types::auth::tokens::InspectResponse>()
				.await
				.expect("failed to decode JWT inspection response");
			assert_eq!(inspected.namespace_id, created.namespace.namespace_id);
			assert_eq!(inspected.subject.as_deref(), Some("end-user-123"));
			assert_eq!(inspected.grants.len(), 1);
			assert_eq!(inspected.issued_ts, issued.issued_ts);
			assert_eq!(inspected.expires_ts, issued.expires_ts);

			let actor_list = client
				.get(format!("{follower_base}/actors/names"))
				.bearer_auth(&issued.token)
				.query(&[("namespace", namespace.as_str())])
				.send()
				.await
				.expect("failed to list actors with a JWT");
			assert_eq!(actor_list.status(), StatusCode::OK);

			let insufficient_body = json!({
				"namespace": namespace,
				"duration": 60,
				"grants": [{
					"resource": "actor",
					"target": "any",
					"operations": ["read"]
				}]
			});
			let insufficient = retry_success(|| {
				client
					.post(format!("{follower_base}/auth/tokens"))
					.bearer_auth(ADMIN_TOKEN)
					.json(&insufficient_body)
			})
			.await
			.json::<rivet_api_types::auth::tokens::CreateResponse>()
			.await
			.expect("failed to decode insufficient JWT issuance response");
			let insufficient = client
				.get(format!("{follower_base}/actors/names"))
				.bearer_auth(&insufficient.token)
				.query(&[("namespace", namespace.as_str())])
				.send()
				.await
				.expect("failed to test an insufficient JWT grant");
			let error = assert_error_response(
				insufficient,
				StatusCode::FORBIDDEN,
				"insufficient_permissions",
			)
			.await;
			assert_eq!(error["group"], "auth");

			let unknown_token = client
				.get(format!("{follower_base}/actors/names"))
				.bearer_auth("unknown-opaque-token")
				.query(&[("namespace", namespace.as_str())])
				.send()
				.await
				.expect("failed to test an unknown token");
			let error =
				assert_error_response(unknown_token, StatusCode::UNAUTHORIZED, "invalid_token")
					.await;
			assert_eq!(error["group"], "auth");

			let admin_on_jwt_endpoint = client
				.get(format!("{follower_base}/auth/tokens/inspect"))
				.bearer_auth(ADMIN_TOKEN)
				.send()
				.await
				.expect("failed to test administrator token on JWT inspection");
			let error = assert_error_response(
				admin_on_jwt_endpoint,
				StatusCode::UNAUTHORIZED,
				"invalid_token",
			)
			.await;
			assert_eq!(error["group"], "auth");

			let jwt_reissuance = client
				.post(format!("{follower_base}/auth/tokens"))
				.bearer_auth(&issued.token)
				.json(&issue_body)
				.send()
				.await
				.expect("failed to test JWT reissuance");
			let error = assert_error_response(
				jwt_reissuance,
				StatusCode::FORBIDDEN,
				"insufficient_permissions",
			)
			.await;
			assert_eq!(error["group"], "auth");

			let malformed_jwt = client
				.get(format!("{follower_base}/auth/tokens/inspect"))
				.bearer_auth("eyJ0eXAiOiJyaXZldC1hdXRoK2p3dCIsImFsZyI6IkVkRFNBIn0.invalid.invalid")
				.send()
				.await
				.expect("failed to inspect malformed JWT");
			let error =
				assert_error_response(malformed_jwt, StatusCode::UNAUTHORIZED, "invalid_token")
					.await;
			assert_eq!(error["group"], "auth");

			let old_route = client
				.post(format!(
					"{follower_base}/namespaces/{}/auth/tokens",
					created.namespace.name
				))
				.bearer_auth(ADMIN_TOKEN)
				.json(&issue_body)
				.send()
				.await
				.expect("failed to test removed namespace-prefixed JWT route");
			assert_eq!(old_route.status(), StatusCode::NOT_FOUND);
		},
	);
}

fn run<F, Fut>(opts: ctx::TestOpts, test_fn: F)
where
	F: FnOnce(ctx::TestCtx) -> Fut,
	Fut: Future<Output = ()>,
{
	let runtime = tokio::runtime::Runtime::new().expect("failed to build runtime");
	runtime.block_on(async {
		let timeout = Duration::from_secs(opts.timeout_secs);
		let ctx = ctx::TestCtx::new_with_opts(opts)
			.await
			.expect("build testctx");
		tokio::time::timeout(timeout, test_fn(ctx))
			.await
			.expect("test timed out");
	});
}

async fn assert_error_response(
	response: Response,
	expected_status: StatusCode,
	expected_error_code: &str,
) -> Value {
	assert_eq!(response.status(), expected_status);
	let body = response
		.json::<Value>()
		.await
		.expect("failed to decode error response");
	assert_eq!(body["code"], expected_error_code);
	body
}

async fn retry_success(build: impl Fn() -> reqwest::RequestBuilder) -> Response {
	let mut last_failure = None;
	for _ in 0..100 {
		let response = build().send().await.expect("JWT issuance request failed");
		if response.status().is_success() {
			return response;
		}
		let status = response.status();
		let body = response.text().await.unwrap_or_default();
		last_failure = Some((status, body));
		tokio::time::sleep(Duration::from_millis(100)).await;
	}
	panic!("JWT issuance did not become ready: {last_failure:?}");
}
