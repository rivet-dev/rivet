import * as dotenv from "dotenv";
dotenv.config({ path: ".env.local" });

import invariant from "invariant";

// Linear
export const LINEAR_WEBHOOK_SECRET = process.env.LINEAR_WEBHOOK_SECRET!;
invariant(LINEAR_WEBHOOK_SECRET, "missing LINEAR_WEBHOOK_SECRET");

export const LINEAR_OAUTH_CLIENT_ID = process.env.LINEAR_OAUTH_CLIENT_ID!;
invariant(LINEAR_OAUTH_CLIENT_ID, "missing LINEAR_OAUTH_CLIENT_ID");

export const LINEAR_OAUTH_CLIENT_AUTHENTICATION =
	process.env.LINEAR_OAUTH_CLIENT_AUTHENTICATION!;
invariant(
	LINEAR_OAUTH_CLIENT_AUTHENTICATION,
	"missing LINEAR_OAUTH_CLIENT_AUTHENTICATION",
);

export const LINEAR_OAUTH_REDIRECT_URI = process.env.LINEAR_OAUTH_REDIRECT_URI!;
invariant(LINEAR_OAUTH_REDIRECT_URI, "missing LINEAR_OAUTH_REDIRECT_URI");
console.log("Redirect URI:", LINEAR_OAUTH_REDIRECT_URI);

// Server
export const PORT = process.env.PORT ? Number.parseInt(process.env.PORT) : 5050;

// ActorCore
export const BASE_PATH = "/actors";
