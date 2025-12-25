#![allow(dead_code, unused_variables)]

use anyhow::*;
use rivet_api_types::{actors, datacenters, namespaces, pagination, runner_configs, runners};
use serde::{Deserialize, Serialize};

use super::get_endpoint;

// MARK: Helper functions

async fn parse_response<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> Result<T> {
	if !response.status().is_success() {
		let status = response.status();
		let text = response.text().await?;
		bail!("request failed with status {}: {}", status, text);
	}

	Ok(response.json().await.context("failed to parse response")?)
}

// MARK: Metadata

#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataResponse {
	pub runtime: String,
	pub version: String,
	pub git_sha: String,
	pub build_timestamp: String,
	pub rustc_version: String,
	pub rustc_host: String,
	pub cargo_target: String,
	pub cargo_profile: String,
}

pub async fn build_metadata_get_request(port: u16) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client.get(format!("{}/metadata", get_endpoint(port))))
}

pub async fn metadata_get(port: u16) -> Result<MetadataResponse> {
	let request = build_metadata_get_request(port).await?;
	let response = request.send().await?;
	parse_response(response).await
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
		serde_html_form::to_string(&query)?
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
		serde_html_form::to_string(&query)?
	)))
}

pub async fn runner_configs_list(
	port: u16,
	query: runner_configs::list::ListQuery,
) -> Result<rivet_api_public::runner_configs::list::ListResponse> {
	let request = build_runner_configs_list_request(port, query).await?;
	let response = request.send().await?;
	parse_response(response).await
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerlessHealthCheckQuery {
	pub namespace: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerlessHealthCheckRequest {
	pub url: String,
	#[serde(default)]
	pub headers: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerlessHealthCheckResponse {
	Success { version: String },
	Failure { error: ServerlessMetadataError },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerlessMetadataError {
	pub message: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub details: Option<String>,
}

pub async fn build_runner_configs_serverless_health_check_request(
	port: u16,
	query: ServerlessHealthCheckQuery,
	request: ServerlessHealthCheckRequest,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client
		.post(format!(
			"{}/runner-configs/serverless-health-check?{}",
			get_endpoint(port),
			serde_html_form::to_string(&query)?,
		))
		.json(&request))
}

pub async fn runner_configs_serverless_health_check(
	port: u16,
	query: ServerlessHealthCheckQuery,
	request: ServerlessHealthCheckRequest,
) -> Result<ServerlessHealthCheckResponse> {
	let req = build_runner_configs_serverless_health_check_request(port, query, request).await?;
	let response = req.send().await?;
	parse_response(response).await
}

pub async fn build_runner_configs_upsert_request(
	port: u16,
	path: rivet_api_peer::runner_configs::UpsertPath,
	query: rivet_api_peer::runner_configs::UpsertQuery,
	request: rivet_api_public::runner_configs::upsert::UpsertRequest,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client
		.put(format!(
			"{}/runner-configs/{}?{}",
			get_endpoint(port),
			path.runner_name,
			serde_html_form::to_string(&query)?,
		))
		.json(&request))
}

pub async fn runner_configs_upsert(
	port: u16,
	path: rivet_api_peer::runner_configs::UpsertPath,
	query: rivet_api_peer::runner_configs::UpsertQuery,
	request: rivet_api_public::runner_configs::upsert::UpsertRequest,
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
		serde_html_form::to_string(&query)?,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshMetadataQuery {
	pub namespace: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshMetadataRequest {}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshMetadataResponse {}

pub async fn build_runner_configs_refresh_metadata_request(
	port: u16,
	runner_name: String,
	query: RefreshMetadataQuery,
	request: RefreshMetadataRequest,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client
		.post(format!(
			"{}/runner-configs/{}/refresh-metadata?{}",
			get_endpoint(port),
			runner_name,
			serde_html_form::to_string(&query)?
		))
		.json(&request))
}

pub async fn runner_configs_refresh_metadata(
	port: u16,
	runner_name: String,
	query: RefreshMetadataQuery,
	request: RefreshMetadataRequest,
) -> Result<RefreshMetadataResponse> {
	let req =
		build_runner_configs_refresh_metadata_request(port, runner_name, query, request).await?;
	let response = req.send().await?;
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
		serde_html_form::to_string(&query)?
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
			serde_html_form::to_string(&query)?
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

#[derive(Debug, Serialize, Deserialize)]
pub struct GetOrCreateQuery {
	pub namespace: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetOrCreateRequest {
	pub datacenter: Option<String>,
	pub name: String,
	pub key: String,
	pub input: Option<String>,
	pub runner_name_selector: String,
	pub crash_policy: rivet_types::actors::CrashPolicy,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetOrCreateResponse {
	pub actor: rivet_types::actors::Actor,
	pub created: bool,
}

pub async fn build_actors_get_or_create_request(
	port: u16,
	query: GetOrCreateQuery,
	request: GetOrCreateRequest,
) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client
		.put(format!(
			"{}/actors?{}",
			get_endpoint(port),
			serde_html_form::to_string(&query)?
		))
		.json(&request))
}

pub async fn actors_get_or_create(
	port: u16,
	query: GetOrCreateQuery,
	request: GetOrCreateRequest,
) -> Result<GetOrCreateResponse> {
	let req = build_actors_get_or_create_request(port, query, request).await?;
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
		serde_html_form::to_string(&query)?
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
		serde_html_form::to_string(&query)?
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
		serde_html_form::to_string(&query)?
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
		serde_html_form::to_string(&query)?
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

// MARK: Datacenters

pub async fn build_datacenters_list_request(port: u16) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client.get(format!("{}/datacenters", get_endpoint(port))))
}

pub async fn datacenters_list(port: u16) -> Result<datacenters::list::ListResponse> {
	let request = build_datacenters_list_request(port).await?;
	let response = request.send().await?;
	parse_response(response).await
}

// MARK: Health

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthFanoutResponse {
	pub datacenters: std::collections::HashMap<String, HealthStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
	pub healthy: bool,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

pub async fn build_health_fanout_request(port: u16) -> Result<reqwest::RequestBuilder> {
	let client = rivet_pools::reqwest::client().await?;
	Ok(client.get(format!("{}/health/fanout", get_endpoint(port))))
}

pub async fn health_fanout(port: u16) -> Result<HealthFanoutResponse> {
	let request = build_health_fanout_request(port).await?;
	let response = request.send().await?;
	parse_response(response).await
}
