/**
 * Entry point: loads the Rivet registry (embedded manager locally, serverless on Render with `RIVET_ENDPOINT`),
 * then starts the production HTTP server when in Rivet Cloud mode.
 *
 * Set `RIVET_PUBLIC_ENDPOINT` for the Vite client bundle — see `vite.config.ts` and README.
 */
import path from "node:path";
import { port, publicStaticDir, useRivetCloud } from "./config/env";
import { PROJECT_ROOT } from "./config/paths";
import { serviceName } from "./config/service-name";
import { startProductionServer } from "./http/server";
import { registry } from "./rivet/registry";

if (useRivetCloud) {
	const publicDir = path.resolve(PROJECT_ROOT, publicStaticDir);
	startProductionServer({ registry, port, publicDir });
} else {
	console.log(
		`${serviceName()} — Rivet manager + static on http://0.0.0.0:${port}`,
	);
}
