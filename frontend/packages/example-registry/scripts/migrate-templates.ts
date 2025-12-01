import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Template metadata from website/src/data/templates/shared.ts
const templateMetadata: Record<string, { technologies: string[]; tags: string[] }> = {
	"ai-agent": {
		technologies: ["rivet", "typescript"],
		tags: ["ai", "real-time"],
	},
	"background-jobs": {
		technologies: ["rivet", "typescript"],
		tags: [],
	},
	"better-auth-external-db": {
		technologies: ["rivet", "typescript"],
		tags: ["database"],
	},
	"bots": {
		technologies: ["rivet", "typescript"],
		tags: ["ai"],
	},
	"chat-room": {
		technologies: ["rivet", "react", "typescript"],
		tags: ["real-time"],
	},
	"cloudflare-workers": {
		technologies: ["rivet", "cloudflare-workers", "typescript"],
		tags: [],
	},
	"cloudflare-workers-hono": {
		technologies: ["rivet", "cloudflare-workers", "hono", "typescript"],
		tags: [],
	},
	"cloudflare-workers-inline-client": {
		technologies: ["rivet", "cloudflare-workers", "typescript"],
		tags: [],
	},
	"counter": {
		technologies: ["rivet", "typescript"],
		tags: ["quickstart"],
	},
	"counter-next-js": {
		technologies: ["rivet", "next-js", "react", "typescript"],
		tags: ["real-time"],
	},
	"counter-serverless": {
		technologies: ["rivet", "typescript"],
		tags: [],
	},
	"crdt": {
		technologies: ["rivet", "typescript"],
		tags: ["real-time"],
	},
	"cursors": {
		technologies: ["rivet", "react", "typescript"],
		tags: ["real-time"],
	},
	"cursors-raw-websocket": {
		technologies: ["rivet", "websocket", "typescript"],
		tags: ["real-time"],
	},
	"database": {
		technologies: ["rivet", "typescript"],
		tags: ["database"],
	},
	"deno": {
		technologies: ["rivet", "deno", "typescript"],
		tags: [],
	},
	"drizzle": {
		technologies: ["rivet", "drizzle", "typescript"],
		tags: ["database"],
	},
	"elysia": {
		technologies: ["rivet", "elysia", "bun", "typescript"],
		tags: [],
	},
	"express": {
		technologies: ["rivet", "express", "typescript"],
		tags: [],
	},
	"game": {
		technologies: ["rivet", "react", "typescript"],
		tags: ["gaming", "real-time"],
	},
	"hono": {
		technologies: ["rivet", "hono", "typescript"],
		tags: [],
	},
	"hono-bun": {
		technologies: ["rivet", "hono", "bun", "typescript"],
		tags: [],
	},
	"hono-react": {
		technologies: ["rivet", "hono", "react", "typescript"],
		tags: ["real-time"],
	},
	"kitchen-sink": {
		technologies: ["rivet", "typescript"],
		tags: [],
	},
	"next-js": {
		technologies: ["rivet", "next-js", "react", "typescript"],
		tags: [],
	},
	"quickstart-actions": {
		technologies: ["rivet", "typescript"],
		tags: ["quickstart"],
	},
	"quickstart-cross-actor-actions": {
		technologies: ["rivet", "typescript"],
		tags: ["quickstart"],
	},
	"quickstart-multi-region": {
		technologies: ["rivet", "typescript"],
		tags: ["quickstart"],
	},
	"quickstart-native-websockets": {
		technologies: ["rivet", "websocket", "typescript"],
		tags: ["quickstart", "real-time"],
	},
	"quickstart-realtime": {
		technologies: ["rivet", "typescript"],
		tags: ["quickstart", "real-time"],
	},
	"quickstart-scheduling": {
		technologies: ["rivet", "typescript"],
		tags: ["quickstart"],
	},
	"quickstart-state": {
		technologies: ["rivet", "typescript"],
		tags: ["quickstart"],
	},
	"rate": {
		technologies: ["rivet", "typescript"],
		tags: [],
	},
	"raw-fetch-handler": {
		technologies: ["rivet", "typescript"],
		tags: [],
	},
	"raw-websocket-handler": {
		technologies: ["rivet", "websocket", "typescript"],
		tags: ["real-time"],
	},
	"raw-websocket-handler-proxy": {
		technologies: ["rivet", "websocket", "typescript"],
		tags: [],
	},
	"react": {
		technologies: ["rivet", "react", "typescript"],
		tags: [],
	},
	"smoke-test": {
		technologies: ["rivet", "typescript"],
		tags: [],
	},
	"starter": {
		technologies: ["rivet", "typescript"],
		tags: ["quickstart"],
	},
	"stream": {
		technologies: ["rivet", "typescript"],
		tags: ["real-time"],
	},
	"sync": {
		technologies: ["rivet", "typescript"],
		tags: [],
	},
	"tenant": {
		technologies: ["rivet", "typescript"],
		tags: [],
	},
	"trpc": {
		technologies: ["rivet", "trpc", "typescript"],
		tags: [],
	},
	"user-generated-actors-freestyle": {
		technologies: ["rivet", "typescript"],
		tags: [],
	},
	"workflows": {
		technologies: ["rivet", "typescript"],
		tags: [],
	},
};

async function main() {
	const examplesDir = path.join(__dirname, "../../../../examples");

	for (const [exampleName, metadata] of Object.entries(templateMetadata)) {
		const packageJsonPath = path.join(examplesDir, exampleName, "package.json");

		try {
			// Read existing package.json
			const content = await fs.readFile(packageJsonPath, "utf-8");
			const packageJson = JSON.parse(content);

			// Add template metadata
			packageJson.template = metadata;

			// Write back with pretty formatting
			await fs.writeFile(
				packageJsonPath,
				JSON.stringify(packageJson, null, "\t") + "\n",
				"utf-8",
			);

			console.log(`✅ Updated ${exampleName}/package.json`);
		} catch (error) {
			console.warn(`⚠️  Could not update ${exampleName}:`, error);
		}
	}

	console.log("\n✅ Migration complete!");
}

main().catch((error) => {
	console.error("Migration failed:", error);
	process.exit(1);
});
