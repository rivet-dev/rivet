import { primordials } from "ext:core/mod.js";
import { KV_NAMESPACE } from "ext:rivet_kv/40_rivet_kv.js";
const { ReflectOwnKeys, ObjectFreeze } = primordials;

import type { Metadata } from "./types/metadata.d.ts";
export type { Metadata } from "./types/metadata.d.ts";

export function deepFreeze(object: Record<any, any>): Readonly<Record<any, any>> {
	// Retrieve the property names defined on object
	const propNames = ReflectOwnKeys(object);

	// Freeze properties before freezing self
	for (const name of propNames) {
		const value = object[name as string];

		if ((value && typeof value === "object") || typeof value === "function") {
			deepFreeze(value);
		}
	}

	return ObjectFreeze(object);
}

export const ACTOR_CONTEXT = {
	// Populated at runtime
	metadata: null as any as Metadata,
	kv: KV_NAMESPACE,
};
