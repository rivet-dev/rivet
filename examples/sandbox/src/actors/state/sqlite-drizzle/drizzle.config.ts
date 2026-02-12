import { defineConfig } from "rivetkit/db/drizzle";

export default defineConfig({
	out: "./src/actors/state/sqlite-drizzle/drizzle",
	schema: "./src/actors/state/sqlite-drizzle/schema.ts",
});
