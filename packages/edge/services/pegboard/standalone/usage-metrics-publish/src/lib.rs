use std::collections::HashMap;

use build::types::BuildKind;
use chirp_workflow::prelude::*;
use fdb_util::SNAPSHOT;
use foundationdb::{self as fdb, options::StreamingMode};
use futures_util::{StreamExt, TryStreamExt};
use pegboard::{keys, protocol};

struct Usage {
	// Percent of core.
	pub cpu: u64,
	/// MiB.
	pub memory: u64,
}

pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let mut interval = tokio::time::interval(std::time::Duration::from_secs(7));
	loop {
		interval.tick().await;

		run_from_env(config.clone(), pools.clone()).await?;
	}
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?
		.wrap_new("pegboard-usage-metrics-publish");
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseCrdbNats::from_pools(pools.clone())?,
		config,
		rivet_connection::Connection::new(client, pools, cache),
		"pegboard-usage-metrics-publish",
	)
	.await?;

	// List all actor ids that are currently running
	let actor_ids = ctx
		.fdb()
		.await?
		.run(|tx, _mc| async move {
			let actor_subspace =
				keys::subspace().subspace(&keys::client::ActorKey::entire_subspace());

			tx.get_ranges_keyvalues(
				fdb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&actor_subspace).into()
				},
				// Not serializable because we don't want to interfere with normal operations
				SNAPSHOT,
			)
			.map(|res| match res {
				Ok(entry) => {
					let key = keys::subspace()
						.unpack::<keys::client::ActorKey>(entry.key())
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

					Ok(key.actor_id)
				}
				Err(err) => Err(Into::<fdb::FdbBindingError>::into(err)),
			})
			.try_collect::<Vec<_>>()
			.await
		})
		.custom_instrument(tracing::info_span!("client_fetch_remaining_actors_tx"))
		.await?;

	let actors_res = ctx
		.op(pegboard::ops::actor::get::Input {
			actor_ids,
			endpoint_type: None,
			allow_errors: true,
		})
		.await?;

	let builds_res = ctx
		.op(build::ops::get::Input {
			build_ids: actors_res
				.actors
				.iter()
				.map(|actor| actor.image_id)
				.collect(),
		})
		.await?;

	let mut usage_by_env_and_flavor = HashMap::new();

	// Aggregate data per env and flavor
	for actor in &actors_res.actors {
		if actor.start_ts.is_none() || actor.destroy_ts.is_some() {
			continue;
		}

		let Some(build) = builds_res
			.builds
			.iter()
			.find(|build| build.build_id == actor.image_id)
		else {
			tracing::error!("build info not found for actor");
			continue;
		};

		let client_flavor = match build.kind {
			BuildKind::DockerImage | BuildKind::OciBundle => protocol::ClientFlavor::Container,
			BuildKind::JavaScript => protocol::ClientFlavor::Isolate,
		};

		let env_usage = usage_by_env_and_flavor
			.entry((actor.env_id, client_flavor))
			.or_insert(Usage { cpu: 0, memory: 0 });

		env_usage.cpu += (actor.resources.cpu_millicores / 10) as u64;
		env_usage.memory += actor.resources.memory_mib as u64;
	}

	// Clear old metrics because they will never be set to 0 (due to no actors being present and thus no
	// metrics update)
	pegboard::metrics::ENV_CPU_USAGE.reset();
	pegboard::metrics::ENV_MEMORY_USAGE.reset();

	// Insert metrics
	for ((env_id, client_flavor), usage) in usage_by_env_and_flavor {
		pegboard::metrics::ENV_CPU_USAGE
			.with_label_values(&[&env_id.to_string(), &client_flavor.to_string()])
			.set(usage.cpu.try_into()?);
		pegboard::metrics::ENV_MEMORY_USAGE
			.with_label_values(&[&env_id.to_string(), &client_flavor.to_string()])
			.set(usage.memory.try_into()?);
	}

	Ok(())
}
