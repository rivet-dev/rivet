use api_helper::ctx::Ctx;
use fdb_util::SERIALIZABLE;
use foundationdb::{self as fdb, options::StreamingMode};
use futures_util::TryStreamExt;
use rivet_api::models;
use rivet_operation::prelude::*;
use serde_json::json;

use crate::auth::Auth;

// MARK: POST /pegboard/image/{}/prewarm
pub async fn prewarm_image(
	ctx: Ctx<Auth>,
	image_id: Uuid,
	body: models::EdgeIntercomPegboardPrewarmImageRequest,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().bypass()?;

	// TODO: If we replicate the algorithm for choosing the correct ATS node from the pb manager, we can
	// remove prewarming from the pb protocol entirely and just prewarm the image here since this api service
	// is in the same dc
	let client_id = ctx
		.fdb()
		.await?
		.run(|tx, _mc| async move {
			let alloc_idx_subspace = pegboard::keys::subspace()
				.subspace(&pegboard::keys::datacenter::ClientsByRemainingMemKey::entire_subspace());

			let mut stream = tx.get_ranges_keyvalues(
				fdb::RangeOption {
					mode: StreamingMode::Small,
					..(&alloc_idx_subspace).into()
				},
				SERIALIZABLE,
			);

			if let Some(entry) = stream.try_next().await? {
				let key = pegboard::keys::subspace()
					.unpack::<pegboard::keys::datacenter::ClientsByRemainingMemKey>(entry.key())
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

				Ok(Some(key.client_id))
			} else {
				Ok(None)
			}
		})
		.custom_instrument(tracing::info_span!("prewarm_fetch_tx"))
		.await?;

	let Some(client_id) = client_id else {
		tracing::error!("no eligible clients found to prewarm image");
		return Ok(json!({}));
	};

	ctx.signal(pegboard::workflows::client::PrewarmImage {
		image_id,
		image_artifact_url_stub: body.image_artifact_url_stub,
	})
	.to_workflow::<pegboard::workflows::client::Workflow>()
	.tag("client_id", client_id)
	.send()
	.await?;

	Ok(json!({}))
}

// MARK: POST /pegboard/client/{}/toggle-drain
pub async fn toggle_drain_client(
	ctx: Ctx<Auth>,
	client_id: Uuid,
	body: models::EdgeIntercomPegboardToggleClientDrainRequest,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().bypass()?;

	if body.draining {
		ctx.signal(pegboard::workflows::client::Drain {
			drain_timeout_ts: unwrap_with!(
				body.drain_complete_ts,
				API_BAD_BODY,
				error = "missing `drain_complete_ts`"
			)
			.parse::<chrono::DateTime<chrono::Utc>>()?
			.timestamp_millis(),
		})
		.to_workflow::<pegboard::workflows::client::Workflow>()
		.tag("client_id", client_id)
		.send()
		.await?;
	} else {
		ctx.signal(pegboard::workflows::client::Undrain {})
			.to_workflow::<pegboard::workflows::client::Workflow>()
			.tag("client_id", client_id)
			.send()
			.await?;
	}

	Ok(json!({}))
}
