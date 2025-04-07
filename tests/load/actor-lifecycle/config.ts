import type { Config } from "./types.ts";

export const CONFIG: Config = {
	rivetEndpoint: __ENV.RIVET_ENDPOINT || "http://localhost:8080",
	rivetServiceToken: __ENV.RIVET_SERVICE_TOKEN || undefined,
	rivetProject: __ENV.RIVET_PROJECT || "default",
	rivetEnvironment: __ENV.RIVET_ENVIRONMENT || "default",
	buildName: __ENV.BUILD || "ws-isolate",
	region: __ENV.REGION || undefined,
	// k6 specific settings
	vus: Number(__ENV.VUS) || 10,
	duration: __ENV.DURATION || "30s",
	rampUpDuration: __ENV.RAMP_UP_DURATION || "10s",
	// Test behavior flags
	disableHealthcheck: __ENV.DISABLE_HEALTHCHECK === "1",
	disableWebsocket: __ENV.DISABLE_WEBSOCKET === "1",
	disableSleep: __ENV.DISABLE_SLEEP === "1",
};
