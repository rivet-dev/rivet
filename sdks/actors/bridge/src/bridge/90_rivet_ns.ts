import { primordials } from "ext:core/mod.js";
import { KV_NAMESPACE } from "ext:rivet_kv/40_rivet_kv.js";
const { ReflectOwnKeys, ObjectFreeze } = primordials;

import type { Metadata } from "./types/metadata.d.ts";
export type { Metadata } from "./types/metadata.d.ts";

export function deepFreeze<T extends object>(object: T): Readonly<T> {
	// Retrieve the property names defined on object
	const propNames = ReflectOwnKeys(object);

	// Freeze properties before freezing self
	for (const name of propNames) {
		// biome-ignore lint/suspicious/noExplicitAny: <explanation>
		const value = (object as any)[name as string];

		// Check if value is an array or object and not null
		if (
			value &&
			(Array.isArray(value) ||
				typeof value === "object" ||
				typeof value === "function")
		) {
			deepFreeze(value);
		}
	}

	return ObjectFreeze(object);
}

export const ACTOR_CONTEXT = {
	// Populated at runtime
	metadata: undefined as unknown as Metadata,
	kv: KV_NAMESPACE,
};
