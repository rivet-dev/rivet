import { TemplateContext } from "../../context";
import * as yaml from "js-yaml";

export function generateDatacenterVectorServer(context: TemplateContext, dcId: string) {
	const clickhouseHost = context.config.networkMode === "host" ? "127.0.0.1" : "clickhouse";
	const vectorConfig = {
		api: {
			enabled: true,
			address: "0.0.0.0:8686"
		},
		sources: {
			vector: {
				type: "vector",
				address: "0.0.0.0:6000",
				version: "2"
			},
			tcp_json: {
				type: "socket",
				mode: "tcp",
				address: "0.0.0.0:6100",
				decoding: {
					codec: "json"
				}
			},
			http_json: {
				type: "http_server",
				address: "0.0.0.0:6200",
				encoding: "json"
			},
			vector_metrics: {
				type: "internal_metrics"
			},
			vector_logs: {
				type: "internal_logs"
			}
		},
		transforms: {
			clickhouse_dynamic_events_filter: {
				type: "filter",
				inputs: [
					"vector"
				],
				condition: {
					type: "vrl",
					source: `.source == "clickhouse"`
				}
			},
			clickhouse_dynamic_events_transform: {
				type: "remap",
				inputs: [
					"clickhouse_dynamic_events_filter"
				],
				source: `# Extract and store metadata
__database = .database
__table = .table
__columns = .columns

# Create a new object with just the columns data
. = {
	"__database": __database,
	"__table": __table,
	# By default insert namespace column since most tables include this
	"namespace": "rivet"
}
	
# Merge in the column data that should be inserted
. = merge!(., __columns)
`
			}
		},
		sinks: {
			clickhouse_dynamic_events: {
				type: "clickhouse",
				inputs: ["clickhouse_dynamic_events_transform"],
				endpoint: `http://${clickhouseHost}:9300`,
				database: "{{ __database }}",
				table: "{{ __table }}",
				compression: "gzip",
				auth: {
					strategy: "basic",
					user: "system",
					password: "default"
				},
				batch: {
					max_events: 1000,
					timeout_secs: 10
				}
			},
			console_vector_logs: {
				type: "console",
				inputs: ["vector_logs"],
				encoding: {
					codec: "json"
				}
			}
		}
	};

	const yamlContent = `# Vector Server (Aggregator) Configuration
# This Vector instance acts as an aggregator that collects logs and metrics from all Vector clients
# across different datacenters and forwards them to ClickHouse and other sinks.

${yaml.dump(vectorConfig)}`;

	context.writeDatacenterServiceFile("vector-server", dcId, "vector.yaml", yamlContent);
}
