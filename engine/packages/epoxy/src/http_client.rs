use anyhow::*;
use epoxy_protocol::{
	PROTOCOL_VERSION,
	protocol::{self, ReplicaId},
	versioned,
};
use futures_util::{StreamExt, stream::FuturesUnordered};
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use std::future::Future;
use vbare::OwnedVersionedData;

use crate::utils;

/// Find the API replica URL for a given replica ID in the topology
fn find_replica_address(
	config: &protocol::ClusterConfig,
	target_replica_id: ReplicaId,
) -> Result<String> {
	config
		.replicas
		.iter()
		.find(|x| x.replica_id == target_replica_id)
		.with_context(|| format!("replica {} not found in topology", target_replica_id))
		.map(|r| r.api_peer_url.clone())
}

#[tracing::instrument(skip_all, fields(%from_replica_id, ?replica_ids, ?quorum_type))]
pub async fn fanout_to_replicas<F, Fut, T>(
	from_replica_id: ReplicaId,
	replica_ids: &[ReplicaId],
	quorum_type: utils::QuorumType,
	request_builder: F,
) -> Result<Vec<T>>
where
	F: Fn(ReplicaId) -> Fut + Clone,
	Fut: Future<Output = Result<T>> + Send,
	T: Send,
{
	let quorum_size = utils::calculate_quorum(replica_ids.len(), quorum_type);

	// Create futures for all replicas (excluding the sender)
	let mut responses = futures_util::stream::iter(
		replica_ids
			.iter()
			.filter(|&&replica_id| replica_id != from_replica_id)
			.map(|&to_replica_id| {
				let request_builder = request_builder.clone();
				async move {
					tokio::time::timeout(
						crate::consts::REQUEST_TIMEOUT,
						request_builder(to_replica_id),
					)
					.await
				}
			}),
	)
	.collect::<FuturesUnordered<_>>()
	.await;
	tracing::info!(?quorum_size, len = ?responses.len(), ?quorum_type, "fanout quorum size");

	// Choose how many successful responses we need before considering a success
	let target_responses = match quorum_type {
		// Only require 1 response
		utils::QuorumType::Any => 1,
		// Include all responses
		utils::QuorumType::All => responses.len(),
		// Subtract 1 from quorum size since we're not counting ourselves
		utils::QuorumType::Fast | utils::QuorumType::Slow => quorum_size - 1,
	};

	// Collect responses until we reach quorum or all futures complete
	let mut successful_responses = Vec::new();
	while successful_responses.len() < target_responses {
		if let Some(response) = responses.next().await {
			match response {
				std::result::Result::Ok(result) => match result {
					std::result::Result::Ok(response) => {
						successful_responses.push(response);
					}
					std::result::Result::Err(err) => {
						tracing::warn!(?err, "received error from replica");
					}
				},
				std::result::Result::Err(err) => {
					tracing::warn!(?err, "received timeout from replica");
				}
			}
		} else {
			// No more responses available
			break;
		}
	}

	Ok(successful_responses)
}

#[tracing::instrument(skip_all)]
pub async fn send_message(
	ctx: &ApiCtx,
	config: &protocol::ClusterConfig,
	request: protocol::Request,
) -> Result<protocol::Response> {
	let replica_url = find_replica_address(config, request.to_replica_id)?;
	send_message_to_address(ctx, replica_url, request).await
}

#[tracing::instrument(skip_all, fields(%replica_url))]
pub async fn send_message_to_address(
	ctx: &ApiCtx,
	replica_url: String,
	request: protocol::Request,
) -> Result<protocol::Response> {
	let from_replica_id = request.from_replica_id;
	let to_replica_id = request.to_replica_id;

	if from_replica_id == to_replica_id {
		tracing::info!(
			to_replica = to_replica_id,
			"sending message to replica directly"
		);

		return crate::replica::message_request::message_request(&ctx, request).await;
	}

	let mut replica_url = url::Url::parse(&replica_url)?;
	replica_url.set_path(&format!("/v{PROTOCOL_VERSION}/epoxy/message"));

	tracing::info!(
		to_replica = to_replica_id,
		%replica_url,
		"sending message to replica via http"
	);

	let client = rivet_pools::reqwest::client().await?;

	// Create the request
	let request = versioned::Request::wrap_latest(request)
		.serialize()
		.context("failed to serialize epoxy request")?;

	// Send the request
	let response_result = client
		.post(replica_url.to_string())
		.body(request)
		.send()
		.custom_instrument(tracing::info_span!("http_request"))
		.await;

	let response = match response_result {
		std::result::Result::Ok(resp) => resp,
		std::result::Result::Err(e) => {
			tracing::error!(
				to_replica = to_replica_id,
				replica_url = %replica_url,
				error = %e,
				error_debug = ?e,
				"failed to send HTTP request to replica"
			);
			bail!(
				"failed to send HTTP request to replica {}: {}",
				to_replica_id,
				e
			);
		}
	};

	// Check if the request was successful
	if !response.status().is_success() {
		tracing::warn!(
			status = %response.status(),
			to_replica = to_replica_id,
			replica_url = %replica_url,
			"message send failed with non-success status"
		);
		bail!(
			"message send to replica {} failed with status: {}",
			to_replica_id,
			response.status()
		);
	}

	let body = response.bytes().await?;
	let response_body = versioned::Response::deserialize(&body)?;

	tracing::info!(
		to_replica = to_replica_id,
		"successfully sent message via http"
	);

	Ok(response_body)
}
