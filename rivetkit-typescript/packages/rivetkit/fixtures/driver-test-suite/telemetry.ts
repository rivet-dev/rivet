import { trace } from "@opentelemetry/api";
import { NodeTracerProvider } from "@opentelemetry/sdk-trace-node";
import { actor, queue, UserError } from "rivetkit";
import { db } from "@/common/database/mod";
import { workflow } from "@/workflow/mod";

// Only a traced runtime gets a JavaScript tracer, so the other driver
// fixtures keep running without an OpenTelemetry context manager.
if (process.env.OTEL_EXPORTER_OTLP_TRACES_ENDPOINT) {
	new NodeTracerProvider().register();
}
const applicationTracer = trace.getTracer("driver-telemetry-fixture");

const jobSchema = queue<{ id: string }>();

export const telemetryActor = actor({
	state: { count: 0 },
	db: db(),
	queues: {
		jobs: jobSchema,
	},
	onRequest: async (c, request) => {
		await c.db.execute("SELECT ? AS path", new URL(request.url).pathname);
		return new Response("ok", { status: 200 });
	},
	actions: {
		getCount: (c) => c.state.count,
		increment: (c, amount: number) => {
			c.state.count += amount;
			return c.state.count;
		},
		sqliteFailure: async (c) => {
			await c.db.execute("SELECT value FROM missing_trace_test_table");
		},
		stateTransaction: async (c, amount: number) => {
			await c.db.transaction(
				async (tx) => {
					await tx.execute("SELECT 1");
					c.state.count += amount;
				},
				{ experimental: { includeState: true } },
			);
			return c.state.count;
		},
		insertAfterReply: (c, token: string) => {
			c.waitUntil(
				c.queue
					.next({ names: ["jobs"], timeout: 10_000 })
					.then((message) => {
						if (!message)
							throw new Error("deferred work was not released");
						return c.db.execute("SELECT ? AS deferred", token);
					}),
			);
			return "replied";
		},
		scheduleTrace: async (c, correlationToken: string) => {
			await c.schedule.after(50, "scheduledTrace", correlationToken);
			return correlationToken;
		},
		scheduledTrace: async (c, correlationToken: string) => {
			await c.db.execute("SELECT ? AS trace", correlationToken);
		},
		consumeJob: async (c) => {
			const message = await c.queue.next({
				names: ["jobs"],
				timeout: 5_000,
			});
			return message?.body ?? null;
		},
		getCountViaClient: async (c) => {
			const client = c.client<any>();
			return await client.telemetryActor.getForId(c.actorId).getCount();
		},
		isolationProbe: async (c, token: string, fail: boolean) => {
			c.log.warn({ correlation_token: token }, "isolation probe");
			if (!(await c.queue.next({ names: ["jobs"], timeout: 10_000 }))) {
				throw new Error("isolation probe was not released");
			}
			await c.db.execute("SELECT ? AS probe", token);
			const client = c.client<any>();
			await client.telemetryActor.getForId(c.actorId).getCount();
			await c.db.execute("SELECT ? AS probe2", token);
			if (fail) {
				throw new UserError("isolation probe failure", {
					code: "isolation_probe_failed",
				});
			}
			return token;
		},
		getCountUnderApplicationSpan: async (c) => {
			return await applicationTracer.startActiveSpan(
				"agent.generate",
				async (span) => {
					try {
						await c.db.execute("SELECT 1 AS under_span");
						const client = c.client<any>();
						const count = await client.telemetryActor
							.getForId(c.actorId)
							.getCount();
						return { count, spanId: span.spanContext().spanId };
					} finally {
						span.end();
					}
				},
			);
		},
	},
});

export const telemetryRunConsumerActor = actor({
	state: {},
	queues: {
		runJobs: jobSchema,
	},
	run: async (c) => {
		while (!c.aborted) {
			await c.queue.waitForNames(["runJobs"], {
				signal: c.abortSignal,
			});
		}
	},
	actions: {},
});

/** Waits for two messages and logs when it sleeps, so a test can send the second one after the sleep. */
export const workflowTracedActor = actor({
	state: { chargeAttempts: 0, wakes: 0 },
	db: db(),
	onWake: (c) => {
		c.state.wakes += 1;
	},
	queues: {
		approve: jobSchema,
		resume: jobSchema,
	},
	onSleep: (c) => {
		c.log.warn(
			{ slept_actor_key: c.key[0] },
			"workflow traced actor slept",
		);
	},
	run: workflow(async (ctx) => {
		await ctx.queue.next("wait-approve", { names: ["approve"] });
		await ctx.step("reserve-stock", async (c) => {
			c.log.warn({ workflow_log_key: c.key[0] }, "reserving stock");
			await c.db.execute("SELECT 'reserve-stock' AS step");
		});
		await ctx.queue.next("wait-resume", { names: ["resume"] });
		await ctx.step({
			name: "charge-card",
			maxRetries: 3,
			retryBackoffBase: 10,
			retryBackoffMax: 10,
			run: async (c) => {
				c.state.chargeAttempts += 1;
				if (c.state.chargeAttempts <= 2) {
					throw new UserError("card declined", {
						code: "card_declined",
					});
				}
			},
		});
		await ctx.step("notify", async (c) => {
			const client = c.client<any>();
			await client.telemetryRunConsumerActor
				.getOrCreate(c.key)
				.send("runJobs", { id: "workflow-notify" });
		});
	}),
	actions: { getWakes: (c) => c.state.wakes },
	options: {
		sleepTimeout: 50,
	},
});

/** Records nothing except `recorded`. */
export const telemetrySampledActor = actor({
	db: db(),
	tracing: {
		sampler: 0,
		actions: {
			recorded: 1,
		},
	},
	actions: {
		unrecorded: async (c, marker: string) => {
			await c.db.execute("SELECT ? AS marker", marker);
			return trace.getActiveSpan()?.spanContext().traceFlags;
		},
		recorded: async (c, marker: string) => {
			await c.db.execute("SELECT ? AS marker", marker);
			return trace.getActiveSpan()?.spanContext().traceFlags;
		},
	},
});

/** Records everything except `unrecorded`. */
export const telemetrySelfSampledActor = actor({
	state: {},
	db: db(),
	queues: {
		jobs: jobSchema,
		runJobs: jobSchema,
	},
	tracing: {
		sampler: 1,
		actions: {
			unrecorded: 0,
		},
	},
	onRequest: () => new Response("ok", { status: 200 }),
	run: async (c) => {
		while (!c.aborted) {
			await c.queue.waitForNames(["runJobs"], {
				signal: c.abortSignal,
			});
		}
	},
	actions: {
		recorded: (_c, marker: string) => marker,
		unrecorded: (_c, marker: string) => marker,
		scheduleBoth: async (c, marker: string) => {
			await c.schedule.after(20, "unrecorded", marker);
			await c.schedule.after(200, "recorded", marker);
		},
	},
});
