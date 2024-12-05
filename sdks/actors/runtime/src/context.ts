import { Connection } from "./connection.ts";
import type { Actor } from "./actor.ts";

export class Context<A extends Actor<any, any, any>> {
	constructor(public readonly connection: Connection<A>) {}
}
