use gas::prelude::*;
use universaldb::utils::IsolationLevel::*;

use crate::keys;
use crate::tunnel::id::{GatewayId, RequestId};

#[derive(Debug, Default)]
pub struct Input {
	pub actor_id: Id,
	pub gateway_id: GatewayId,
	pub request_id: RequestId,
}

#[operation]
pub async fn pegboard_actor_hibernating_request_upsert(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<()> {
	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let last_ping_ts_key =
				keys::hibernating_request::LastPingTsKey::new(input.gateway_id, input.request_id);

			if let Some(last_ping_ts) = tx.read_opt(&last_ping_ts_key, Serializable).await? {
				tx.delete(&keys::actor::HibernatingRequestKey::new(
					input.actor_id,
					last_ping_ts,
					input.gateway_id,
					input.request_id,
				));
			}

			let now = util::timestamp::now();
			tx.write(&last_ping_ts_key, now)?;
			tx.write(
				&keys::actor::HibernatingRequestKey::new(
					input.actor_id,
					now,
					input.gateway_id,
					input.request_id,
				),
				(),
			)?;

			Ok(())
		})
		.custom_instrument(tracing::info_span!("hibernating_request_upsert_tx"))
		.await
}
