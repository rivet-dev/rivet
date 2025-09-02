import type { Config } from "./types.ts";

export const CONFIG: Config = {
	rivetEndpoint: __ENV.RIVET_ENDPOINT || "http://localhost:6420",
	rivetNamespace: __ENV.RIVET_NAMESPACE || "default",

	// k6 specific settings
	vus: Number(__ENV.VUS) || 10,
	duration: __ENV.DURATION || "30s",
	rampUpDuration: __ENV.RAMP_UP_DURATION || "10s",
	// Test behavior flags
	disableHealthcheck: __ENV.DISABLE_HEALTHCHECK === "1",
	disableSleep: __ENV.DISABLE_SLEEP === "1",
};
