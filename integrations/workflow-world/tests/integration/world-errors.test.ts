import { randomUUID } from "node:crypto";
import {
	EntityConflictError,
	RunExpiredError,
	WorkflowRunNotFoundError,
} from "@workflow/errors";
import { SPEC_VERSION_CURRENT } from "@workflow/world";
import { setupTest } from "rivetkit/test";
import { afterEach, describe, expect, test } from "vitest";
import { registry } from "../../src/actors.ts";
import { RivetClientWorld } from "../../src/index.ts";

const uid = () => randomUUID().slice(0, 8);

async function rejection(promise: Promise<unknown>) {
	try {
		await promise;
	} catch (error) {
		return error;
	}
	throw new Error("expected the call to reject");
}

describe("World errors reach the runtime with their Workflow SDK type", () => {
	let world: RivetClientWorld | undefined;
	afterEach(async () => {
		if (world) await world.close();
		world = undefined;
	});

	test("conflicting and late starts reject with errors the runtime can classify", async (c) => {
		const { client } = await setupTest(c, registry);
		world = new RivetClientWorld({ client, runtimeUrl: "http://127.0.0.1:1" });
		const created = await world.events.create(null, {
			eventType: "run_created",
			specVersion: SPEC_VERSION_CURRENT,
			eventData: { deploymentId: "rivet", workflowName: "errors", input: {} },
		});
		const runId = created.run?.runId;
		if (!runId) throw new Error("expected run id");
		await world.events.create(runId, {
			eventType: "run_started",
			specVersion: SPEC_VERSION_CURRENT,
		});

		const lazyStart = {
			eventType: "step_started",
			specVersion: SPEC_VERSION_CURRENT,
			correlationId: `step-${uid()}`,
			eventData: { stepName: "lazy", input: {} },
		} as const;
		await world.events.create(runId, lazyStart);
		const losingStart = await rejection(world.events.create(runId, lazyStart));

		const pending = `step-${uid()}`;
		await world.events.create(runId, {
			eventType: "step_created",
			specVersion: SPEC_VERSION_CURRENT,
			correlationId: pending,
			eventData: { stepName: "s", input: {} },
		});
		await world.events.create(runId, {
			eventType: "run_cancelled",
			specVersion: SPEC_VERSION_CURRENT,
		});
		const lateStepStart = await rejection(
			world.events.create(runId, {
				eventType: "step_started",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: pending,
			}),
		);
		const lateRunStart = await rejection(
			world.events.create(runId, {
				eventType: "run_started",
				specVersion: SPEC_VERSION_CURRENT,
			}),
		);

		expect(EntityConflictError.is(losingStart)).toBe(true);
		expect(RunExpiredError.is(lateStepStart)).toBe(true);
		expect(RunExpiredError.is(lateRunStart)).toBe(true);
	});

	test("reading an unknown run rejects with WorkflowRunNotFoundError", async (c) => {
		const { client } = await setupTest(c, registry);
		world = new RivetClientWorld({ client, runtimeUrl: "http://127.0.0.1:1" });
		const missing = await rejection(world.runs.get(`wrun_${uid()}`));
		expect(WorkflowRunNotFoundError.is(missing)).toBe(true);
	});
});
