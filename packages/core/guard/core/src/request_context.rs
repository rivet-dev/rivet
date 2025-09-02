use std::{net::IpAddr, time::SystemTime};

use anyhow::*;
use uuid::Uuid;

use crate::analytics::GuardHttpRequest;

// Properties not currently tracked but should be added in future iterations:
// - client_ssl_cipher: Requires TLS connection introspection
// - client_ssl_protocol: Requires TLS connection introspection
// - client_tcp_rtt_ms: Requires network-level measurements
// - client_request_bytes: Total request size including headers
// - service_dns_response_time_ms: Requires DNS timing instrumentation
// - service_ssl_protocol: Requires upstream TLS introspection
// - service_tcp_handshake_duration_ms: Requires connection-level timing
// - service_tls_handshake_duration_ms: Requires TLS handshake timing
// - service_request_header_send_duration_ms: Requires granular timing
// - service_response_header_receive_duration_ms: Requires granular timing
// - guard_response_bytes: Total response size including headers
// - guard_time_to_first_byte_ms: Requires granular timing
// - security_rule_id: Requires security/firewall rule integration

#[derive(Clone)]
pub struct RequestContext {
	// Request tracking data
	pub request_id: Uuid,
	pub client_ip: Option<IpAddr>,
	pub client_request_body_bytes: Option<u64>,
	pub client_request_host: Option<String>,
	pub client_request_method: Option<String>,
	pub client_request_path: Option<String>,
	pub client_request_protocol: Option<String>,
	pub client_request_referer: Option<String>,
	pub client_request_scheme: Option<String>,
	pub client_request_uri: Option<String>,
	pub client_request_user_agent: Option<String>,
	pub client_src_port: Option<u16>,
	pub client_x_requested_with: Option<String>,

	// Guard tracking data
	pub guard_datacenter_id: Option<Uuid>,
	pub guard_cluster_id: Option<Uuid>,
	pub guard_server_id: Option<Uuid>,
	pub guard_end_timestamp: Option<SystemTime>,
	pub guard_response_body_bytes: Option<u64>,
	pub guard_response_content_type: Option<String>,
	pub guard_response_status: Option<u16>,
	pub guard_start_timestamp: SystemTime,

	// Service tracking data
	pub service_ip: Option<IpAddr>,
	pub service_response_duration_ms: Option<u32>,
	pub service_response_http_expires: Option<String>,
	pub service_response_http_last_modified: Option<String>,
	pub service_response_status: Option<u16>,
	pub service_actor_id: Option<rivet_util::Id>,

	// ClickHouse inserter handle
	clickhouse_inserter: Option<clickhouse_inserter::ClickHouseInserterHandle>,
}

impl RequestContext {
	pub fn new(clickhouse_inserter: Option<clickhouse_inserter::ClickHouseInserterHandle>) -> Self {
		Self::new_with_request_id(Uuid::new_v4(), clickhouse_inserter)
	}

	pub fn new_with_request_id(
		request_id: Uuid,
		clickhouse_inserter: Option<clickhouse_inserter::ClickHouseInserterHandle>,
	) -> Self {
		Self {
			request_id,
			client_ip: None,
			client_request_body_bytes: None,
			client_request_host: None,
			client_request_method: None,
			client_request_path: None,
			client_request_protocol: None,
			client_request_referer: None,
			client_request_scheme: None,
			client_request_uri: None,
			client_request_user_agent: None,
			client_src_port: None,
			client_x_requested_with: None,
			guard_datacenter_id: None,
			guard_cluster_id: None,
			guard_server_id: None,
			guard_end_timestamp: None,
			guard_response_body_bytes: None,
			guard_response_content_type: None,
			guard_response_status: None,
			guard_start_timestamp: SystemTime::now(),
			service_ip: None,
			service_response_duration_ms: None,
			service_response_http_expires: None,
			service_response_http_last_modified: None,
			service_response_status: None,
			service_actor_id: None,
			clickhouse_inserter,
		}
	}

	// Finalize the request and insert analytics event
	pub async fn insert_event(&mut self) -> Result<()> {
		let Some(inserter) = &self.clickhouse_inserter else {
			return Ok(()); // No inserter available
		};

		// Set end timestamp
		self.guard_end_timestamp = Some(SystemTime::now());

		// Convert IP addresses to strings for ClickHouse IPv4 type
		let client_ip = match self.client_ip {
			Some(IpAddr::V4(ip)) => ip.to_string(),
			Some(IpAddr::V6(_)) => "0.0.0.0".to_string(), // Fallback for IPv6 addresses
			None => "0.0.0.0".to_string(),                // Default fallback
		};

		let service_ip = match self.service_ip {
			Some(IpAddr::V4(ip)) => ip.to_string(),
			Some(IpAddr::V6(_)) => "0.0.0.0".to_string(), // Fallback for IPv6 addresses
			None => "127.0.0.1".to_string(),              // Default fallback
		};

		// Convert SystemTime to nanoseconds since Unix epoch for ClickHouse DateTime64(9)
		let guard_start_timestamp = self
			.guard_start_timestamp
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or_default()
			.as_nanos() as u64;

		let guard_end_timestamp = self
			.guard_end_timestamp
			.unwrap_or_else(SystemTime::now)
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or_default()
			.as_nanos() as u64;

		// Build the analytics event inline with defaults for missing values
		let analytics_event = GuardHttpRequest {
			request_id: self.request_id,
			client_ip,
			client_request_body_bytes: self.client_request_body_bytes.unwrap_or_default(),
			client_request_host: self.client_request_host.clone().unwrap_or_default(),
			client_request_method: self.client_request_method.clone().unwrap_or_default(),
			client_request_path: self.client_request_path.clone().unwrap_or_default(),
			client_request_protocol: self.client_request_protocol.clone().unwrap_or_default(),
			client_request_referer: self.client_request_referer.clone().unwrap_or_default(),
			client_request_scheme: self.client_request_scheme.clone().unwrap_or_default(),
			client_request_uri: self.client_request_uri.clone().unwrap_or_default(),
			client_request_user_agent: self.client_request_user_agent.clone().unwrap_or_default(),
			client_src_port: self.client_src_port.unwrap_or_default(),
			client_x_requested_with: self.client_x_requested_with.clone().unwrap_or_default(),
			guard_datacenter_id: self.guard_datacenter_id.unwrap_or_default(),
			guard_cluster_id: self.guard_cluster_id.unwrap_or_default(),
			guard_server_id: self.guard_server_id.unwrap_or_default(),
			guard_end_timestamp,
			guard_response_body_bytes: self.guard_response_body_bytes.unwrap_or_default(),
			guard_response_content_type: self
				.guard_response_content_type
				.clone()
				.unwrap_or_default(),
			guard_response_status: self.guard_response_status.unwrap_or_default(),
			guard_start_timestamp,
			service_ip,
			service_response_duration_ms: self.service_response_duration_ms.unwrap_or_default(),
			service_response_http_expires: self
				.service_response_http_expires
				.clone()
				.unwrap_or_default(),
			service_response_http_last_modified: self
				.service_response_http_last_modified
				.clone()
				.unwrap_or_default(),
			service_response_status: self.service_response_status.unwrap_or_default(),
			service_actor_id: self
				.service_actor_id
				.map(|x| x.to_string())
				.unwrap_or_default(),
		};

		// Insert the event asynchronously
		inserter.insert("db_guard_analytics", "http_requests", analytics_event)?;

		Ok(())
	}
}

impl std::fmt::Debug for RequestContext {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RequestContext").finish_non_exhaustive()
	}
}
