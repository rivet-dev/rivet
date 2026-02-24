// This file consolidates all environment variables that affect RivetKit's behavior.
//
// IMPORTANT: When adding or modifying environment variables here, also update the
// documentation at: website/src/content/docs/general/environment-variables.mdx

import { getEnvUniversal } from "@/utils";

// Rivet configuration
export const getRivetEngine = (): string | undefined =>
	getEnvUniversal("RIVET_ENGINE");
export const getRivetEndpoint = (): string | undefined =>
	getEnvUniversal("RIVET_ENDPOINT");
export const getRivetToken = (): string | undefined =>
	getEnvUniversal("RIVET_TOKEN");
export const getRivetNamespace = (): string | undefined =>
	getEnvUniversal("RIVET_NAMESPACE");
export const getRivetRunner = (): string | undefined =>
	getEnvUniversal("RIVET_RUNNER");
export const getRivetTotalSlots = (): number | undefined => {
	const value = getEnvUniversal("RIVET_TOTAL_SLOTS");
	return value !== undefined ? parseInt(value, 10) : undefined;
};
export const getRivetRunnerKey = (): string | undefined =>
	getEnvUniversal("RIVET_RUNNER_KEY");
export const getRivetRunEngine = (): boolean =>
	getEnvUniversal("RIVET_RUN_ENGINE") === "1";
export const getRivetRunEngineVersion = (): string | undefined =>
	getEnvUniversal("RIVET_RUN_ENGINE_VERSION");
export const getRivetRunnerKind = (): string | undefined =>
	getEnvUniversal("RIVET_RUNNER_KIND");
export const getRivetRunnerVersion = (): number | undefined => {
	const value = getEnvUniversal("RIVET_RUNNER_VERSION");
	return value !== undefined ? parseInt(value, 10) : undefined;
};
export const getRivetPublicEndpoint = (): string | undefined =>
	getEnvUniversal("RIVET_PUBLIC_ENDPOINT");
export const getRivetPublicToken = (): string | undefined =>
	getEnvUniversal("RIVET_PUBLIC_TOKEN");
// There is no RIVET_PUBLIC_NAMESPACE because the frontend and backend cannot
// use different namespaces

// RivetKit configuration
export const getRivetkitInspectorToken = (): string | undefined =>
	getEnvUniversal("RIVET_INSPECTOR_TOKEN");
export const getRivetkitInspectorDisable = (): boolean =>
	getEnvUniversal("RIVET_INSPECTOR_DISABLE") === "1";
export const getRivetkitStoragePath = (): string | undefined =>
	getEnvUniversal("RIVETKIT_STORAGE_PATH");

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
export const isDev = (): boolean => getNodeEnv() !== "production";

// Experimental
/**
 * Enables experimental OTel tracing for Rivet Actors.
 *
 * When disabled, actors use an in-memory no-op traces implementation.
 */
export const getRivetExperimentalOtel = (): boolean =>
	getEnvUniversal("RIVET_EXPERIMENTAL_OTEL") === "1";
