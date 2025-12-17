import * as yaml from "js-yaml";
import type { TemplateContext } from "../../context";

export function generateCorePrometheus(context: TemplateContext) {
	// Prometheus configuration with remote write enabled
	// Metrics are scraped by OTEL collector and sent via remote write
	const prometheusConfig = {
		global: {
			scrape_interval: "15s",
			evaluation_interval: "15s",
		},
		scrape_configs: [],
	};

	context.writeCoreServiceFile(
		"prometheus",
		"prometheus.yml",
		yaml.dump(prometheusConfig),
	);
}
