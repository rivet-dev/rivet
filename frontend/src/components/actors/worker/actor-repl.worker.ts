import { fromJs } from "esast-util-from-js";
import { toJs } from "estree-util-to-js";
import { createActorInspectorClient } from "rivetkit/inspector";
import {
	createHighlighterCore,
	createOnigurumaEngine,
	type HighlighterCore,
} from "shiki";
import { match } from "ts-pattern";
import {
	type InitMessage,
	MessageSchema,
	type ReplErrorCode,
	type Response,
	ResponseSchema,
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

let init: null | Omit<InitMessage, "type"> = null;

addEventListener("message", async (event) => {
	const { success, error, data } = MessageSchema.safeParse(event.data);

	if (!success) {
		console.error("Malformed message", event.data, error);
		return;
	}

	if (data.type === "init") {
		init = structuredClone(data);
		respond({
			type: "ready",
		});
		return;
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
					const response = await callAction({ name: rpc, args });
					return response;
				};

			const exposedActor = Object.fromEntries(
				actor.rpcs?.map((rpc) => [rpc, createRpc(rpc)]) ?? [],
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
});

function respond(msg: Response) {
	return postMessage(ResponseSchema.parse(msg));
}

async function callAction({ name, args }: { name: string; args: unknown[] }) {
	if (!init) throw new Error("Actor not initialized");

	const url = new URL(`inspect`, init.endpoint).href;

	const additionalHeaders = match(__APP_TYPE__)
		.with("engine", () => {
			return init?.engineToken
				? { "X-Rivet-Token": init.engineToken || "" }
				: ({} as Record<string, string>);
		})
		.otherwise(() => ({}));

	// we need to build this from scratch because we don't have access to
	// createInspectorActorContext in the worker
	// and we want to avoid bundling the entire RivetKit here, issues with @react-refresh
	const client = createActorInspectorClient(url, {
		headers: {
			Authorization: init.inspectorToken
				? `Bearer ${init.inspectorToken}`
				: "",
			"x-rivet-target": "actor",
			"x-rivet-actor": init.id,
			"X-RivetKit-Query": JSON.stringify({
				getForId: { actorId: init.id },
			}),
			...additionalHeaders,
		},
	});

	const response = await client.action.$post({
		json: { name, params: args },
	});

	if (!response.ok) {
		try {
			return await response.json();
		} catch {
			return await response.text();
		}
	}

	return (await response.json()).result;
}
