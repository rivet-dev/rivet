import { actor } from "rivetkit";
import { db } from "rivetkit/db";

type LifecycleCounts = {
	create: number;
	migrate: number;
	cleanup: number;
};

const clientActorIds = new WeakMap<object, string>();

const createCounts = new Map<string, number>();
const migrateCounts = new Map<string, number>();
const cleanupCounts = new Map<string, number>();

function increment(map: Map<string, number>, actorId: string) {
	map.set(actorId, (map.get(actorId) ?? 0) + 1);
}

function getCounts(actorId: string): LifecycleCounts {
	return {
		create: createCounts.get(actorId) ?? 0,
		migrate: migrateCounts.get(actorId) ?? 0,
		cleanup: cleanupCounts.get(actorId) ?? 0,
	};
}

const baseProvider = db({
	onMigrate: async (dbHandle) => {
		await dbHandle.execute(`
			CREATE TABLE IF NOT EXISTS lifecycle_data (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				value TEXT NOT NULL,
				created_at INTEGER NOT NULL
			)
		`);
	},
});

const lifecycleProvider = {
	createClient: async (ctx: Parameters<typeof baseProvider.createClient>[0]) => {
		const client = await baseProvider.createClient(ctx);
		clientActorIds.set(client as object, ctx.actorId);
		increment(createCounts, ctx.actorId);
		return client;
	},
	onMigrate: async (client: Parameters<typeof baseProvider.onMigrate>[0]) => {
		const actorId = clientActorIds.get(client as object);
		if (actorId) {
			increment(migrateCounts, actorId);
		}
		await baseProvider.onMigrate(client);
	},
	onDestroy: async (client: Parameters<NonNullable<typeof baseProvider.onDestroy>>[0]) => {
		const actorId = clientActorIds.get(client as object);
		if (actorId) {
			increment(cleanupCounts, actorId);
		}
			await baseProvider.onDestroy?.(client);
		},
	};

const failingLifecycleProvider = {
	createClient: async (ctx: Parameters<typeof baseProvider.createClient>[0]) => {
		const client = await baseProvider.createClient(ctx);
		clientActorIds.set(client as object, ctx.actorId);
		increment(createCounts, ctx.actorId);
		return client;
	},
	onMigrate: async (client: Parameters<typeof baseProvider.onMigrate>[0]) => {
		const actorId = clientActorIds.get(client as object);
		if (actorId) {
			increment(migrateCounts, actorId);
		}
		throw new Error("forced migrate failure");
	},
	onDestroy: async (client: Parameters<NonNullable<typeof baseProvider.onDestroy>>[0]) => {
		const actorId = clientActorIds.get(client as object);
		if (actorId) {
			increment(cleanupCounts, actorId);
		}
		await baseProvider.onDestroy?.(client);
	},
};

export const dbLifecycle = actor({
	db: lifecycleProvider,
	actions: {
		getActorId: (c) => c.actorId,
		ping: () => "pong",
		insertValue: async (c, value: string) => {
			await c.db.execute(
				"INSERT INTO lifecycle_data (value, created_at) VALUES (?, ?)",
				value,
				Date.now(),
			);
		},
		getCount: async (c) => {
			const results = await c.db.execute<{ count: number }>(
				`SELECT COUNT(*) as count FROM lifecycle_data`,
			);
			return results[0]?.count ?? 0;
		},
		triggerSleep: (c) => {
			c.sleep();
		},
		triggerDestroy: (c) => {
			c.destroy();
		},
	},
	options: {
		sleepTimeout: 100,
	},
});

export const dbLifecycleFailing = actor({
	db: failingLifecycleProvider,
	actions: {
		ping: () => "pong",
	},
});

export const dbLifecycleObserver = actor({
	actions: {
		getCounts: (_c, actorId: string) => {
			return getCounts(actorId);
		},
	},
});
