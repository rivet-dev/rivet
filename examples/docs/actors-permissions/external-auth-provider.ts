import { actor, UserError } from "rivetkit";

interface ConnParams {
	apiKey: string;
}

interface Caller {
	userId: string;
	tier: string;
}

// Validate the API key with the auth provider, or reject the connection.
async function identify(params: ConnParams): Promise<Caller> {
	const response = await fetch(
		`https://api.my-auth-provider.com/validate`,
		{
			method: "POST",
			headers: { "X-API-Key": params.apiKey },
		},
	);

	if (!response.ok) {
		throw new UserError("Invalid API key", { code: "invalid_api_key" });
	}

	const data = await response.json();
	return { userId: data.id, tier: data.tier };
}

const apiActor = actor({
	// The caller for each live connection, keyed by connection ID
	createVars: () => ({ callers: new Map<string, Caller>() }),

	onBeforeConnect: async (_c, params: ConnParams) => {
		await identify(params);
	},

	onConnect: async (c, conn) => {
		c.vars.callers.set(conn.id, await identify(conn.params));
	},

	onDisconnect: (c, conn) => {
		c.vars.callers.delete(conn.id);
	},

	actions: {
		premiumAction: (c) => {
			if (c.vars.callers.get(c.conn.id)?.tier !== "premium") {
				throw new UserError("Premium subscription required", {
					code: "forbidden",
				});
			}
			return "Premium content";
		},
	},
});
