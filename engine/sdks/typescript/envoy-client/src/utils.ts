import { logger } from "./log";

export type ShutdownReason = "normal" | "serverless-early-exit";

export class BufferMap<T> {
	#inner: Map<string, T>;
	constructor() {
		this.#inner = new Map();
	}

	get(buffers: ArrayBuffer[]): T | undefined {
		return this.#inner.get(cyrb53(buffers));
	}

	set(buffers: ArrayBuffer[], value: T) {
		this.#inner.set(cyrb53(buffers), value);
	}

	delete(buffers: ArrayBuffer[]): boolean {
		return this.#inner.delete(cyrb53(buffers));
	}

	has(buffers: ArrayBuffer[]): boolean {
		return this.#inner.has(cyrb53(buffers));
	}
}

function cyrb53(buffers: ArrayBuffer[], seed: number = 0): string {
	let h1 = 0xdeadbeef ^ seed, h2 = 0x41c6ce57 ^ seed;
	for (const buffer of buffers) {
		const bytes = new Uint8Array(buffer);
		for (const b of bytes) {
			h1 = Math.imul(h1 ^ b, 2654435761);
			h2 = Math.imul(h2 ^ b, 1597334677);
		}
	}
	h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507) ^ Math.imul(h2 ^ (h2 >>> 13), 3266489909);
	h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507) ^ Math.imul(h1 ^ (h1 >>> 13), 3266489909);
	return (4294967296 * (2097151 & h2) + (h1 >>> 0)).toString(16);
}

export class EnvoyShutdownError extends Error {
	constructor() {
		super("Envoy shut down");
	}
}

/** Resolves after the configured debug latency, or immediately if none. */
export function injectLatency(ms?: number): Promise<void> {
	if (!ms) return Promise.resolve();
	return new Promise((resolve) => setTimeout(resolve, ms));
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

const U16_MAX = 65535;

/**
 * Wrapping greater than comparison for u16 values.
 * Based on shared_state.rs wrapping_gt implementation.
 */
export function wrappingGtU16(a: number, b: number): boolean {
	return a !== b && wrappingSub(a, b, U16_MAX) < U16_MAX / 2;
}

/**
 * Wrapping less than comparison for u16 values.
 * Based on shared_state.rs wrapping_lt implementation.
 */
export function wrappingLtU16(a: number, b: number): boolean {
	return a !== b && wrappingSub(b, a, U16_MAX) < U16_MAX / 2;
}

/**
 * Wrapping greater than or equal comparison for u16 values.
 */
export function wrappingGteU16(a: number, b: number): boolean {
	return a === b || wrappingGtU16(a, b);
}

/**
 * Wrapping less than or equal comparison for u16 values.
 */
export function wrappingLteU16(a: number, b: number): boolean {
	return a === b || wrappingLtU16(a, b);
}

/**
 * Performs wrapping addition for u16 values.
 */
export function wrappingAddU16(a: number, b: number): number {
	return (a + b) % (U16_MAX + 1);
}

/**
 * Performs wrapping subtraction for u16 values.
 */
export function wrappingSubU16(a: number, b: number): number {
	return wrappingSub(a, b, U16_MAX);
}

/**
 * Performs wrapping subtraction for unsigned integers.
 */
function wrappingSub(a: number, b: number, max: number): number {
	const result = a - b;
	if (result < 0) {
		return result + max + 1;
	}
	return result;
}

export function arraysEqual(a: ArrayBuffer, b: ArrayBuffer): boolean {
	const ua = new Uint8Array(a);
	const ub = new Uint8Array(b);
	if (ua.length !== ub.length) return false;
	for (let i = 0; i < ua.length; i++) {
		if (ua[i] !== ub[i]) return false;
	}
	return true;
}

/**
 * Polyfill for Promise.withResolvers().
 *
 * This is specifically for Cloudflare Workers. Their implementation of Promise.withResolvers does not work correctly.
 */
export function promiseWithResolvers<T>(): {
	promise: Promise<T>;
	resolve: (value: T | PromiseLike<T>) => void;
	reject: (reason?: any) => void;
} {
	let resolve!: (value: T | PromiseLike<T>) => void;
	let reject!: (reason?: any) => void;
	const promise = new Promise<T>((res, rej) => {
		resolve = res;
		reject = rej;
	});
	return { promise, resolve, reject };
}

export function idToStr(id: ArrayBuffer): string {
	const bytes = new Uint8Array(id);
	return Array.from(bytes)
		.map((byte) => byte.toString(16).padStart(2, "0"))
		.join("");
}

export function stringifyError(error: unknown): string {
	if (error instanceof Error) {
		return `${error.name}: ${error.message}${error.stack ? `\n${error.stack}` : ""}`;
	} else if (typeof error === "string") {
		return error;
	} else if (typeof error === "object" && error !== null) {
		try {
			return `${JSON.stringify(error)}`;
		} catch {
			return `[object ${error.constructor?.name || "Object"}]`;
		}
	} else {
		return String(error);
	}
}
