import type { AsyncSqlRunner } from './async-db.js';
import type { PersistedChunkOwner, PersistedChunkRow, PersistedImageChunk } from './helpers.js';

export function createChunkStore(runner: AsyncSqlRunner) {
	return {
		async read(owner: PersistedChunkOwner): Promise<PersistedChunkRow[]> {
			const rows = await runner.query(
				`SELECT image_id, chunk_index, chunk_count, data
				 FROM flue_image_chunks
				 WHERE owner_kind = ? AND owner_id = ? AND owner_part = ?
				 ORDER BY image_id, chunk_index`,
				[owner.kind, owner.id, owner.part],
			);
			return rows.map(parseChunkRow);
		},
		async replace(
			owner: PersistedChunkOwner,
			chunks: readonly PersistedImageChunk[],
		): Promise<void> {
			await deleteOwner(runner, owner);
			for (const chunk of chunks) {
				await runner.query(
					`INSERT INTO flue_image_chunks
					 (owner_kind, owner_id, owner_part, image_id, chunk_index, chunk_count, data)
					 VALUES (?, ?, ?, ?, ?, ?, ?)`,
					[owner.kind, owner.id, owner.part, chunk.imageId, chunk.index, chunk.count, chunk.data],
				);
			}
		},
		async delete(owner: PersistedChunkOwner): Promise<void> {
			await deleteOwner(runner, owner);
		},
		async deleteMany(owners: readonly PersistedChunkOwner[]): Promise<void> {
			for (const owner of owners) await deleteOwner(runner, owner);
		},
		async deleteOwner(kind: PersistedChunkOwner['kind'], id: string): Promise<void> {
			await runner.query('DELETE FROM flue_image_chunks WHERE owner_kind = ? AND owner_id = ?', [
				kind,
				id,
			]);
		},
	};
}

function parseChunkRow(row: Record<string, unknown>): PersistedChunkRow {
	const index = Number(row.chunk_index);
	const count = Number(row.chunk_count);
	if (
		typeof row.image_id !== 'string' ||
		!Number.isInteger(index) ||
		!Number.isInteger(count) ||
		typeof row.data !== 'string'
	) {
		throw new Error('[flue] Persisted image chunk row is malformed.');
	}
	return { imageId: row.image_id, index, count, data: row.data };
}

async function deleteOwner(runner: AsyncSqlRunner, owner: PersistedChunkOwner): Promise<void> {
	await runner.query(
		'DELETE FROM flue_image_chunks WHERE owner_kind = ? AND owner_id = ? AND owner_part = ?',
		[owner.kind, owner.id, owner.part],
	);
}
