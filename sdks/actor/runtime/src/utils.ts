import * as errors from "./errors.ts";

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
