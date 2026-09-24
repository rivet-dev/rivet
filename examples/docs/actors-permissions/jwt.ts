import { actor, UserError } from "rivetkit";

interface ConnParams {
	token: string;
}

interface ConnState {
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

const jwtActor = actor({
	state: {},

	createConnState: async (c, params: ConnParams): Promise<ConnState> => {
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
	},

	actions: {
		protectedAction: (c) => {
			if (!c.conn.state.permissions.includes("write")) {
				throw new UserError("Write permission required", {
					code: "forbidden",
				});
			}
			return { success: true };
		},
	},
});
