#[path = "common/mod.rs"]
mod common;

use reqwest::{Client, StatusCode};
use serde_json::json;

#[test]
fn legacy_actor_access_keeps_jwt_endpoints_protected() {
	common::run(
		common::TestOpts::new(1)
			.with_insecure_allow_unauthenticated()
			.with_timeout(90),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let namespace = format!("legacy-auth-{:016x}", rand::random::<u64>());
			common::api::peer::namespaces_create(
				dc.api_peer_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: namespace.clone(),
					display_name: "Legacy auth test".into(),
				},
			)
			.await
			.expect("failed to create test namespace");

			let client = Client::new();
			let base = format!("http://127.0.0.1:{}", dc.guard_port());
			let actor_list = client
				.get(format!("{base}/actors/names"))
				.query(&[("namespace", namespace.as_str())])
				.send()
				.await
				.expect("failed to list actors without a token");
			assert_eq!(actor_list.status(), StatusCode::OK);

			let issue_body = json!({ "namespace": namespace, "grants": [] });
			let unauthenticated_issue = client
				.post(format!("{base}/auth/tokens"))
				.json(&issue_body)
				.send()
				.await
				.expect("failed to request an unauthenticated token");
			assert_eq!(unauthenticated_issue.status(), StatusCode::UNAUTHORIZED);

			let unauthenticated_inspect = client
				.get(format!("{base}/auth/tokens/inspect"))
				.send()
				.await
				.expect("failed to inspect an unauthenticated token");
			assert_eq!(unauthenticated_inspect.status(), StatusCode::UNAUTHORIZED);
		},
	);
}
