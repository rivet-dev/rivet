use anyhow::*;
use epoxy_protocol::{
	PROTOCOL_VERSION,
	protocol::{self, ReplicaId},
	versioned,
};
use futures_util::{StreamExt, stream::FuturesUnordered};
use std::future::Future;
use versioned_data_util::OwnedVersionedData;

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

	// Collect responses until we reach quorum or all futures complete
	//
	// Subtract 1 from quorum size since we're not counting ourselves
	let mut successful_responses = Vec::new();
	while successful_responses.len() < quorum_size - 1 {
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

pub async fn send_message(
	config: &protocol::ClusterConfig,
	to_replica_id: ReplicaId,
	request: protocol::Request,
) -> Result<protocol::Response> {
	let replica_url = find_replica_address(config, to_replica_id)?;
	send_message_to_address(replica_url, to_replica_id, request).await
}

pub async fn send_message_to_address(
	replica_url: String,
	to_replica_id: ReplicaId,
	request: protocol::Request,
) -> Result<protocol::Response> {
	let mut replica_url = url::Url::parse(&replica_url)?;
	replica_url.set_path(&format!("/v{PROTOCOL_VERSION}/epoxy/message"));

	tracing::info!(
		to_replica = to_replica_id,
		%replica_url,
		"sending message to replica via http"
	);

	let client = rivet_pools::reqwest::client().await?;

	// Create the request
	let request = versioned::Request::latest(request);

	// Send the request
	let response_result = client
		.post(replica_url.to_string())
		.body(request.serialize()?)
		.send()
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
