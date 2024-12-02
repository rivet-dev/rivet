import { ScriptContext } from "../module.gen.ts";

export interface Request extends Record<string, never> {}

export interface Response {
	pong: string;
}

export async function run(
	_ctx: ScriptContext,
	_req: Request,
): Promise<Response> {
	return { pong: "pong" };
}
