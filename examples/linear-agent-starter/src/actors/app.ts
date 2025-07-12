import { setup } from "actor-core";
import { createClient } from "actor-core/client";
import { BASE_PATH, PORT } from "../config";
import { issueAgent } from "./issue-agent";
import { linearAppUser } from "./linear-app-user";
import { oauthSession } from "./oauth-session";

export const app = setup({
	actors: { issueAgent, oauthSession, linearAppUser },
	basePath: BASE_PATH,
});

export type App = typeof app;

export const actorClient = createClient<App>(
	`http://127.0.0.1:${PORT}${BASE_PATH}`,
);
