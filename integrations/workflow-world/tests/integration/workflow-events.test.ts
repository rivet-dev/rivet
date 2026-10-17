import { randomUUID } from "node:crypto";
import { setupTest } from "rivetkit/test";
import { afterEach, describe, expect, test } from "vitest";
import {
	FIRST_EVENT_SLOT,
	SPEC_VERSION_CURRENT,
	eventIdToSlot,
} from "@workflow/world";
import { registry } from "../../src/actors.ts";

const uid = () => randomUUID().slice(0, 8);
type Client = Awaited<ReturnType<typeof setupTest>>["client"];

function runActor(client: Client, runId: string) {
	return client.workflowRun.getOrCreate([runId]);
}

async function createRun(client: Client, runId: string) {
	return runActor(client, runId).appendEvent(runId, {
		eventType: "run_created",
		specVersion: SPEC_VERSION_CURRENT,
		eventData: { deploymentId: "rivet", workflowName: "evt", input: { n: 1 } },
	});
}

describe("workflowRun event dedup + materialization", () => {
	let dispose: (() => Promise<void>) | undefined;
	afterEach(async () => {
		if (dispose) await dispose();
		dispose = undefined;
	});

	test("run_created materializes the run row and is idempotent on redelivery", async (c) => {
		const { client } = await setupTest(c, registry);
		const runId = `wrun_${uid()}`;
		const first = await createRun(client, runId);
		expect(first.run?.runId).toBe(runId);
		expect(first.run?.status).toBe("pending");

		// Redeliver the same creation event: must collapse, not create a second.
		const second = await createRun(client, runId);
		expect(second.run?.runId).toBe(runId);

		const events = await runActor(client, runId).listEvents({
			runId,
			pagination: { limit: 100 },
		});
		const created = events.data.filter((e) => e.eventType === "run_created");
		expect(created).toHaveLength(1);
	});

	test("step_created / wait_created materialize and dedupe by correlationId", async (c) => {
		const { client } = await setupTest(c, registry);
		const runId = `wrun_${uid()}`;
		await createRun(client, runId);

		const stepId = `step-${uid()}`;
		await runActor(client, runId).appendEvent(runId, {
			eventType: "step_created",
			specVersion: SPEC_VERSION_CURRENT,
			correlationId: stepId,
			eventData: { stepName: "s1", input: {} },
		});
		await runActor(client, runId).appendEvent(runId, {
			eventType: "step_created",
			specVersion: SPEC_VERSION_CURRENT,
			correlationId: stepId,
			eventData: { stepName: "s1", input: {} },
		});
		const step = await runActor(client, runId).getStep(runId, stepId, {
			resolveData: "none",
		});
		expect(step.stepId).toBe(stepId);

		const waitCorrelationId = `wait-${uid()}`;
		const waitData = {
			eventType: "wait_created" as const,
			specVersion: SPEC_VERSION_CURRENT,
			correlationId: waitCorrelationId,
			eventData: { resumeAt: new Date(Date.now() + 60_000) },
		};
		const firstWait = await runActor(client, runId).appendEvent(
			runId,
			waitData,
		);
		const secondWait = await runActor(client, runId).appendEvent(
			runId,
			waitData,
		);
		expect(secondWait.wait?.waitId).toBe(firstWait.wait?.waitId);

		const events = await runActor(client, runId).listEvents({
			runId,
			pagination: { limit: 100 },
		});
		expect(
			events.data.filter((e) => e.eventType === "step_created"),
		).toHaveLength(1);
		expect(
			events.data.filter((e) => e.eventType === "wait_created"),
		).toHaveLength(1);
	});

	test("listEventsByCorrelationId advances its cursor", async (c) => {
		const { client } = await setupTest(c, registry);
		const runId = `wrun_${uid()}`;
		await createRun(client, runId);
		const correlationId = `hook-${uid()}`;
		for (let i = 0; i < 3; i++) {
			await runActor(client, runId).appendEvent(runId, {
				eventType: "hook_received",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId,
				eventData: { payload: { i } },
			});
		}

		const first = await runActor(client, runId).listEventsByCorrelationId({
			runId,
			correlationId,
			pagination: { limit: 2 },
		});
		expect(first.data).toHaveLength(2);
		expect(first.hasMore).toBe(true);
		const second = await runActor(client, runId).listEventsByCorrelationId({
			runId,
			correlationId,
			pagination: { limit: 2, cursor: first.cursor ?? undefined },
		});
		expect(second.data).toHaveLength(1);
		expect(second.hasMore).toBe(false);
		expect(second.data[0]?.eventId).not.toBe(first.data[0]?.eventId);
		expect(second.data[0]?.eventId).not.toBe(first.data[1]?.eventId);
	});

	test("actor actions reject keys that do not match their actor", async (c) => {
		const { client } = await setupTest(c, registry);
		const runId = `wrun_${uid()}`;
		const otherRunId = `wrun_${uid()}`;
		await expect(
			runActor(client, runId).appendEvent(otherRunId, {
				eventType: "run_created",
				specVersion: SPEC_VERSION_CURRENT,
				eventData: { deploymentId: "rivet", workflowName: "evt", input: {} },
			}),
		).rejects.toThrow();
		await expect(runActor(client, runId).getRun(otherRunId)).rejects.toThrow();
		await expect(
			client.workflowRun
				.getOrCreate([runId])
				.enqueue(
					otherRunId,
					`msg_${uid()}`,
					"__wkf_workflow_probe",
					"http://127.0.0.1:1",
					JSON.stringify({ runId: otherRunId }),
				),
		).rejects.toThrow();
	});

	test("ownsHook only matches the actor's persisted hook and token", async (c) => {
		const { client } = await setupTest(c, registry);
		const runId = `wrun_${uid()}`;
		await createRun(client, runId);
		const hookId = `hook-${uid()}`;
		const token = `token-${uid()}`;
		await runActor(client, runId).appendEvent(runId, {
			eventType: "hook_created",
			specVersion: SPEC_VERSION_CURRENT,
			correlationId: hookId,
			eventData: { token },
		});
		expect(await runActor(client, runId).ownsHook(hookId, token)).toBe(true);
		expect(await runActor(client, runId).ownsHook(hookId, "wrong-token")).toBe(
			false,
		);
	});

	test("run_started recreates the run when run_created was lost (resilient start)", async (c) => {
		const { client } = await setupTest(c, registry);
		const runId = `wrun_${uid()}`;
		// No run_created first — simulate it being lost. run_started carries the
		// creation fields, so the run must be materialized and marked running.
		const started = await runActor(client, runId).appendEvent(runId, {
			eventType: "run_started",
			specVersion: SPEC_VERSION_CURRENT,
			eventData: {
				deploymentId: "rivet",
				workflowName: "resilient",
				input: { recovered: true },
			},
		});
		expect(started.run?.runId).toBe(runId);
		expect(started.run?.status).toBe("running");

		const run = await runActor(client, runId).getRun(runId);
		expect(run.workflowName).toBe("resilient");
		expect(run.input).toEqual({ recovered: true });
	});

	test("lazy step_started atomically creates the step and its replay event", async (c) => {
		const { client } = await setupTest(c, registry);
		const runId = `wrun_${uid()}`;
		const stepId = `step-${uid()}`;
		await createRun(client, runId);
		const result = await runActor(client, runId).appendEvent(runId, {
			eventType: "step_started",
			specVersion: SPEC_VERSION_CURRENT,
			correlationId: stepId,
			eventData: {
				stepName: "lazy",
				input: { value: 1 },
				ownerMessageId: `msg_${uid()}`,
			},
		});
		expect(result.stepCreated).toBe(true);
		expect(result.step?.status).toBe("running");

		const events = await runActor(client, runId).listEvents({
			runId,
			pagination: { limit: 100 },
		});
		expect(
			events.data
				.filter((event) => event.correlationId === stepId)
				.map((event) => event.eventType),
		).toEqual(["step_created", "step_started"]);
		// A second lazy start lost the create-claim: it must not get the step,
		// or it would run the body again.
		await expect(
			runActor(client, runId).appendEvent(runId, {
				eventType: "step_started",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: stepId,
				eventData: { stepName: "lazy", input: { value: 1 } },
			}),
		).rejects.toThrow(/already created/);
		const replay = await runActor(client, runId).appendEvent(runId, {
			eventType: "step_started",
			specVersion: SPEC_VERSION_CURRENT,
			correlationId: stepId,
		});
		expect(replay.step?.status).toBe("running");
		expect(replay.step?.attempt).toBe(1);

		await expect(
			runActor(client, runId).appendEvent(runId, {
				eventType: "step_started",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: stepId,
				eventData: { stepName: "lazy", input: { value: 2 } },
			}),
		).rejects.toThrow();
	});

	test("rejects child events without a run and after the run terminates", async (c) => {
		const { client } = await setupTest(c, registry);
		const missingRunId = `wrun_${uid()}`;
		await expect(
			runActor(client, missingRunId).appendEvent(missingRunId, {
				eventType: "step_created",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: `step-${uid()}`,
				eventData: { stepName: "orphan", input: {} },
			}),
		).rejects.toThrow();

		const runId = `wrun_${uid()}`;
		await createRun(client, runId);
		await runActor(client, runId).appendEvent(runId, {
			eventType: "run_completed",
			specVersion: SPEC_VERSION_CURRENT,
			eventData: { output: null },
		});
		await expect(
			runActor(client, runId).appendEvent(runId, {
				eventType: "hook_received",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: `hook-${uid()}`,
				eventData: { payload: {} },
			}),
		).rejects.toThrow();
	});

	test("concurrent run_created turns for one run yield exactly one creation event", async (c) => {
		const { client } = await setupTest(c, registry);
		const runId = `wrun_${uid()}`;
		// Two turns racing to create the same run (recovery re-enqueue vs live, or
		// duplicate delivery). The creation-only unique index must collapse them.
		const results = await Promise.all([
			createRun(client, runId),
			createRun(client, runId),
		]);
		for (const r of results) expect(r.run?.runId).toBe(runId);

		const events = await runActor(client, runId).listEvents({
			runId,
			pagination: { limit: 100 },
		});
		expect(
			events.data.filter((e) => e.eventType === "run_created"),
		).toHaveLength(1);
	});

	test("concurrent step_created turns for one correlationId yield exactly one step", async (c) => {
		const { client } = await setupTest(c, registry);
		const runId = `wrun_${uid()}`;
		await createRun(client, runId);
		const stepId = `step-${uid()}`;
		// Two genuinely-concurrent creation turns for the same correlationId is
		// enough to exercise the creation-only unique index race (matches the
		// run_created concurrency test); the SQL-level dedup itself is covered
		// exhaustively by the deterministic unit suite.
		await Promise.all([
			runActor(client, runId).appendEvent(runId, {
				eventType: "step_created",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: stepId,
				eventData: { stepName: "s", input: {} },
			}),
			runActor(client, runId).appendEvent(runId, {
				eventType: "step_created",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: stepId,
				eventData: { stepName: "s", input: {} },
			}),
		]);
		const events = await runActor(client, runId).listEvents({
			runId,
			pagination: { limit: 100 },
		});
		expect(
			events.data.filter((e) => e.eventType === "step_created"),
		).toHaveLength(1);
		const steps = await runActor(client, runId).listSteps({
			runId,
			pagination: { limit: 100 },
		});
		expect(steps.data.filter((s) => s.stepId === stepId)).toHaveLength(1);
	});

	test("event ids are dense slots from 1, even under concurrent appends", async (c) => {
		const { client } = await setupTest(c, registry);
		const runId = `wrun_${uid()}`;
		await createRun(client, runId);
		await Promise.all(
			Array.from({ length: 12 }, (_, i) =>
				runActor(client, runId).appendEvent(runId, {
					eventType: "step_created",
					specVersion: SPEC_VERSION_CURRENT,
					correlationId: `step-${i}-${uid()}`,
					eventData: { stepName: "fanout", input: { i } },
				}),
			),
		);
		const events = await runActor(client, runId).listEvents({
			runId,
			pagination: { limit: 100 },
		});
		const slots = events.data.map((e) => eventIdToSlot(e.eventId));
		expect(slots).toEqual(
			Array.from({ length: 13 }, (_, i) => FIRST_EVENT_SLOT + i),
		);
	});

	test("a write behind the log commits at the next slot and reports the skipped events", async (c) => {
		const { client } = await setupTest(c, registry);
		const runId = `wrun_${uid()}`;
		await createRun(client, runId);
		const stepIds = [`step-a-${uid()}`, `step-b-${uid()}`];
		for (const stepId of stepIds) {
			await runActor(client, runId).appendEvent(runId, {
				eventType: "step_created",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: stepId,
				eventData: { stepName: "s", input: {} },
			});
		}
		// The writer only saw run_created, so it expected slot 2.
		const stale = await runActor(client, runId).appendEvent(
			runId,
			{
				eventType: "wait_created",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: `wait-${uid()}`,
				eventData: { resumeAt: new Date(Date.now() + 60_000) },
			},
			{ eventCount: 1 },
		);
		expect(eventIdToSlot(stale.event?.eventId ?? "")).toBe(4);
		expect(stale.events?.map((e) => e.correlationId)).toEqual(stepIds);
		expect(stale.hasMore).toBe(false);

		// A writer that is current gets no report.
		const current = await runActor(client, runId).appendEvent(
			runId,
			{
				eventType: "step_created",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: `step-c-${uid()}`,
				eventData: { stepName: "s", input: {} },
			},
			{ eventCount: 4 },
		);
		expect(eventIdToSlot(current.event?.eventId ?? "")).toBe(5);
		expect(current.events).toBeUndefined();
	});

	test("a finished run accepts a running step's outcome but refuses new starts", async (c) => {
		const { client } = await setupTest(c, registry);
		const runId = `wrun_${uid()}`;
		await createRun(client, runId);
		const running = `step-running-${uid()}`;
		const pending = `step-pending-${uid()}`;
		for (const stepId of [running, pending]) {
			await runActor(client, runId).appendEvent(runId, {
				eventType: "step_created",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: stepId,
				eventData: { stepName: "s", input: {} },
			});
		}
		await runActor(client, runId).appendEvent(runId, {
			eventType: "step_started",
			specVersion: SPEC_VERSION_CURRENT,
			correlationId: running,
		});
		await runActor(client, runId).appendEvent(runId, {
			eventType: "run_cancelled",
			specVersion: SPEC_VERSION_CURRENT,
		});

		// Redelivered start of the step that was already running.
		await expect(
			runActor(client, runId).appendEvent(runId, {
				eventType: "step_started",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: running,
			}),
		).rejects.toThrow(/terminal state/);
		await expect(
			runActor(client, runId).appendEvent(runId, {
				eventType: "step_started",
				specVersion: SPEC_VERSION_CURRENT,
				correlationId: pending,
			}),
		).rejects.toThrow(/terminal state/);

		const completed = await runActor(client, runId).appendEvent(runId, {
			eventType: "step_completed",
			specVersion: SPEC_VERSION_CURRENT,
			correlationId: running,
			eventData: { result: { ok: true } },
		});
		expect(completed.step?.status).toBe("completed");
	});
});
