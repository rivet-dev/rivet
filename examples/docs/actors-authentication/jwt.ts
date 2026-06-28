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

// Example JWT verification function - in production use a JWT library
function verifyJwt(token: string, secret: string): JwtPayload {
  // This is a simplified example - use jsonwebtoken or similar in production
  const parts = token.split(".");
  if (parts.length !== 3) throw new Error("Invalid token");
  const payload = JSON.parse(atob(parts[1])) as JwtPayload;
  return payload;
}

const jwtActor = actor({
  state: {},

  createConnState: (c, params: ConnParams): ConnState => {
    try {
      const payload = verifyJwt(params.token, process.env.JWT_SECRET || "secret");
      return {
        userId: payload.sub,
        role: payload.role,
        permissions: payload.permissions || [],
      };
    } catch {
      throw new UserError("Invalid or expired token", { code: "invalid_token" });
    }
  },

  actions: {
    protectedAction: (c) => {
      if (!c.conn.state.permissions.includes("write")) {
        throw new UserError("Write permission required", { code: "forbidden" });
      }
      return { success: true };
    },
  },
});
