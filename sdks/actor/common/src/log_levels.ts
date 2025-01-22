export type LogLevel = "DEBUG" | "INFO" | "WARN" | "ERROR" | "CRITICAL";

export const LogLevels: Record<LogLevel, LevelIndex> = {
	DEBUG: 0,
	INFO: 1,
	WARN: 2,
	ERROR: 3,
	CRITICAL: 4,
} as const;

export const LevelNameMap: Record<number, LogLevel> = {
	0: "DEBUG",
	1: "INFO",
	2: "WARN",
	3: "ERROR",
	4: "CRITICAL",
};

export type LevelIndex = number;
