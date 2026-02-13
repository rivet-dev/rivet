export { stringifyError } from "@/common/utils";
export { assertUnreachable } from "./common/utils";

/**
 * Joins multiple abort signals into one.
 *
 * The returned signal aborts when the first input signal aborts.
 * Uses `AbortSignal.any(...)` when available, with a runtime fallback.
 */
export function joinSignals(
	...signals: Array<AbortSignal | undefined | null>
): AbortSignal {
	const validSignals = signals.filter(
		(signal): signal is AbortSignal => signal != null,
	);

	if (validSignals.length === 0) {
		return new AbortController().signal;
	}

	if (validSignals.length === 1) {
		return validSignals[0];
	}

	const signalAny = (
		AbortSignal as typeof AbortSignal & {
			any?: (signals: AbortSignal[]) => AbortSignal;
		}
	).any;
	if (typeof signalAny === "function") {
		return signalAny(validSignals);
	}

	const controller = new AbortController();
	const cleanups: Array<() => void> = [];

	const abortWithSignal = (signal: AbortSignal) => {
		if (controller.signal.aborted) {
			return;
		}

		for (const cleanup of cleanups) {
			cleanup();
		}

		const reason = (signal as AbortSignal & { reason?: unknown }).reason;
		controller.abort(reason);
	};

	for (const signal of validSignals) {
		if (signal.aborted) {
			abortWithSignal(signal);
			break;
		}

		const onAbort = () => abortWithSignal(signal);
		signal.addEventListener("abort", onAbort, { once: true });
		cleanups.push(() => signal.removeEventListener("abort", onAbort));
	}

	return controller.signal;
}

/**
 * Returns a promise that resolves after the given number of milliseconds.
 */
export function sleep(ms: number): Promise<void> {
	return new Promise<void>((resolve) => setTimeout(resolve, ms));
}

/**
 * Creates a fixed-rate interval tick function that can be awaited in a loop.
 *
 * @example
 * const tick = interval(100);
 * while (!c.aborted) {
 *   await tick();
 *   if (c.aborted) break;
 *   // ... game logic
 * }
 */
export function interval(ms: number): () => Promise<void> {
	return () => sleep(ms);
}

import type { Context as HonoContext, Handler as HonoHandler } from "hono";
import { stringify as uuidstringify } from "uuid";
import { stringifyError } from "@/common/utils";
import pkgJson from "../package.json" with { type: "json" };
import { getLogger } from "./common/log";

export const VERSION = pkgJson.version;

let _userAgent: string | undefined;

function logger() {
	return getLogger("utils");
}

export function httpUserAgent(): string {
	// Return cached value if already initialized
	if (_userAgent !== undefined) {
		return _userAgent;
	}

	// Library
	let userAgent = `RivetKit/${VERSION}`;

	// Navigator
	const navigatorObj =
		typeof navigator !== "undefined" ? navigator : undefined;
	if (navigatorObj?.userAgent) userAgent += ` ${navigatorObj.userAgent}`;

	_userAgent = userAgent;

	return userAgent;
}

export type UpgradeWebSocket = (
	createEvents: (c: HonoContext) => any,
) => HonoHandler;

export type GetUpgradeWebSocket = () => UpgradeWebSocket;

export function getEnvUniversal(key: string): string | undefined {
	if (typeof Deno !== "undefined") {
		return Deno.env.get(key);
	} else if (typeof process !== "undefined") {
		// Do this after Deno since `process` is sometimes polyfilled
		return process.env[key];
	}
}

export function dbg<T>(x: T): T {
	console.trace(`=== DEBUG ===\n${x}`);
	return x;
}

/**
 * Converts various ArrayBuffer-like types to Uint8Array.
 * Handles ArrayBuffer, ArrayBufferView (including typed arrays), and passes through existing Uint8Array.
 *
 * @param data - The ArrayBuffer or ArrayBufferView to convert
 * @returns A Uint8Array view of the data
 */
export function toUint8Array(data: ArrayBuffer | ArrayBufferView): Uint8Array {
	if (data instanceof Uint8Array) {
		return data;
	} else if (data instanceof ArrayBuffer) {
		return new Uint8Array(data);
	} else if (ArrayBuffer.isView(data)) {
		// Handle other ArrayBufferView types (Int8Array, Uint16Array, DataView, etc.)
		return new Uint8Array(
			data.buffer.slice(
				data.byteOffset,
				data.byteOffset + data.byteLength,
			),
		);
	} else {
		throw new TypeError("Input must be ArrayBuffer or ArrayBufferView");
	}
}

// Long timeouts
//
// JavaScript timers use a signed 32-bit integer for delays, so values above 2^31-1 (~24.8 days)
// are not reliable and may fire immediately or overflow.
//
// https://developer.mozilla.org/en-US/docs/Web/API/Window/setTimeout#maximum_delay_value
const TIMEOUT_MAX = 2147483647; // 2^31-1

export type LongTimeoutHandle = { abort: () => void };

/**
 * Polyfill for Promise.withResolvers().
 *
 * This is specifically for Cloudflare Workers. Their implementation of Promise.withResolvers does not work correctly.
 */
export function promiseWithResolvers<T>(onReject: (reason?: any) => void): {
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
	promise.catch(onReject);
	return { promise, resolve, reject };
}

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
 * A tiny utility that coalesces/enqueues async operations so only the latest
 * queued task runs per cycle, while callers receive a promise that resolves
 * when the task for the cycle they joined has completed.
 */
export class SinglePromiseQueue {
	/** Next operation to execute in the queue. If attempting to enqueue another op, it will override the existing op. */
	#queuedOp?: () => Promise<void>;

	/** The currently running promise of #drainLoop. Do not await this, instead await `pending` to await the current cycle. */
	runningDrainLoop?: Promise<void>;

	/** Pending resolver fro the currently queued entry. */
	#pending?: ReturnType<typeof promiseWithResolvers<void>>;

	/** Queue the next operation and return a promise that resolves when it flushes. */
	enqueue(op: () => Promise<void>): Promise<void> {
		// Replace any previously queued operation with the latest one
		this.#queuedOp = op;

		// Ensure a shared resolver exists for all callers in this cycle
		if (!this.#pending) {
			this.#pending = promiseWithResolvers<void>((reason) => logger().warn({ msg: "unhandled single promise queue rejection", reason }));
		}

		const waitForThisCycle = this.#pending.promise;

		// Start runner if not already running
		if (!this.runningDrainLoop) {
			this.runningDrainLoop = this.#drainLoop();
		}

		return waitForThisCycle;
	}

	/** Drain queued operations sequentially until there is nothing left. */
	async #drainLoop(): Promise<void> {
		try {
			while (this.#queuedOp) {
				// Capture current cycle resolver then reset for the next cycle
				const resolver = this.#pending;
				this.#pending = undefined;

				// Capture and clear the currently queued operation
				const op = this.#queuedOp;
				this.#queuedOp = undefined;

				try {
					await op();
					// Notify all waiters for this cycle
					resolver?.resolve();
				} catch (err) {
					logger().error({
						msg: "error in SinglePromiseQueue drain loop",
						error: stringifyError(err),
					});
					// Reject all waiters for this cycle
					resolver?.reject(err);
				}
			}
		} finally {
			this.runningDrainLoop = undefined;
		}
	}
}

export function bufferToArrayBuffer(buf: Buffer | Uint8Array): ArrayBuffer {
	return buf.buffer.slice(
		buf.byteOffset,
		buf.byteOffset + buf.byteLength,
	) as ArrayBuffer;
}

/**
 * Properly combines a base URL endpoint with a path, preserving any base path in the endpoint.
 *
 * @example
 * combineUrlPath("http://localhost:8787/rivet", "/actors/action")
 * // Returns: "http://localhost:8787/rivet/actors/action"
 *
 * @example
 * combineUrlPath("http://localhost:8787/rivet", "/actors?type=foo", { namespace: "test" })
 * // Returns: "http://localhost:8787/rivet/actors?type=foo&namespace=test"
 *
 * @param endpoint The base URL endpoint that may contain a path component
 * @param path The path to append to the endpoint (may include query parameters)
 * @param queryParams Optional additional query parameters to append
 * @returns The properly combined URL string
 */
export function combineUrlPath(
	endpoint: string,
	path: string,
	queryParams?: Record<string, string | undefined>,
): string {
	const baseUrl = new URL(endpoint);

	// Extract path and query from the provided path
	const pathParts = path.split("?");
	const pathOnly = pathParts[0];
	const existingQuery = pathParts[1] || "";

	// Remove trailing slash from base path and ensure path starts with /
	const basePath = baseUrl.pathname.replace(/\/$/, "");
	const cleanPath = pathOnly.startsWith("/") ? pathOnly : `/${pathOnly}`;
	// Combine paths and remove any double slashes
	const fullPath = (basePath + cleanPath).replace(/\/\//g, "/");

	// Build query string
	const queryParts: string[] = [];
	if (existingQuery) {
		queryParts.push(existingQuery);
	}
	if (queryParams) {
		for (const [key, value] of Object.entries(queryParams)) {
			if (value !== undefined) {
				queryParts.push(
					`${encodeURIComponent(key)}=${encodeURIComponent(value)}`,
				);
			}
		}
	}

	const fullQuery = queryParts.length > 0 ? `?${queryParts.join("&")}` : "";
	return `${baseUrl.protocol}//${baseUrl.host}${fullPath}${fullQuery}`;
}

export function arrayBuffersEqual(
	buf1: ArrayBuffer,
	buf2: ArrayBuffer,
): boolean {
	if (buf1.byteLength !== buf2.byteLength) return false;

	const view1 = new Uint8Array(buf1);
	const view2 = new Uint8Array(buf2);

	for (let i = 0; i < view1.length; i++) {
		if (view1[i] !== view2[i]) return false;
	}
	return true;
}

export const EXTRA_ERROR_LOG = {
	issues: "https://github.com/rivet-dev/rivetkit/issues",
	support: "https://rivet.dev/discord",
	version: VERSION,
};

export type Runtime = "deno" | "bun" | "node";

export function detectRuntime(): Runtime {
	const userAgent =
		typeof navigator !== "undefined" ? navigator.userAgent : "";
	if (userAgent.includes("Deno")) {
		return "deno";
	}
	if (userAgent.includes("Bun")) {
		return "bun";
	}
	return "node";
}

export type DeepReadonly<T> = {
	readonly [K in keyof T]: T[K] extends object ? DeepReadonly<T[K]> : T[K];
};

export type DeepMutable<T> = {
	-readonly [K in keyof T]: T[K] extends object ? DeepMutable<T[K]> : T[K];
};
