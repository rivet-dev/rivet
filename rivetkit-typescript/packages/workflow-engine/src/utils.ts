/**
 * Sleep for a given number of milliseconds.
 */
export function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Safely parse JSON with a meaningful error message.
 */
export function safeJsonParse<T>(value: string, context: string): T {
	try {
		return JSON.parse(value) as T;
	} catch (error) {
		throw new Error(
			`Failed to parse ${context}: ${error instanceof Error ? error.message : String(error)}`,
		);
	}
}
