const effect = ((fn?: () => unknown) => fn?.()) as unknown as {
	(fn?: () => unknown): unknown;
	root: (fn: () => undefined | (() => void)) => () => void;
};

effect.root = (fn) => {
	const cleanup = fn();
	return typeof cleanup === "function" ? cleanup : () => {};
};

(globalThis as Record<string, unknown>).$state = <T>(value: T) => value;
(globalThis as Record<string, unknown>).$effect = effect;
