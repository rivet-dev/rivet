import { actor, UserError } from "rivetkit";

const ROLE_HIERARCHY = { user: 1, moderator: 2, admin: 3 };

interface Caller {
	role: keyof typeof ROLE_HIERARCHY;
	permissions: string[];
}

// Example token validation function
async function validateToken(
	token: string,
): Promise<{ role: keyof typeof ROLE_HIERARCHY; permissions: string[] }> {
	// In production, verify JWT or call auth service
	return { role: "user", permissions: ["read", "edit_posts"] };
}

function requireRole(requiredRole: keyof typeof ROLE_HIERARCHY) {
	return (caller: Caller | undefined) => {
		if (!caller || ROLE_HIERARCHY[caller.role] < ROLE_HIERARCHY[requiredRole]) {
			throw new UserError(`${requiredRole} role required`, {
				code: "forbidden",
			});
		}
	};
}

function requirePermission(permission: string) {
	return (caller: Caller | undefined) => {
		if (!caller?.permissions.includes(permission)) {
			throw new UserError(`Permission '${permission}' required`, {
				code: "forbidden",
			});
		}
	};
}

const forumActor = actor({
	// The caller for each live connection, keyed by connection ID
	createVars: () => ({ callers: new Map<string, Caller>() }),

	onBeforeConnect: async (c, params: { token: string }) => {
		await validateToken(params.token);
	},

	onConnect: async (c, conn) => {
		const user = await validateToken(conn.params.token);
		c.vars.callers.set(conn.id, { role: user.role, permissions: user.permissions });
	},

	onDisconnect: (c, conn) => {
		c.vars.callers.delete(conn.id);
	},

	actions: {
		deletePost: (c, postId: string) => {
			requireRole("moderator")(c.vars.callers.get(c.conn.id));
			// Delete post...
		},

		editPost: (c, postId: string, content: string) => {
			requirePermission("edit_posts")(c.vars.callers.get(c.conn.id));
			// Edit post...
		},
	},
});
