use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use universaldb::prelude::*;

use crate::keys;

/// How many actor workflows are signalled at once.
const SIGNAL_CONCURRENCY: usize = 16;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub envoy_key: String,
	/// The connection whose loss is being reported. A newer connection id means the envoy
	/// reconnected and its actors must stay where they are.
	pub envoy_conn_id: Id,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
	/// The envoy reconnected or is pinging again, so nothing was signalled.
	Alive,
	/// Every actor still assigned to the envoy was told it lost its envoy.
	Lost { actor_count: usize },
}

/// Tells every actor still assigned to a disconnected envoy that its envoy is lost, unless the
/// envoy registered a newer connection or pinged within `envoy_lost_threshold`.
///
/// This is the fast path for envoy loss; the actor workflow's own liveness check is the backstop
/// when the process that ran this sweep is gone. Signalling is idempotent because the actor
/// workflow ignores a `Lost` for a generation it has already left.
#[operation]
pub async fn pegboard_envoy_lost_actors(ctx: &OperationCtx, input: &Input) -> Result<Outcome> {
	let envoy_lost_threshold = ctx.config().pegboard().envoy_lost_threshold();

	let actors = ctx
		.udb()?
		.txn("pegboard_envoy_lost_actors", |tx| async move {
			let tx = tx.with_subspace(keys::subspace());
			let now = util::timestamp::now();

			let envoy_conn_id_key =
				keys::envoy::EnvoyConnIdKey::new(input.namespace_id, input.envoy_key.clone());
			let last_ping_ts_key =
				keys::envoy::LastPingTsKey::new(input.namespace_id, input.envoy_key.clone());
			let (current_envoy_conn_id, last_ping_ts) = tokio::try_join!(
				tx.read_opt(&envoy_conn_id_key, Serializable),
				tx.read_opt(&last_ping_ts_key, Serializable),
			)?;

			// A newer connection owns the registration
			if current_envoy_conn_id.is_some_and(|id| id != input.envoy_conn_id) {
				return Ok(None);
			}
			// The envoy is pinging again
			if last_ping_ts.is_some_and(|ts| ts >= now - envoy_lost_threshold) {
				return Ok(None);
			}

			let actor_subspace = keys::subspace().subspace(&keys::envoy::ActorKey::subspace(
				input.namespace_id,
				input.envoy_key.clone(),
			));
			let actors = tx
				.get_ranges_keyvalues(
					universaldb::RangeOption {
						mode: StreamingMode::WantAll,
						..(&actor_subspace).into()
					},
					Serializable,
				)
				.map(|res| -> Result<(Id, u32)> {
					let (key, generation) = tx.read_entry::<keys::envoy::ActorKey>(&res?)?;

					Ok((key.actor_id, generation))
				})
				.try_collect::<Vec<_>>()
				.await?;

			Ok(Some(actors))
		})
		.custom_instrument(tracing::info_span!("envoy_lost_actors_tx"))
		.await?;

	let Some(actors) = actors else {
		return Ok(Outcome::Alive);
	};
	let actor_count = actors.len();

	// Try to get through all of the actors even on error
	let mut first_error = None;
	let mut results = futures_util::stream::iter(actors)
		.map(|(actor_id, generation)| async move {
			let res = ctx
				.signal(crate::workflows::actor2::Lost {
					generation,
					reason: crate::workflows::actor2::LostReason::EnvoyConnectionLost,
				})
				.to_workflow::<crate::workflows::actor2::Workflow>()
				.tag("actor_id", actor_id)
				.graceful_not_found()
				.send()
				.await;

			(actor_id, res)
		})
		.buffer_unordered(SIGNAL_CONCURRENCY);
	while let Some((actor_id, res)) = results.next().await {
		if let Err(err) = res {
			tracing::warn!(
				?actor_id,
				?err,
				"failed to signal actor that its envoy was lost"
			);
			if first_error.is_none() {
				first_error = Some(err);
			}
		}
	}

	if let Some(err) = first_error {
		return Err(err);
	}

	Ok(Outcome::Lost { actor_count })
}
