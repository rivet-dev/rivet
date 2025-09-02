use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Properties not currently collected but should be added in future iterations:
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardHttpRequest {
	pub request_id: Uuid,
	pub client_ip: String,
	pub client_request_body_bytes: u64,
	pub client_request_host: String,
	pub client_request_method: String,
	pub client_request_path: String,
	pub client_request_protocol: String,
	pub client_request_referer: String,
	pub client_request_scheme: String,
	pub client_request_uri: String,
	pub client_request_user_agent: String,
	pub client_src_port: u16,
	pub client_x_requested_with: String,
	pub guard_datacenter_id: Uuid,
	pub guard_cluster_id: Uuid,
	pub guard_server_id: Uuid,
	pub guard_end_timestamp: u64,
	pub guard_response_body_bytes: u64,
	pub guard_response_content_type: String,
	pub guard_response_status: u16,
	pub guard_start_timestamp: u64,
	pub service_ip: String,
	pub service_response_duration_ms: u32,
	pub service_response_http_expires: String,
	pub service_response_http_last_modified: String,
	pub service_response_status: u16,
	pub service_actor_id: String,
}
