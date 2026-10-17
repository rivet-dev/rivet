import { actor } from "rivetkit";
import type {
	DatabaseProvider,
	RawAccess,
	SqliteDatabase,
} from "@/common/database/config";

type SyncAccess = RawAccess & { sqlite: SqliteDatabase };

// Embedders that host their own runtime use the provider-level database the same way: they return the
// result of `nativeDatabaseProvider.open` as their client.
const awaitedDb = {
	createClient: async (ctx) => {
		if (!ctx.nativeDatabaseProvider) {
			throw new Error("native SQLite is required for synchronous access");
		}
		const sqlite = await ctx.nativeDatabaseProvider.open(ctx.actorId);
		return { sqlite, close: async () => {} } as unknown as SyncAccess;
	},
	onMigrate: async (client) => {
		await client.sqlite.execute(
			"CREATE TABLE IF NOT EXISTS sync_rows (id INTEGER PRIMARY KEY AUTOINCREMENT, value TEXT NOT NULL)",
		);
	},
} satisfies DatabaseProvider<SyncAccess>;

// Synchronous calls need deferred commits, which an actor opts into through this internal marker.
const syncDb = {
	...awaitedDb,
	[Symbol.for("rivetkit.sqlite.deferredCommits")]: true,
};

function sqlite(c: { db: SyncAccess }) {
	const { executeSync, commitTicket, whenDurable } = c.db.sqlite;
	if (!executeSync || !commitTicket || !whenDurable) {
		throw new Error("the native database provider has no synchronous API");
	}
	return {
		executeSync: executeSync.bind(c.db.sqlite),
		commitTicket: commitTicket.bind(c.db.sqlite),
		whenDurable: whenDurable.bind(c.db.sqlite),
	};
}

function insertSync(c: { db: SyncAccess }, value: string) {
	sqlite(c).executeSync("INSERT INTO sync_rows (value) VALUES (?)", [value]);
}

export const dbActorSync = actor({
	db: syncDb,
	actions: {
		insertSyncAndWaitDurable: async (c, values: string[]) => {
			const db = sqlite(c);
			for (const value of values) {
				db.executeSync("INSERT INTO sync_rows (value) VALUES (?)", [
					value,
				]);
			}
			await db.whenDurable(db.commitTicket());
		},
		insertSync,
		valuesSync: (c) =>
			sqlite(c)
				.executeSync("SELECT value FROM sync_rows ORDER BY id")
				.rows.map((row) => row[0]),
		goToSleep: (c) => {
			c.sleep();
		},
	},
});

export const dbActorSyncAwaited = actor({
	db: awaitedDb,
	actions: { insertSync },
});
