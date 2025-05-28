import { setup } from "actor-core";
import { oauthSession } from "./oauth-session";
import { issueAgent } from "./issue-agent";
import { linearAppUser } from "./linear-app-user";
import { createClient } from "actor-core/client";
import { PORT, BASE_PATH } from "../config";

export const app = setup({
	actors: { issueAgent, oauthSession, linearAppUser },
	basePath: BASE_PATH,
});

export type App = typeof app;

export const actorClient = createClient<App>(
	`http://127.0.0.1:${PORT}${BASE_PATH}`,
);
