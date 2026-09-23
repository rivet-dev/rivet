#[path = "common/mod.rs"]
mod common;

use std::{collections::HashMap, time::Duration};

use futures_util::{SinkExt, StreamExt};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use tokio_tungstenite::{
	connect_async,
	tungstenite::{Message, client::IntoClientRequest},
};

#[test]
fn scoped_jwt_connects_envoy_and_actor_gateway() {
	let admin_token = format!("jwt-transport-admin-{:016x}", rand::random::<u64>());
	common::run(
		common::TestOpts::new(1)
			.with_auth_admin_token(admin_token.clone())
			.with_timeout(90),
		move |ctx| async move {
			let dc = ctx.leader_dc();
			let namespace = format!("jwt-transport-{:016x}", rand::random::<u64>());
			let created_namespace = common::api::peer::namespaces_create(
				dc.api_peer_port(),
				rivet_api_peer::namespaces::CreateRequest {
					name: namespace.clone(),
					display_name: "JWT Transport Test".into(),
				},
			)
			.await
			.expect("failed to create test namespace");

			let client = Client::new();
			let base = format!("http://127.0.0.1:{}", dc.guard_port());
			let runner_token = issue_jwt(
				&client,
				&base,
				&admin_token,
				&namespace,
				json!([{ "resource": "runner", "target": "any", "operations": ["create"] }]),
			)
			.await;

			let runner_name = "jwt-test-envoy";
			let mut datacenters = HashMap::new();
			datacenters.insert(
				dc.config.dc_name().expect("datacenter name").to_string(),
				rivet_api_types::namespaces::runner_configs::RunnerConfig {
					kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Normal {
						drain_on_version_upgrade: None,
						actor_eviction_delay: None,
						actor_eviction_period: None,
						actor_eviction_rate: None,
					},
					metadata: None,
					drain_on_version_upgrade: Some(true),
				},
			);
			let upsert = common::api::public::build_runner_configs_upsert_request(
				dc.guard_port(),
				rivet_api_peer::runner_configs::UpsertPath {
					runner_name: runner_name.into(),
				},
				rivet_api_peer::runner_configs::UpsertQuery {
					namespace: namespace.clone(),
				},
				rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
			)
			.await
			.expect("failed to build runner config request")
			.bearer_auth(&admin_token)
			.send()
			.await
			.expect("failed to upsert runner config");
			assert_eq!(upsert.status(), StatusCode::OK);

			let envoy = common::test_envoy::TestEnvoyBuilder::new(&namespace)
				.with_pool_name(runner_name)
				.with_token(runner_token.clone())
				.with_actor_behavior("test-actor", |_| {
					Box::new(common::test_envoy::EchoActor::new())
				})
				.build(dc)
				.await
				.expect("failed to build test envoy");
			envoy.start().await.expect("scoped JWT must connect Envoy");
			envoy.wait_ready().await;

			let actor = common::api::public::build_actors_create_request(
				dc.guard_port(),
				rivet_api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				rivet_api_types::actors::create::CreateRequest {
					datacenter: None,
					name: "test-actor".into(),
					key: Some("jwt-key".into()),
					input: None,
					runner_name_selector: runner_name.into(),
					crash_policy: rivet_types::actors::CrashPolicy::Sleep,
				},
			)
			.await
			.expect("failed to build actor creation request")
			.bearer_auth(&admin_token)
			.send()
			.await
			.expect("failed to create test actor");
			assert_eq!(actor.status(), StatusCode::OK);
			let actor = actor
				.json::<rivet_api_types::actors::create::CreateResponse>()
				.await
				.expect("failed to decode actor creation response");
			let actor_id = actor.actor.actor_id.to_string();
			tokio::time::timeout(Duration::from_secs(10), async {
				while !envoy.has_actor(&actor_id).await {
					tokio::time::sleep(Duration::from_millis(50)).await;
				}
			})
			.await
			.expect("test actor did not start");

			let client_token = issue_jwt(
				&client,
				&base,
				&admin_token,
				&namespace,
				json!([
					{ "resource": "actor", "target": "any", "operations": ["read"] },
					{ "resource": "actor_gateway", "target": { "id": actor_id }, "operations": ["read"] }
				]),
			)
			.await;

			for (key, token, expected_code) in [
				("missing-auth", None, "invalid_token"),
				("invalid-auth", Some("invalid-token"), "invalid_token"),
				(
					"read-only-auth",
					Some(client_token.as_str()),
					"insufficient_permissions",
				),
			] {
				let mut request = client
					.get(format!("{base}/gateway/test-actor/ping"))
					.query(&[
						("rvt-namespace", namespace.as_str()),
						("rvt-method", "getOrCreate"),
						("rvt-runner", runner_name),
						("rvt-key", key),
					]);
				if let Some(token) = token {
					request = request.header("x-rivet-token", token);
				}
				let response = request.send().await.expect("failed to request query actor");
				assert!(!response.status().is_success());
				let error: Value = response.json().await.expect("invalid gateway error");
				assert_eq!(error["code"], expected_code);

				let actor = dc
					.workflow_ctx
					.op(pegboard::ops::actor::get_for_key::Input {
						namespace_id: created_namespace.namespace.namespace_id,
						name: "test-actor".into(),
						key: key.into(),
						pool_name: Some(runner_name.into()),
						fetch_error: false,
					})
					.await
					.expect("failed to check for unauthorized actor creation");
				assert!(
					matches!(actor, pegboard::ops::actor::get_for_key::Output::NotFound),
					"unauthorized query created an actor: {actor:?}"
				);
			}

			let response = client
				.get(format!("{base}/request/ping"))
				.header("x-rivet-target", "actor")
				.header("x-rivet-actor", &actor_id)
				.header("x-rivet-token", &client_token)
				.send()
				.await
				.expect("failed to request actor through token header");
			let status = response.status();
			let body = response.text().await.expect("invalid actor response");
			assert_eq!(status, StatusCode::OK, "{body}");
			let body: Value = serde_json::from_str(&body).expect("invalid actor response");
			assert_eq!(body["status"], "ok");

			let missing_actor = client
				.get(format!("{base}/request/ping"))
				.header("x-rivet-target", "actor")
				.header("x-rivet-token", &client_token)
				.send()
				.await
				.expect("failed to test missing actor header");
			assert_eq!(missing_actor.status(), StatusCode::BAD_REQUEST);
			let error: Value = missing_actor.json().await.expect("invalid gateway error");
			assert_eq!(error["group"], "guard");
			assert_eq!(error["code"], "missing_header");

			let response = client
				.get(format!("{base}/gateway/{actor_id}@{client_token}/ping"))
				.send()
				.await
				.unwrap_or_else(|_| panic!("failed to request actor through token URL"));
			assert_eq!(response.status(), StatusCode::OK);
			let body: Value = response.json().await.expect("invalid actor response");
			assert_eq!(body["status"], "ok");

			let missing_namespace = client
				.get(format!("{base}/gateway/test-actor/ping"))
				.query(&[("rvt-method", "get"), ("rvt-token", client_token.as_str())])
				.send()
				.await
				.expect("failed to test missing namespace query parameter");
			assert_eq!(missing_namespace.status(), StatusCode::BAD_REQUEST);
			let error: Value = missing_namespace
				.json()
				.await
				.expect("invalid gateway error");
			assert_eq!(error["group"], "guard");
			assert_eq!(error["code"], "query_invalid_params");

			let response = client
				.get(format!("{base}/gateway/test-actor/ping"))
				.query(&[
					("rvt-namespace", namespace.as_str()),
					("rvt-method", "get"),
					("rvt-key", "jwt-key"),
					("rvt-token", client_token.as_str()),
				])
				.send()
				.await
				.unwrap_or_else(|_| panic!("failed to request actor through query token"));
			assert_eq!(response.status(), StatusCode::OK);
			let body: Value = response.json().await.expect("invalid actor response");
			assert_eq!(body["status"], "ok");

			let ws_url = format!("ws://127.0.0.1:{}/ws", dc.guard_port());
			let mut request = ws_url.into_client_request().expect("invalid WebSocket URL");
			request.headers_mut().insert(
				"Sec-WebSocket-Protocol",
				format!(
					"rivet, rivet_target.actor, rivet_actor.{actor_id}, rivet_token.{client_token}"
				)
				.parse()
				.expect("invalid WebSocket subprotocol"),
			);
			let (mut socket, response) = connect_async(request)
				.await
				.expect("scoped JWT must authorize actor WebSocket");
			assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
			socket
				.send(Message::Text("ping".into()))
				.await
				.expect("failed to send WebSocket message");
			let message = tokio::time::timeout(Duration::from_secs(5), socket.next())
				.await
				.expect("WebSocket response timed out")
				.expect("WebSocket closed before response")
				.expect("WebSocket response failed");
			assert_eq!(
				message.into_text().expect("expected text response"),
				"Echo: ping"
			);

			let runner = common::test_runner::TestRunnerBuilder::new(&namespace)
				.with_runner_name(runner_name)
				.with_token(runner_token)
				.build(dc)
				.await
				.expect("failed to build test runner");
			runner
				.start()
				.await
				.expect("scoped JWT must connect runner");
			tokio::time::timeout(Duration::from_secs(5), runner.wait_ready())
				.await
				.expect("scoped JWT runner did not become ready");
		},
	);
}

async fn issue_jwt(
	client: &Client,
	base: &str,
	admin_token: &str,
	namespace: &str,
	grants: Value,
) -> String {
	for _ in 0..100 {
		let response = client
			.post(format!("{base}/auth/tokens"))
			.bearer_auth(admin_token)
			.json(&json!({
				"namespace": namespace,
				"duration": 120,
				"grants": grants,
			}))
			.send()
			.await
			.expect("failed to request JWT");
		if response.status().is_success() {
			return response
				.json::<rivet_api_types::auth::tokens::CreateResponse>()
				.await
				.expect("invalid JWT issuance response")
				.token;
		}
		tokio::time::sleep(Duration::from_millis(100)).await;
	}
	panic!("JWT issuer did not become ready");
}
