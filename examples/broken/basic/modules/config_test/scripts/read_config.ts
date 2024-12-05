import { RuntimeError, ScriptContext } from "../module.gen.ts";

export interface Request {
}

export interface Response {
	config: {
		foo: string;
		bar: number;
	};
}

export async function run(
	ctx: ScriptContext,
	req: Request,
): Promise<Response> {
	return {
		config: ctx.config,
	};
}
