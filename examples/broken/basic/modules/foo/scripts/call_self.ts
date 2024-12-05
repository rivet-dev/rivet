import { ScriptContext } from "../module.gen.ts";

export interface Request extends Record<string, never> {}

export interface Response {
	response: {
		pong: string;
	};
}

export async function run(
	ctx: ScriptContext,
	_req: Request,
): Promise<Response> {
	return {
		response: await ctx.modules.foo.ping({}),
	};
}
