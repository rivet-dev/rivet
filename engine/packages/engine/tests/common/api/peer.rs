#![allow(dead_code, unused_variables)]

use anyhow::*;
use rivet_api_types::{actors, namespaces, pagination, runner_configs, runners};

use super::get_endpoint;

// MARK: Helper functions

async fn parse_response<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> Result<T> {
	if !response.status().is_success() {
		let status = response.status();
		let text = response.text().await?;
		bail!("request failed with status {}: {}", status, text);
	}

	Ok(response.json().await?)
}

// MARK: Namespaces

pub async fn build_namespaces_list_request(
	port: u16,
	query: namespaces::list::ListQuery,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client.get(format!(
		"{}/namespaces?{}",
		get_endpoint(port),
		&serde_html_form::to_string(&query)?
	)))
}

pub async fn namespaces_list(
	port: u16,
	query: namespaces::list::ListQuery,
) -> Result<namespaces::list::ListResponse> {
	let request = build_namespaces_list_request(port, query).await?;
	let response = request.send().await?;
	parse_response(response).await
}

pub async fn build_namespaces_create_request(
	port: u16,
	request: rivet_api_peer::namespaces::CreateRequest,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client
		.post(format!("{}/namespaces", get_endpoint(port)))
		.json(&request))
}

pub async fn namespaces_create(
	port: u16,
	request: rivet_api_peer::namespaces::CreateRequest,
) -> Result<rivet_api_peer::namespaces::CreateResponse> {
	let req = build_namespaces_create_request(port, request).await?;
	let response = req.send().await?;
	parse_response(response).await
}

// MARK: Runner Configs

pub async fn build_runner_configs_list_request(
	port: u16,
	query: runner_configs::list::ListQuery,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client.get(format!(
		"{}/runner-configs?{}",
		get_endpoint(port),
		&serde_html_form::to_string(&query)?
	)))
}

pub async fn runner_configs_list(
	port: u16,
	query: runner_configs::list::ListQuery,
) -> Result<runner_configs::list::ListResponse> {
	let request = build_runner_configs_list_request(port, query).await?;
	let response = request.send().await?;
	parse_response(response).await
}

pub async fn build_runner_configs_upsert_request(
	port: u16,
	path: rivet_api_peer::runner_configs::UpsertPath,
	query: rivet_api_peer::runner_configs::UpsertQuery,
	request: rivet_api_peer::runner_configs::UpsertRequest,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client
		.put(format!(
			"{}/runner-configs/{}?{}",
			get_endpoint(port),
			path.runner_name,
			&serde_html_form::to_string(&query)?,
		))
		.json(&request))
}

pub async fn runner_configs_upsert(
	port: u16,
	path: rivet_api_peer::runner_configs::UpsertPath,
	query: rivet_api_peer::runner_configs::UpsertQuery,
	request: rivet_api_peer::runner_configs::UpsertRequest,
) -> Result<rivet_api_peer::runner_configs::UpsertResponse> {
	let req = build_runner_configs_upsert_request(port, path, query, request).await?;
	let response = req.send().await?;
	parse_response(response).await
}

pub async fn build_runner_configs_delete_request(
	port: u16,
	path: rivet_api_peer::runner_configs::DeletePath,
	query: rivet_api_peer::runner_configs::DeleteQuery,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client.delete(format!(
		"{}/runner-configs/{}?{}",
		get_endpoint(port),
		path.runner_name,
		&serde_html_form::to_string(&query)?,
	)))
}

pub async fn runner_configs_delete(
	port: u16,
	path: rivet_api_peer::runner_configs::DeletePath,
	query: rivet_api_peer::runner_configs::DeleteQuery,
) -> Result<rivet_api_peer::runner_configs::DeleteResponse> {
	let request = build_runner_configs_delete_request(port, path, query).await?;
	let response = request.send().await?;
	parse_response(response).await
}

// MARK: Actors

pub async fn build_actors_list_request(
	port: u16,
	query: actors::list::ListQuery,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client.get(format!(
		"{}/actors?{}",
		get_endpoint(port),
		&serde_html_form::to_string(&query)?
	)))
}

pub async fn actors_list(
	port: u16,
	query: actors::list::ListQuery,
) -> Result<actors::list::ListResponse> {
	let request = build_actors_list_request(port, query).await?;
	let response = request.send().await?;
	parse_response(response).await
}

pub async fn build_actors_create_request(
	port: u16,
	query: actors::create::CreateQuery,
	request: actors::create::CreateRequest,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client
		.post(format!(
			"{}/actors?{}",
			get_endpoint(port),
			&serde_html_form::to_string(&query)?
		))
		.json(&request))
}

pub async fn actors_create(
	port: u16,
	query: actors::create::CreateQuery,
	request: actors::create::CreateRequest,
) -> Result<actors::create::CreateResponse> {
	let req = build_actors_create_request(port, query, request).await?;
	let response = req.send().await?;
	parse_response(response).await
}

pub async fn build_actors_delete_request(
	port: u16,
	path: actors::delete::DeletePath,
	query: actors::delete::DeleteQuery,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client.delete(format!(
		"{}/actors/{}?{}",
		get_endpoint(port),
		path.actor_id,
		&serde_html_form::to_string(&query)?
	)))
}

pub async fn actors_delete(
	port: u16,
	path: actors::delete::DeletePath,
	query: actors::delete::DeleteQuery,
) -> Result<actors::delete::DeleteResponse> {
	let request = build_actors_delete_request(port, path, query).await?;
	let response = request.send().await?;
	parse_response(response).await
}

pub async fn build_actors_list_names_request(
	port: u16,
	query: actors::list_names::ListNamesQuery,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client.get(format!(
		"{}/actors/names?{}",
		get_endpoint(port),
		&serde_html_form::to_string(&query)?
	)))
}

pub async fn actors_list_names(
	port: u16,
	query: actors::list_names::ListNamesQuery,
) -> Result<actors::list_names::ListNamesResponse> {
	let request = build_actors_list_names_request(port, query).await?;
	let response = request.send().await?;
	parse_response(response).await
}

// MARK: Runners

pub async fn build_runners_list_request(
	port: u16,
	query: runners::list::ListQuery,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client.get(format!(
		"{}/runners?{}",
		get_endpoint(port),
		&serde_html_form::to_string(&query)?
	)))
}

pub async fn runners_list(
	port: u16,
	query: runners::list::ListQuery,
) -> Result<runners::list::ListResponse> {
	let request = build_runners_list_request(port, query).await?;
	let response = request.send().await?;
	parse_response(response).await
}

pub async fn build_runners_list_names_request(
	port: u16,
	query: runners::list_names::ListNamesQuery,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client.get(format!(
		"{}/runners/names?{}",
		get_endpoint(port),
		&serde_html_form::to_string(&query)?
	)))
}

pub async fn runners_list_names(
	port: u16,
	query: runners::list_names::ListNamesQuery,
) -> Result<runners::list_names::ListNamesResponse> {
	let request = build_runners_list_names_request(port, query).await?;
	let response = request.send().await?;
	parse_response(response).await
}

// MARK: Internal

pub async fn build_cache_purge_request(
	port: u16,
	request: rivet_api_peer::internal::CachePurgeRequest,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client
		.post(format!("{}/cache/purge", get_endpoint(port)))
		.json(&request))
}

pub async fn cache_purge(
	port: u16,
	request: rivet_api_peer::internal::CachePurgeRequest,
) -> Result<rivet_api_peer::internal::CachePurgeResponse> {
	let req = build_cache_purge_request(port, request).await?;
	let response = req.send().await?;
	parse_response(response).await
}

pub async fn build_epoxy_replica_reconfigure_request(
	port: u16,
	request: rivet_api_peer::internal::ReplicaReconfigureRequest,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client
		.post(format!(
			"{}/epoxy/coordinator/replica-reconfigure",
			get_endpoint(port)
		))
		.json(&request))
}

pub async fn epoxy_replica_reconfigure(
	port: u16,
	request: rivet_api_peer::internal::ReplicaReconfigureRequest,
) -> Result<rivet_api_peer::internal::ReplicaReconfigureResponse> {
	let req = build_epoxy_replica_reconfigure_request(port, request).await?;
	let response = req.send().await?;
	parse_response(response).await
}

pub async fn build_get_epoxy_state_request(port: u16) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client.get(format!("{}/epoxy/coordinator/state", get_endpoint(port))))
}

pub async fn get_epoxy_state(port: u16) -> Result<rivet_api_peer::internal::GetEpoxyStateResponse> {
	let request = build_get_epoxy_state_request(port).await?;
	let response = request.send().await?;
	parse_response(response).await
}

pub async fn build_set_epoxy_state_request(
	port: u16,
	request: rivet_api_peer::internal::SetEpoxyStateRequest,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client
		.post(format!("{}/epoxy/coordinator/state", get_endpoint(port)))
		.json(&request))
}

pub async fn set_epoxy_state(
	port: u16,
	request: rivet_api_peer::internal::SetEpoxyStateRequest,
) -> Result<rivet_api_peer::internal::SetEpoxyStateResponse> {
	let req = build_set_epoxy_state_request(port, request).await?;
	let response = req.send().await?;
	parse_response(response).await
}

pub async fn build_set_tracing_config_request(
	port: u16,
	request: rivet_api_peer::internal::SetTracingConfigRequest,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client
		.put(format!("{}/debug/tracing/config", get_endpoint(port)))
		.json(&request))
}

pub async fn set_tracing_config(
	port: u16,
	request: rivet_api_peer::internal::SetTracingConfigRequest,
) -> Result<rivet_api_peer::internal::SetTracingConfigResponse> {
	let req = build_set_tracing_config_request(port, request).await?;
	let response = req.send().await?;
	parse_response(response).await
}
