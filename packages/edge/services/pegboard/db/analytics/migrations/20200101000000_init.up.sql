-- row_updated_at = determines the latest version of this row
-- created_at = used in ORDER BY in order mitigate random insertion order
-- ttl_only_drop_parts = 0 means we can't drop by entire partitions bc the destroy ts is not in PARTITION BY
CREATE TABLE IF NOT EXISTS actors
(
    namespace LowCardinality(String),
    actor_id String,
    project_id UUID,
    env_id UUID,
    datacenter_id UUID,
    tags Map(String, String),
    build_id UUID,
    build_kind UInt8,
    build_compression UInt8,
    network_mode UInt8,
    network_ports Map(String, Tuple(
        internal_port UInt16,
        routing_guard Bool,
        routing_host Bool,
        routing_guard_protocol UInt8,
        routing_host_protocol UInt8
    )),
    network_ports_ingress Map(String, Tuple(
        port_number UInt16,
        ingress_port_number UInt16,
        protocol UInt8
    )),
    network_ports_host Map(String, Tuple(
        port_number UInt16,
        protocol UInt8
    )),
    network_ports_proxied Map(String, Tuple(
        ip String,
        source UInt8
    )),
    client_id UUID,
    client_wan_hostname String,
    selected_cpu_millicores UInt32,
    selected_memory_mib UInt32,
    root_user_enabled Bool,
    env_vars Int64,
    env_var_bytes Int64,
    args Int64,
    args_bytes Int64,
    durable Bool,
    kill_timeout Int64,
    cpu_millicores UInt32,
    memory_mib UInt32,
    created_at DateTime64(9),
    started_at DateTime64(9),
    connectable_at DateTime64(9),
    finished_at DateTime64(9),
    destroyed_at DateTime64(9),
    row_updated_at DateTime64(9),

	INDEX idx_actor_id actor_id TYPE bloom_filter GRANULARITY 1
)
ENGINE = ReplicatedReplacingMergeTree(row_updated_at)
PARTITION BY (namespace, env_id, toStartOfHour(created_at))
ORDER BY (env_id, created_at, actor_id)
TTL toDate(multiIf(destroyed_at > 0, destroyed_at + toIntervalDay(90), toDateTime64('2099-12-31 23:59:59', 9)))
SETTINGS index_granularity = 8192, ttl_only_drop_parts = 0;
