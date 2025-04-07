import { fromJs } from "esast-util-from-js";
import { toJs } from "estree-util-to-js";
import {
	type ToClient,
	ToClientSchema,
	type InspectData,
	type ToServer,
} from "actor-core/inspector/protocol/actor";

import type { ResponseOk, Request } from "actor-core/protocol/http";
import {
	type HighlighterCore,
	createHighlighterCore,
	createOnigurumaEngine,
} from "shiki";
import {
	MessageSchema,
	type ReplErrorCode,
	type Response,
	ResponseSchema,
} from "./actor-worker-schema";
import { endWithSlash } from "../../lib/utils";

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

let init: null | ({ ws: WebSocket; url: URL } & InspectData) = null;

async function connect(endpoint: string) {
	const url = new URL("inspect", endWithSlash(endpoint));
	const ws = new WebSocket(url);

	await waitForOpen(ws);

	ws.send(
		JSON.stringify({
			type: "info",
		} satisfies ToServer),
	);

	const { type: _, ...info } = await waitForMessage(ws, "info");
	init = { ...info, ws, url: new URL(endpoint) };

	ws.addEventListener("message", (event) => {
		try {
			const data = ToClientSchema.parse(JSON.parse(event.data));

			if (data.type === "info") {
				return respond({
					type: "inspect",
					data: {
						...data,
					},
				});
			}
			if (data.type === "error") {
				return respond({
					type: "error",
					data: data.message,
				});
			}
		} catch (error) {
			console.warn("Malformed message", event.data, error);
			return;
		}
	});

	ws.addEventListener("close", () => {
		respond({
			type: "lost-connection",
		});
		setTimeout(() => {
			connect(endpoint);
		}, 500);
	});

	respond({
		type: "ready",
		data: {
			...info,
		},
	});
}

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
			await Promise.race([
				connect(data.endpoint),
				wait(5000).then(() => {
					throw new Error("Timeout");
				}),
			]);

			return;
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

			const createRpc =
				(rpc: string) =>
				async (...args: unknown[]) => {
					const url = new URL(
						`rpc/${rpc}`,
						endWithSlash(actor.url.href),
					);
					const response = await fetch(url, {
						method: "POST",
						body: JSON.stringify({
							a: args,
						} satisfies Request),
					});

					if (!response.ok) {
						throw new Error("RPC failed");
					}

					const data = (await response.json()) as ResponseOk;
					return data.o;
				};

			const exposedActor = Object.fromEntries(
				init?.rpcs.map((rpc) => [rpc, createRpc(rpc)]) ?? [],
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
			actor.ws.send(
				JSON.stringify({
					type: "setState",
					state,
				} satisfies ToServer),
			);
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

function waitForOpen(ws: WebSocket) {
	const { promise, resolve, reject } = Promise.withResolvers();
	ws.addEventListener("open", () => {
		resolve(undefined);
	});
	ws.addEventListener("error", (event) => {
		reject();
	});
	ws.addEventListener("close", (event) => {
		reject();
	});

	return Promise.race([
		promise,
		wait(5000).then(() => {
			throw new Error("Timeout");
		}),
	]);
}

function waitForMessage<T extends ToClient["type"]>(
	ws: WebSocket,
	type: T,
): Promise<Extract<ToClient, { type: T }>> {
	const { promise, resolve, reject } =
		Promise.withResolvers<Extract<ToClient, { type: T }>>();

	function onMessage(event: MessageEvent) {
		try {
			const data = ToClientSchema.parse(JSON.parse(event.data));

			if (data.type === type) {
				resolve(data as Extract<ToClient, { type: T }>);
				ws.removeEventListener("message", onMessage);
			}
		} catch (e) {
			console.error(e);
		}
	}

	ws.addEventListener("message", onMessage);
	ws.addEventListener("error", (event) => {
		ws.removeEventListener("message", onMessage);
		reject();
	});

	return Promise.race([
		promise,
		wait(5000).then(() => {
			throw new Error("Timeout");
		}),
	]);
}
