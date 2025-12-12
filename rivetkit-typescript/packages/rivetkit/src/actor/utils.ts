import * as errors from "./errors";
import { loggerWithoutContext } from "./log";

export function assertUnreachable(x: never): never {
	loggerWithoutContext().error({
		msg: "unreachable",
		value: `${x}`,
		stack: new Error().stack,
	});
	throw new errors.Unreachable(x);
}

export const throttle = <
	// biome-ignore lint/suspicious/noExplicitAny: we want to allow any function
	Fn extends (...args: any) => any,
>(
	fn: Fn,
	delay: number,
) => {
	let lastRan = false;
	let lastArgs: Parameters<Fn> | null = null;

	return (...args: Parameters<Fn>) => {
		if (!lastRan) {
			fn.apply(this, args);
			lastRan = true;
			const timer = () =>
				setTimeout(() => {
					lastRan = false;
					if (lastArgs) {
						fn.apply(this, lastArgs);
						lastRan = true;
						lastArgs = null;
						timer();
					}
				}, delay);
			timer();
		} else lastArgs = args;
	};
};

export class DeadlineError extends Error {
	constructor() {
		super("Promise did not complete before deadline.");
	}
}

export function deadline<T>(promise: Promise<T>, timeout: number): Promise<T> {
	const controller = new AbortController();
	const signal = controller.signal;

	// Set a timeout to abort the operation
	const timeoutId = setTimeout(() => controller.abort(), timeout);

	return Promise.race<T>([
		promise,
		new Promise<T>((_, reject) => {
			signal.addEventListener("abort", () => reject(new DeadlineError()));
		}),
	]).finally(() => {
		clearTimeout(timeoutId);
	});
}

export class Lock<T> {
	private _locked = false;
	private _waiting: Array<() => void> = [];

	constructor(private _value: T) {}

	async lock(fn: (value: T) => Promise<void>): Promise<void> {
		if (this._locked) {
			await new Promise<void>((resolve) => this._waiting.push(resolve));
		}
		this._locked = true;

		try {
			await fn(this._value);
		} finally {
			this._locked = false;
			const next = this._waiting.shift();
			if (next) next();
		}
	}
}

export function generateSecureToken(length = 32) {
	const array = new Uint8Array(length);
	crypto.getRandomValues(array);
	// Replace base64 chars that are not URL safe with URL-safe chars and strip padding
	return btoa(String.fromCharCode(...array))
		.replace(/\+/g, "-")
		.replace(/\//g, "_")
		.replace(/=/g, "");
}

/**
 * Checks if a path is an actor state path within the persisted actor data.
 */
export function isStatePath(path: string): boolean {
	return path === "state" || path.startsWith("state.");
}

/**
 * Checks if a path is a connection state path within the persisted actor data.
 */
export function isConnStatePath(path: string): boolean {
	if (!path.startsWith("connections.")) {
		return false;
	}
	const stateIndex = path.indexOf(".state", 12); // Start after "connections."
	if (stateIndex === -1) {
		return false;
	}
	const afterState = stateIndex + 6; // ".state".length = 6
	// Check if ".state" is followed by end of string or "."
	return path.length === afterState || path[afterState] === ".";
}
