import { actor, UserError } from "rivetkit";

const ROLE_HIERARCHY = { user: 1, moderator: 2, admin: 3 };

interface ConnState {
  role: keyof typeof ROLE_HIERARCHY;
  permissions: string[];
}

// Example token validation function
async function validateToken(token: string): Promise<{ role: keyof typeof ROLE_HIERARCHY; permissions: string[] }> {
  // In production, verify JWT or call auth service
  return { role: "user", permissions: ["read", "edit_posts"] };
}

function requireRole(requiredRole: keyof typeof ROLE_HIERARCHY) {
  return (c: { conn: { state: ConnState } }) => {
    const userRole = c.conn.state.role;
    if (ROLE_HIERARCHY[userRole] < ROLE_HIERARCHY[requiredRole]) {
      throw new UserError(`${requiredRole} role required`, { code: "forbidden" });
    }
  };
}

function requirePermission(permission: string) {
  return (c: { conn: { state: ConnState } }) => {
    if (!c.conn.state.permissions?.includes(permission)) {
      throw new UserError(`Permission '${permission}' required`, { code: "forbidden" });
    }
  };
}

const forumActor = actor({
  state: {},

  createConnState: async (c, params: { token: string }): Promise<ConnState> => {
    const user = await validateToken(params.token);
    return { role: user.role, permissions: user.permissions };
  },

  actions: {
    deletePost: (c, postId: string) => {
      requireRole("moderator")(c);
      // Delete post...
    },

    editPost: (c, postId: string, content: string) => {
      requirePermission("edit_posts")(c);
      // Edit post...
    },
  },
});
