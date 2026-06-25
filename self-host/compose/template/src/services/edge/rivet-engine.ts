import type { Datacenter } from "../../config";
import type { TemplateContext } from "../../context";

const API_PEER_PORT = 6421;
const GUARD_PORT = 6420;

export function generateDatacenterRivetEngine(
	context: TemplateContext,
	datacenter: Datacenter,
) {
	const clickhouseHost =
		context.config.networkMode === "host" ? "127.0.0.1" : "clickhouse";
	const datacenters: Record<string, any> = {};

	for (const dc of context.config.datacenters) {
		const serviceHost = context.getServiceHost("rivet-engine", dc.name, 0);
		datacenters[dc.name] = {
			datacenter_label: dc.id,
			is_leader: dc.id === 1,
			peer_url: `http://${serviceHost}:${API_PEER_PORT}`,
			public_url: `http://${serviceHost}:${GUARD_PORT}`,
			valid_hosts: [`${serviceHost}`, `127.0.0.1`, `localhost`],
		};
	}

	// Generate a separate config file for each engine node
	for (let i = 0; i < datacenter.engines; i++) {
		const serviceHost = context.getServiceHost(
			"rivet-engine",
			datacenter.name,
			0,
		);
		const topology = {
			datacenter_label: datacenter.id,
			datacenters,
		};

		// Config structure matching Rust schema in engine/packages/config/src/config/mod.rs.
		// Values that match the engine's defaults are omitted.
		const config = {
			auth: {
				admin_token: "dev",
			},
			api_peer: {
				host: "0.0.0.0",
			},
			topology,
			postgres: {
				url: `postgresql://postgres:postgres@${context.getServiceHost("postgres", datacenter.name)}:5432/rivet_engine`,
			},
			clickhouse: {
				http_url: `http://${clickhouseHost}:9300`,
				native_url: `http://${clickhouseHost}:9301`,
				username: "system",
				password: "default",
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
