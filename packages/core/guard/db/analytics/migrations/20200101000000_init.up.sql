CREATE TABLE IF NOT EXISTS http_requests
(
    namespace LowCardinality(String),
	request_id UUID,
	client_ip IPv4,
	client_request_body_bytes UInt64,
	client_request_host String,
	client_request_method LowCardinality(String),
	client_request_path String,
	client_request_protocol LowCardinality(String),
	client_request_referer String,
	client_request_scheme LowCardinality(String),
	client_request_uri String,
	client_request_user_agent String,
	client_src_port UInt16,
	client_x_requested_with String,
	guard_datacenter_id UUID,
	guard_cluster_id UUID,
	guard_server_id UUID,
	guard_end_timestamp DateTime64(9),
	guard_response_body_bytes UInt64,
	guard_response_content_type String,
	guard_response_status UInt16,
	guard_start_timestamp DateTime64(9),
	service_ip IPv4,
	service_response_duration_ms UInt32,
	service_response_http_expires String,
	service_response_http_last_modified String,
	service_response_status UInt16,
	service_actor_id String
)
ENGINE = ReplicatedMergeTree()
PARTITION BY toStartOfHour(guard_start_timestamp)
ORDER BY (namespace, guard_start_timestamp, request_id)
TTL toDate(guard_start_timestamp + toIntervalDay(30))
SETTINGS index_granularity = 8192, ttl_only_drop_parts = 1;
