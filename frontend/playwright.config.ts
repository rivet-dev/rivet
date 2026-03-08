import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, devices } from "@playwright/test";
import dotenv from "dotenv";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Load environment variables
// dotenv.config({ path: path.resolve(__dirname, ".env") });
dotenv.config({ path: path.resolve(__dirname, ".env.local") });

export default defineConfig({
	testDir: "./e2e",
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	workers: process.env.CI ? 1 : undefined,
	reporter: "html",
	use: {
		baseURL: "http://localhost:43710",
		trace: "on-first-retry",
	},
	globalSetup: "./e2e/global.setup.ts",
	projects: [
		{
			name: "cloud:setup",
			testMatch: /auth\.setup\.ts/,
		},
		{
			name: "cloud",
			use: {
				...devices["Desktop Chrome"],
				storageState: ".auth/cloud/user.json",
			},
			dependencies: ["cloud:setup"],
			testDir: "./e2e/cloud",
		},
		{
			name: "engine",
			use: {
				...devices["Desktop Chrome"],
			},
			testDir: "./e2e/engine",
		},
	],
	webServer: [
		{
			name: "Cloud",
			command: "pnpm dev:cloud",
			url: "http://localhost:43710",
			reuseExistingServer: !process.env.CI,
		},
		{
			name: "Engine",
			command: "pnpm dev:engine",
			url: "http://localhost:43708",
			reuseExistingServer: !process.env.CI,
		},
	],
});
