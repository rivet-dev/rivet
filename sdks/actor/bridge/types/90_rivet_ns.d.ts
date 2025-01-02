// DO NOT MODIFY
//
// Generated from sdks/actor/bridge/

import type { Kv } from "./40_rivet_kv.d.ts";
import type { Metadata } from "./types/metadata.d.ts";
export declare function deepFreeze<T extends object>(object: T): Readonly<T>;
export interface ActorContext {
	metadata: Metadata;
	kv: Kv;
}
export declare const ACTOR_CONTEXT: ActorContext;
