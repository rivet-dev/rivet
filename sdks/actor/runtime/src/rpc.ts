import type { AnyActor } from "./actor.ts";
import type { Connection } from "./connection.ts";

export class Rpc<A extends AnyActor> {
	constructor(public readonly connection: Connection<A>) {}
}
