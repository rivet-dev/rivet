// TODO: briefly document this file is used for consolidating all env vars that affect rivet's behavior

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

// RivetKit configuration
export const getRivetkitLogMessage = (): boolean =>
	!!getEnvUniversal("_RIVETKIT_LOG_MESSAGE");
export const getRivetkitInspectorToken = (): string | undefined =>
	getEnvUniversal("RIVETKIT_INSPECTOR_TOKEN");
export const getRivetkitInspectorDisable = (): boolean =>
	!!getEnvUniversal("RIVETKIT_INSPECTOR_DISABLE");
export const getRivetkitErrorStack = (): boolean =>
	getEnvUniversal("_RIVETKIT_ERROR_STACK") === "1";

// Logging configuration
export const getLogLevel = (): string | undefined =>
	getEnvUniversal("LOG_LEVEL");
export const getLogTarget = (): boolean =>
	getEnvUniversal("LOG_TARGET") === "1";
export const getLogTimestamp = (): boolean =>
	getEnvUniversal("LOG_TIMESTAMP") === "1";
export const getRivetLogHeaders = (): boolean =>
	!!getEnvUniversal("_RIVET_LOG_HEADERS");

// Environment configuration
export const getNodeEnv = (): string | undefined => getEnvUniversal("NODE_ENV");
export const getNextPhase = (): string | undefined =>
	getEnvUniversal("NEXT_PHASE");
export const isDev = (): boolean => getNodeEnv() !== "production";
