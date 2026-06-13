import { Predicate } from "effect";

/**
 * Refinement that narrows `unknown` to an object with `key` set to a
 * `string`.
 */
export const hasStringProperty =
	<K extends PropertyKey>(
		key: K,
	): Predicate.Refinement<unknown, { readonly [P in K]: string }> =>
	(u): u is { readonly [P in K]: string } =>
		Predicate.hasProperty(u, key) && Predicate.isString(u[key]);
