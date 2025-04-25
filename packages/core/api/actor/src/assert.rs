use api_helper::ctx::Ctx;
use futures_util::{
	stream::{self},
	FutureExt, StreamExt,
};
use redis::{FromRedisValue, RedisResult, ToRedisArgs, Value};
use rivet_api::apis::actors_api::actors_get;
use rivet_api::apis::configuration::Configuration;
use rivet_cache::CacheKey;
use rivet_operation::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// Make unwrap_or_bail available
#[macro_export]
macro_rules! unwrap_or_bail {
	($option:expr, $msg:expr) => {
		match $option {
			Some(val) => val,
			None => bail!($msg),
		}
	};
}

use crate::auth::Auth;

#[derive(Debug, Clone)]
struct ActorValidationData {
	is_valid: bool,
	game_name_id: String,
	env_name_id: String,
}

// Implement Redis serialization traits for ActorValidationData
impl ToRedisArgs for ActorValidationData {
	fn write_redis_args<W>(&self, out: &mut W)
	where
		W: ?Sized + redis::RedisWrite,
	{
		// Format: "is_valid:game_name_id:env_name_id"
		let serialized = format!(
			"{}:{}:{}",
			if self.is_valid { "1" } else { "0" },
			self.game_name_id,
			self.env_name_id
		);
		serialized.write_redis_args(out);
	}
}

impl FromRedisValue for ActorValidationData {
	fn from_redis_value(v: &Value) -> RedisResult<Self> {
		let s: String = redis::from_redis_value(v)?;
		let parts: Vec<&str> = s.split(':').collect();

		if parts.len() < 3 {
			return Err(redis::RedisError::from((
				redis::ErrorKind::TypeError,
				"Invalid ActorValidationData format",
			)));
		}

		let is_valid = parts[0] == "1";
		let game_name_id = parts[1].to_string();
		let env_name_id = parts[2].to_string();

		Ok(ActorValidationData {
			is_valid,
			game_name_id,
			env_name_id,
		})
	}
}

/// Cache key for actor validation
///
/// Cache is per game/environment/actor_id combination
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct ActorValidationCacheKey {
	game_id: Uuid,
	env_id: Uuid,
	actor_id: Uuid,
}

// Implement CacheKey trait for ActorValidationCacheKey
impl CacheKey for ActorValidationCacheKey {
	fn cache_key(&self) -> String {
		format!("actor:{}:{}:{}", self.game_id, self.env_id, self.actor_id)
	}

	fn simple_cache_key(&self) -> String {
		self.cache_key()
	}
}

/// Returns a list of valid actor IDs that belong to the given environment and game.
/// Filters out any invalid actor IDs silently.
///
/// Process steps:
/// 1. Creates cache keys for each actor ID to check
/// 2. Retrieves game and environment metadata
/// 3. Uses a batch caching mechanism to efficiently validate multiple actors
/// 4. For actors not in cache:
///    a. Retrieves cluster and datacenter information
///    b. Filters for valid datacenters with worker/guard pools
///    c. Concurrently validates each actor against multiple datacenters
///    d. Stores validation results in cache
/// 5. Returns only the actor IDs that were successfully validated
///
/// The validation uses a distributed approach, checking each actor across
/// multiple datacenters until it's found or all datacenters are exhausted.
pub async fn actor_for_env(
	ctx: &Ctx<Auth>,
	actor_ids: &[Uuid],
	game_id: Uuid,
	env_id: Uuid,
	_error_code: Option<&'static str>,
) -> GlobalResult<Vec<Uuid>> {
	if actor_ids.is_empty() {
		return Ok(Vec::new());
	}

	// Create cache keys for each actor ID
	let cache_keys = actor_ids
		.iter()
		.map(|&actor_id| ActorValidationCacheKey {
			game_id,
			env_id,
			actor_id,
		})
		.collect::<Vec<_>>();

	// Get game and environment information
	let game_res = match op!([ctx] game_get {
		game_ids: vec![game_id.into()],
	})
	.await
	{
		Ok(res) => res,
		Err(err) => {
			tracing::warn!(?err, ?game_id, "Failed to get game");
			return Ok(Vec::new());
		}
	};

	let game = match game_res.games.first() {
		Some(game) => game,
		None => {
			tracing::warn!(?game_id, "Game not found");
			return Ok(Vec::new());
		}
	};

	let env_res = match op!([ctx] game_namespace_get {
		namespace_ids: vec![env_id.into()],
	})
	.await
	{
		Ok(res) => res,
		Err(err) => {
			tracing::warn!(?err, ?env_id, "Failed to get environment");
			return Ok(Vec::new());
		}
	};

	let env = match env_res.namespaces.first() {
		Some(env) => env,
		None => {
			tracing::warn!(?env_id, "Environment not found");
			return Ok(Vec::new());
		}
	};

	// Setup shared context for batch validation
	let game_name_id = game.name_id.clone();
	let env_name_id = env.name_id.clone();

	// Use batch caching for actor validation
	let actor_validation_results = ctx
		.cache()
		.fetch_all("actor_validation", cache_keys, {
			let ctx = ctx.clone();
			let game_name_id = game_name_id.clone();
			let env_name_id = env_name_id.clone();

			move |mut cache, keys_to_fetch| {
				let ctx = ctx.clone();
				let game_name_id = game_name_id.clone();
				let env_name_id = env_name_id.clone();

				async move {
					// We don't need to track game/env pairs since they should all be the same
					// in a given call, but we could verify it if needed
					let _game_env_pairs = keys_to_fetch
						.iter()
						.map(|key| (key.game_id, key.env_id))
						.collect::<std::collections::HashSet<_>>();

					// Get actor IDs to validate
					let actor_ids_to_validate = keys_to_fetch
						.iter()
						.map(|key| key.actor_id)
						.collect::<Vec<_>>();

					if actor_ids_to_validate.is_empty() {
						return Ok(cache);
					}

					// Get datacenters for validation
					let clusters_res = match ctx
						.op(cluster::ops::get_for_game::Input {
							game_ids: vec![game_id],
						})
						.await
					{
						Ok(res) => res,
						Err(err) => {
							tracing::warn!(?err, ?game_id, "Failed to get clusters for game");
							return Ok(cache);
						}
					};

					let cluster_id = match clusters_res.games.first() {
						Some(game) => game.cluster_id,
						None => {
							tracing::warn!(?game_id, "Game not found in any cluster");
							return Ok(cache);
						}
					};

					let dc_list_res = match ctx
						.op(cluster::ops::datacenter::list::Input {
							cluster_ids: vec![cluster_id],
						})
						.await
					{
						Ok(res) => res,
						Err(err) => {
							tracing::warn!(?err, ?cluster_id, "Failed to list datacenters");
							return Ok(cache);
						}
					};

					let datacenter_ids = match dc_list_res.clusters.into_iter().next() {
						Some(cluster) => cluster.datacenter_ids,
						None => {
							tracing::warn!(?cluster_id, "Cluster not found");
							return Ok(cache);
						}
					};

					let dcs_res = match ctx
						.op(cluster::ops::datacenter::get::Input { datacenter_ids })
						.await
					{
						Ok(res) => res,
						Err(err) => {
							tracing::warn!(?err, "Failed to get datacenters");
							return Ok(cache);
						}
					};

					// Filter valid datacenters
					let filtered_datacenters = dcs_res
						.datacenters
						.into_iter()
						.filter(|dc| {
							crate::utils::filter_edge_dc(ctx.config(), dc).unwrap_or(false)
						})
						.collect::<Vec<_>>();

					if filtered_datacenters.is_empty() {
						tracing::warn!("No valid datacenters with worker and guard pools");
						return Ok(cache);
					}

					// Track validation results for each actor
					let validation_results = Arc::new(Mutex::new(HashMap::<Uuid, bool>::new()));

					// Create a stream of all datacenter + actor_id combinations
					let mut validation_tasks =
						stream::iter(filtered_datacenters.into_iter().flat_map(|dc| {
							let dc_clone = dc.clone();
							let ids = actor_ids_to_validate.clone();
							ids.into_iter()
								.map(move |actor_id| (dc_clone.clone(), actor_id))
						}))
						.map(|(dc, actor_id)| {
							let validation_results = validation_results.clone();
							let game_name_id = game_name_id.clone();
							let env_name_id = env_name_id.clone();

							async move {
								// Skip this task if actor already validated
								{
									let map = validation_results.lock().await;
									if map.get(&actor_id).map_or(false, |&v| v) {
										return GlobalResult::Ok(());
									}
								}

								let config = Configuration {
									client: rivet_pools::reqwest::client().await?,
									base_path: ctx
										.config()
										.server()?
										.rivet
										.edge_api_url_str(&dc.name_id)?,
									bearer_access_token: ctx.auth().api_token.clone(),
									..Default::default()
								};

								// Pass the request to the edge api with project and environment name_ids
								match actors_get(
									&config,
									&actor_id.to_string(),
									Some(&game_name_id),
									Some(&env_name_id),
									None, // endpoint_type
								)
								.await
								{
									Ok(_) => {
										// Actor exists and belongs to this game/env
										let mut map = validation_results.lock().await;
										map.insert(actor_id, true);
									}
									Err(err) => {
										tracing::debug!(?err, ?actor_id, "Actor validation failed");
										// Only mark as invalid if not already validated
										let mut map = validation_results.lock().await;
										map.entry(actor_id).or_insert(false);
									}
								};

								GlobalResult::Ok(())
							}
							.boxed()
						})
						.buffer_unordered(16); // Process up to 16 concurrent validation requests

					// Process results (just consume the stream)
					while let Some(_) = validation_tasks.next().await {}

					// Get the validation results
					let validation_results = validation_results.lock().await.clone();

					// Resolve cache entries
					for key in keys_to_fetch {
						let is_valid = validation_results
							.get(&key.actor_id)
							.copied()
							.unwrap_or(false);

						// Add to cache
						cache.resolve(
							&key,
							ActorValidationData {
								is_valid,
								game_name_id: game_name_id.clone(),
								env_name_id: env_name_id.clone(),
							},
						);
					}

					Ok(cache)
				}
			}
		})
		.await?;

	// Filter valid actor IDs
	let valid_actors = actor_ids
		.iter()
		.filter_map(|&actor_id| {
			let cache_key = ActorValidationCacheKey {
				game_id,
				env_id,
				actor_id,
			};

			// Check if the actor is valid in the cache results
			actor_validation_results
				.iter()
				.find(|(k, _)| *k == cache_key)
				.and_then(
					|(_, data)| {
						if data.is_valid {
							Some(actor_id)
						} else {
							None
						}
					},
				)
		})
		.collect::<Vec<_>>();

	Ok(valid_actors)
}
