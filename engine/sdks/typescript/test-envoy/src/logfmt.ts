import { inspect } from "node:util";

type LogLevel = "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR" | "FATAL";

interface LoggerConfig {
	enableInspect: boolean;
}

export const LOGGER_CONFIG: LoggerConfig = {
	enableInspect: true,
};

const LOG_LEVEL_COLORS: Record<LogLevel, string> = {
	FATAL: "\x1b[31m",
	ERROR: "\x1b[31m",
	WARN: "\x1b[33m",
	INFO: "\x1b[32m",
	DEBUG: "\x1b[36m",
	TRACE: "\x1b[36m",
};

const RESET_COLOR = "\x1b[0m";

export function stringify(data: Record<string, unknown>): string {
	let line = "";
	const entries = Object.entries(data);

	for (let i = 0; i < entries.length; i++) {
		const [key, valueRaw] = entries[i];

		let isNull = false;
		let valueString: string;
		if (valueRaw == null) {
			isNull = true;
			valueString = "";
		} else {
			valueString = String(valueRaw);
		}

		// Clip value unless specifically the error message
		if (valueString.length > 512 && key !== "msg" && key !== "error") {
			valueString = `${valueString.slice(0, 512)}...`;
		}

		const needsQuoting =
			valueString.indexOf(" ") > -1 || valueString.indexOf("=") > -1;
		const needsEscaping =
			valueString.indexOf('"') > -1 || valueString.indexOf("\\") > -1;

		valueString = valueString.replace(/\n/g, "\\n");
		if (needsEscaping) valueString = valueString.replace(/["\\]/g, "\\$&");
		if (needsQuoting || needsEscaping) valueString = `"${valueString}"`;
		if (valueString === "" && !isNull) valueString = '""';

		// Special message colors
		let color = "\x1b[2m";
		if (key === "level") {
			const levelColor = LOG_LEVEL_COLORS[valueString as LogLevel];
			if (levelColor) {
				color = levelColor;
			}
		} else if (key === "msg") {
			color = "\x1b[32m";
		}

		line += `\x1b[0m\x1b[1m${key}\x1b[0m\x1b[2m=\x1b[0m${color}${valueString}${RESET_COLOR}`;

		if (i !== entries.length - 1) {
			line += " ";
		}
	}

	return line;
}

export function formatTimestamp(date: Date): string {
	const year = date.getUTCFullYear();
	const month = String(date.getUTCMonth() + 1).padStart(2, "0");
	const day = String(date.getUTCDate()).padStart(2, "0");
	const hours = String(date.getUTCHours()).padStart(2, "0");
	const minutes = String(date.getUTCMinutes()).padStart(2, "0");
	const seconds = String(date.getUTCSeconds()).padStart(2, "0");
	const milliseconds = String(date.getUTCMilliseconds()).padStart(3, "0");

	return `${year}-${month}-${day}T${hours}:${minutes}:${seconds}.${milliseconds}Z`;
}

export function castToLogValue(v: unknown): unknown {
	if (
		typeof v === "string" ||
		typeof v === "number" ||
		typeof v === "bigint" ||
		typeof v === "boolean" ||
		v === null ||
		v === undefined
	) {
		return v;
	}
	if (LOGGER_CONFIG.enableInspect) {
		return inspect(v, { compact: true, breakLength: Infinity, colors: false });
	}
	if (v instanceof Error) {
		return String(v);
	}
	try {
		return JSON.stringify(v);
	} catch {
		return "[cannot stringify]";
	}
}
