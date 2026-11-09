import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

// How long a pulled message stays hidden before another consumer can pull it
const VISIBILITY_TIMEOUT_MS = 30_000;
// Deliveries before a message that is never acknowledged is dead-lettered
const MAX_DELIVERIES = 4;
const MAX_BATCH = 100;

interface PulledMessage {
	leaseId: string;
	body: unknown;
}

async function publish(db: RawAccess, body: unknown): Promise<void> {
	await db.execute(
		"INSERT INTO messages (body) VALUES (?)",
		JSON.stringify(body),
	);
}

// Lease up to `limit` visible messages. A message that is not acknowledged
// before its lease expires becomes visible again.
async function pull(db: RawAccess, limit: number): Promise<PulledMessage[]> {
	const count = Number.isFinite(limit)
		? Math.min(Math.max(Math.floor(limit), 1), MAX_BATCH)
		: 10;
	const now = Date.now();

	return await db.transaction(async (tx) => {
		const rows = await tx.execute<{
			id: number;
			body: string;
			deliveries: number;
		}>(
			"SELECT id, body, deliveries FROM messages WHERE leased_until <= ? ORDER BY id LIMIT ?",
			now,
			count,
		);

		const pulled: PulledMessage[] = [];
		for (const row of rows) {
			// Stop redelivering a message that keeps timing out
			if (row.deliveries >= MAX_DELIVERIES) {
				await tx.execute(
					"INSERT OR REPLACE INTO dead_letters (id, body, failed_at) VALUES (?, ?, ?)",
					row.id,
					row.body,
					now,
				);
				await tx.execute("DELETE FROM messages WHERE id = ?", row.id);
				continue;
			}

			const leaseId = crypto.randomUUID();
			await tx.execute(
				"UPDATE messages SET lease_id = ?, leased_until = ?, deliveries = deliveries + 1 WHERE id = ?",
				leaseId,
				now + VISIBILITY_TIMEOUT_MS,
				row.id,
			);
			pulled.push({ leaseId, body: JSON.parse(row.body) });
		}
		return pulled;
	});
}

// Delete acknowledged messages. An ack after the lease expired still works
// until another consumer pulls the message again, which replaces its lease ID.
async function ack(db: RawAccess, leaseIds: string[]): Promise<void> {
	await db.transaction(async (tx) => {
		for (const leaseId of leaseIds) {
			await tx.execute(
				"DELETE FROM messages WHERE lease_id = ?",
				leaseId,
			);
		}
	});
}

export const pullQueue = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS messages (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          body TEXT NOT NULL,
          lease_id TEXT,
          leased_until INTEGER NOT NULL DEFAULT 0,
          deliveries INTEGER NOT NULL DEFAULT 0
        );
      `);
			await db.execute(
				"CREATE INDEX IF NOT EXISTS messages_lease_id ON messages (lease_id)",
			);
			await db.execute(`
        CREATE TABLE IF NOT EXISTS dead_letters (
          id INTEGER PRIMARY KEY,
          body TEXT NOT NULL,
          failed_at INTEGER NOT NULL
        );
      `);
		},
	}),

	// docs:start actions
	// Consumers that use the Rivet client call actions
	actions: {
		publish: (c, body: unknown) => publish(c.db, body),
		pull: (c, limit: number) => pull(c.db, limit),
		ack: (c, leaseIds: string[]) => ack(c.db, leaseIds),
	},
	// docs:end actions

	// docs:start http
	// Producers and consumers in other languages use plain HTTP
	onRequest: async (c, request) => {
		const url = new URL(request.url);
		if (request.method !== "POST") {
			return new Response("Method Not Allowed", { status: 405 });
		}

		switch (url.pathname) {
			case "/messages": {
				const { body } = (await request.json()) as { body: unknown };
				await publish(c.db, body);
				return Response.json({ ok: true });
			}
			case "/pull": {
				const { limit } = (await request.json()) as { limit?: number };
				return Response.json({
					messages: await pull(c.db, limit ?? 10),
				});
			}
			case "/ack": {
				const { leaseIds } = (await request.json()) as {
					leaseIds: string[];
				};
				await ack(c.db, leaseIds);
				return Response.json({ ok: true });
			}
			default:
				return new Response("Not Found", { status: 404 });
		}
	},
	// docs:end http
});
