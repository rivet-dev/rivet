import { actor, UserError } from "rivetkit";

interface ConnParams {
	token: string;
}

interface Caller {
	userId: string;
	role: string;
	permissions: string[];
}

interface JwtPayload {
	sub: string;
	role: string;
	permissions?: string[];
}

// Supply this from your auth provider's SDK or a JWT library such as `jose`.
// It must verify the signature and check the issuer, audience, and expiry.
// Decoding the payload without verifying the signature authenticates nobody:
// any client can forge a token.
declare function verifyAccessToken(token: string): Promise<JwtPayload>;

// Verify the token and resolve the caller, or reject the connection.
async function identify(params: ConnParams): Promise<Caller> {
	let payload: JwtPayload;
	try {
		payload = await verifyAccessToken(params.token);
	} catch {
		throw new UserError("Invalid or expired token", {
			code: "invalid_token",
		});
	}

	return {
		userId: payload.sub,
		role: payload.role,
		permissions: payload.permissions ?? [],
	};
}

const jwtActor = actor({
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
		protectedAction: (c) => {
			const caller = c.vars.callers.get(c.conn.id);
			if (!caller?.permissions.includes("write")) {
				throw new UserError("Write permission required", {
					code: "forbidden",
				});
			}
			return { success: true };
		},
	},
});
