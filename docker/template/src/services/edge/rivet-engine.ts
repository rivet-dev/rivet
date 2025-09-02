import { TemplateContext } from "../../context";
import { Datacenter } from "../../config";

const API_PEER_PORT = 6422;
const GUARD_PORT = 6420;

export function generateDatacenterRivetEngine(
	context: TemplateContext,
	datacenter: Datacenter,
) {
	const clickhouseHost = context.config.networkMode === "host" ? "127.0.0.1" : "clickhouse";
	const datacenters = [];

	for (let dc of context.config.datacenters) {
		let serviceHost = context.getServiceHost("rivet-engine", dc.name, 0);
		datacenters.push({
			name: dc.name,
			datacenter_label: dc.id,
			is_leader: dc.id == 1,
			api_peer_url: `http://${serviceHost}:${API_PEER_PORT}`,
			guard_url: `http://${serviceHost}:${GUARD_PORT}`,
		});
	}

	// Generate a separate config file for each engine node
	for (let i = 0; i < datacenter.engines; i++) {
		let serviceHost = context.getServiceHost("rivet-engine", datacenter.name, 0);
		const topology = {
			datacenter_label: datacenter.id,
			datacenters,
		};

		// Config structure matching Rust schema in packages/common/config/src/config/mod.rs
		const config = {
			// guard config
			guard: {
				port: GUARD_PORT,
				// https is optional and not configured for local development
			},
			// api_public config
			api_public: {
				lan_host: serviceHost,
				host: "0.0.0.0",
				port: 6421,
			},
			// api_peer config
			api_peer: {
				host: "0.0.0.0",
				port: API_PEER_PORT,
			},
			// pegboard config
			pegboard: {
				lan_host: serviceHost,
				host: "0.0.0.0",
				port: 6423,
			},
			// logs config
			logs: {
				// redirect_logs_dir is optional
			},
			// topology config
			topology,
			// database & pubsub config
			postgres: {
				url: `postgresql://postgres:postgres@${context.getServiceHost("postgres", datacenter.name)}:5432/rivet_engine`,
			},
			// cache config
			cache: {
				driver: "in_memory",
			},
			// clickhouse config (optional)
			clickhouse: {
				http_url: `http://${clickhouseHost}:9300`, // TODO:
				native_url: `http://${clickhouseHost}:9301`, // TODO:
				username: "system",
				password: "default",
				// TODO: Move this to init migrations
				provision_users: {
					vector: {
						username: "vector",
						password: "vector",
						role: "write",
					},
				},
				secure: false,
			},
			// vector_http config (optional)
			vector_http: {
				host: context.getServiceHost("vector-client", datacenter.name),
				port: 5022,
			},
		};

		context.writeDatacenterServiceFile(
			"rivet-engine",
			datacenter.name,
			"config.jsonc",
			JSON.stringify(config, null, "\t"),
			i,
		);
	}
}
