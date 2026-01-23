import type { Registry } from "rivetkit";
import { logger } from "./log.ts";

export interface ConvexHandlerOptions {
	/**
	 * Base path for the Rivet API.
	 * @default "/"
	 */
	basePath?: string;
}

export interface SerializedRequest {
	method: string;
	url: string;
	headers: Record<string, string>;
	body?: string;
}

export interface SerializedResponse {
	status: number;
	statusText: string;
	headers: Record<string, string>;
	body: string;
}

// Runner version set to seconds since epoch when the module loads in development mode.
//
// This creates a version number that increments each time the code is updated
// and the module reloads, allowing the engine to detect code changes via the
// /metadata endpoint and hot-reload all actors by draining older runners.
//
// We use seconds (not milliseconds) because the runner version is a u32 on the engine side.
const DEV_RUNNER_VERSION = Math.floor(Date.now() / 1000);

function isDev(): boolean {
	return process.env.NODE_ENV !== "production";
}

export function configureRunnerVersion(registry: Registry<any>) {
	if (isDev()) {
		logger().debug({ msg: "dev mode detected, setting runner version for hot-reload", version: DEV_RUNNER_VERSION });
		registry.config.runner = {
			...registry.config.runner,
			version: DEV_RUNNER_VERSION,
		};
	}
}
