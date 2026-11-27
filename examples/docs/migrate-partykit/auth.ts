import { actor, setup, UserError } from "rivetkit";
import { createClient } from "rivetkit/client";

interface ConnParams {
	token: string;
}

async function verifyToken(token: string): Promise<{ userId: string } | null> {
	// Verify a JWT or call your auth provider here
	return token.length > 0 ? { userId: "user-123" } : null;
}

// docs:start server
export const privateRoom = actor({
	createVars: () => ({ users: new Map<string, string>() }),

	// Replaces static onBeforeConnect(request, lobby). Throw to reject.
	onBeforeConnect: async (_c, params: ConnParams) => {
		const user = await verifyToken(params?.token ?? "");
		if (!user) {
			throw new UserError("Unauthorized", { code: "unauthorized" });
		}
	},

	// Runs after onBeforeConnect accepts the connection
	onConnect: async (c, conn) => {
		const user = await verifyToken(conn.params.token);
		if (user) c.vars.users.set(conn.id, user.userId);
	},

	onDisconnect: (c, conn) => {
		c.vars.users.delete(conn.id);
	},

	actions: {
		whoAmI: (c) => c.vars.users.get(c.conn.id),
	},
});
// docs:end server

export const registry = setup({ use: { privateRoom } });

// docs:start client
const client = createClient<typeof registry>("http://localhost:6420");

// Replaces new PartySocket({ ..., query: { token } })
const room = client.privateRoom.getOrCreate(["team-42"], {
	params: { token: "jwt-from-your-auth-provider" },
});
const conn = room.connect();
console.log(await conn.whoAmI());
// docs:end client
