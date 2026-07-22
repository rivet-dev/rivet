import type { SessionData, SessionStore } from '@flue/runtime/adapter-kit';
import type { AsyncSqlDb } from './async-db.js';
import { createChunkStore } from './chunk-store.js';
import {
	hydratePersistedSessionEntry,
	prepareSessionEntry,
	samePersistedChunks,
	sessionEntryChunkOwner,
} from './helpers.js';

export function createAsyncSessionStore(db: AsyncSqlDb): SessionStore {
	return new AsyncSessionStore(db);
}

class AsyncSessionStore implements SessionStore {
	private db: AsyncSqlDb;

	constructor(db: AsyncSqlDb) {
		this.db = db;
	}

	async save(id: string, data: SessionData): Promise<void> {
		const { entries: sessionEntries, ...session } = data;
		const entries = sessionEntries.map((entry, position) => {
			const prepared = prepareSessionEntry(entry);
			return { entry, position, data: JSON.stringify(prepared.value), chunks: prepared.chunks };
		});

		await this.db.transaction(async (tx) => {
			const chunkStore = createChunkStore(tx);
			await tx.query(
				`INSERT INTO flue_sessions (id, data) VALUES (?, ?)
				 ON CONFLICT (id) DO UPDATE SET data = excluded.data`,
				[id, JSON.stringify(session)],
			);

			const existingRows = await tx.query(
				'SELECT entry_id, position, data FROM flue_session_entries WHERE session_id = ?',
				[id],
			);
			const existing = new Map(existingRows.map((row) => [row.entry_id, row]));
			const retained = new Set<string>();
			for (const { entry, position, data: entryData, chunks } of entries) {
				retained.add(entry.id);
				const current = existing.get(entry.id);
				const owner = sessionEntryChunkOwner(id, entry.id);
				const currentChunks = await chunkStore.read(owner);
				const entryChanged = Number(current?.position) !== position || current?.data !== entryData;
				const chunksChanged = !samePersistedChunks(currentChunks, chunks);
				if (entryChanged) {
					await tx.query(
						`INSERT INTO flue_session_entries (session_id, entry_id, position, data)
						 VALUES (?, ?, ?, ?)
						 ON CONFLICT (session_id, entry_id) DO UPDATE SET
						   position = excluded.position,
						   data = excluded.data`,
						[id, entry.id, position, entryData],
					);
				}
				if (chunksChanged) await chunkStore.replace(owner, chunks);
			}

			for (const row of existingRows) {
				if (typeof row.entry_id === 'string' && !retained.has(row.entry_id)) {
					await chunkStore.delete(sessionEntryChunkOwner(id, row.entry_id));
					await tx.query(
						'DELETE FROM flue_session_entries WHERE session_id = ? AND entry_id = ?',
						[id, row.entry_id],
					);
				}
			}
		});
	}

	async load(id: string): Promise<SessionData | null> {
		return this.db.transaction(async (tx) => {
			const chunkStore = createChunkStore(tx);
			const rows = await tx.query('SELECT data FROM flue_sessions WHERE id = ? LIMIT 1', [id]);
			const row = rows[0];
			if (!row) return null;
			if (typeof row.data !== 'string') {
				throw new Error('[flue] Persisted session row is malformed.');
			}
			const session = JSON.parse(row.data) as Omit<SessionData, 'entries'>;
			const entryRows = await tx.query(
				'SELECT entry_id, data FROM flue_session_entries WHERE session_id = ? ORDER BY position ASC',
				[id],
			);
			const entries = [];
			for (const entryRow of entryRows) {
				if (typeof entryRow.entry_id !== 'string' || typeof entryRow.data !== 'string') {
					throw new Error('[flue] Persisted session entry row is malformed.');
				}
				entries.push(
					hydratePersistedSessionEntry(
						JSON.parse(entryRow.data),
						await chunkStore.read(sessionEntryChunkOwner(id, entryRow.entry_id)),
					),
				);
			}
			return { ...session, entries };
		});
	}

	async delete(id: string): Promise<void> {
		await this.db.transaction(async (tx) => {
			await createChunkStore(tx).deleteOwner('session_entry', id);
			await tx.query('DELETE FROM flue_session_entries WHERE session_id = ?', [id]);
			await tx.query('DELETE FROM flue_sessions WHERE id = ?', [id]);
		});
	}
}
