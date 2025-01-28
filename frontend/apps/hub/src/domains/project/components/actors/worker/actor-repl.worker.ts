import { ActorHandleRaw } from "@rivet-gg/actor-client";
import type { InspectRpcResponse } from "@rivet-gg/actor-protocol/ws/to_client";
import { fromJs } from "esast-util-from-js";
import { toJs } from "estree-util-to-js";
import {
	type HighlighterCore,
	createHighlighterCore,
	createOnigurumaEngine,
} from "shiki";
import {
	type Connection,
	MessageSchema,
	type ReplErrorCode,
	type Response,
	ResponseSchema,
	type State,
} from "./actor-worker-schema";

class ReplError extends Error {
	constructor(
		public readonly code: ReplErrorCode,
		message: string,
	) {
		super(message);
	}

	static unsupported() {
		return new ReplError("unsupported", "Actor unsupported");
	}
}

export let highlighter: HighlighterCore | undefined;

async function formatCode(code: string) {
	highlighter ??= await createHighlighterCore({
		themes: [import("shiki/themes/github-dark-default.mjs")],
		langs: [import("@shikijs/langs/typescript")],
		engine: createOnigurumaEngine(import("shiki/wasm")),
	});

	return highlighter.codeToTokens(code, {
		lang: "typescript",
		theme: "github-dark-default",
	});
}

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function evaluateCode(code: string, args: Record<string, unknown>) {
	const argsString = Object.keys(args);
	const argValues = Object.values(args);

	let jsCode: ReturnType<typeof toJs>;
	try {
		const program = fromJs(code, {
			module: true,
			allowAwaitOutsideFunction: true,
			allowReturnOutsideFunction: true,
		});

		const lastStatement = program.body[program.body.length - 1];
		if (lastStatement.type === "ExpressionStatement") {
			program.body[program.body.length - 1] = {
				type: "ReturnStatement",
				argument: lastStatement.expression,
			};
		}

		jsCode = toJs(program);
	} catch (e) {
		throw new ReplError("syntax", "Syntax error");
	}

	return new Function(
		"window",
		...argsString,
		`"use strict";
        return (async () => {
            ${jsCode.value}
    })()
    `,
	)({}, ...argValues);
}

const createConsole = (id: string) => {
	return new Proxy(
		{ ...console },
		{
			get(target, prop) {
				return (...args: unknown[]) => {
					respond({
						type: "log",
						id,
						data: {
							method: prop as "log" | "warn" | "error",
							data: args,
							timestamp: new Date().toISOString(),
						},
					});
					return Reflect.get(target, prop)(...args);
				};
			},
		},
	);
};

let init: null | ({ handle: ActorHandleRaw } & InspectRpcResponse);

addEventListener("message", async (event) => {
	const { success, data } = MessageSchema.safeParse(event.data);

	if (!success) {
		return;
	}

	if (data.type === "init") {
		if (init) {
			respond({
				type: "error",
				data: new Error("Actor already initialized"),
			});
			return;
		}

		try {
			const handle = new ActorHandleRaw(
				`${data.endpoint}/__inspect`,
				undefined,
				"cbor",
			);

			handle.connect();

			if (!handle) {
				respond({
					type: "error",
					data: new Error("Could not connect to actor"),
				});

				throw new Error("Could not connect to actor, bailing out");
			}

			const inspect = await Promise.race([
				handle.rpc<[], InspectRpcResponse>("inspect"),
				wait(5000).then(() => undefined),
			]);

			if (!inspect) {
				respond({
					type: "error",
					data: ReplError.unsupported(),
				});

				throw ReplError.unsupported();
			}

			if (inspect.state.enabled) {
				handle.on("_state-changed", (state: State) => {
					respond({
						type: "state-change",
						data: state,
					});
				});
			}

			handle.on("_connections-changed", (connections: Connection[]) => {
				respond({
					type: "connections-change",
					data: connections,
				});
			});

			init = { handle, ...inspect };
			return respond({
				type: "ready",
				data: {
					...inspect,
				},
			});
		} catch (e) {
			return respond({
				type: "error",
				data: e,
			});
		}
	}

	if (data.type === "code") {
		const actor = init;
		if (!actor) {
			respond({
				type: "error",
				data: new Error("Actor not initialized"),
			});
			return;
		}

		try {
			const formatted = await formatCode(data.data);
			respond({
				type: "formatted",
				id: data.id,
				data: formatted,
			});

			const exposedActor = Object.fromEntries(
				init?.rpcs.map((rpc) => [
					rpc,
					actor.handle.rpc.bind(actor.handle, rpc),
				]) ?? [],
			);

			const evaluated = await evaluateCode(data.data, {
				console: createConsole(data.id),
				wait,
				actor: exposedActor,
			});
			return respond({
				type: "result",
				id: data.id,
				data: evaluated,
			});
		} catch (e) {
			return respond({
				type: "error",
				id: data.id,
				data: e,
			});
		}
	}

	if (data.type === "set-state") {
		const actor = init;
		if (!actor) {
			respond({
				type: "error",
				data: new Error("Actor not initialized"),
			});
			return;
		}

		try {
			const state = JSON.parse(data.data);
			await actor.handle.rpc("setState", state);
			return respond({
				type: "state-change",
				data: {
					enabled: true,
					native: data.data,
				},
			});
		} catch (e) {
			return respond({
				type: "error",
				data: e,
			});
		}
	}
});

function respond(msg: Response) {
	return postMessage(ResponseSchema.parse(msg));
}
