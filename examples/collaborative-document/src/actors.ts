import { actor, setup } from "rivetkit";
import { applyAwarenessUpdate, Awareness, encodeAwarenessUpdate } from "y-protocols/awareness";
import { randomUUID } from "node:crypto";
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

export const document = actor({
	// Track client awareness IDs per connection.
	connState: {
		clientIds: [] as number[],
	},

	// Persistent metadata that survives restarts: https://rivet.dev/docs/actors/state
	createState: (_c, input: DocumentInput) => ({
		title: input.title,
		createdAt: input.createdAt,
		updatedAt: input.createdAt,
	}),

	// Load Yjs state from durable KV storage.
	createVars: async (c) => {
		const doc = new Y.Doc();
		const stored = await c.kv.get("yjs:doc", { type: "binary" });
		if (stored) {
			Y.applyUpdate(doc, stored);
		}
		const awareness = new Awareness(doc);
		return { doc, awareness };
	},

	onDisconnect: (c, conn) => {
		const clientIds = conn.state.clientIds;
		if (clientIds.length === 0) {
			return;
		}
		c.vars.awareness.removeStates(clientIds, "disconnect");
		const update = encodeAwarenessUpdate(c.vars.awareness, clientIds);
		c.broadcast("awareness", { update: toNumbers(update) });
		conn.state.clientIds = [];
	},

	actions: {
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
				const snapshot = Y.encodeStateAsUpdate(c.vars.doc);
				await c.kv.put("yjs:doc", snapshot);
				c.state.updatedAt = Date.now();
				c.broadcast("sync", { update });
				return;
			}

			const bytes = toBytes(update);
			applyAwarenessUpdate(c.vars.awareness, bytes, "client");
			if (typeof clientId === "number" && c.conn) {
				const knownIds = c.conn.state.clientIds;
				if (!knownIds.includes(clientId)) {
					knownIds.push(clientId);
				}
			}
			c.broadcast("awareness", { update });
		},
		getAwareness: (c) => {
			const clients = Array.from(c.vars.awareness.getStates().keys());
			const update = encodeAwarenessUpdate(c.vars.awareness, clients);
			return toNumbers(update);
		},
	},
});

export const documentList = actor({
	// One coordinator per workspace that indexes document actors.
	state: {
		documents: [] as DocumentSummary[],
	},

	actions: {
		createDocument: async (c, title: string) => {
			const documentId = randomUUID();
			const createdAt = Date.now();
			const workspaceId = c.key[0] ?? "default";
			const safeTitle = title.trim() || "Untitled document";

			const client = c.client<typeof registry>();
			const handle = await client.document.create([workspaceId, documentId], {
				input: { title: safeTitle, createdAt },
			});
			await handle.resolve();

			const summary: DocumentSummary = {
				id: documentId,
				title: safeTitle,
				createdAt,
				updatedAt: createdAt,
			};
			c.state.documents.push(summary);
			return summary;
		},
		listDocuments: (c) => c.state.documents,
		deleteDocument: (c, documentId: string) => {
			c.state.documents = c.state.documents.filter(
				(document) => document.id !== documentId,
			);
			return true;
		},
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { document, documentList },
});
