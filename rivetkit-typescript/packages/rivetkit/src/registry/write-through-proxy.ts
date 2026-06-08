import onChange from "@rivetkit/on-change";

/**
 * Creates a proxy that tracks deep mutations on an object and calls `commit`
 * after every change. Uses `@rivetkit/on-change` internally, which correctly
 * detects mutations via methods on Map, Set, Date, TypedArrays, and arrays.
 *
 * If the value is not an object (primitive, null, undefined), it is returned
 * as-is since primitives cannot be proxied or mutated.
 *
 * @param value - The root value to watch.
 * @param commit - Called after every detected mutation with the root object.
 * @param beforeChange - Called before every mutation with the new value being
 *   assigned. Throw to reject the change.
 */
export function createWriteThroughProxy<T>(
	value: T,
	commit: (next: T) => void,
	beforeChange?: (newValue: unknown) => void,
): T {
	if (!value || typeof value !== "object") {
		return value;
	}

	return onChange(
		value as T & Record<string, any>,
		() => {
			commit(value);
		},
		{
			// Rejection is throw-based: beforeChange throws to prevent the
			// mutation. We always return true so on-change applies the change
			// if beforeChange did not throw.
			onValidate(_path: string, newValue: unknown) {
				beforeChange?.(newValue);
				return true;
			},
		},
	) as T;
}
