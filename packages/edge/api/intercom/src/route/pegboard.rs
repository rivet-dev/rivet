use api_helper::ctx::Ctx;
use chirp_workflow::prelude::*;
use cluster::types::BuildDeliveryMethod;
use fdb_util::SERIALIZABLE;
use foundationdb::{self as fdb, options::StreamingMode};
use futures_util::TryStreamExt;
use pegboard::protocol;
use rivet_api::models;
use serde_json::json;

use crate::auth::Auth;

// MARK: POST /pegboard/image/{}/prewarm
pub async fn prewarm_image(
	ctx: Ctx<Auth>,
	image_id: Uuid,
	body: serde_json::Value,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().bypass()?;

	let client_id = ctx
		.fdb()
		.await?
		.run(|tx, _mc| async move {
			let alloc_idx_subspace = pegboard::keys::subspace()
				.subspace(&pegboard::keys::datacenter::ClientsByRemainingMemKey::entire_subspace());
			let ping_threshold_ts =
				util::timestamp::now() - pegboard::workflows::client::CLIENT_ELIGIBLE_THRESHOLD_MS;

			let mut stream = tx.get_ranges_keyvalues(
				fdb::RangeOption {
					mode: StreamingMode::Iterator,
					..(&alloc_idx_subspace).into()
				},
				SERIALIZABLE,
			);

			while let Some(entry) = stream.try_next().await? {
				let key = pegboard::keys::subspace()
					.unpack::<pegboard::keys::datacenter::ClientsByRemainingMemKey>(entry.key())
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

				// Scan by last ping
				if key.last_ping_ts < ping_threshold_ts {
					continue;
				}

				return Ok(Some(key.client_id));
			}

			Ok(None)
		})
		.custom_instrument(tracing::info_span!("prewarm_fetch_tx"))
		.await?;

	let Some(client_id) = client_id else {
		tracing::error!("no eligible clients found to prewarm image");
		return Ok(json!({}));
	};

	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;
	let (dc_res, builds_res) = tokio::try_join!(
		ctx.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![dc_id],
		}),
		ctx.op(build::ops::get::Input {
			build_ids: vec![image_id],
		}),
	)?;

	let dc = unwrap!(dc_res.datacenters.first());
	let build = unwrap!(builds_res.builds.first());

	// Only prewarm if using ATS
	let BuildDeliveryMethod::TrafficServer = dc.build_delivery_method else {
		tracing::debug!("skipping prewarm since we're not using ats build delivery method");
		return Ok(json!({}));
	};

	// Get the artifact size
	let uploads_res = op!([ctx] upload_get {
		upload_ids: vec![build.upload_id.into()],
	})
	.await?;
	let upload = unwrap!(uploads_res.uploads.first());
	let artifact_size_bytes = upload.content_length;

	let res = ctx
		.signal(pegboard::workflows::client::PrewarmImage2 {
			image: protocol::Image {
				id: image_id,
				artifact_url_stub: pegboard::util::image_artifact_url_stub(
					ctx.config(),
					build.upload_id,
					&build::utils::file_name(build.kind, build.compression),
				)?,
				// We will never need to fall back to fetching directly from S3. This short
				// circuits earlier in the fn.
				fallback_artifact_url: None,
				artifact_size_bytes,
				kind: build.kind.into(),
				compression: build.compression.into(),
			},
		})
		.to_workflow::<pegboard::workflows::client::Workflow>()
		.tag("client_id", client_id)
		.send()
		.await;

	if let Some(WorkflowError::WorkflowNotFound) = res.as_workflow_error() {
		tracing::warn!(
			?client_id,
			"client workflow not found, likely already stopped"
		);
	} else {
		res?;
	}

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
		let res = ctx.signal(pegboard::workflows::client::Drain {
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
		.await;

		if let Some(WorkflowError::WorkflowNotFound) = res.as_workflow_error() {
			tracing::warn!(
				?client_id,
				"client workflow not found, likely already stopped"
			);
		} else {
			res?;
		}
	} else {
		let res = ctx.signal(pegboard::workflows::client::Undrain {})
			.to_workflow::<pegboard::workflows::client::Workflow>()
			.tag("client_id", client_id)
			.send()
			.await;

		if let Some(WorkflowError::WorkflowNotFound) = res.as_workflow_error() {
			tracing::warn!(
				?client_id,
				"client workflow not found, likely already stopped"
			);
		} else {
			res?;
		}
	}

	Ok(json!({}))
}

async fn resolve_image_fallback_artifact_url(
	ctx: &Ctx<Auth>,
	dc_build_delivery_method: BuildDeliveryMethod,
	build: &build::types::Build,
) -> GlobalResult<Option<String>> {
	if let BuildDeliveryMethod::S3Direct = dc_build_delivery_method {
		tracing::debug!("using s3 direct delivery");

		// Build client
		let s3_client = s3_util::Client::with_bucket_and_endpoint(
			ctx.config(),
			"bucket-build",
			s3_util::EndpointKind::EdgeInternal,
		)
		.await?;

		let presigned_req = s3_client
			.get_object()
			.bucket(s3_client.bucket())
			.key(format!(
				"{upload_id}/{file_name}",
				upload_id = build.upload_id,
				file_name = build::utils::file_name(build.kind, build.compression),
			))
			.presigned(
				s3_util::aws_sdk_s3::presigning::PresigningConfig::builder()
					.expires_in(std::time::Duration::from_secs(15 * 60))
					.build()?,
			)
			.await?;

		let addr_str = presigned_req.uri().to_string();
		tracing::debug!(addr = %addr_str, "resolved artifact s3 presigned request");

		Ok(Some(addr_str))
	} else {
		Ok(None)
	}
}
