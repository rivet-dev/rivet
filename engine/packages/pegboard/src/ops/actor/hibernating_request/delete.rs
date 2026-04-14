use gas::prelude::*;
use rivet_runner_protocol as protocol;
use universaldb::utils::IsolationLevel::*;

use crate::keys;

#[derive(Debug, Default)]
pub struct Input {
	pub actor_id: Id,
	pub gateway_id: protocol::GatewayId,
	pub request_id: protocol::RequestId,
}

#[operation]
pub async fn pegboard_actor_hibernating_request_delete(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<()> {
	tracing::info!(
		actor_id=%input.actor_id,
		gateway_id=%protocol::util::id_to_string(&input.gateway_id),
		request_id=%protocol::util::id_to_string(&input.request_id),
		"deleting hibernating request"
	);

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

			tx.delete(&last_ping_ts_key);

			Ok(())
		})
		.custom_instrument(tracing::info_span!("hibernating_request_delete_tx"))
		.await
}
