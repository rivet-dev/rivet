/**
 * Sleep for a given number of milliseconds.
 */
export function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

const TIMEOUT_MAX = 2147483647;

export type LongTimeoutHandle = { abort: () => void };

export function setLongTimeout(
	listener: () => void,
	after: number,
): LongTimeoutHandle {
	let timeout: ReturnType<typeof setTimeout> | undefined;

	function start(remaining: number) {
		if (remaining <= TIMEOUT_MAX) {
			timeout = setTimeout(listener, remaining);
		} else {
			timeout = setTimeout(() => {
				start(remaining - TIMEOUT_MAX);
			}, TIMEOUT_MAX);
		}
	}

	start(after);

	return {
		abort: () => {
			if (timeout !== undefined) clearTimeout(timeout);
		},
	};
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
