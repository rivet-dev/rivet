import { getEnvUniversal } from "@/utils";

/**
 * Logging configuration functions.
 *
 * These functions provide access to logging-related environment variables.
 */

/**
 * Get log level (e.g., "debug", "info", "warn", "error").
 * Defaults to "warn" if not set.
 */
export function getLogLevelEnv(): string | undefined {
	return getEnvUniversal("LOG_LEVEL");
}

/**
 * Check if log target should be included in log output.
 * Returns true when LOG_TARGET=1.
 */
export function isLogTargetEnabled(): boolean {
	return getEnvUniversal("LOG_TARGET") === "1";
}

/**
 * Check if timestamps should be included in log output.
 * Returns true when LOG_TIMESTAMP=1.
 */
export function isLogTimestampEnabled(): boolean {
	return getEnvUniversal("LOG_TIMESTAMP") === "1";
}

/**
 * Check if detailed message logging is enabled for debugging.
 * Returns true when LOG_MESSAGE=1.
 */
export function isLogMessageEnabled(): boolean {
	return getEnvUniversal("LOG_MESSAGE") === "1";
}

/**
 * Check if stack traces should be included in error stringification.
 * Returns true when LOG_ERROR_STACK=1.
 */
export function isErrorStackEnabled(): boolean {
	return getEnvUniversal("LOG_ERROR_STACK") === "1";
}

/**
 * Check if HTTP headers should be logged.
 * Returns true when LOG_HEADERS=1.
 */
export function isLogHeadersEnabled(): boolean {
	return getEnvUniversal("LOG_HEADERS") === "1";
}
