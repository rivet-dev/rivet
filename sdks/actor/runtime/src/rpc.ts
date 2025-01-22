import type { AnyActor } from "./actor";
import type { Connection } from "./connection";

/**
 * Context for an remote procedure call.
 *
 * @typeParam A Actor this RPC belongs to
 * @see {@link https://rivet.gg/docs/rpc|RPC Documentation}
 */
export class Rpc<A extends AnyActor> {
	/**
	 * Should not be called directly.
	 *
	 * @param connection - The connection associated with the RPC.
	 */
	constructor(public readonly connection: Connection<A>) {}
}
