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

actor({
	// @ts-expect-error An action must be a function or a group of actions.
	actions: { invalid: 5 },
});
