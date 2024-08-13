use std::{collections::HashMap, sync::Once};

use rivet_api::{apis::*, models};
use rivet_operation::prelude::*;

static GLOBAL_INIT: Once = Once::new();

struct Ctx {
	pub op_ctx: OperationContext<()>,
	pub ns_auth_token: String,
	pub game_id: Uuid,
	pub game_id_str: String,
	pub datacenter_id: Uuid,
	pub image_id: Uuid,
}

impl Ctx {
	async fn init() -> GlobalResult<Ctx> {
		GLOBAL_INIT.call_once(|| {
			tracing_subscriber::fmt()
				.pretty()
				.with_max_level(tracing::Level::INFO)
				.with_target(false)
				.init();
		});

		let pools = rivet_pools::from_env("api-servers-test").await?;
		let cache = rivet_cache::CacheInner::new(
			"api-servers-test".to_string(),
			util::env::var("RIVET_SOURCE_HASH")?,
			pools.redis_cache()?,
		);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("create client")
			.wrap_new("api-servers-test");
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			"api-servers-test".to_string(),
			std::time::Duration::from_secs(60),
			conn,
			Uuid::new_v4(),
			Uuid::new_v4(),
			util::timestamp::now(),
			util::timestamp::now(),
			(),
		);

		let (datacenter_id, primary_datacenter_name_id) = Self::setup_datacenter(&op_ctx).await?;
		let (game_id, image_id) = Self::setup_game(&op_ctx, datacenter_id).await?;
		let ns_auth_token = Self::setup_public_token(&op_ctx, game_id).await?;

		Ok(Ctx {
			op_ctx,
			ns_auth_token,
			game_id,
			game_id_str: game_id.to_string(),
			datacenter_id,
			image_id,
		})
	}

	fn chirp(&self) -> &chirp_client::Client {
		self.op_ctx.chirp()
	}

	fn op_ctx(&self) -> &OperationContext<()> {
		&self.op_ctx
	}

	pub fn config(
		&self,
		bearer_token: String,
	) -> GlobalResult<rivet_api::apis::configuration::Configuration> {
		Ok(rivet_api::apis::configuration::Configuration {
			base_path: "http://traefik.traefik.svc.cluster.local:80".into(),
			bearer_access_token: Some(bearer_token),
			client: {
				let mut headers = http::header::HeaderMap::new();
				headers.insert(
					http::header::HOST,
					unwrap!(http::header::HeaderValue::from_str(unwrap!(
						util::env::domain_main_api()
					))),
				);
				headers.insert(
					"cf-connecting-ip",
					http::header::HeaderValue::from_static("127.0.0.1"),
				);
				unwrap!(reqwest::Client::builder().default_headers(headers).build())
			},
			..Default::default()
		})
	}

	pub async fn setup_datacenter(ctx: &OperationContext<()>) -> GlobalResult<(Uuid, String)> {
		tracing::info!("setup region");

		let region_res = op!([ctx] faker_region {}).await?;
		let region_id = unwrap!(region_res.region_id).as_uuid();

		let get_res = op!([ctx] region_get {
			region_ids: vec![region_id.into()],
		})
		.await?;
		let region_data = unwrap!(get_res.regions.first());

		Ok((region_id, region_data.name_id.clone()))
	}

	pub async fn setup_game(
		ctx: &OperationContext<()>,
		region_id: Uuid,
	) -> GlobalResult<(Uuid, Uuid)> {
		let game_res = op!([ctx] faker_game {
			..Default::default()
		})
		.await?;
		let game_id = unwrap!(game_res.game_id);

		let build_res = op!([ctx] faker_build {
			game_id: Some(game_id),
			image: proto::backend::faker::Image::DsEcho as i32,
		})
		.await?;

		Ok((
			unwrap!(game_res.game_id).as_uuid(),
			unwrap!(build_res.build_id).as_uuid(),
		))
	}

	pub async fn setup_public_token(
		ctx: &OperationContext<()>,
		game_id: Uuid,
	) -> GlobalResult<String> {
		let token_res = op!([ctx] cloud_service_game_token_create {
			game_id: Some(game_id.into()),
		})
		.await?;

		Ok(token_res.token)
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn create_http() -> GlobalResult<()> {
	let ctx = Ctx::init().await?;

	let ctx_config = ctx.config(ctx.ns_auth_token.clone())?;

	let res = games_servers_api::games_servers_create(
		&ctx_config,
		&ctx.game_id_str,
		models::GamesServersCreateServerRequest {
			arguments: None,
			datacenter: ctx.datacenter_id.to_string(),
			environment: Some(HashMap::new()),
			image_id: ctx.image_id,
			kill_timeout: Some(0),
			webhook_url: None,
			tags: None,
			network: Box::new(models::GamesServersCreateServerNetworkRequest {
				mode: Some(models::GamesServersNetworkMode::Bridge),
				ports: vec![(
					"testing2".to_string(),
					models::GamesServersCreateServerPortRequest {
						protocol: models::GamesServersPortProtocol::Http,
						routing: Some(Box::new(models::GamesServersPortRouting {
							game_guard: Some(serde_json::Value::Object(serde_json::Map::new())),
							host: None,
						})),
						internal_port: Some(12523),
					},
				)]
				// Collect into hashmap
				.into_iter()
				.collect(),
			}),
			resources: Box::new(models::GamesServersResources {
				cpu: 100,
				memory: 200,
			}),
		},
	);

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn list_builds_with_tags() -> GlobalResult<()> {
	let ctx = Ctx::init().await?;

	let ctx_config = ctx.config(ctx.ns_auth_token.clone())?;

	let res = games_servers_api::games_servers_create(
		&ctx_config,
		&ctx.game_id_str,
		models::GamesServersCreateServerRequest {
			arguments: None,
			datacenter: ctx.datacenter_id.to_string(),
			environment: Some(HashMap::new()),
			image_id: ctx.image_id,
			kill_timeout: Some(0),
			webhook_url: None,
			tags: None,
			network: Box::new(models::GamesServersCreateServerNetworkRequest {
				mode: Some(models::GamesServersNetworkMode::Bridge),
				ports: vec![(
					"testing2".to_string(),
					models::GamesServersCreateServerPortRequest {
						protocol: models::GamesServersPortProtocol::Http,
						routing: Some(Box::new(models::GamesServersPortRouting {
							game_guard: Some(serde_json::Value::Object(serde_json::Map::new())),
							host: None,
						})),
						internal_port: Some(12523),
					},
				)]
				// Collect into hashmap
				.into_iter()
				.collect(),
			}),
			resources: Box::new(models::GamesServersResources {
				cpu: 100,
				memory: 200,
			}),
		},
	);

	Ok(())
}
