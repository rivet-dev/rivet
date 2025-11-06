import { logger } from "./log";

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

	let delay = Math.min(initialDelay * multiplier ** attempt, maxDelay);

	if (jitter) {
		// Add random jitter between 0% and 25% of the delay
		delay = delay * (1 + Math.random() * 0.25);
	}

	return Math.floor(delay);
}

export interface ParsedCloseReason {
	group: string;
	error: string;
	rayId?: string;
}

/**
 * Parses a WebSocket close reason in the format: {group}.{error} or {group}.{error}#{ray_id}
 *
 * Examples:
 *   - "ws.eviction#t1s80so6h3irenp8ymzltfoittcl00"
 *   - "ws.client_closed"
 *
 * Returns undefined if the format is invalid
 */
export function parseWebSocketCloseReason(
	reason: string,
): ParsedCloseReason | undefined {
	const [mainPart, rayId] = reason.split("#");
	const [group, error] = mainPart.split(".");

	if (!group || !error) {
		logger()?.warn({ msg: "failed to parse close reason", reason });
		return undefined;
	}

	return {
		group,
		error,
		rayId,
	};
}
