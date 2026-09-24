import { actor, setup, UserError } from "rivetkit";

interface ConnParams {
	authToken: string;
}

interface ConnState {
	userId: string;
	role: "member" | "admin";
}

// Replace this with your session store or auth provider.
async function verifySession(authToken: string): Promise<ConnState | null> {
	if (authToken === "admin-token") return { userId: "u_1", role: "admin" };
	if (authToken === "member-token") return { userId: "u_2", role: "member" };
	return null;
}

export const document = actor({
	state: { body: "" },

	// 1. Identify the caller once, at connect time.
	createConnState: async (_c, params: ConnParams): Promise<ConnState> => {
		const session = await verifySession(params.authToken);
		if (!session) {
			throw new UserError("Invalid token", { code: "invalid_token" });
		}
		return session;
	},

	actions: {
		read: (c) => c.state.body,

		// 2. Gate the operation on the identity you established.
		edit: (c, body: string) => {
			if (c.conn.state.role !== "admin") {
				throw new UserError("Admins only", { code: "forbidden" });
			}
			c.state.body = body;
		},
	},
});

export const registry = setup({ use: { document } });
