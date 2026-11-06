import { actor, event, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

type Status = "approved" | "rejected" | "expired";

interface ApprovalContext {
	db: RawAccess;
	broadcast(name: string, ...args: unknown[]): void;
}

// Resolve a pending expense once. The status check makes repeat calls no-ops.
async function resolve(c: ApprovalContext, id: string, status: Status) {
	const rows = await c.db.execute<{ id: string }>(
		"UPDATE expenses SET status = ? WHERE id = ? AND status = 'pending' RETURNING id",
		status,
		id,
	);
	if (rows.length > 0) {
		c.broadcast("expenseResolved", { id, status });
	}
}

export const expenseAgent = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS expenses (
          id TEXT PRIMARY KEY,
          amount INTEGER NOT NULL,
          status TEXT NOT NULL
        )
      `);
		},
	}),

	events: {
		approvalRequested: event<{ id: string; amount: number }>(),
		expenseResolved: event<{ id: string; status: Status }>(),
	},

	actions: {
		submit: async (c, id: string, amount: number) => {
			await c.db.execute(
				"INSERT INTO expenses (id, amount, status) VALUES (?, ?, 'pending')",
				id,
				amount,
			);
			c.broadcast("approvalRequested", { id, amount });

			// Expire the request if nobody answers within 7 days
			await c.schedule.after(7 * 24 * 60 * 60 * 1000, "expire", id);
		},

		// A reviewer's client calls these
		approve: (c, id: string) => resolve(c, id, "approved"),
		reject: (c, id: string) => resolve(c, id, "rejected"),

		// Called by the schedule. Clients can call any action, so reject them.
		expire: (c, id: string) => {
			if (!c.scheduledFire) {
				throw new UserError("expire only runs from a schedule");
			}
			return resolve(c, id, "expired");
		},
	},
});
