export function unreachable(x: never): never {
	throw `Unreachable: ${x}`;
}

export interface BackoffOptions {
	initialDelay?: number;
	maxDelay?: number;
	multiplier?: number;
	jitter?: boolean;
}

export function calculateBackoff(
	attempt: number,
	options: BackoffOptions = {},
): number {
	const {
		initialDelay = 1000,
		maxDelay = 30000,
		multiplier = 2,
		jitter = true,
	} = options;

	let delay = Math.min(initialDelay * Math.pow(multiplier, attempt), maxDelay);

	if (jitter) {
		// Add random jitter between 0% and 25% of the delay
		delay = delay * (1 + Math.random() * 0.25);
	}

	return Math.floor(delay);
}
