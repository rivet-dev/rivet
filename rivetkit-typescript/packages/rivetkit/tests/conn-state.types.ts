import { actor } from "../src/actor/definition";
import type { Client } from "../src/client/client";
import type { Registry } from "../src/registry";

// The context parameter of `createConnState` is unannotated, so TypeScript
// infers it in the same pass as the actions. `c.conn.state` must still get its
// return type.
const definition = actor({
	createConnState: (_c, params: { token: string }) => ({
		userId: params.token,
	}),
	actions: {
		whoami: (c) => c.conn.state.userId,
		add: (c, amount: number) => amount + c.conn.params.token.length,
		nested: {
			token: (c) => c.conn.params.token,
		},
	},
});

// Actions declared before hooks with annotated `params` or `input` still
// compile.
actor({
	actions: {
		params: (c) => c.conn.params,
	},
	createState: (_c, input: { start: number }) => ({ count: input.start }),
	createConnState: (_c, params: { token: string }) => ({
		userId: params.token,
	}),
	onBeforeConnect: (_c, params: { token: string }) => {
		void params;
	},
});

// `tracing` may come before `actions` and name only some of them.
actor({
	tracing: { actions: { recorded: 1 } },
	actions: {
		recorded: (c) => c.conn.state,
		unrecorded: (_c, marker: string) => marker,
	},
});

declare const client: Client<Registry<{ user: typeof definition }>>;

const handle = client.user.getOrCreate(["key"], { params: { token: "t" } });
const whoami: Promise<string> = handle.whoami();
const added: Promise<number> = handle.add(1);
const token: Promise<string> = handle.nested.token();
void whoami;
void added;
void token;

// @ts-expect-error Action arguments keep their declared types.
handle.add("1");

// @ts-expect-error An action must be a function or a group of actions.
actor({ actions: { invalid: 5 } });

// @ts-expect-error `tracing` can only name actions that exist.
actor({ actions: { a: (_c) => 1 }, tracing: { actions: { b: 1 } } });
