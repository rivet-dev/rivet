import { ScriptContext, Empty } from "../module.gen.ts";
import { FetchResponse } from "../actors/counter.ts";

export interface Request {
  key: string;
}

export type Response = FetchResponse;

export async function run(ctx: ScriptContext, req: Request): Promise<Response> {
	return await ctx.actors.counter
		.getOrCreateAndCall<undefined, Empty, FetchResponse>(
      req.key,
			undefined,
			"rpcFetchCount",
			{},
		);
}
