// DO NOT MODIFY
//
// Generated with scripts/sdk_actor/compile_bridge.ts

import { primordials } from "ext:core/mod.js";
import { KV_NAMESPACE } from "ext:rivet_kv/40_rivet_kv.js";
const { ReflectOwnKeys, ObjectFreeze } = primordials;
export function deepFreeze(object) {
    // Retrieve the property names defined on object
    const propNames = ReflectOwnKeys(object);
    // Freeze properties before freezing self
    for (const name of propNames) {
        // biome-ignore lint/suspicious/noExplicitAny: Unknown object type
        const value = object[name];
        // Check if value is an array or object and not null
        if (value &&
            (Array.isArray(value) ||
                typeof value === "object" ||
                typeof value === "function")) {
            deepFreeze(value);
        }
    }
    return ObjectFreeze(object);
}
export const ACTOR_CONTEXT = {
    // Populated at runtime
    metadata: undefined,
    kv: KV_NAMESPACE,
};
