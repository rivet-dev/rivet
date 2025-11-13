use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;
use uuid::Uuid;

use crate::keys;

#[derive(Debug, Default)]
pub struct Input {
	pub actor_id: Id,
}

#[operation]
pub async fn pegboard_actor_hibernating_request_list(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Vec<Uuid>> {
	let hibernating_request_eligible_threshold = ctx
		.config()
		.pegboard()
		.hibernating_request_eligible_threshold();

	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let ping_threshold_ts = util::timestamp::now() - hibernating_request_eligible_threshold;
			let hr_subspace_start = tx.pack(&keys::actor::HibernatingRequestKey::subspace_with_ts(
				input.actor_id,
				ping_threshold_ts,
			));
			let hr_subspace_end = keys::subspace()
				.subspace(&keys::actor::HibernatingRequestKey::subspace(
					input.actor_id,
				))
				.range()
				.1;

			tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(hr_subspace_start, hr_subspace_end).into()
				},
				Serializable,
			)
			.map(|res| {
				let key = tx.unpack::<keys::actor::HibernatingRequestKey>(res?.key())?;
				Ok(key.request_id)
			})
			.try_collect::<Vec<_>>()
			.await
		})
		.custom_instrument(tracing::info_span!("hibernating_request_list_tx"))
		.await
}
