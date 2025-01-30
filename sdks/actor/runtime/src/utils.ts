import * as errors from "./errors";

export function assertUnreachable(x: never): never {
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

export function deadline<T>(promise: Promise<T>, timeout: number): Promise<T> {
	const controller = new AbortController();
	const signal = controller.signal;

	// Set a timeout to abort the operation
	const timeoutId = setTimeout(() => controller.abort(), timeout);

	return Promise.race<T>([
		promise,
		new Promise<T>((_, reject) => {
			signal.addEventListener("abort", () =>
				reject(new Error("Operation timed out")),
			);
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

/**
 * Like `Partial` but makes all sub-properties `Partial` too.
 */
export type RecursivePartial<T> = {
	[P in keyof T]?: T[P] extends (infer U)[]
		? RecursivePartial<U>[]
		: T[P] extends object | undefined
			? RecursivePartial<T[P]>
			: T[P];
};
