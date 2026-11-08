import { randomUUID } from "node:crypto";
import { actor, event, setup } from "rivetkit";
import { db } from "rivetkit/db";
import {
	Awareness,
	applyAwarenessUpdate,
	encodeAwarenessUpdate,
	removeAwarenessStates,
} from "y-protocols/awareness";
import * as Y from "yjs";

export type DocumentSummary = {
	id: string;
	title: string;
	createdAt: number;
	updatedAt: number;
};

export type SyncEvent = { update: number[] };
export type AwarenessEvent = { update: number[] };

type DocumentInput = { title: string; createdAt: number };

type UpdateKind = "sync" | "awareness";

const toNumbers = (update: Uint8Array) => Array.from(update);
const toBytes = (update: number[]) => Uint8Array.from(update);

// docs:start document
export const document = actor({
	events: {
		sync: event<SyncEvent>(),
		awareness: event<AwarenessEvent>(),
	},

	// Metadata and the Yjs snapshot survive restarts: https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS document (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					title TEXT NOT NULL,
					snapshot BLOB,
					created_at INTEGER NOT NULL,
					updated_at INTEGER NOT NULL
				)
			`);
		},
	}),

	// Save the metadata from the creation input once, when the Actor is created.
	onCreate: async (c, input: DocumentInput) => {
		await c.db.execute(
			"INSERT INTO document (id, title, created_at, updated_at) VALUES (1, ?, ?, ?)",
			input.title,
			input.createdAt,
			input.createdAt,
		);
	},

	// Rebuild the live Yjs document from the last saved snapshot.
	createVars: async (c) => {
		const doc = new Y.Doc();
		const [row] = await c.db.execute<{ snapshot: Uint8Array | null }>(
			"SELECT snapshot FROM document WHERE id = 1",
		);
		if (row?.snapshot) {
			Y.applyUpdate(doc, row.snapshot);
		}
		const awareness = new Awareness(doc);
		// Track client awareness IDs per connection, keyed by connection ID.
		const clientIdsByConn = new Map<string, number[]>();
		return { doc, awareness, clientIdsByConn };
	},
	// docs:end document

	// docs:start presence
	// docs:start disconnect
	onDisconnect: (c, conn) => {
		const clientIds = c.vars.clientIdsByConn.get(conn.id) ?? [];
		c.vars.clientIdsByConn.delete(conn.id);
		if (clientIds.length === 0) {
			return;
		}
		removeAwarenessStates(c.vars.awareness, clientIds, "disconnect");
		const update = encodeAwarenessUpdate(c.vars.awareness, clientIds);
		c.broadcast("awareness", { update: toNumbers(update) });
	},
	// docs:end disconnect

	// docs:start relay
	actions: {
		// docs:start sync
		getContent: (c) => {
			const update = Y.encodeStateAsUpdate(c.vars.doc);
			return toNumbers(update);
		},
		applyUpdate: async (
			c,
			update: number[],
			kind: UpdateKind,
			clientId?: number,
		) => {
			if (kind === "sync") {
				const bytes = toBytes(update);
				Y.applyUpdate(c.vars.doc, bytes, "client");
				// docs:start persist
				const snapshot = Y.encodeStateAsUpdate(c.vars.doc);
				await c.db.execute(
					"UPDATE document SET snapshot = ?, updated_at = ? WHERE id = 1",
					snapshot,
					Date.now(),
				);
				// docs:end persist
				c.broadcast("sync", { update });
				return;
			}
			// docs:end sync

			// docs:start awareness
			const bytes = toBytes(update);
			applyAwarenessUpdate(c.vars.awareness, bytes, "client");
			if (typeof clientId === "number" && c.conn) {
				const knownIds = c.vars.clientIdsByConn.get(c.conn.id) ?? [];
				if (!knownIds.includes(clientId)) {
					knownIds.push(clientId);
				}
				c.vars.clientIdsByConn.set(c.conn.id, knownIds);
			}
			c.broadcast("awareness", { update });
		},
		getAwareness: (c) => {
			const clients = Array.from(c.vars.awareness.getStates().keys());
			const update = encodeAwarenessUpdate(c.vars.awareness, clients);
			return toNumbers(update);
		},
		// docs:end awareness
	},
	// docs:end relay
	// docs:end presence
});

// docs:start list
export const documentList = actor({
	// One coordinator per workspace that indexes document actors in SQLite.
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS documents (
					id TEXT PRIMARY KEY,
					title TEXT NOT NULL,
					created_at INTEGER NOT NULL,
					updated_at INTEGER NOT NULL
				)
			`);
		},
	}),

	actions: {
		createDocument: async (c, title: string) => {
			const documentId = randomUUID();
			const createdAt = Date.now();
			const workspaceId = c.key[0] ?? "default";
			const safeTitle = title.trim() || "Untitled document";

			const client = c.client<typeof registry>();
			const handle = await client.document.create(
				[workspaceId, documentId],
				{
					input: { title: safeTitle, createdAt },
				},
			);
			await handle.resolve();

			const summary: DocumentSummary = {
				id: documentId,
				title: safeTitle,
				createdAt,
				updatedAt: createdAt,
			};
			await c.db.execute(
				"INSERT INTO documents (id, title, created_at, updated_at) VALUES (?, ?, ?, ?)",
				summary.id,
				summary.title,
				summary.createdAt,
				summary.updatedAt,
			);
			return summary;
		},
		listDocuments: async (c): Promise<DocumentSummary[]> => {
			return (await c.db.execute(
				"SELECT id, title, created_at AS createdAt, updated_at AS updatedAt FROM documents ORDER BY created_at",
			)) as DocumentSummary[];
		},
		deleteDocument: async (c, documentId: string) => {
			await c.db.execute("DELETE FROM documents WHERE id = ?", documentId);
			return true;
		},
	},
});

// Register actors and start the server: https://rivet.dev/docs/setup
export const registry = setup({
	use: { document, documentList },
});

registry.start();
// docs:end list
