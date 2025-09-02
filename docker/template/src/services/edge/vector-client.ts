import { TemplateContext } from "../../context";
import * as yaml from "js-yaml";

export function generateDatacenterVectorClient(context: TemplateContext, dcId: string) {
	const vectorConfig = {
		api: {
			enabled: true,
			address: "0.0.0.0:8686"
		},
		sources: {
			http_events: {
				type: "http_server",
				address: "0.0.0.0:5022",
				encoding: "ndjson"
			},
		},
		transforms: {},
			sinks: {
				vector_server: {
					type: "vector",
					inputs: [
						"http_events"
					],
					address: `${context.getServiceHost("vector-server", dcId)}:6000`,
					version: "2",
				buffer: {
					type: "disk",
					max_size: 2147483648,
					when_full: "block"
				}
			},
			console_debug: {
				type: "console",
				inputs: ["http_events"],
				encoding: {
					codec: "json"
				},
			}
		}
	};

	const yamlContent = `# Vector Client Configuration for Datacenter: ${dcId}
# This Vector instance runs on each server to collect local logs and forward them
# to the central Vector server aggregator.

${yaml.dump(vectorConfig)} `;

	context.writeDatacenterServiceFile("vector-client", dcId, "vector.yaml", yamlContent);
}
