import type { LoggerOptions } from "pino";

// Lightweight version of @google-cloud/pino-logging-gcp-config.
// This only maps pino levels to GCP severity for compute environments.

const PINO_TO_GCP_SEVERITY: Record<string, string> = {
	trace: "DEBUG",
	debug: "DEBUG",
	info: "INFO",
	warn: "WARNING",
	error: "ERROR",
	fatal: "CRITICAL",
};

export function pinoLevelToGcpSeverity(
	pinoSeverityLabel: string,
	pinoSeverityLevel: number,
): Record<string, unknown> {
	const severity =
		PINO_TO_GCP_SEVERITY[pinoSeverityLabel] ?? "INFO";
	return { severity, level: pinoSeverityLevel };
}

export function createGcpLoggingPinoConfig(
	pinoLoggerOptionsMixin?: LoggerOptions,
): LoggerOptions {
	const formattersMixin = pinoLoggerOptionsMixin?.formatters;
	return {
		...pinoLoggerOptionsMixin,
		formatters: {
			...formattersMixin,
			level: pinoLevelToGcpSeverity,
		},
	};
}
