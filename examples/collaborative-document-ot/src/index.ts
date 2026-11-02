import { actor, event, setup, UserError } from "rivetkit";
import { db } from "rivetkit/db";
import { apply, assertOperation, type Operation, transform } from "./ot.ts";

export type { Operation } from "./ot.ts";

export type Snapshot = { text: string; revision: number };

export type OperationEvent = {
	revision: number;
	clientId: string;
	operation: Operation;
};

// docs:start document
export const document = actor({
	// docs:start setup
	// Each document keeps its operation log and latest text in its own SQLite database: https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS operations (
					revision INTEGER PRIMARY KEY,
					client_id TEXT NOT NULL,
					operation TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS snapshot (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					revision INTEGER NOT NULL,
					text TEXT NOT NULL
				)
			`);
			await db.execute(
				"INSERT OR IGNORE INTO snapshot (id, revision, text) VALUES (1, 0, '')",
			);
		},
	}),
	events: {
		operation: event<OperationEvent>(),
	},
	// docs:end setup

	actions: {
		// docs:start snapshot
		getSnapshot: async (c): Promise<Snapshot> => {
			const [row] = await c.db.execute<Snapshot>(
				"SELECT text, revision FROM snapshot WHERE id = 1",
			);
			return { text: row.text, revision: row.revision };
		},
		// docs:end snapshot

		// docs:start submit
		submitOperation: async (
			c,
			clientId: string,
			baseRevision: number,
			operation: Operation,
		) => {
			try {
				assertOperation(operation);
			} catch (error) {
				throw new UserError((error as Error).message, {
					code: "invalid_operation",
				});
			}

			// The transaction runs one submission at a time, so no other
			// operation can commit between reading the log and appending to it.
			const committed = await c.db.transaction(
				async (tx) => {
					const [current] = await tx.execute<Snapshot>(
						"SELECT text, revision FROM snapshot WHERE id = 1",
					);
					if (
						!Number.isInteger(baseRevision) ||
						baseRevision < 0 ||
						baseRevision > current.revision
					) {
						throw new UserError("unknown base revision", {
							code: "invalid_revision",
						});
					}

					// docs:start transform
					// Rewrite the operation past everything committed since the
					// revision the client based it on.
					const concurrent = await tx.execute<{ operation: string }>(
						"SELECT operation FROM operations WHERE revision > ? ORDER BY revision ASC",
						baseRevision,
					);
					let transformed = operation;
					for (const row of concurrent) {
						const committedOp = JSON.parse(
							row.operation,
						) as Operation;
						[transformed] = transform(transformed, committedOp);
					}
					// docs:end transform

					// docs:start append
					let text: string;
					try {
						text = apply(current.text, transformed);
					} catch (error) {
						throw new UserError((error as Error).message, {
							code: "invalid_operation",
						});
					}
					const revision = current.revision + 1;
					await tx.execute(
						"INSERT INTO operations (revision, client_id, operation, created_at) VALUES (?, ?, ?, ?)",
						revision,
						clientId,
						JSON.stringify(transformed),
						Date.now(),
					);
					await tx.execute(
						"UPDATE snapshot SET revision = ?, text = ? WHERE id = 1",
						revision,
						text,
					);
					// docs:end append
					return { revision, clientId, operation: transformed };
				},
				{ name: "submit-operation" },
			);

			// docs:start broadcast
			// Every editor gets the transformed operation, including the sender,
			// which treats it as the acknowledgment.
			c.broadcast("operation", committed);
			return committed.revision;
			// docs:end broadcast
		},
		// docs:end submit
	},
});
// docs:end document

// Register actors and start the server: https://rivet.dev/docs/setup
export const registry = setup({
	use: { document },
});

registry.start();
