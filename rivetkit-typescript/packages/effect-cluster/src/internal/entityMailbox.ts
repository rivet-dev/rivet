/**
 * SQLite mailbox primitives for one entity actor. Callers compose them and
 * wrap the composition in `ActorSql.transaction` when it must commit
 * atomically.
 *
 * Portions adapted from Effect (MIT License,
 * Copyright (c) 2023 Effectful Technologies Inc). See `LICENSE-effect`.
 *
 * @internal
 */
import * as Effect from "effect/Effect";
import type { ActorSql } from "./sql.ts";

/** @internal */
export const mailboxCapacity = 4096;

/** @internal */
export const maximumEncodedSize = 2 * 1024 * 1024;

/** @internal */
export class MailboxFullError extends Error {
	readonly _tag = "MailboxFull";
}

/** @internal */
export class EncodedMessageTooLargeError extends Error {
	readonly _tag = "EncodedMessageTooLarge";
}

/** @internal */
export type PersistResult =
	| {
			readonly _tag: "Success";
	  }
	| {
			readonly _tag: "Duplicate";
			readonly originalId: string;
			readonly processed: boolean;
	  };

type ExistingMessageRow = {
	readonly request_id: string;
	readonly discard: number;
	readonly processed: number;
	readonly reply_to: string | null;
};

type CountRow = {
	readonly count: number;
};

const textEncoder = new TextEncoder();

// A UTF-16 code unit encodes to at most 3 UTF-8 bytes, so most strings skip
// the byte-length copy.
const exceedsMaximumEncodedSize = (text: string): boolean =>
	text.length * 3 > maximumEncodedSize &&
	textEncoder.encode(text).byteLength > maximumEncodedSize;

/** @internal */
export const persistRequest = (
	sql: ActorSql,
	envelopeText: string,
	primaryKey: string | null,
	options: {
		readonly discard?: boolean | undefined;
		readonly deliverAt?: number | null | undefined;
		readonly replyTo?: string | null | undefined;
		readonly capacity?: number | undefined;
	} = {},
): Effect.Effect<
	PersistResult,
	MailboxFullError | EncodedMessageTooLargeError
> =>
	Effect.gen(function* () {
		const discard = options.discard ?? false;
		const deliverAt = options.deliverAt ?? null;
		const replyTo = options.replyTo ?? null;
		const capacity = options.capacity ?? mailboxCapacity;
		if (exceedsMaximumEncodedSize(envelopeText)) {
			return yield* Effect.fail(
				new EncodedMessageTooLargeError(
					"Encoded entity request exceeds 2 MB",
				),
			);
		}
		const envelope = JSON.parse(envelopeText) as {
			readonly _tag?: unknown;
			readonly requestId?: unknown;
		};
		if (
			envelope._tag !== "Request" ||
			typeof envelope.requestId !== "string"
		) {
			return yield* Effect.die(
				new TypeError("Expected an encoded Request envelope"),
			);
		}
		const requestId = envelope.requestId;

		const existing = (yield* sql.exec<ExistingMessageRow>(
			`SELECT m.request_id, m.discard, m.processed, m.reply_to
       FROM cluster_messages m
       WHERE m.request_id = ? OR (? IS NOT NULL AND m.message_id = ?)
       LIMIT 1`,
			requestId,
			primaryKey,
			primaryKey,
		))[0];
		if (existing !== undefined) {
			if (
				replyTo !== null &&
				existing.discard === 0 &&
				existing.processed === 0
			) {
				const replyTos = decodeReplyTargets(existing.reply_to);
				const alreadyTargeted = replyTos.some(
					(target) =>
						target.replyTo === replyTo &&
						target.requestId === requestId,
				);
				if (!alreadyTargeted) replyTos.push({ replyTo, requestId });
				yield* sql.exec(
					"UPDATE cluster_messages SET reply_to = ? WHERE request_id = ?",
					JSON.stringify(replyTos),
					existing.request_id,
				);
			}
			return {
				_tag: "Duplicate",
				originalId: existing.request_id,
				processed: existing.processed === 1,
			} satisfies PersistResult;
		}

		// A request counts against capacity until it is processed and, for
		// streams, until its chunks are acknowledged.
		const pendingRows = yield* sql.exec<CountRow>(
			"SELECT COUNT(*) AS count FROM cluster_messages WHERE processed = 0",
		);
		const pending = pendingRows[0]?.count ?? 0;
		let unacked = 0;
		if (pending < capacity) {
			const unackedRows = yield* sql.exec<CountRow>(
				`SELECT COUNT(DISTINCT r.request_id) AS count
         FROM cluster_replies r
         JOIN cluster_messages m ON m.request_id = r.request_id
         WHERE r.kind = 'Chunk' AND r.acked = 0 AND m.processed = 1`,
			);
			unacked = unackedRows[0]?.count ?? 0;
		}
		if (pending + unacked >= capacity) {
			return yield* Effect.fail(
				new MailboxFullError(
					`Entity mailbox has reached its ${capacity} request capacity`,
				),
			);
		}

		yield* sql.exec(
			`INSERT INTO cluster_messages
         (request_id, message_id, envelope, discard, processed, last_reply_id, deliver_at, reply_to)
       VALUES (?, ?, ?, ?, 0, NULL, ?, ?)`,
			requestId,
			primaryKey,
			envelopeText,
			discard ? 1 : 0,
			deliverAt,
			replyTo === null ? null : JSON.stringify([{ replyTo, requestId }]),
		);
		return { _tag: "Success" } satisfies PersistResult;
	});

/** @internal */
export const saveReply = (
	sql: ActorSql,
	replyText: string,
): Effect.Effect<void, EncodedMessageTooLargeError> =>
	Effect.gen(function* () {
		if (exceedsMaximumEncodedSize(replyText)) {
			return yield* Effect.fail(
				new EncodedMessageTooLargeError(
					"Encoded entity reply chunk exceeds 2 MB",
				),
			);
		}
		const reply = JSON.parse(replyText) as {
			readonly _tag?: unknown;
			readonly requestId?: unknown;
			readonly id?: unknown;
			readonly sequence?: unknown;
		};
		if (
			(reply._tag !== "Chunk" && reply._tag !== "WithExit") ||
			typeof reply.requestId !== "string" ||
			typeof reply.id !== "string"
		) {
			return yield* Effect.die(
				new TypeError("Expected an encoded Chunk or WithExit reply"),
			);
		}
		yield* sql.exec(
			`INSERT OR IGNORE INTO cluster_replies
         (reply_id, request_id, reply, kind, sequence, acked)
       VALUES (?, ?, ?, ?, ?, 0)`,
			reply.id,
			reply.requestId,
			replyText,
			reply._tag,
			reply._tag === "Chunk" ? reply.sequence : null,
		);
		yield* sql.exec(
			`UPDATE cluster_messages
       SET last_reply_id = ?, processed = CASE WHEN ? = 'WithExit' THEN 1 ELSE processed END
       WHERE request_id = ?`,
			reply.id,
			reply._tag,
			reply.requestId,
		);
	});

/**
 * A pinned caller of a scheduled ask. The reply is pushed to the `replyTo`
 * entity under the caller's own request id, which the caller registered
 * before sending, so the push cannot arrive ahead of the registration.
 *
 * @internal
 */
export interface ReplyTarget {
	readonly replyTo: string;
	readonly requestId: string;
}

/** @internal */
export interface StoredMessage {
	readonly requestId: string;
	readonly envelope: string;
	readonly lastSentChunk: string | undefined;
	readonly discard: boolean;
	readonly deliverAt?: number | undefined;
	readonly replyTos?: ReadonlyArray<ReplyTarget> | undefined;
}

type StoredMessageRow = {
	readonly request_id: string;
	readonly envelope: string;
	readonly discard: number;
	readonly deliver_at: number | null;
	readonly reply_to: string | null;
	readonly last_reply: string | null;
};

const isReplyTarget = (item: unknown): item is ReplyTarget =>
	typeof item === "object" &&
	item !== null &&
	typeof (item as ReplyTarget).replyTo === "string" &&
	typeof (item as ReplyTarget).requestId === "string";

const decodeReplyTargets = (value: unknown): Array<ReplyTarget> => {
	if (typeof value !== "string") return [];
	try {
		const decoded = JSON.parse(value);
		if (Array.isArray(decoded) && decoded.every(isReplyTarget))
			return decoded;
	} catch {
		// Malformed rows deliver nowhere instead of poisoning the replay.
	}
	return [];
};

const rowToMessage = (row: StoredMessageRow): StoredMessage => {
	const replyTos = decodeReplyTargets(row.reply_to);
	return {
		requestId: row.request_id,
		envelope: row.envelope,
		lastSentChunk: row.last_reply ?? undefined,
		discard: row.discard === 1,
		...(row.deliver_at === null
			? undefined
			: { deliverAt: row.deliver_at }),
		...(replyTos.length === 0 ? undefined : { replyTos }),
	};
};

/** @internal */
export const loadUnprocessed = (
	sql: ActorSql,
	now?: number,
): Effect.Effect<Array<StoredMessage>> =>
	Effect.map(
		sql.exec<StoredMessageRow>(
			`SELECT m.request_id, m.envelope, m.discard, m.deliver_at, m.reply_to, r.reply AS last_reply
       FROM cluster_messages m
       LEFT JOIN cluster_replies r ON r.reply_id = m.last_reply_id
       WHERE m.processed = 0 AND (m.deliver_at IS NULL OR m.deliver_at <= ?)
       ORDER BY m.rowid ASC`,
			now ?? Date.now(),
		),
		(rows) => rows.map(rowToMessage),
	);

/** @internal */
export const loadMessage = (
	sql: ActorSql,
	requestId: string,
): Effect.Effect<StoredMessage | undefined> =>
	Effect.map(
		sql.exec<StoredMessageRow>(
			`SELECT m.request_id, m.envelope, m.discard, m.deliver_at, m.reply_to, r.reply AS last_reply
       FROM cluster_messages m
       LEFT JOIN cluster_replies r ON r.reply_id = m.last_reply_id
       WHERE m.request_id = ?
       LIMIT 1`,
			requestId,
		),
		(rows) => (rows[0] === undefined ? undefined : rowToMessage(rows[0])),
	);

/** @internal */
export interface NextReply {
	readonly reply: string;
	readonly kind: "Chunk" | "WithExit";
}

/** @internal */
export const loadNextReply = (
	sql: ActorSql,
	requestId: string,
): Effect.Effect<NextReply | undefined> =>
	Effect.gen(function* () {
		const chunk = (yield* sql.exec<NextReply>(
			`SELECT reply, kind
       FROM cluster_replies
       WHERE request_id = ? AND kind = 'Chunk' AND acked = 0
       ORDER BY sequence ASC
       LIMIT 1`,
			requestId,
		))[0];
		if (chunk !== undefined) return chunk;
		const exit = (yield* sql.exec<NextReply>(
			`SELECT reply, kind
       FROM cluster_replies
       WHERE request_id = ? AND kind = 'WithExit'
       LIMIT 1`,
			requestId,
		))[0];
		return exit;
	});

/** @internal */
export const completeTell = (
	sql: ActorSql,
	requestId: string,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			`UPDATE cluster_messages
       SET processed = 1, last_reply_id = NULL
       WHERE request_id = ?`,
			requestId,
		),
	);

/** @internal */
export const ackChunk = (
	sql: ActorSql,
	requestId: string,
	replyId: string,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			`UPDATE cluster_replies
       SET acked = 1
       WHERE request_id = ? AND reply_id = ? AND kind = 'Chunk'`,
			requestId,
			replyId,
		),
	);

/** @internal */
export const clearReplies = (
	sql: ActorSql,
	requestId: string,
): Effect.Effect<void> =>
	Effect.gen(function* () {
		yield* sql.exec(
			"DELETE FROM cluster_replies WHERE request_id = ?",
			requestId,
		);
		yield* sql.exec(
			`UPDATE cluster_messages
       SET processed = 0, last_reply_id = NULL
       WHERE request_id = ?`,
			requestId,
		);
	});
