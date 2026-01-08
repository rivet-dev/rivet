use anyhow::{Context, Result, bail};
use base64::Engine;
use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::util::udb::{SimpleTuple, SimpleTupleValue};

#[derive(Parser)]
pub enum SubCommand {
	/// Get debug information for the local replica
	ReplicaDebug {
		/// API peer endpoint (defaults to topology peer_url)
		#[clap(long)]
		endpoint: Option<String>,
	},
	/// Get coordinator cluster state
	CoordinatorState {
		/// API peer endpoint (defaults to topology peer_url)
		#[clap(long)]
		endpoint: Option<String>,
	},
	/// Trigger coordinator to reconfigure all replicas
	CoordinatorReconfigure {
		/// API peer endpoint (defaults to topology peer_url)
		#[clap(long)]
		endpoint: Option<String>,
	},
	/// Set coordinator cluster state
	CoordinatorSetState {
		/// JSON file containing the cluster config
		#[clap(long)]
		config_file: String,
		/// API peer endpoint (defaults to topology peer_url)
		#[clap(long)]
		endpoint: Option<String>,
	},
	/// Get debug information for a specific key on the local replica
	KeyDebug {
		/// Key to inspect using tuple format (same as UDB CLI).
		key: String,
		/// Interpret the key as base64-encoded instead of tuple format
		#[clap(long)]
		base64: bool,
		/// API peer endpoint (defaults to topology peer_url)
		#[clap(long)]
		endpoint: Option<String>,
	},
	/// Get debug information for a specific key across all replicas (fanout)
	KeyDebugFanout {
		/// Key to inspect using tuple format (same as UDB CLI).
		key: String,
		/// Interpret the key as base64-encoded instead of tuple format
		#[clap(long)]
		base64: bool,
		/// API peer endpoint (defaults to topology peer_url)
		#[clap(long)]
		endpoint: Option<String>,
	},
	/// Get value at a key from local replica only
	GetLocal {
		/// Key to get using tuple format (same as UDB CLI).
		key: String,
		/// Interpret the key as base64-encoded instead of tuple format
		#[clap(long)]
		base64: bool,
		/// Optional type hint for value parsing (u64, i64, f64, uuid, id, str, bytes, json, raw)
		#[clap(short = 't', long = "type")]
		type_hint: Option<String>,
		/// API peer endpoint (defaults to topology peer_url)
		#[clap(long)]
		endpoint: Option<String>,
	},
	/// Get value at a key with optimistic read (local + cache + fanout)
	GetOptimistic {
		/// Key to get using tuple format (same as UDB CLI).
		key: String,
		/// Interpret the key as base64-encoded instead of tuple format
		#[clap(long)]
		base64: bool,
		/// Optional type hint for value parsing (u64, i64, f64, uuid, id, str, bytes, json, raw)
		#[clap(short = 't', long = "type")]
		type_hint: Option<String>,
		/// API peer endpoint (defaults to topology peer_url)
		#[clap(long)]
		endpoint: Option<String>,
	},
	/// Set value at a key in UDB (same as UDB CLI set)
	Set {
		/// Key to set using tuple format (same as UDB CLI).
		key: String,
		/// Value to set, with optional type prefix (e.g. "u64:123")
		value: String,
		/// Interpret the key as base64-encoded instead of tuple format
		#[clap(long)]
		base64: bool,
		/// Optional type hint for the value
		#[clap(short = 't', long = "type")]
		type_hint: Option<String>,
		/// API peer endpoint (defaults to topology peer_url)
		#[clap(long)]
		endpoint: Option<String>,
	},
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		match self {
			Self::ReplicaDebug { endpoint } => {
				let endpoint = get_endpoint(&config, endpoint)?;
				let url = format!("{}/epoxy/replica/debug", endpoint);

				let response = make_get_request(&url).await?;
				print_json(&response)?;

				Ok(())
			}
			Self::CoordinatorState { endpoint } => {
				let endpoint = get_endpoint(&config, endpoint)?;
				let url = format!("{}/epoxy/coordinator/state", endpoint);

				let response = make_get_request(&url).await?;
				print_json(&response)?;

				Ok(())
			}
			Self::CoordinatorReconfigure { endpoint } => {
				let endpoint = get_endpoint(&config, endpoint)?;
				let url = format!("{}/epoxy/coordinator/replica-reconfigure", endpoint);

				let client = rivet_pools::reqwest::client().await?;
				let response = client
					.post(&url)
					.json(&serde_json::json!({}))
					.send()
					.await
					.context("failed to send request")?;

				if response.status().is_success() {
					println!("Reconfigure signal sent successfully");
				} else {
					let status = response.status();
					let body = response.text().await.unwrap_or_default();
					anyhow::bail!("Failed to trigger reconfigure: {} - {}", status, body);
				}

				Ok(())
			}
			Self::CoordinatorSetState {
				config_file,
				endpoint,
			} => {
				let endpoint = get_endpoint(&config, endpoint)?;
				let url = format!("{}/epoxy/coordinator/state", endpoint);

				// Read config file
				let config_content = tokio::fs::read_to_string(&config_file)
					.await
					.context("failed to read config file")?;
				let cluster_config: serde_json::Value = serde_json::from_str(&config_content)
					.context("failed to parse config file as JSON")?;

				let client = rivet_pools::reqwest::client().await?;
				let response = client
					.post(&url)
					.json(&SetEpoxyStateRequest {
						config: cluster_config,
					})
					.send()
					.await
					.context("failed to send request")?;

				if response.status().is_success() {
					println!("Coordinator state updated successfully");
				} else {
					let status = response.status();
					let body = response.text().await.unwrap_or_default();
					anyhow::bail!("Failed to set coordinator state: {} - {}", status, body);
				}

				Ok(())
			}
			Self::KeyDebug {
				key,
				base64: is_base64,
				endpoint,
			} => {
				let endpoint = get_endpoint(&config, endpoint)?;
				let encoded_key = encode_key(&key, is_base64)?;
				let url = format!("{}/epoxy/replica/key/{}", endpoint, encoded_key);

				let response = make_get_request(&url).await?;
				print_json(&response)?;

				Ok(())
			}
			Self::KeyDebugFanout {
				key,
				base64: is_base64,
				endpoint,
			} => {
				let endpoint = get_endpoint(&config, endpoint)?;
				let encoded_key = encode_key(&key, is_base64)?;
				let url = format!("{}/epoxy/replica/key/{}/fanout", endpoint, encoded_key);

				let response = make_get_request(&url).await?;
				print_json(&response)?;

				Ok(())
			}
			Self::GetLocal {
				key,
				base64: is_base64,
				type_hint,
				endpoint,
			} => {
				let endpoint = get_endpoint(&config, endpoint)?;
				let encoded_key = encode_key(&key, is_base64)?;
				let url = format!("{}/epoxy/replica/kv/{}/local", endpoint, encoded_key);

				print_kv_response(&url, type_hint.as_deref()).await
			}
			Self::GetOptimistic {
				key,
				base64: is_base64,
				type_hint,
				endpoint,
			} => {
				let endpoint = get_endpoint(&config, endpoint)?;
				let encoded_key = encode_key(&key, is_base64)?;
				let url = format!("{}/epoxy/replica/kv/{}/optimistic", endpoint, encoded_key);

				print_kv_response(&url, type_hint.as_deref()).await
			}
			Self::Set {
				key,
				value,
				base64: is_base64,
				type_hint,
				endpoint,
			} => {
				let endpoint = get_endpoint(&config, endpoint)?;
				let encoded_key = encode_key(&key, is_base64)?;
				let url = format!("{}/epoxy/replica/kv/{}", endpoint, encoded_key);

				// Parse value using SimpleTupleValue (same as udb CLI)
				let parsed_value =
					SimpleTupleValue::parse_str_with_type_hint(type_hint.as_deref(), &value)?;
				let value_bytes = parsed_value.serialize()?;
				let value_b64 = base64::engine::general_purpose::STANDARD.encode(&value_bytes);

				let client = rivet_pools::reqwest::client().await?;
				let response = client
					.put(&url)
					.json(&SetKvRequest {
						value: Some(value_b64),
					})
					.send()
					.await
					.context("failed to send request")?;

				if response.status().is_success() {
					let result: SetKvResponse =
						response.json().await.context("failed to parse response")?;
					println!("{}", result.result);
				} else {
					let status = response.status();
					let body = response.text().await.unwrap_or_default();
					anyhow::bail!("Failed to set value: {} - {}", status, body);
				}

				Ok(())
			}
		}
	}
}

#[derive(Serialize, Deserialize)]
struct SetEpoxyStateRequest {
	config: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct GetKvResponse {
	exists: bool,
	value: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct SetKvRequest {
	value: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct SetKvResponse {
	result: String,
}

fn get_endpoint(config: &rivet_config::Config, endpoint: Option<String>) -> Result<String> {
	match endpoint {
		Some(e) => Ok(e),
		None => {
			let topology = config
				.topology
				.as_ref()
				.context("topology not configured")?;
			let dc = topology.current_dc()?;
			Ok(dc.peer_url.to_string().trim_end_matches('/').to_string())
		}
	}
}

/// Encodes a key for use in the API URL.
///
/// If `is_base64` is true, the key is assumed to already be base64-encoded.
/// Otherwise, the key is parsed as a tuple path (e.g., "/rivet/epoxy/replica/1/key_instance/b:deadbeef")
/// and tuple-packed before being base64-encoded.
fn encode_key(key: &str, is_base64: bool) -> Result<String> {
	use base64::Engine;

	if is_base64 {
		// Validate it's valid base64
		base64::engine::general_purpose::STANDARD
			.decode(key)
			.context("invalid base64 key")?;
		Ok(key.to_string())
	} else {
		// Parse as tuple path and encode
		let (tuple, _relative, back_count) =
			SimpleTuple::parse(key).context("failed to parse key as tuple path")?;

		if back_count > 0 {
			bail!("relative paths with '..' are not supported for key lookup");
		}

		// Tuple-pack the key
		let packed = universaldb::tuple::pack(&tuple);

		// Base64 encode for URL
		Ok(base64::engine::general_purpose::STANDARD.encode(&packed))
	}
}

async fn make_get_request<T: serde::de::DeserializeOwned>(url: &str) -> Result<T> {
	let client = rivet_pools::reqwest::client().await?;
	let response = client
		.get(url)
		.send()
		.await
		.context("failed to send request")?;

	if response.status().is_success() {
		let body = response
			.json::<T>()
			.await
			.context("failed to parse response")?;
		Ok(body)
	} else {
		let status = response.status();
		let body = response.text().await.unwrap_or_default();
		anyhow::bail!("Request failed: {} - {}", status, body);
	}
}

fn print_json(value: &serde_json::Value) -> Result<()> {
	let output = colored_json::to_colored_json_auto(value)?;
	println!("{}", output);
	Ok(())
}

async fn print_kv_response(url: &str, type_hint: Option<&str>) -> Result<()> {
	let response: GetKvResponse = make_get_request(url).await?;

	if !response.exists {
		println!("key does not exist");
	} else if let Some(value_b64) = response.value {
		let value_bytes = base64::engine::general_purpose::STANDARD
			.decode(&value_b64)
			.context("invalid base64 value from server")?;

		if type_hint == Some("raw") {
			println!("{}", SimpleTupleValue::Unknown(value_bytes));
		} else {
			match SimpleTupleValue::deserialize(type_hint, &value_bytes) {
				Ok(parsed) => {
					let mut s = String::new();
					parsed.write(&mut s, false).unwrap();
					println!("{s}");
				}
				Err(err) => println!("error: {err:#}"),
			}
		}
	}

	Ok(())
}
