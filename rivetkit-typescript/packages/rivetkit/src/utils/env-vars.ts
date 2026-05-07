// This file consolidates all environment variables that affect RivetKit's behavior.
//
// IMPORTANT: When adding or modifying environment variables here, also update the
// documentation at: website/src/content/docs/general/environment-variables.mdx

import { getEnvUniversal } from "@/utils";

function parseNumberEnv(name: string): number | undefined {
	const value = getEnvUniversal(name);
	if (value === undefined || value === "") return undefined;
	return Number(value);
}

function collectEnvKeys(): string[] {
	if (typeof Deno !== "undefined") {
		try {
			return Object.keys(Deno.env.toObject());
		} catch {
			return [];
		}
	}

	if (typeof process !== "undefined") {
		return Object.keys(process.env);
	}

	return [];
}

// Rivet configuration
export const getRivetEngine = (): string | undefined =>
	getEnvUniversal("RIVET_ENGINE");
export const getRivetEndpoint = (): string | undefined =>
	getEnvUniversal("RIVET_ENDPOINT");
export const getRivetToken = (): string | undefined =>
	getEnvUniversal("RIVET_TOKEN");
export const getRivetNamespace = (): string | undefined =>
	getEnvUniversal("RIVET_NAMESPACE");
export const getRivetPool = (): string | undefined =>
	getEnvUniversal("RIVET_POOL");
export const getRivetVersion = (): number | undefined =>
	parseNumberEnv("RIVET_VERSION");
export const getRivetPublicEndpoint = (): string | undefined =>
	getEnvUniversal("RIVET_PUBLIC_ENDPOINT");
export const getRivetPublicToken = (): string | undefined =>
	getEnvUniversal("RIVET_PUBLIC_TOKEN");
// There is no RIVET_PUBLIC_NAMESPACE because the frontend and backend cannot
// use different namespaces

// RivetKit configuration
export const getRivetkitInspectorDisable = (): boolean =>
	getEnvUniversal("RIVET_INSPECTOR_DISABLE") === "1";
export const getRivetkitStoragePath = (): string | undefined =>
	getEnvUniversal("RIVETKIT_STORAGE_PATH");
export const getRivetkitRuntime = (): string | undefined =>
	getEnvUniversal("RIVETKIT_RUNTIME");

// Logging configuration
// DEPRECATED: LOG_LEVEL will be removed in a future version
export const getLogLevel = (): string | undefined =>
	getEnvUniversal("RIVET_LOG_LEVEL") ?? getEnvUniversal("LOG_LEVEL");
export const getLogTarget = (): boolean =>
	getEnvUniversal("RIVET_LOG_TARGET") === "1";
export const getLogTimestamp = (): boolean =>
	getEnvUniversal("RIVET_LOG_TIMESTAMP") === "1";
export const getLogMessage = (): boolean =>
	getEnvUniversal("RIVET_LOG_MESSAGE") === "1";
export const getLogErrorStack = (): boolean =>
	getEnvUniversal("RIVET_LOG_ERROR_STACK") === "1";
export const getLogHeaders = (): boolean =>
	getEnvUniversal("RIVET_LOG_HEADERS") === "1";

// Environment configuration
export const getNodeEnv = (): string | undefined => getEnvUniversal("NODE_ENV");
export const getNextPhase = (): string | undefined =>
	getEnvUniversal("NEXT_PHASE");
export const isDev = (): boolean => getNodeEnv() === "development";

export function invalidRivetEnvironmentVariables(): Array<{
	name: string;
	message: string;
}> {
	const invalid = new Map<string, string>();

	for (const name of collectEnvKeys()) {
		if (name === "RIVET_RUNNER" || name.startsWith("RIVET_RUNNER_")) {
			invalid.set(
				name,
				`${name} has been removed. Use registry.start() or registry.fetchHandler() with RIVET_POOL and RIVET_VERSION instead.`,
			);
		}
	}

	const renamed: Record<string, string> = {
		RIVET_ENVOY_VERSION: "RIVET_VERSION",
		RIVET_POOL_NAME: "RIVET_POOL",
	};
	for (const [oldName, newName] of Object.entries(renamed)) {
		if (getEnvUniversal(oldName) !== undefined) {
			invalid.set(
				oldName,
				`${oldName} has been removed. Use ${newName} instead.`,
			);
		}
	}

	if (getEnvUniversal("RIVET_MODE") !== undefined) {
		invalid.set(
			"RIVET_MODE",
			"RIVET_MODE has been removed. Runtime mode is selected by registry.start() or registry.fetchHandler().",
		);
	}
	if (getEnvUniversal("RIVET_ENVOY_KIND") !== undefined) {
		invalid.set(
			"RIVET_ENVOY_KIND",
			"RIVET_ENVOY_KIND has been removed. Runtime mode is selected by registry.start() or registry.fetchHandler().",
		);
	}
	if (getEnvUniversal("RIVET_ENVOY_KEY") !== undefined) {
		invalid.set(
			"RIVET_ENVOY_KEY",
			"RIVET_ENVOY_KEY has been removed. Envoy keys are managed by RivetKit.",
		);
	}

	for (const name of [
		"RIVET_ENGINE",
		"RIVET_RUN_ENGINE",
		"RIVET_RUN_ENGINE_VERSION",
		"RIVET_TOTAL_SLOTS",
	]) {
		if (getEnvUniversal(name) !== undefined) {
			invalid.set(name, `${name} has been removed from RivetKit setup.`);
		}
	}

	return Array.from(invalid, ([name, message]) => ({ name, message }));
}

// Experimental
/**
 * Enables experimental OTel tracing for Rivet Actors.
 *
 * When disabled, actors use an in-memory no-op traces implementation.
 */
export const getRivetExperimentalOtel = (): boolean =>
	getEnvUniversal("RIVET_EXPERIMENTAL_OTEL") === "1";
