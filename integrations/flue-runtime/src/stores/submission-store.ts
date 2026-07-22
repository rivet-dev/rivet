import type {
	AgentAttemptMarker,
	AgentDispatchAdmission,
	AgentSubmission,
	AgentSubmissionStore,
	AgentTurnJournal,
	AgentTurnJournalPhase,
	CreateTurnJournalInput,
	DirectAgentSubmissionInput,
	DispatchAgentSubmissionInput,
	DispatchInput,
	SubmissionAttemptRef,
	SubmissionClaimRef,
	SubmissionDurability,
} from '@flue/runtime/adapter-kit';
import type { AsyncSqlDb, AsyncSqlRow, AsyncSqlRunner } from './async-db.js';
import { createChunkStore } from './chunk-store.js';
import {
	DURABILITY_DEFAULT_MAX_ATTEMPTS,
	DURABILITY_DEFAULT_TIMEOUT_MS,
	LEASE_DURATION_MS,
} from './constants.js';
import {
	createDispatchAgentSubmissionInput,
	createSubmissionSessionKey,
	hydratePersistedDirectSubmission,
	isSubmissionPayload,
	matchesPersistedDirectSubmission,
	parseAcceptedAt,
	prepareDirectSubmission,
	samePersistedChunks,
	submissionChunkOwner,
	type PersistedChunkRow,
} from './helpers.js';

const submissionColumns = [
	'sequence',
	'submission_id',
	'session_key',
	'kind',
	'payload',
	'status',
	'accepted_at',
	'attempt_id',
	'input_applied_at',
	'recovery_requested_at',
	'started_at',
	'error',
	'attempt_count',
	'max_retry',
	'timeout_at',
	'owner_id',
	'lease_expires_at',
].join(', ');

function prefixed(table: string): string {
	return submissionColumns
		.split(', ')
		.map((column) => `${table}.${column}`)
		.join(', ');
}

export function createAsyncSubmissionStore(db: AsyncSqlDb): AgentSubmissionStore {
	return new AsyncSubmissionStore(db);
}

class AsyncSubmissionStore implements AgentSubmissionStore {
	private pendingSessionDeletions = new Map<string, Promise<void>>();
	private db: AsyncSqlDb;

	constructor(db: AsyncSqlDb) {
		this.db = db;
	}

	async getSubmission(submissionId: string): Promise<AgentSubmission | null> {
		return this.db.transaction(async (tx) => {
			const rows = await tx.query(
				`SELECT ${submissionColumns} FROM flue_agent_submissions WHERE submission_id = ? LIMIT 1`,
				[submissionId],
			);
			const row = rows[0];
			return row
				? parseSubmission(
						row,
						await createChunkStore(tx).read(submissionChunkOwner(submissionId)),
					)
				: null;
		});
	}

	async getTurnJournal(submissionId: string): Promise<AgentTurnJournal | null> {
		const rows = await this.db.query(
			`SELECT submission_id, session_key, kind, attempt_id, operation_id, turn_id,
			        phase, revision, created_at, updated_at, checkpoint_leaf_id,
			        tool_request_json, stream_key, stream_consumed_at, committed, committed_leaf_id
			 FROM flue_agent_turn_journals
			 WHERE submission_id = ?
			 LIMIT 1`,
			[submissionId],
		);
		return rows[0] ? parseTurnJournal(rows[0]) : null;
	}

	async hasUnsettledSubmissions(): Promise<boolean> {
		const rows = await this.db.query(
			`SELECT 1 FROM flue_agent_submissions WHERE status IN ('queued', 'running') LIMIT 1`,
		);
		return rows.length > 0;
	}

	async listRunnableSubmissions(): Promise<AgentSubmission[]> {
		return this.db.transaction(async (tx) => {
			const rows = await tx.query(
				`SELECT ${prefixed('current_sub')}
				 FROM flue_agent_submissions AS current_sub
				 WHERE current_sub.status = 'queued'
				   AND NOT EXISTS (
				     SELECT 1
				     FROM flue_agent_submissions AS earlier
				     WHERE earlier.session_key = current_sub.session_key
				       AND earlier.status IN ('queued', 'running')
				       AND earlier.sequence < current_sub.sequence
				   )
				 ORDER BY current_sub.sequence ASC`,
			);
			return this.parseOperationalRows(rows, 'queued', tx);
		});
	}

	async listRunningSubmissions(): Promise<AgentSubmission[]> {
		return this.db.transaction(async (tx) => {
			const rows = await tx.query(
				`SELECT ${submissionColumns}
				 FROM flue_agent_submissions
				 WHERE status = 'running'
				 ORDER BY sequence ASC`,
			);
			return this.parseOperationalRows(rows, 'active', tx);
		});
	}

	async beginTurnJournal(input: CreateTurnJournalInput): Promise<boolean> {
		const now = Date.now();
		const rows = await this.db.query(
			`INSERT INTO flue_agent_turn_journals
			 (submission_id, session_key, kind, attempt_id, operation_id, turn_id,
			  phase, revision, created_at, updated_at, checkpoint_leaf_id,
			  tool_request_json, stream_key, stream_consumed_at, committed, committed_leaf_id)
			 VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?, NULL, NULL, 0, NULL)
			 ON CONFLICT (submission_id) DO UPDATE SET
			   attempt_id = excluded.attempt_id,
			   operation_id = excluded.operation_id,
			   turn_id = excluded.turn_id,
			   phase = excluded.phase,
			   revision = flue_agent_turn_journals.revision + 1,
			   updated_at = excluded.updated_at,
			   checkpoint_leaf_id = excluded.checkpoint_leaf_id,
			   tool_request_json = excluded.tool_request_json,
			   stream_key = NULL,
			   stream_consumed_at = NULL,
			   committed = 0,
			   committed_leaf_id = NULL
			 RETURNING submission_id`,
			[
				input.submissionId,
				input.sessionKey,
				input.kind,
				input.attemptId,
				input.operationId,
				input.turnId,
				input.phase,
				now,
				now,
				input.checkpointLeafId ?? null,
				input.toolRequest === undefined ? null : JSON.stringify(input.toolRequest),
			],
		);
		return rows.length > 0;
	}

	async updateTurnJournalPhase(
		attempt: SubmissionAttemptRef,
		phase: AgentTurnJournalPhase,
		options: { checkpointLeafId?: string; toolRequest?: unknown; streamKey?: string } = {},
	): Promise<boolean> {
		const rows = await this.db.query(
			`UPDATE flue_agent_turn_journals
			 SET phase = ?, revision = revision + 1, updated_at = ?,
			     checkpoint_leaf_id = COALESCE(?, checkpoint_leaf_id),
			     tool_request_json = COALESCE(?, tool_request_json),
			     stream_key = COALESCE(?, stream_key)
			 WHERE submission_id = ? AND attempt_id = ? AND committed = 0
			 RETURNING submission_id`,
			[
				phase,
				Date.now(),
				options.checkpointLeafId ?? null,
				options.toolRequest === undefined ? null : JSON.stringify(options.toolRequest),
				options.streamKey ?? null,
				attempt.submissionId,
				attempt.attemptId,
			],
		);
		return rows.length > 0;
	}

	async commitTurnJournal(
		attempt: SubmissionAttemptRef,
		committedLeafId: string,
	): Promise<boolean> {
		const rows = await this.db.query(
			`UPDATE flue_agent_turn_journals
			 SET phase = 'committed', revision = revision + 1, updated_at = ?,
			     committed = 1, committed_leaf_id = ?
			 WHERE submission_id = ? AND attempt_id = ? AND committed = 0
			 RETURNING submission_id`,
			[Date.now(), committedLeafId, attempt.submissionId, attempt.attemptId],
		);
		return rows.length > 0;
	}

	async markStreamConsumed(attempt: SubmissionAttemptRef, streamKey: string): Promise<boolean> {
		const now = Date.now();
		const rows = await this.db.query(
			`UPDATE flue_agent_turn_journals
			 SET revision = revision + 1, updated_at = ?, stream_consumed_at = ?
			 WHERE submission_id = ? AND attempt_id = ? AND committed = 0
			   AND stream_key = ? AND stream_consumed_at IS NULL
			 RETURNING submission_id`,
			[now, now, attempt.submissionId, attempt.attemptId, streamKey],
		);
		return rows.length > 0;
	}

	async replaceTurnJournalAttempt(
		attempt: SubmissionAttemptRef,
		nextAttemptId: string,
		lease?: { ownerId: string; leaseExpiresAt: number },
	): Promise<AgentSubmission | null> {
		return this.db.transaction(async (tx) => {
			const now = Date.now();
			const subRows = lease
				? await tx.query(
						`UPDATE flue_agent_submissions
						 SET attempt_id = ?, recovery_requested_at = NULL, started_at = ?, attempt_count = attempt_count + 1,
						     owner_id = ?, lease_expires_at = ?
						 WHERE submission_id = ? AND status = 'running' AND attempt_id = ?
						 RETURNING ${submissionColumns}`,
						[
							nextAttemptId,
							now,
							lease.ownerId,
							lease.leaseExpiresAt,
							attempt.submissionId,
							attempt.attemptId,
						],
					)
				: await tx.query(
						`UPDATE flue_agent_submissions
						 SET attempt_id = ?, recovery_requested_at = NULL, started_at = ?, attempt_count = attempt_count + 1
						 WHERE submission_id = ? AND status = 'running' AND attempt_id = ?
						 RETURNING ${submissionColumns}`,
						[nextAttemptId, now, attempt.submissionId, attempt.attemptId],
					);
			const row = subRows[0];
			if (!row) return null;
			await tx.query(
				`UPDATE flue_agent_turn_journals
				 SET attempt_id = ?, revision = revision + 1, updated_at = ?
				 WHERE submission_id = ? AND attempt_id = ? AND committed = 0`,
				[nextAttemptId, now, attempt.submissionId, attempt.attemptId],
			);
			return parseSubmission(
				row,
				await createChunkStore(tx).read(submissionChunkOwner(attempt.submissionId)),
			);
		});
	}

	async appendStreamChunkSegment(
		streamKey: string,
		segmentIndex: number,
		body: string,
	): Promise<boolean> {
		const rows = await this.db.query(
			`INSERT INTO flue_agent_stream_chunks (stream_key, segment_index, body)
			 VALUES (?, ?, ?)
			 ON CONFLICT (stream_key, segment_index) DO NOTHING
			 RETURNING stream_key`,
			[streamKey, segmentIndex, body],
		);
		return rows.length > 0;
	}

	async getStreamChunkSegments(
		streamKey: string,
	): Promise<Array<{ segmentIndex: number; body: string }>> {
		const rows = await this.db.query(
			`SELECT segment_index, body
			 FROM flue_agent_stream_chunks
			 WHERE stream_key = ?
			 ORDER BY segment_index ASC`,
			[streamKey],
		);
		return rows.map((row) => {
			const segmentIndex = Number(row.segment_index);
			if (!Number.isInteger(segmentIndex) || typeof row.body !== 'string') {
				throw new Error('[flue] Persisted stream chunk row is malformed.');
			}
			return { segmentIndex, body: row.body };
		});
	}

	async deleteStreamChunkSegments(streamKey: string): Promise<void> {
		await this.db.query('DELETE FROM flue_agent_stream_chunks WHERE stream_key = ?', [streamKey]);
	}

	async admitDispatch(input: DispatchInput): Promise<AgentDispatchAdmission> {
		return this.admitSubmission(createDispatchAgentSubmissionInput(input));
	}

	async admitDirect(input: DirectAgentSubmissionInput): Promise<AgentSubmission> {
		const admission = await this.admitSubmission(input);
		if (admission.kind !== 'submission') {
			throw new Error('[flue] Internal direct admission returned an unexpected result.');
		}
		return admission.submission;
	}

	async claimSubmission(claim: SubmissionClaimRef): Promise<AgentSubmission | null> {
		const now = Date.now();
		const timeoutAt = now + DURABILITY_DEFAULT_TIMEOUT_MS;
		return this.db.transaction(async (tx) => {
			const rows = await tx.query(
				`UPDATE flue_agent_submissions
				 SET status = 'running',
				     attempt_id = ?,
				     started_at = ?,
				     attempt_count = attempt_count + 1,
				     max_retry = ?,
				     timeout_at = CASE WHEN timeout_at = 0 THEN ? ELSE timeout_at END,
				     owner_id = ?,
				     lease_expires_at = ?
				 WHERE submission_id = ?
				   AND status = 'queued'
				   AND NOT EXISTS (
				     SELECT 1
				     FROM flue_agent_submissions AS earlier
				     WHERE earlier.session_key = flue_agent_submissions.session_key
				       AND earlier.status IN ('queued', 'running')
				       AND earlier.sequence < flue_agent_submissions.sequence
				   )
				 RETURNING ${submissionColumns}`,
				[
					claim.attemptId,
					now,
					DURABILITY_DEFAULT_MAX_ATTEMPTS,
					timeoutAt,
					claim.ownerId,
					claim.leaseExpiresAt,
					claim.submissionId,
				],
			);
			return rows[0]
				? parseSubmission(
						rows[0],
						await createChunkStore(tx).read(submissionChunkOwner(claim.submissionId)),
					)
				: null;
		});
	}

	async markSubmissionInputApplied(
		attempt: SubmissionAttemptRef,
		durability?: SubmissionDurability,
	): Promise<boolean> {
		const now = Date.now();
		const rows = await this.db.query(
			`UPDATE flue_agent_submissions
			 SET input_applied_at = COALESCE(input_applied_at, ?),
			     max_retry = CASE WHEN input_applied_at IS NULL THEN ? ELSE max_retry END,
			     timeout_at = CASE WHEN input_applied_at IS NULL THEN ? ELSE timeout_at END
			 WHERE submission_id = ? AND status = 'running' AND attempt_id = ?
			 RETURNING submission_id`,
			[
				now,
				durability?.maxRetry ?? DURABILITY_DEFAULT_MAX_ATTEMPTS,
				durability?.timeoutAt ?? now + DURABILITY_DEFAULT_TIMEOUT_MS,
				attempt.submissionId,
				attempt.attemptId,
			],
		);
		return rows.length > 0;
	}

	async requestSubmissionRecovery(attempt: SubmissionAttemptRef): Promise<boolean> {
		const rows = await this.db.query(
			`UPDATE flue_agent_submissions
			 SET recovery_requested_at = COALESCE(recovery_requested_at, ?)
			 WHERE submission_id = ? AND status = 'running' AND attempt_id = ?
			 RETURNING submission_id`,
			[Date.now(), attempt.submissionId, attempt.attemptId],
		);
		return rows.length > 0;
	}

	async requeueSubmissionBeforeInputApplied(attempt: SubmissionAttemptRef): Promise<boolean> {
		const rows = await this.db.query(
			`UPDATE flue_agent_submissions
			 SET status = 'queued', attempt_id = NULL, recovery_requested_at = NULL,
			     started_at = NULL, owner_id = NULL, lease_expires_at = 0
			 WHERE submission_id = ? AND status = 'running'
			   AND attempt_id = ? AND input_applied_at IS NULL
			 RETURNING submission_id`,
			[attempt.submissionId, attempt.attemptId],
		);
		return rows.length > 0;
	}

	async completeSubmission(attempt: SubmissionAttemptRef): Promise<boolean> {
		const rows = await this.db.query(
			`UPDATE flue_agent_submissions
			 SET status = 'settled', settled_at = ?, error = NULL
			 WHERE submission_id = ? AND status = 'running' AND attempt_id = ?
			 RETURNING submission_id`,
			[Date.now(), attempt.submissionId, attempt.attemptId],
		);
		return rows.length > 0;
	}

	async failSubmission(attempt: SubmissionAttemptRef, error: unknown): Promise<boolean> {
		const rows = await this.db.query(
			`UPDATE flue_agent_submissions
			 SET status = 'settled', settled_at = ?, error = ?
			 WHERE submission_id = ? AND status = 'running' AND attempt_id = ?
			 RETURNING submission_id`,
			[
				Date.now(),
				error instanceof Error ? error.message : String(error),
				attempt.submissionId,
				attempt.attemptId,
			],
		);
		return rows.length > 0;
	}

	async insertAttemptMarker(attempt: SubmissionAttemptRef): Promise<void> {
		await this.db.query(
			`INSERT INTO flue_agent_attempt_markers (submission_id, attempt_id, created_at)
			 VALUES (?, ?, ?)
			 ON CONFLICT (submission_id, attempt_id) DO NOTHING`,
			[attempt.submissionId, attempt.attemptId, Date.now()],
		);
	}

	async deleteAttemptMarker(attempt: SubmissionAttemptRef): Promise<void> {
		await this.db.query(
			'DELETE FROM flue_agent_attempt_markers WHERE submission_id = ? AND attempt_id = ?',
			[attempt.submissionId, attempt.attemptId],
		);
	}

	async listAttemptMarkers(): Promise<AgentAttemptMarker[]> {
		const rows = await this.db.query(
			'SELECT submission_id, attempt_id, created_at FROM flue_agent_attempt_markers',
		);
		return rows.map((row) => {
			const createdAt = Number(row.created_at);
			if (
				typeof row.submission_id !== 'string' ||
				typeof row.attempt_id !== 'string' ||
				!Number.isFinite(createdAt)
			) {
				throw new Error('[flue] Persisted attempt marker row is malformed.');
			}
			return { submissionId: row.submission_id, attemptId: row.attempt_id, createdAt };
		});
	}

	async renewLeases(ownerId: string, submissionIds: string[]): Promise<void> {
		if (submissionIds.length === 0) return;
		const placeholders = submissionIds.map(() => '?').join(', ');
		await this.db.query(
			`UPDATE flue_agent_submissions
			 SET lease_expires_at = ?
			 WHERE owner_id = ? AND status = 'running'
			   AND submission_id IN (${placeholders})`,
			[Date.now() + LEASE_DURATION_MS, ownerId, ...submissionIds],
		);
	}

	async listExpiredSubmissions(): Promise<AgentSubmission[]> {
		return this.db.transaction(async (tx) => {
			const rows = await tx.query(
				`SELECT ${submissionColumns}
				 FROM flue_agent_submissions
				 WHERE status = 'running' AND lease_expires_at > 0 AND lease_expires_at < ?
				 ORDER BY sequence ASC`,
				[Date.now()],
			);
			return this.parseOperationalRows(rows, 'active', tx);
		});
	}

	deleteSession(sessionKey: string, deleteSessionTree: () => Promise<void>): Promise<void> {
		const existing = this.pendingSessionDeletions.get(sessionKey);
		if (existing) return existing;
		const deletion = this.runSessionDeletion(sessionKey, deleteSessionTree);
		this.pendingSessionDeletions.set(sessionKey, deletion);
		const clear = () => {
			if (this.pendingSessionDeletions.get(sessionKey) === deletion) {
				this.pendingSessionDeletions.delete(sessionKey);
			}
		};
		void deletion.then(clear, clear);
		return deletion;
	}

	async listPendingSessionDeletions(): Promise<string[]> {
		const rows = await this.db.query('SELECT session_key FROM flue_agent_session_deletions');
		return rows.map((row) => String(row.session_key));
	}

	private async admitSubmission(
		input: DispatchAgentSubmissionInput | DirectAgentSubmissionInput,
	): Promise<AgentDispatchAdmission> {
		const { kind, submissionId } = input;
		const prepared =
			kind === 'direct' ? prepareDirectSubmission(input) : { value: input, chunks: [] };
		const payload = JSON.stringify(prepared.value);
		const acceptedAt = parseAcceptedAt(input.acceptedAt, `${kind} admission`);
		const sessionKey = createSubmissionSessionKey(input.id);

		return this.db.transaction(async (tx) => {
			const chunkStore = createChunkStore(tx);
			if (kind === 'dispatch') {
				const receiptRows = await tx.query(
					'SELECT dispatch_id, accepted_at FROM flue_agent_dispatch_receipts WHERE dispatch_id = ? LIMIT 1',
					[submissionId],
				);
				if (receiptRows[0]) {
					return { kind: 'retained_receipt', receipt: parseDispatchReceipt(receiptRows[0]) };
				}
			}

			const deletingRows = await tx.query(
				'SELECT 1 FROM flue_agent_session_deletions WHERE session_key = ? LIMIT 1',
				[sessionKey],
			);
			if (deletingRows.length > 0) {
				throw new Error(
					'[flue] Durable agent submission admission is unavailable while this session is being deleted. Retry after deletion completes.',
				);
			}

			await tx.query(
				`INSERT OR IGNORE INTO flue_agent_submissions
				 (submission_id, session_key, kind, payload, status, accepted_at)
				 VALUES (?, ?, ?, ?, 'queued', ?)`,
				[submissionId, sessionKey, kind, payload, acceptedAt],
			);

			const readRows = await tx.query(
				`SELECT ${submissionColumns} FROM flue_agent_submissions WHERE submission_id = ? LIMIT 1`,
				[submissionId],
			);
			const row = readRows[0];
			if (!row)
				throw new Error(`[flue] Durable ${kind} admission did not create a submission row.`);
			if (row.kind !== kind) return { kind: 'conflict' };
			const owner = submissionChunkOwner(submissionId);
			if (row.payload !== payload) {
				const persistedChunks = await chunkStore.read(owner);
				if (
					kind !== 'direct' ||
					typeof row.payload !== 'string' ||
					!matchesPersistedDirectSubmission(
						input,
						JSON.parse(row.payload) as DirectAgentSubmissionInput,
						persistedChunks,
					)
				) {
					return { kind: 'conflict' };
				}
				return { kind: 'submission', submission: parseSubmission(row, persistedChunks) };
			}
			const persistedChunks = await chunkStore.read(owner);
			if (persistedChunks.length === 0 && prepared.chunks.length > 0) {
				await chunkStore.replace(owner, prepared.chunks);
			} else if (!samePersistedChunks(persistedChunks, prepared.chunks)) {
				return { kind: 'conflict' };
			}
			return { kind: 'submission', submission: parseSubmission(row, prepared.chunks) };
		});
	}

	private async runSessionDeletion(
		sessionKey: string,
		deleteSessionTree: () => Promise<void>,
	): Promise<void> {
		const startedAt = Date.now();
		await this.db.transaction(async (tx) => {
			const active = await tx.query(
				`SELECT 1 FROM flue_agent_submissions
				 WHERE session_key = ? AND status IN ('queued', 'running')
				 LIMIT 1`,
				[sessionKey],
			);
			if (active.length > 0) {
				throw new Error(
					'[flue] Session cannot be deleted while durable agent submissions are queued or running. Wait for accepted work to settle, then retry deletion.',
				);
			}
			await tx.query(
				`INSERT OR IGNORE INTO flue_agent_session_deletions (session_key, started_at) VALUES (?, ?)`,
				[sessionKey, startedAt],
			);
		});

		try {
			await deleteSessionTree();
		} catch (error) {
			await this.db.query('DELETE FROM flue_agent_session_deletions WHERE session_key = ?', [
				sessionKey,
			]);
			throw error;
		}

		await this.db.transaction(async (tx) => {
			const deletionRows = await tx.query(
				'SELECT started_at FROM flue_agent_session_deletions WHERE session_key = ?',
				[sessionKey],
			);
			const deletionRow = deletionRows[0];
			const markerStartedAt = deletionRow ? Number(deletionRow.started_at) : NaN;
			if (!deletionRow || !Number.isFinite(markerStartedAt)) return;
			await tx.query(
				`INSERT OR IGNORE INTO flue_agent_dispatch_receipts (dispatch_id, accepted_at)
				 SELECT submission_id, accepted_at
				 FROM flue_agent_submissions
				 WHERE session_key = ? AND kind = 'dispatch' AND status = 'settled'
				   AND accepted_at <= ?`,
				[sessionKey, markerStartedAt],
			);
			await tx.query(
				`DELETE FROM flue_agent_stream_chunks
				 WHERE stream_key IN (
				   SELECT j.stream_key FROM flue_agent_turn_journals j
				   INNER JOIN flue_agent_submissions s ON j.submission_id = s.submission_id
				   WHERE s.session_key = ? AND s.status = 'settled' AND s.accepted_at <= ?
				     AND j.stream_key IS NOT NULL
				 )`,
				[sessionKey, markerStartedAt],
			);
			await tx.query(
				`DELETE FROM flue_agent_turn_journals
				 WHERE submission_id IN (
				   SELECT submission_id FROM flue_agent_submissions
				   WHERE session_key = ? AND status = 'settled' AND accepted_at <= ?
				 )`,
				[sessionKey, markerStartedAt],
			);
			const deletedSubmissionRows = await tx.query(
				`SELECT submission_id FROM flue_agent_submissions
				 WHERE session_key = ? AND status = 'settled' AND accepted_at <= ?`,
				[sessionKey, markerStartedAt],
			);
			const submissionOwners = deletedSubmissionRows.flatMap((row) =>
				typeof row.submission_id === 'string' ? [submissionChunkOwner(row.submission_id)] : [],
			);
			await createChunkStore(tx).deleteMany(submissionOwners);
			await tx.query(
				`DELETE FROM flue_agent_submissions
				 WHERE session_key = ? AND status = 'settled' AND accepted_at <= ?`,
				[sessionKey, markerStartedAt],
			);
			await tx.query('DELETE FROM flue_agent_session_deletions WHERE session_key = ?', [
				sessionKey,
			]);
		});
	}

	private async parseOperationalRows(
		rows: AsyncSqlRow[],
		status: 'queued' | 'active',
		runner: AsyncSqlRunner,
	): Promise<AgentSubmission[]> {
		const submissions: AgentSubmission[] = [];
		const chunkStore = createChunkStore(runner);
		for (const row of rows) {
			try {
				submissions.push(
					parseSubmission(
						row,
						await chunkStore.read(submissionChunkOwner(String(row.submission_id))),
					),
				);
			} catch (error) {
				const sequence = Number(row.sequence);
				if (!Number.isFinite(sequence)) throw error;
				await failSubmissionSequence(runner, sequence, status, error);
			}
		}
		return submissions;
	}
}

function parseDispatchReceipt(row: AsyncSqlRow): { submissionId: string; acceptedAt: number } {
	const acceptedAt = Number(row.accepted_at);
	if (typeof row.dispatch_id !== 'string' || !Number.isFinite(acceptedAt)) {
		throw new Error('[flue] Persisted dispatch receipt row is malformed.');
	}
	return { submissionId: row.dispatch_id, acceptedAt };
}

function parseSubmission(
	row: AsyncSqlRow,
	chunks: readonly PersistedChunkRow[],
): AgentSubmission {
	const sequence = Number(row.sequence);
	const acceptedAt = Number(row.accepted_at);
	const attemptCount = Number(row.attempt_count);
	const maxRetry = Number(row.max_retry);
	const timeoutAt = Number(row.timeout_at);
	const leaseExpiresAt = Number(row.lease_expires_at);
	const attemptId = row.attempt_id != null ? String(row.attempt_id) : undefined;
	const inputAppliedAt = row.input_applied_at != null ? Number(row.input_applied_at) : undefined;
	const recoveryRequestedAt =
		row.recovery_requested_at != null ? Number(row.recovery_requested_at) : undefined;
	const startedAt = row.started_at != null ? Number(row.started_at) : undefined;
	const ownerId = row.owner_id != null ? String(row.owner_id) : undefined;

	if (
		!Number.isFinite(sequence) ||
		typeof row.submission_id !== 'string' ||
		typeof row.session_key !== 'string' ||
		(row.kind !== 'dispatch' && row.kind !== 'direct') ||
		typeof row.payload !== 'string' ||
		(row.status !== 'queued' && row.status !== 'running' && row.status !== 'settled') ||
		!Number.isFinite(acceptedAt) ||
		(row.status === 'queued' &&
			(attemptId !== undefined ||
				inputAppliedAt !== undefined ||
				recoveryRequestedAt !== undefined ||
				startedAt !== undefined)) ||
		(row.status === 'running' && (attemptId === undefined || startedAt === undefined)) ||
		!Number.isFinite(attemptCount) ||
		!Number.isFinite(maxRetry) ||
		!Number.isFinite(timeoutAt) ||
		!Number.isFinite(leaseExpiresAt)
	) {
		throw new Error('[flue] Persisted agent submission row is malformed.');
	}

	const parsedPayload = JSON.parse(row.payload) as unknown;
	const input =
		row.kind === 'direct'
			? hydratePersistedDirectSubmission(parsedPayload as DirectAgentSubmissionInput, chunks)
			: parsedPayload;
	if (
		!isSubmissionPayload(input, {
			kind: row.kind,
			submissionId: row.submission_id,
			sessionKey: row.session_key,
			acceptedAt,
		})
	) {
		throw new Error('[flue] Persisted agent submission payload is malformed.');
	}

	const error = row.error != null ? String(row.error) : undefined;
	return {
		sequence,
		submissionId: row.submission_id,
		sessionKey: row.session_key,
		kind: row.kind,
		input,
		status: row.status,
		acceptedAt,
		...(attemptId !== undefined ? { attemptId } : {}),
		...(inputAppliedAt !== undefined ? { inputAppliedAt } : {}),
		...(recoveryRequestedAt !== undefined ? { recoveryRequestedAt } : {}),
		...(startedAt !== undefined ? { startedAt } : {}),
		...(error !== undefined ? { error } : {}),
		attemptCount,
		maxRetry,
		timeoutAt,
		...(ownerId !== undefined ? { ownerId } : {}),
		leaseExpiresAt,
	};
}

function parseTurnJournal(row: AsyncSqlRow): AgentTurnJournal {
	const revision = Number(row.revision);
	const createdAt = Number(row.created_at);
	const updatedAt = Number(row.updated_at);
	const committed = Number(row.committed);
	const streamConsumedAt =
		row.stream_consumed_at != null ? Number(row.stream_consumed_at) : undefined;

	if (
		typeof row.submission_id !== 'string' ||
		typeof row.session_key !== 'string' ||
		(row.kind !== 'dispatch' && row.kind !== 'direct') ||
		typeof row.attempt_id !== 'string' ||
		typeof row.operation_id !== 'string' ||
		typeof row.turn_id !== 'string' ||
		(row.phase !== 'before_provider' &&
			row.phase !== 'provider_started' &&
			row.phase !== 'tool_request_recorded' &&
			row.phase !== 'committed') ||
		!Number.isFinite(revision) ||
		!Number.isFinite(createdAt) ||
		!Number.isFinite(updatedAt) ||
		(row.checkpoint_leaf_id != null && typeof row.checkpoint_leaf_id !== 'string') ||
		(row.stream_key != null && typeof row.stream_key !== 'string') ||
		(streamConsumedAt !== undefined && !Number.isFinite(streamConsumedAt)) ||
		(committed !== 0 && committed !== 1) ||
		(row.committed_leaf_id != null && typeof row.committed_leaf_id !== 'string')
	) {
		throw new Error('[flue] Persisted turn journal row is malformed.');
	}

	return {
		submissionId: row.submission_id,
		sessionKey: row.session_key,
		kind: row.kind,
		attemptId: row.attempt_id,
		operationId: row.operation_id,
		turnId: row.turn_id,
		phase: row.phase,
		revision,
		createdAt,
		updatedAt,
		...(typeof row.checkpoint_leaf_id === 'string'
			? { checkpointLeafId: row.checkpoint_leaf_id }
			: {}),
		...(typeof row.tool_request_json === 'string'
			? { toolRequest: JSON.parse(row.tool_request_json) as unknown }
			: {}),
		...(typeof row.stream_key === 'string' ? { streamKey: row.stream_key } : {}),
		...(streamConsumedAt !== undefined ? { streamConsumedAt } : {}),
		committed: committed === 1,
		...(typeof row.committed_leaf_id === 'string'
			? { committedLeafId: row.committed_leaf_id }
			: {}),
	};
}

async function failSubmissionSequence(
	runner: AsyncSqlRunner,
	sequence: number,
	status: 'queued' | 'active',
	error: unknown,
): Promise<void> {
	const statusFilter = status === 'queued' ? "status = 'queued'" : "status = 'running'";
	await runner.query(
		`UPDATE flue_agent_submissions
		 SET status = 'settled', settled_at = ?, error = ?
		 WHERE sequence = ? AND ${statusFilter}`,
		[Date.now(), error instanceof Error ? error.message : String(error), sequence],
	);
}
