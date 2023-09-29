mod common;

use common::*;
use proto::backend::{self, pkg::*};
use rivet_api::{apis::*, models};
use rivet_operation::prelude::*;
use serde_json::json;

#[test]
fn test_redis() {
	use redis::cluster::ClusterClient;
	use redis::Commands;

	// let url = "rediss://default:201RrfgpMbA3iLpdMyB6NnqmCXhBiLWn@redis-redis-cluster.redis-chirp.svc.cluster.local:6379";
	// let url = "rediss://default:201RrfgpMbA3iLpdMyB6NnqmCXhBiLWn@redis-redis-cluster-0.redis-redis-cluster-headless.redis-chirp.svc.cluster.local:6379";
	// let url = "rediss://default:201RrfgpMbA3iLpdMyB6NnqmCXhBiLWn@10.42.0.228:6379";
	// let nodes = vec![url];
	let nodes = vec![
		"rediss://default:201RrfgpMbA3iLpdMyB6NnqmCXhBiLWn@redis-redis-cluster-0.redis-redis-cluster-headless.redis-chirp.svc.cluster.local:6379",
		"rediss://default:201RrfgpMbA3iLpdMyB6NnqmCXhBiLWn@redis-redis-cluster-1.redis-redis-cluster-headless.redis-chirp.svc.cluster.local:6379",
		"rediss://default:201RrfgpMbA3iLpdMyB6NnqmCXhBiLWn@redis-redis-cluster-2.redis-redis-cluster-headless.redis-chirp.svc.cluster.local:6379",
	];
	println!("connecting {nodes:?}");
	let client = ClusterClient::new(nodes).unwrap();
	println!("connections");
	let mut connection = client.get_connection().unwrap();

	println!("set");
	let _: () = connection.set("test", "test_data").unwrap();
	println!("get");
	let rv: String = connection.get("test").unwrap();

	assert_eq!(rv, "test_data");
}

// #[tokio::test(flavor = "multi_thread")]
// async fn create_custom_lobby() {
// 	let ctx = Ctx::init().await;

// 	{
// 		tracing::info!("creating custom lobby");

// 		let res = matchmaker_lobbies_api::matchmaker_lobbies_create(
// 			&ctx.config(ctx.ns_auth_token.clone()),
// 			models::MatchmakerLobbiesCreateRequest {
// 				game_mode: LOBBY_GROUP_NAME_ID_BRIDGE.to_string(),
// 				region: Some(ctx.primary_region_name_id.clone()),
// 				publicity: models::MatchmakerCustomLobbyPublicity::Public,
// 				lobby_config: Some(Some(json!({ "foo": "bar" }))),
// 				verification_data: None,
// 				captcha: Some(Box::new(models::CaptchaConfig {
// 					hcaptcha: Some(Box::new(models::CaptchaConfigHcaptcha {
// 						client_response: "10000000-aaaa-bbbb-cccc-000000000001".to_string(),
// 					})),
// 					turnstile: None,
// 				})),
// 			},
// 		)
// 		.await
// 		.unwrap();

// 		assert_lobby_state(&ctx, &res.lobby).await;
// 	}
// }
