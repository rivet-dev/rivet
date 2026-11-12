import { actor, UserError } from "rivetkit";

interface ConnParams {
	authToken: string;
}

interface Caller {
	userId: string;
	role: string;
}

interface TokenCache {
	[token: string]: {
		userId: string;
		role: string;
		expiresAt: number;
	};
}

// Example token validation function
async function validateToken(
	token: string,
): Promise<{ sub: string; role: string } | null> {
	// In production, verify JWT or call auth service
	if (token.length > 0) {
		return { sub: "user-123", role: "member" };
	}
	return null;
}

// Validate a token, reusing a recent result from the actor's cache.
async function identify(
	cache: TokenCache,
	params: ConnParams,
): Promise<Caller> {
	const token = params.authToken;

	// Check cache first
	const cached = cache[token];
	if (cached && cached.expiresAt > Date.now()) {
		return { userId: cached.userId, role: cached.role };
	}

	// Validate token (expensive operation)
	const payload = await validateToken(token);
	if (!payload) {
		throw new UserError("Invalid token", { code: "invalid_token" });
	}

	// Cache the result
	cache[token] = {
		userId: payload.sub,
		role: payload.role,
		expiresAt: Date.now() + 5 * 60 * 1000, // 5 minutes
	};

	return { userId: payload.sub, role: payload.role };
}

const cachedAuthActor = actor({
	createVars: () => ({
		tokenCache: {} as TokenCache,
		// The caller for each live connection, keyed by connection ID
		callers: new Map<string, Caller>(),
	}),

	// Validates the token and fills the cache
	onBeforeConnect: async (c, params: ConnParams) => {
		await identify(c.vars.tokenCache, params);
	},

	// Reads the cached result instead of validating again
	onConnect: async (c, conn) => {
		c.vars.callers.set(conn.id, await identify(c.vars.tokenCache, conn.params));
	},

	onDisconnect: (c, conn) => {
		c.vars.callers.delete(conn.id);
	},

	actions: {
		getData: (c) => ({ userId: c.vars.callers.get(c.conn.id)?.userId }),
	},
});
