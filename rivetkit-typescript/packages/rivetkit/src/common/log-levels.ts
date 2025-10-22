export type LogLevel =
	| "TRACE"
	| "DEBUG"
	| "INFO"
	| "WARN"
	| "ERROR"
	| "CRITICAL";

export const LogLevels: Record<LogLevel, LevelIndex> = {
	TRACE: 0,
	DEBUG: 1,
	INFO: 2,
	WARN: 3,
	ERROR: 4,
	CRITICAL: 5,
} as const;

export const LevelNameMap: Record<number, LogLevel> = {
	0: "TRACE",
	1: "DEBUG",
	2: "INFO",
	3: "WARN",
	4: "ERROR",
	5: "CRITICAL",
};

export type LevelIndex = number;
