import { Connection } from "./connection.ts";
import type { Actor } from "./actor.ts";

export class Rpc<A extends Actor<any, any, any>> {
	constructor(public readonly connection: Connection<A>) {}
}
