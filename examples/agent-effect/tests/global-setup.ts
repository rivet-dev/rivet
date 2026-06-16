import type { TestProject } from "vitest/node";
import {
	getOrStartSharedTestEngine,
	releaseSharedTestEngine,
	TEST_ENGINE_TOKEN,
} from "./shared-engine.ts";

declare module "vitest" {
	export interface ProvidedContext {
		rivetEngine: {
			endpoint: string;
			token: string;
		};
	}
}

// Spawns a single rivet-engine for the whole test run on random ports with an
// isolated tmpdir-backed db, then exposes its endpoint to test workers via
// vitest's `provide`/`inject`. Released (and killed) in teardown.
export default async function setup({ provide }: TestProject) {
	process.env.RIVET_LOG_LEVEL ??= "SILENT";

	const engine = await getOrStartSharedTestEngine();
	provide("rivetEngine", {
		endpoint: engine.endpoint,
		token: TEST_ENGINE_TOKEN,
	});
	return async () => {
		await releaseSharedTestEngine();
	};
}
