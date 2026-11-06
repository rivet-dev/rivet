import { actor, setup, UserError } from "rivetkit";
import { createClient } from "rivetkit/client";

interface ConnParams {
	token: string;
}

async function verifyToken(token: string): Promise<boolean> {
	return token.length > 0;
}

// docs:start actor
// The token check your Worker ran before calling the stub moves into the actor
export const document = actor({
	// Runs before every connection and every one-off action call
	onBeforeConnect: async (_c, params: ConnParams | null) => {
		if (!params?.token || !(await verifyToken(params.token))) {
			throw new UserError("unauthorized", { code: "unauthorized" });
		}
	},
	actions: {
		read: (c) => `contents of ${c.key[0]}`,
	},
});
// docs:end actor

export const registry = setup({ use: { document } });

// docs:start client
const client = createClient<typeof registry>("http://localhost:6420");
const doc = client.document.getOrCreate(["doc-1"], {
	params: { token: "user-token" },
});
console.log(await doc.read());
// docs:end client
