import { Predicate } from "effect";

/**
 * Identifies the SDK as the RPC system on action spans. Stamped onto
 * the `rpc.system.name` OTel attribute.
 */
export const rpcSystem = "rivet.actors";

/**
 * Cross-wire trace metadata. Carries just enough of an `Effect.Tracer`
 * span to reconstitute it on the server as a `Tracer.externalSpan`
 * parent for the handler's span.
 */
export interface TraceMeta {
	readonly traceId: string;
	readonly spanId: string;
	readonly sampled: boolean;
}

/**
 * Pull a valid `TraceMeta` out of the wire `ActionMeta` envelope, or
 * `undefined` if the caller didn't ship one (or shipped something
 * malformed). Kept lenient because the meta envelope is forward-
 * extensible — future fields shouldn't break trace extraction.
 */
export const readTraceMeta = (meta: unknown): TraceMeta | undefined => {
	if (!Predicate.isObject(meta)) return undefined;
	const trace = meta.trace;
	if (!Predicate.isObject(trace)) return undefined;
	if (
		!Predicate.isString(trace.traceId) ||
		!Predicate.isString(trace.spanId) ||
		!Predicate.isBoolean(trace.sampled)
	) {
		return undefined;
	}
	return {
		traceId: trace.traceId,
		spanId: trace.spanId,
		sampled: trace.sampled,
	};
};
