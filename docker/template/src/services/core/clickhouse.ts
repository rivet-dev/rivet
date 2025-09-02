import { TemplateContext } from "../../context";

export function generateCoreClickhouse(context: TemplateContext) {
	// ClickHouse configuration
	const configXml = `<?xml version="1.0"?>
<yandex>
	<logger>
		<level>debug</level>
		<log>/var/log/clickhouse-server/clickhouse-server.log</log>
		<errorlog>/var/log/clickhouse-server/clickhouse-server.err.log</errorlog>
		<size>1000M</size>
		<count>10</count>
	</logger>

	<listen_host>::</listen_host>
	<http_port>9300</http_port>
	<tcp_port>9301</tcp_port>
	
	<!-- Additional client configuration for init scripts -->
	<tcp_port_secure>9440</tcp_port_secure>
	<tcp_with_proxy_port>9010</tcp_with_proxy_port>

	<interserver_http_port>9009</interserver_http_port>

	<max_connections>4096</max_connections>
	<keep_alive_timeout>3</keep_alive_timeout>
	<max_concurrent_queries>100</max_concurrent_queries>
	<uncompressed_cache_size>8589934592</uncompressed_cache_size>
	<mark_cache_size>5368709120</mark_cache_size>

	<path>/var/lib/clickhouse/</path>
	<tmp_path>/var/lib/clickhouse/tmp/</tmp_path>
	<user_files_path>/var/lib/clickhouse/user_files/</user_files_path>
	<access_control_path>/var/lib/clickhouse/access/</access_control_path>

	<users_config>users.xml</users_config>
	<default_profile>default</default_profile>
	<default_database>default</default_database>
	<timezone>UTC</timezone>

	<remote_servers incl="clickhouse_remote_servers" />
	<zookeeper incl="zookeeper-servers" optional="true" />
	<macros incl="macros" optional="true" />

	<builtin_dictionaries_reload_interval>3600</builtin_dictionaries_reload_interval>
	<max_session_timeout>3600</max_session_timeout>
	<default_session_timeout>60</default_session_timeout>

	<query_log>
		<database>system</database>
		<table>query_log</table>
		<flush_interval_milliseconds>7500</flush_interval_milliseconds>
	</query_log>

	<dictionaries_config>*_dictionary.xml</dictionaries_config>
</yandex>
`;

	const usersXml = `<?xml version="1.0"?>
<yandex>
	<users>
		<system>
			<password type="plaintext_password">default</password>
			<networks incl="networks" replace="replace">
				<ip>::/0</ip>
			</networks>
			<profile>default</profile>
			<quota>default</quota>
			<access_management>1</access_management>
		</system>
	</users>

	<profiles>
		<default>
			<max_memory_usage>10000000000</max_memory_usage>
			<use_uncompressed_cache>0</use_uncompressed_cache>
			<load_balancing>random</load_balancing>
		</default>
	</profiles>

	<quotas>
		<default>
			<interval>
				<duration>3600</duration>
				<queries>0</queries>
				<errors>0</errors>
				<result_rows>0</result_rows>
				<read_rows>0</read_rows>
				<execution_time>0</execution_time>
			</interval>
		</default>
	</quotas>
</yandex>
`;

	const clientConfigXml = `<?xml version="1.0"?>
<config>
    <tcp_port>9301</tcp_port>
    <host>127.0.0.1</host>
</config>`;

	const initSql = `CREATE DATABASE IF NOT EXISTS otel;

CREATE TABLE IF NOT EXISTS otel.otel_logs (
	Timestamp DateTime64(9) CODEC(Delta, ZSTD(1)),
	ObservedTimestamp DateTime64(9) CODEC(Delta, ZSTD(1)),
	TraceId String CODEC(ZSTD(1)),
	SpanId String CODEC(ZSTD(1)),
	TraceFlags UInt32 CODEC(ZSTD(1)),
	SeverityText LowCardinality(String) CODEC(ZSTD(1)),
	SeverityNumber Int32 CODEC(ZSTD(1)),
	ServiceName LowCardinality(String) CODEC(ZSTD(1)),
	Body String CODEC(ZSTD(1)),
	ResourceSchemaUrl String CODEC(ZSTD(1)),
	ResourceAttributes Map(LowCardinality(String), String) CODEC(ZSTD(1)),
	ScopeSchemaUrl String CODEC(ZSTD(1)),
	ScopeName String CODEC(ZSTD(1)),
	ScopeVersion String CODEC(ZSTD(1)),
	ScopeAttributes Map(LowCardinality(String), String) CODEC(ZSTD(1)),
	LogAttributes Map(LowCardinality(String), String) CODEC(ZSTD(1)),
	INDEX idx_trace_id TraceId TYPE bloom_filter(0.001) GRANULARITY 1,
	INDEX idx_res_attr_key mapKeys(ResourceAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_res_attr_value mapValues(ResourceAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_scope_attr_key mapKeys(ScopeAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_scope_attr_value mapValues(ScopeAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_log_attr_key mapKeys(LogAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_log_attr_value mapValues(LogAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_body Body TYPE tokenbf_v1(32768, 3, 0) GRANULARITY 1
) ENGINE = MergeTree()
PARTITION BY toDate(Timestamp)
ORDER BY (ServiceName, SeverityText, toUnixTimestamp(Timestamp), TraceId)
TTL toDateTime(Timestamp) + toIntervalDay(3)
SETTINGS index_granularity = 8192, ttl_only_drop_parts = 1;

CREATE TABLE IF NOT EXISTS otel.otel_traces (
	Timestamp DateTime64(9) CODEC(Delta, ZSTD(1)),
	TraceId String CODEC(ZSTD(1)),
	SpanId String CODEC(ZSTD(1)),
	ParentSpanId String CODEC(ZSTD(1)),
	TraceState String CODEC(ZSTD(1)),
	SpanName LowCardinality(String) CODEC(ZSTD(1)),
	SpanKind LowCardinality(String) CODEC(ZSTD(1)),
	ServiceName LowCardinality(String) CODEC(ZSTD(1)),
	ResourceAttributes Map(LowCardinality(String), String) CODEC(ZSTD(1)),
	ScopeName String CODEC(ZSTD(1)),
	ScopeVersion String CODEC(ZSTD(1)),
	SpanAttributes Map(LowCardinality(String), String) CODEC(ZSTD(1)),
	Duration Int64 CODEC(ZSTD(1)),
	StatusCode LowCardinality(String) CODEC(ZSTD(1)),
	StatusMessage String CODEC(ZSTD(1)),
	Events Nested (
		Timestamp DateTime64(9),
		Name LowCardinality(String),
		Attributes Map(LowCardinality(String), String)
	) CODEC(ZSTD(1)),
	Links Nested (
		TraceId String,
		SpanId String,
		TraceState String,
		Attributes Map(LowCardinality(String), String)
	) CODEC(ZSTD(1)),
	INDEX idx_trace_id TraceId TYPE bloom_filter(0.001) GRANULARITY 1,
	INDEX idx_res_attr_key mapKeys(ResourceAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_res_attr_value mapValues(ResourceAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_span_attr_key mapKeys(SpanAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_span_attr_value mapValues(SpanAttributes) TYPE bloom_filter(0.01) GRANULARITY 1
) ENGINE = MergeTree()
PARTITION BY toDate(Timestamp)
ORDER BY (ServiceName, SpanName, toUnixTimestamp(Timestamp), TraceId)
TTL toDateTime(Timestamp) + toIntervalDay(7)
SETTINGS index_granularity = 8192, ttl_only_drop_parts = 1;

CREATE TABLE IF NOT EXISTS otel.otel_metrics (
	ResourceAttributes Map(LowCardinality(String), String) CODEC(ZSTD(1)),
	ResourceSchemaUrl String CODEC(ZSTD(1)),
	ScopeName String CODEC(ZSTD(1)),
	ScopeVersion String CODEC(ZSTD(1)),
	ScopeAttributes Map(LowCardinality(String), String) CODEC(ZSTD(1)),
	ScopeDroppedAttrCount UInt32 CODEC(ZSTD(1)),
	ScopeSchemaUrl String CODEC(ZSTD(1)),
	MetricName String CODEC(ZSTD(1)),
	MetricDescription String CODEC(ZSTD(1)),
	MetricUnit String CODEC(ZSTD(1)),
	Attributes Map(LowCardinality(String), String) CODEC(ZSTD(1)),
	StartTimeUnix DateTime64(9) CODEC(Delta, ZSTD(1)),
	TimeUnix DateTime64(9) CODEC(Delta, ZSTD(1)),
	Value Float64 CODEC(ZSTD(1)),
	Flags UInt32 CODEC(ZSTD(1)),
	Exemplars Nested (
		FilteredAttributes Map(LowCardinality(String), String),
		TimeUnix DateTime64(9),
		Value Float64,
		SpanId String,
		TraceId String
	) CODEC(ZSTD(1)),
	AggTemp Int32 CODEC(ZSTD(1)),
	IsMonotonic Bool CODEC(ZSTD(1)),
	INDEX idx_res_attr_key mapKeys(ResourceAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_res_attr_value mapValues(ResourceAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_scope_attr_key mapKeys(ScopeAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_scope_attr_value mapValues(ScopeAttributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_attr_key mapKeys(Attributes) TYPE bloom_filter(0.01) GRANULARITY 1,
	INDEX idx_attr_value mapValues(Attributes) TYPE bloom_filter(0.01) GRANULARITY 1
) ENGINE = MergeTree()
PARTITION BY toDate(TimeUnix)
ORDER BY (MetricName, Attributes, toUnixTimestamp(TimeUnix))
TTL toDateTime(TimeUnix) + toIntervalDay(30)
SETTINGS index_granularity = 8192, ttl_only_drop_parts = 1;
`;

	context.writeCoreServiceFile("clickhouse", "config.xml", configXml);
	context.writeCoreServiceFile("clickhouse", "users.xml", usersXml);
	context.writeCoreServiceFile("clickhouse", "client-config.xml", clientConfigXml);
	context.writeCoreServiceFile("clickhouse", "init/01-create-otel-table.sql", initSql);
}