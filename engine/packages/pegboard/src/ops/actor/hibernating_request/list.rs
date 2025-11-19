use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;

use crate::keys;
use crate::tunnel::id::{GatewayId, RequestId};

#[derive(Debug, Default)]
pub struct Input {
	pub actor_id: Id,
}

#[derive(Debug)]
pub struct HibernatingRequestItem {
	pub gateway_id: GatewayId,
	pub request_id: RequestId,
}

#[operation]
pub async fn pegboard_actor_hibernating_request_list(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Vec<HibernatingRequestItem>> {
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
				Ok(HibernatingRequestItem {
					gateway_id: key.gateway_id,
					request_id: key.request_id,
				})
			})
			.try_collect::<Vec<_>>()
			.await
		})
		.custom_instrument(tracing::info_span!("hibernating_request_list_tx"))
		.await
}
