/**
 * Resolve a {@link MaybeGetter} to its underlying value.
 *
 * If `value` is a function, it is invoked and its return value is used.
 * If the resolved value is `undefined`, the optional `defaultValue` is
 * returned instead.
 *
 * @param value - A value or a getter function returning the value.
 * @param defaultValue - Fallback if the resolved value is `undefined`.
 * @returns The resolved value, or the default.
 */
import type { MaybeGetter } from "./types.js";

export function extract<T>(value: MaybeGetter<T>): T;
export function extract<T>(value: MaybeGetter<T | undefined>, defaultValue: T): T;
export function extract(value: unknown, defaultValue?: unknown): unknown {
	const resolved = typeof value === "function" ? (value as () => unknown)() : value;
	return resolved === undefined ? defaultValue : resolved;
}
