import * as yaml from "js-yaml";
import type { TemplateContext } from "../../context";
import type { DatacenterConfig } from "../../config";

export function generateDatacenterOtelCollector(
	context: TemplateContext,
	datacenter: DatacenterConfig,
) {
	const clickhouseHost =
		context.config.networkMode === "host" ? "127.0.0.1" : "clickhouse";
	const prometheusHost =
		context.config.networkMode === "host" ? "127.0.0.1" : "prometheus";

	// Build scrape configs for all engines in this datacenter
	const scrapeConfigs: any[] = [];
	for (let i = 0; i < datacenter.engines; i++) {
		const engineHost = context.getServiceHost(
			"rivet-engine",
			datacenter.name,
			i,
		);
		scrapeConfigs.push({
			job_name: `rivet-engine-${datacenter.name}-${i}`,
			scrape_interval: "15s",
			static_configs: [
				{
					targets:
						context.config.networkMode === "host"
							? ["host.docker.internal:6430"]
							: [`${engineHost}:6430`],
				},
			],
		});
	}

	const otelConfig = {
		receivers: {
			otlp: {
				protocols: {
					grpc: {
						endpoint: "0.0.0.0:4317",
					},
					http: {
						endpoint: "0.0.0.0:4318",
					},
				},
			},
			prometheus: {
				config: {
					scrape_configs: scrapeConfigs,
				},
			},
		},
		processors: {
			resource: {
				attributes: [
					{
						key: "rivet.project",
						value: "dev",
						action: "upsert",
					},
					{
						key: "rivet.datacenter",
						value: datacenter.name,
						action: "upsert",
					},
				],
			},
			batch: {
				timeout: "5s",
				send_batch_size: 10000,
			},
		},
		exporters: {
			clickhouse: {
				endpoint: `http://${clickhouseHost}:9300`,
				database: "otel",
				username: "default",
				password: "${env:CLICKHOUSE_PASSWORD}",
				async_insert: true,
				ttl: "72h",
				compress: "lz4",
				create_schema: true,
				logs_table_name: "otel_logs",
				traces_table_name: "otel_traces",
				timeout: "5s",
				retry_on_failure: {
					enabled: true,
					initial_interval: "5s",
					max_interval: "30s",
					max_elapsed_time: "300s",
				},
			},
			prometheusremotewrite: {
				endpoint: `http://${prometheusHost}:9090/api/v1/write`,
				tls: {
					insecure: true,
				},
				resource_to_telemetry_conversion: {
					enabled: true,
				},
			},
		},
		service: {
			// telemetry: {
			// 	logs: {
			// 		level: "debug",
			// 	},
			// },
			pipelines: {
				logs: {
					receivers: ["otlp"],
					processors: ["resource", "batch"],
					exporters: ["clickhouse"],
				},
				traces: {
					receivers: ["otlp"],
					processors: ["resource", "batch"],
					exporters: ["clickhouse"],
				},
				metrics: {
					receivers: ["prometheus"],
					processors: ["resource", "batch"],
					exporters: ["prometheusremotewrite"],
				},
			},
		},
	};

	const yamlContent = yaml.dump(otelConfig);

	context.writeDatacenterServiceFile(
		"otel-collector",
		datacenter.name,
		"config.yaml",
		yamlContent,
	);
}
