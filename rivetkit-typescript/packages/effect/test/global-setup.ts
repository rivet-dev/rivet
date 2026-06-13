import type { TestProject } from "vitest/node";
import {
	getOrStartSharedTestEngine,
	releaseSharedTestEngine,
	TEST_ENGINE_TOKEN,
} from "./shared-engine";

declare module "vitest" {
	export interface ProvidedContext {
		rivetEngine: {
			endpoint: string;
			token: string;
		};
	}
}

/**
 * Spawns a single rivet-engine for the test run on random ports
 * with an isolated tmpdir-backed db, then exposes its endpoint to
 * test workers via vitest's `provide`/`inject`. The engine outlives
 * a single test file but never two test runs: `globalTeardown`
 * releases the refcount in `shared-engine.ts`, which kills the
 * process and wipes its dbRoot.
 *
 * Each test file should create its own namespace + runner config
 * against this endpoint so envoy registrations from one file can't
 * pollute another.
 */
export default async function setup({ provide }: TestProject) {
	// `test.env` in vitest.config only applies to test workers, not the
	// main vitest process where this setup spawns the engine. Mirror it
	// here so the engine inherits a quiet log level.
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
