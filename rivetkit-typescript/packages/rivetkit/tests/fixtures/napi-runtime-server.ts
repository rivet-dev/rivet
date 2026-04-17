import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { getEnginePath } from "@rivetkit/engine-cli";
import { z } from "zod/v4";
import { UserError, actor, event, queue, setup } from "../../src/mod";
import { buildNativeRegistry } from "../../src/registry/native";

const textDecoder = new TextDecoder();
const fixtureDir = dirname(fileURLToPath(import.meta.url));
const repoEngineBinary = resolve(
	fixtureDir,
	"../../../../../target/debug/rivet-engine",
);

const endpoint = process.env.RIVETKIT_TEST_ENDPOINT ?? "http://127.0.0.1:6642";
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
			await c.sql.run(
				"CREATE TABLE IF NOT EXISTS increments (value INTEGER NOT NULL)",
			);
			await c.sql.run("INSERT INTO increments (value) VALUES (?)", [
				c.state.count,
			]);

			const rows = await c.sql.query(
				"SELECT value FROM increments ORDER BY rowid ASC",
			);
			return {
				count: c.state.count,
				sqliteValues: rows.rows.map(([value]) => Number(value)),
			};
		},
		snapshot: async (c) => {
			const kvValue = await c.kv.get("count");
			await c.sql.run(
				"CREATE TABLE IF NOT EXISTS increments (value INTEGER NOT NULL)",
			);
			const rows = await c.sql.query(
				"SELECT value FROM increments ORDER BY rowid ASC",
			);

			return {
				count: c.state.count,
				kvCount: kvValue ? Number(textDecoder.decode(kvValue)) : null,
				sqliteValues: rows.rows.map(([value]) => Number(value)),
			};
		},
		incrementWithoutSql: async (c, amount: number) => {
			c.state.count += amount;
			await c.kv.put("count", String(c.state.count));
			return {
				count: c.state.count,
			};
		},
		stateSnapshot: async (c) => {
			const kvValue = await c.kv.get("count");
			return {
				count: c.state.count,
				kvCount: kvValue ? Number(textDecoder.decode(kvValue)) : null,
			};
		},
		getCountViaClient: async (c) => {
			const client = c.client<any>();
			return await client.integrationActor.getForId(c.actorId).getCount();
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

const registry = setup({
	use: {
		integrationActor,
	},
	endpoint,
	namespace: process.env.RIVET_NAMESPACE ?? "default",
	token: process.env.RIVET_TOKEN ?? "dev",
	envoy: {
		poolName: process.env.RIVETKIT_TEST_POOL_NAME ?? "default",
	},
});

const { registry: nativeRegistry, serveConfig } = await buildNativeRegistry(
	registry.parseConfig(),
);
serveConfig.engineBinaryPath = resolveEngineBinaryPath();

await nativeRegistry.serve(serveConfig);
