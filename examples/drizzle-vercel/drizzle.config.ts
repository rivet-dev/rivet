import { defineConfig } from "@rivetkit/db/drizzle";

export default defineConfig({
	out: "./drizzle",
	schema: "./src/db/schema.ts",
});
