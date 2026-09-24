import { mkdirSync } from "node:fs";
import { DatabaseSync } from "node:sqlite";
import { betterAuth } from "better-auth";
import { getMigrations } from "better-auth/db/migration";
import { demoAccount } from "../demo-account.ts";

if (!process.env.BETTER_AUTH_SECRET || !process.env.BETTER_AUTH_URL) {
	throw new Error("Set BETTER_AUTH_SECRET and BETTER_AUTH_URL");
}

mkdirSync(".data", { recursive: true });
const database = new DatabaseSync(".data/auth.sqlite");

export const auth = betterAuth({
	database,
	secret: process.env.BETTER_AUTH_SECRET,
	baseURL: process.env.BETTER_AUTH_URL,
	emailAndPassword: { enabled: true },
});

// Create the local schema and demo account once. Better Auth handles passwords and sessions.
const { runMigrations } = await getMigrations(auth.options);
await runMigrations();
if (
	!database
		.prepare('SELECT id FROM "user" WHERE email = ?')
		.get(demoAccount.email)
) {
	await auth.api.signUpEmail({ body: { ...demoAccount, name: "Demo user" } });
}
