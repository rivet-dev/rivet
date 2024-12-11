#!/usr/bin/env -S deno run -A

import { resolve } from "@std/path";
import { stringify } from "@std/yaml";

const modifyHeaderYaml = `# DO NOT MODIFY
#
# Generated with scripts/docker/generate_configs.ts

`;

async function generateRivetGuard() {
	const outputDevFull = resolve(
		import.meta.dirname!,
		"../../docker/dev-full/rivet-guard/traefik.yaml",
	);
	const outputMonolith = resolve(
		import.meta.dirname!,
		"../../docker/monolith/rivet-guard/traefik.yaml",
	);

	// Base config
	const config = {
		entryPoints: {
			traefik: {
				address: ":9980",
			},
			"lb-7080": {
				address: ":7080",
			},
			"lb-7443": {
				address: ":7443",
			},
		} as Record<string, unknown>,
		api: {
			insecure: true,
		},
		log: {
			level: "INFO",
		},
		accessLog: {},
		providers: {
			http: {
				endpoint:
					"http://rivet-server:8081/traefik-provider/config/game-guard?datacenter=f288913c-735d-4188-bf9b-2fcf6eac7b9c",
				pollInterval: "0.5s",
			},
		},
	};

	// Generate TCP & UDP ports
	for (let p = 7500; p < 7600; p++) {
		config.entryPoints[`lb-${p}-tcp`] = {
			address: `:${p}/tcp`,
			transport: {
				respondingTimeouts: {
					readTimeout: "12h",
					writeTimeout: "12h",
					idleTimeout: "30s",
				},
			},
		};
		config.entryPoints[`lb-${p}-udp`] = {
			address: `:${p}/udp`,
			udp: {
				timeout: "15s",
			},
		};
	}

	await Deno.writeTextFile(outputDevFull, modifyHeaderYaml + stringify(config));
	await Deno.writeTextFile(outputMonolith, modifyHeaderYaml + stringify(config));
	console.log(`Config written to ${outputDevFull}`);
}

async function main() {
	await generateRivetGuard();
}

main();
