import type {
	ListEventsRequest,
	ListPage,
	ListPageRequest,
	SessionEvent,
	SessionPersistDriver,
	SessionRecord,
} from "sandbox-agent";
import type { RawAccess } from "@/db/config";

type PersistSessionRow = {
	record_json: string;
};

type PersistEventRow = {
	id: string;
	event_index: number;
	session_id: string;
	created_at: number;
	connection_id: string;
	sender: SessionEvent["sender"];
	payload_json: string;
};

function parseCursor(cursor?: string): number {
	if (!cursor) {
		return 0;
	}

	const value = Number(cursor);
	if (!Number.isFinite(value) || value < 0) {
		return 0;
	}

	return Math.floor(value);
}

function nextCursor(
	offset: number,
	limit: number,
	itemCount: number,
): string | undefined {
	if (itemCount < limit) {
		return undefined;
	}

	return String(offset + itemCount);
}

export class SqliteSessionPersistDriver implements SessionPersistDriver {
	#db: RawAccess;
	#persistRawEvents: boolean;

	constructor(db: RawAccess, persistRawEvents: boolean) {
		this.#db = db;
		this.#persistRawEvents = persistRawEvents;
	}

	async getSession(id: string): Promise<SessionRecord | undefined> {
		const rows = await this.#db.execute<PersistSessionRow>(
			"SELECT record_json FROM sandbox_agent_sessions WHERE id = ? LIMIT 1",
			id,
		);
		const row = rows[0];
		if (!row) {
			return undefined;
		}
		return JSON.parse(row.record_json) as SessionRecord;
	}

	async listSessions(
		request: ListPageRequest = {},
	): Promise<ListPage<SessionRecord>> {
		const limit = request.limit ?? 50;
		const offset = parseCursor(request.cursor);
		const rows = await this.#db.execute<PersistSessionRow>(
			`
				SELECT record_json
				FROM sandbox_agent_sessions
				ORDER BY created_at DESC, id DESC
				LIMIT ? OFFSET ?
			`,
			limit,
			offset,
		);

		return {
			items: rows.map(
				(row) => JSON.parse(row.record_json) as SessionRecord,
			),
			nextCursor: nextCursor(offset, limit, rows.length),
		};
	}

	async updateSession(session: SessionRecord): Promise<void> {
		await this.#db.execute(
			`
				INSERT INTO sandbox_agent_sessions (id, created_at, record_json)
				VALUES (?, ?, ?)
				ON CONFLICT(id) DO UPDATE SET
					created_at = excluded.created_at,
					record_json = excluded.record_json
			`,
			session.id,
			session.createdAt,
			JSON.stringify(session),
		);
	}

	async listEvents(
		request: ListEventsRequest,
	): Promise<ListPage<SessionEvent>> {
		const limit = request.limit ?? 200;
		const offset = parseCursor(request.cursor);
		const rows = await this.#db.execute<PersistEventRow>(
			`
				SELECT
					id,
					event_index,
					session_id,
					created_at,
					connection_id,
					sender,
					payload_json
				FROM sandbox_agent_events
				WHERE session_id = ?
				ORDER BY event_index ASC
				LIMIT ? OFFSET ?
			`,
			request.sessionId,
			limit,
			offset,
		);

		return {
			items: rows.map(
				(row) =>
					({
						id: row.id,
						eventIndex: row.event_index,
						sessionId: row.session_id,
						createdAt: row.created_at,
						connectionId: row.connection_id,
						sender: row.sender,
						payload: JSON.parse(row.payload_json),
					}) satisfies SessionEvent,
			),
			nextCursor: nextCursor(offset, limit, rows.length),
		};
	}

	async insertEvent(_sessionId: string, event: SessionEvent): Promise<void> {
		const payload = JSON.stringify(event.payload);
		await this.#db.execute(
			`
				INSERT INTO sandbox_agent_events (
					id,
					session_id,
					event_index,
					created_at,
					connection_id,
					sender,
					payload_json,
					raw_payload_json
				)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?)
				ON CONFLICT(session_id, event_index) DO UPDATE SET
					id = excluded.id,
					created_at = excluded.created_at,
					connection_id = excluded.connection_id,
					sender = excluded.sender,
					payload_json = excluded.payload_json,
					raw_payload_json = excluded.raw_payload_json
			`,
			event.id,
			event.sessionId,
			event.eventIndex,
			event.createdAt,
			event.connectionId,
			event.sender,
			payload,
			this.#persistRawEvents ? payload : null,
		);
	}
}
