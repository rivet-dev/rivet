// DO NOT MODIFY
//
// Generated with scripts/sdk_actors/compile_bridge.ts

import { primordials } from "ext:core/mod.js";
import { KV_NAMESPACE } from "ext:rivet_kv/40_rivet_kv.js";
const { ReflectOwnKeys, ObjectFreeze } = primordials;
export function deepFreeze(object) {
    // Retrieve the property names defined on object
    const propNames = ReflectOwnKeys(object);
    // Freeze properties before freezing self
    for (const name of propNames) {
        const value = object[name];
        if ((value && typeof value === "object") || typeof value === "function") {
            deepFreeze(value);
        }
    }
    return ObjectFreeze(object);
}
export const ACTOR_CONTEXT = {
    // Populated at runtime
    metadata: null,
    kv: KV_NAMESPACE,
};
