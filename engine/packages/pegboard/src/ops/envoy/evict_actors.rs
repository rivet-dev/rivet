use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use std::time::Duration;
use universaldb::prelude::*;

use crate::keys;

#[derive(Debug)]
pub struct Throttle {
	/// Actors per second.
	pub rate: f32,
	/// Max eviction duration.
	pub period: Duration,
}

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub envoy_key: String,
	pub throttle: Option<Throttle>,
}

#[operation]
// Disable timeout for this op
#[timeout = u64::MAX]
pub async fn pegboard_envoy_evict_actors(ctx: &OperationCtx, input: &Input) -> Result<()> {
	let actors = ctx
		.udb()?
		.txn("pegboard_envoy_evict_actors", |tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let actor_subspace = keys::subspace().subspace(&keys::envoy::ActorKey::subspace(
				input.namespace_id,
				input.envoy_key.clone(),
			));

			tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&actor_subspace).into()
				},
				Serializable,
			)
			.map(|res| {
				let (key, generation) = tx.read_entry::<keys::envoy::ActorKey>(&res?)?;

				Ok((key.actor_id, generation))
			})
			.try_collect::<Vec<_>>()
			.await
		})
		.custom_instrument(tracing::info_span!("envoy_list_actors_tx"))
		.await?;

	if actors.is_empty() {
		return Ok(());
	}

	let mut error = None;

	let period = if let Some(throttle) = &input.throttle {
		let period = Duration::try_from_secs_f32(1.0 / throttle.rate).unwrap_or_default();
		let max_period = throttle.period / u32::try_from(actors.len())?;
		period.min(max_period)
	} else {
		Duration::ZERO
	};

	if period != Duration::ZERO {
		let mut interval = tokio::time::interval(period);

		let mut iter = actors.into_iter();
		while let Some((actor_id, generation)) = iter.next() {
			interval.tick().await;

			// Try to get through all of the actors even on error
			if let Err(err) = ctx
				.signal(crate::workflows::actor2::GoingAway { generation })
				.to_workflow::<crate::workflows::actor2::Workflow>()
				.tag("actor_id", actor_id)
				.graceful_not_found()
				.send()
				.await
			{
				if error.is_none() {
					error = Some(err);
				}
			}
		}
	} else {
		for (actor_id, generation) in actors {
			// Try to get through all of the actors even on error
			if let Err(err) = ctx
				.signal(crate::workflows::actor2::GoingAway { generation })
				.to_workflow::<crate::workflows::actor2::Workflow>()
				.tag("actor_id", actor_id)
				.graceful_not_found()
				.send()
				.await
			{
				if error.is_none() {
					error = Some(err);
				}
			}
		}
	}

	if let Some(err) = error {
		return Err(err);
	}

	Ok(())
}
