import { TemplateContext } from "../../context";
import * as yaml from "js-yaml";

export function generateDatacenterOtelCollectorClient(
	context: TemplateContext,
	dcId: string,
) {
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
		},
		processors: {
			batch: {
				timeout: "5s",
				send_batch_size: 10000,
			},
		},
		exporters: {
			"otlp/server": {
				endpoint: `${context.getServiceHost("otel-collector-server", dcId)}:4317`,
				tls: {
					insecure: true,
				},
			},
		},
		service: {
			pipelines: {
				logs: {
					receivers: ["otlp"],
					processors: ["batch"],
					exporters: ["otlp/server"],
				},
				traces: {
					receivers: ["otlp"],
					processors: ["batch"],
					exporters: ["otlp/server"],
				},
				metrics: {
					receivers: ["otlp"],
					processors: ["batch"],
					exporters: ["otlp/server"],
				},
			},
		},
	};

	const yamlContent = yaml.dump(otelConfig);

	context.writeDatacenterServiceFile(
		"otel-collector-client",
		dcId,
		"config.yaml",
		yamlContent,
	);
}
