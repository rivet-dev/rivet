import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { trace } from "@opentelemetry/api";
import { NodeTracerProvider } from "@opentelemetry/sdk-trace-node";
import { getEnginePath } from "@rivetkit/engine-cli";
import { z } from "zod/v4";
import { db } from "../../src/db/mod";
import { actor, event, queue, setup, UserError } from "../../src/mod";
import { buildNativeRegistry } from "../../src/registry/native";

const fixtureDir = dirname(fileURLToPath(import.meta.url));
const repoEngineBinary = resolve(
	fixtureDir,
	"../../../../../target/debug/rivet-engine",
);

const endpoint = process.env.RIVETKIT_TEST_ENDPOINT ?? "http://127.0.0.1:6642";

// Register context propagation without exporting application spans; tests read their IDs.
new NodeTracerProvider().register();
const applicationTracer = trace.getTracer("napi-runtime-fixture");
const connParamsSchema = z.object({
	userId: z.string().min(1),
});
const validatedActionArgsSchema = z.tuple([
	z.object({
		amount: z.number().int().nonnegative(),
	}),
]);
const countChangedSchema = z.object({
	count: z.number().int(),
});
const jobSchema = z.object({
	id: z.string().min(1),
});

function resolveEngineBinaryPath(): string {
	if (existsSync(repoEngineBinary)) {
		return repoEngineBinary;
	}

	return getEnginePath();
}

const integrationActor = actor({
	state: { count: 0 },
	db: db(),
	connParamsSchema,
	actionInputSchemas: {
		validatedAction: validatedActionArgsSchema,
		emitValidatedEvent: z.tuple([countChangedSchema]),
		enqueueValidatedJob: z.tuple([jobSchema]),
	},
	events: {
		countChanged: event({ schema: countChangedSchema }),
	},
	queues: {
		jobs: queue({ message: jobSchema }),
	},
	onBeforeConnect: async () => {},
	onRequest: async (c, request) => {
		await c.db.execute("SELECT ? AS path", new URL(request.url).pathname);
		return new Response("ok", { status: 200 });
	},
	actions: {
		ping: async (c) => {
			return c.conn.params.userId;
		},
		getCount: async (c) => {
			return c.state.count;
		},
		validatedAction: async (_c, payload: { amount: number }) => {
			return payload.amount;
		},
		emitValidatedEvent: async (c, payload: { count: number }) => {
			c.broadcast("countChanged", payload);
			return payload.count;
		},
		enqueueValidatedJob: async (c, payload: { id: string }) => {
			await c.queue.send("jobs", payload);
			return payload.id;
		},
		increment: async (c, amount: number) => {
			c.state.count += amount;

			await c.kv.put("count", String(c.state.count));
			await c.db.execute(
				"CREATE TABLE IF NOT EXISTS increments (value INTEGER NOT NULL)",
			);
			await c.db.execute(
				"INSERT INTO increments (value) VALUES (?)",
				c.state.count,
			);

			const rows = await c.db.execute<{ value: number }>(
				"SELECT value FROM increments ORDER BY rowid ASC",
			);
			return {
				count: c.state.count,
				sqliteValues: rows.map(({ value }) => Number(value)),
			};
		},
		snapshot: async (c) => {
			const kvValue = await c.kv.get("count");
			await c.db.execute(
				"CREATE TABLE IF NOT EXISTS increments (value INTEGER NOT NULL)",
			);
			const rows = await c.db.execute<{ value: number }>(
				"SELECT value FROM increments ORDER BY rowid ASC",
			);

			return {
				count: c.state.count,
				kvCount: kvValue ? Number(kvValue) : null,
				sqliteValues: rows.map(({ value }) => Number(value)),
			};
		},
		incrementWithoutSql: async (c, amount: number) => {
			c.state.count += amount;
			await c.kv.put("count", String(c.state.count));
			return {
				count: c.state.count,
			};
		},
		scheduleTrace: async (c, correlationToken: string) => {
			await c.schedule.after(50, "scheduledTrace", correlationToken);
			return correlationToken;
		},
		scheduledTrace: async (c, correlationToken: string) => {
			await c.db.execute("SELECT ? AS trace", correlationToken);
		},
		sqliteFailure: async (c) => {
			await c.db.execute("SELECT value FROM missing_trace_test_table");
		},
		stateSnapshot: async (c) => {
			const kvValue = await c.kv.get("count");
			return {
				count: c.state.count,
				kvCount: kvValue ? Number(kvValue) : null,
			};
		},
		// Both calls reach the queue before the test releases either one.
		isolationProbe: async (c, token: string, fail: boolean) => {
			c.log.warn({ correlation_token: token }, "isolation probe");
			if (!(await c.queue.next({ names: ["jobs"], timeout: 10_000 }))) {
				throw new Error("isolation probe was not released");
			}
			await c.db.execute("SELECT ? AS probe", token);
			const client = c.client<any>();
			await client.integrationActor
				.getForId(c.actorId, {
					params: { userId: "internal-integration-test" },
				})
				.getCount();
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
						const count = await client.integrationActor
							.getForId(c.actorId, {
								params: { userId: "internal-integration-test" },
							})
							.getCount();
						return { count, spanId: span.spanContext().spanId };
					} finally {
						span.end();
					}
				},
			);
		},
		// The queue gate releases the database work only after the reply.
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
		consumeJob: async (c) => {
			const message = await c.queue.next({
				names: ["jobs"],
				timeout: 5_000,
			});
			return message?.body ?? null;
		},
		getCountViaClient: async (c) => {
			const client = c.client<any>();
			return await client.integrationActor
				.getForId(c.actorId, {
					params: { userId: "internal-integration-test" },
				})
				.getCount();
		},
		throwTypedError: async () => {
			throw new UserError("native typed error", {
				code: "boom",
				metadata: {
					source: "native",
				},
			});
		},
		throwUntypedError: async () => {
			throw new Error("native untyped error");
		},
		goToSleep: async (c) => {
			c.sleep();
			return { ok: true };
		},
	},
});

const runConsumerActor = actor({
	state: {},
	queues: {
		runJobs: queue({ message: jobSchema }),
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

const registry = setup({
	use: {
		integrationActor,
		runConsumerActor,
	},
	endpoint,
	namespace: process.env.RIVET_NAMESPACE ?? "default",
	token: process.env.RIVET_TOKEN ?? "dev",
	envoy: {
		poolName: process.env.RIVETKIT_TEST_POOL_NAME ?? "default",
	},
});

const {
	runtime: nativeRuntime,
	registry: nativeRegistry,
	serveConfig,
} = await buildNativeRegistry(registry.parseConfig());
if (!process.env.RIVETKIT_TEST_ENDPOINT) {
	serveConfig.engineBinaryPath = resolveEngineBinaryPath();
}

const shutdown = () => {
	void nativeRuntime.shutdownRegistry(nativeRegistry);
};
process.once("SIGINT", shutdown);
process.once("SIGTERM", shutdown);
try {
	await nativeRuntime.serveRegistry(nativeRegistry, serveConfig);
} finally {
	process.removeListener("SIGINT", shutdown);
	process.removeListener("SIGTERM", shutdown);
}
