import {
	type DestinationStream,
	type LevelWithSilent,
	type Logger,
	pino,
	stdTimeFunctions,
} from "pino";
import { z } from "zod/v4";
import { getLogLevel, getLogTarget, getLogTimestamp } from "@/utils/env-vars";

export type { Logger } from "pino";

let baseLogger: Logger | undefined;
let configuredLogLevel: LogLevel | undefined;

/** Cache of child loggers by logger name. */
const loggerCache = new Map<string, Logger>();

export const LogLevelSchema = z.enum([
	"trace",
	"debug",
	"info",
	"warn",
	"error",
	"fatal",
	"silent",
]);

export type LogLevel = z.infer<typeof LogLevelSchema>;

export function getPinoLevel(logLevel?: LogLevel): LevelWithSilent {
	// Priority: provided > configured > env > default
	if (logLevel) {
		return logLevel;
	}

	if (configuredLogLevel) {
		return configuredLogLevel;
	}

	const raw = (getLogLevel() || "warn").toString().toLowerCase();

	const parsed = LogLevelSchema.safeParse(raw);
	if (parsed.success) {
		return parsed.data;
	}

	// Default to info if invalid
	return "info";
}

export function getIncludeTarget(): boolean {
	return getLogTarget();
}

/**
 * Configure a custom base logger.
 */
export function configureBaseLogger(logger: Logger): void {
	baseLogger = logger;
	loggerCache.clear();
}

/**
 * Configure the default logger with optional log level.
 */
export function configureDefaultLogger(logLevel?: LogLevel) {
	// Store the configured log level
	if (logLevel) {
		configuredLogLevel = logLevel;
	}

	baseLogger = pino(
		{
			level: getPinoLevel(logLevel),
			messageKey: "msg",
			// Do not include pid/hostname in output
			base: {},
			// Keep the numeric level so the logfmt sink can match Pino's levels.
			formatters: {
				level(_label: string, number: number) {
					return { level: number };
				},
			},
			timestamp: getLogTimestamp() ? stdTimeFunctions.epochTime : false,
		},
		createLogfmtDestination(),
	);

	loggerCache.clear();
}

/**
 * Get or initialize the base logger.
 */
export function getBaseLogger(): Logger {
	if (!baseLogger) {
		configureDefaultLogger();
	}
	return baseLogger!;
}

/**
 * Returns a child logger with `target` bound for the given name.
 */
export function getLogger(name = "default"): Logger {
	// Check cache first
	const cached = loggerCache.get(name);
	if (cached) {
		return cached;
	}

	// Create
	const base = getBaseLogger();

	// Add target to log if enabled
	const child = getIncludeTarget() ? base.child({ target: name }) : base;

	// Cache the logger
	loggerCache.set(name, child);

	return child;
}

const PINO_LEVEL_LABELS: Record<number, string> = {
	10: "trace",
	20: "debug",
	30: "info",
	40: "warn",
	50: "error",
	60: "fatal",
};

function createLogfmtDestination(): DestinationStream {
	return {
		write(msg: string): void {
			const line = formatLogfmtLine(msg);
			if (typeof process !== "undefined" && process.stdout?.write) {
				process.stdout.write(`${line}\n`);
			} else {
				console.log(line);
			}
		},
	};
}

function formatLogfmtLine(raw: string): string {
	let data: Record<string, unknown>;
	try {
		data = JSON.parse(raw);
	} catch {
		return raw.trimEnd();
	}

	const parts: string[] = [];
	appendLogfmtEntry(parts, "level", formatPinoLevel(data.level));

	if (data.time !== undefined) {
		appendLogfmtEntry(parts, "ts", data.time);
	}

	for (const [key, value] of Object.entries(data)) {
		if (key === "level" || key === "time") {
			continue;
		}
		appendLogfmtEntry(parts, key, value);
	}

	return parts.join(" ");
}

function formatPinoLevel(level: unknown): string {
	if (typeof level === "number") {
		return PINO_LEVEL_LABELS[level] ?? level.toString();
	}

	if (typeof level === "string") {
		return level.toLowerCase();
	}

	return "info";
}

function appendLogfmtEntry(parts: string[], key: string, value: unknown): void {
	const safeKey = key.replace(/[\s="]/g, "");
	if (safeKey.length === 0) {
		return;
	}

	parts.push(`${safeKey}=${formatLogfmtValue(value)}`);
}

function formatLogfmtValue(value: unknown): string {
	if (typeof value === "number" || typeof value === "boolean") {
		return String(value);
	}

	if (value === null || value === undefined) {
		return "null";
	}

	if (typeof value === "string") {
		return quoteLogfmtString(value);
	}

	return quoteLogfmtString(JSON.stringify(value));
}

function quoteLogfmtString(value: string): string {
	if (!/[\s="]/.test(value)) {
		return value;
	}

	return `"${value.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n")}"`;
}
