import { afterAll, describe, expect, it } from "vitest";
import {
	getOrStartSharedTestEngine,
	releaseSharedTestEngine,
} from "./shared-engine";

describe("metrics endpoint", () => {
	afterAll(async () => {
		await releaseSharedTestEngine();
	});

	it("serves Prometheus metrics for the shared engine", async () => {
		const engine = await getOrStartSharedTestEngine();
		const response = await fetch(engine.metricsEndpoint);
		const body = await response.text();

		expect(response.status).toBe(200);
		expect(response.headers.get("content-type")).toContain("text/plain");
		expect(body).toMatch(/^# HELP rivet_tokio_thread_count /m);
		expect(body).toMatch(/^# TYPE rivet_tokio_thread_count gauge$/m);
		expect(body).toMatch(/^rivet_tokio_thread_count \d+$/m);
		expect(body).toMatch(/^# HELP rivet_tokio_task_total /m);
		expect(body).toMatch(/^# TYPE rivet_tokio_task_total counter$/m);
	});
});
