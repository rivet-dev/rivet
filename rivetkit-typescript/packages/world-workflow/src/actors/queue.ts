/**
 * Queue actor.
 *
 * One actor per `ValidQueueName` (e.g. `__wkf_workflow_0`). Messages are
 * persisted in actor state so they survive restarts and hibernation. A
 * follow-up can switch the pending list to an actor-owned durable queue
 * (`queue<T>()`) if we want concurrent consumers.
 *
 * Responsibilities:
 *
 * - Accept `queue()` calls and enqueue messages with idempotency and retry
 *   tracking.
 * - Dispatch messages to a handler registered at runtime via an HTTP
 *   endpoint. The handler is owned by user code; the queue actor only calls
 *   back into it.
 * - Apply retry backoff on failed messages up to the configured policy.
 *
 * Handler dispatch is driven by the `processNext` action which the host
 * `createQueueHandler` HTTP wrapper invokes to pull one message at a time.
 */

import { actor } from "rivetkit";
import { v4 as uuidv4 } from "uuid";
import type { ValidQueueName } from "../types";

interface QueueRetryPolicy {
	maxAttempts?: number;
	initialBackoffMs?: number;
	maxBackoffMs?: number;
	backoffMultiplier?: number;
}

interface PendingMessage {
	id: string;
	queueName: ValidQueueName;
	payload: unknown;
	attempt: number;
	createdAt: number;
	deliverAfter: number;
	idempotencyKey?: string;
	retryPolicy?: QueueRetryPolicy;
}

interface QueueActorState {
	queueName?: ValidQueueName;
	pending: PendingMessage[];
	idempotencyKeys: Record<string, string>;
	inFlight: Record<string, PendingMessage>;
}

function computeBackoffMs(attempt: number, policy?: QueueRetryPolicy): number {
	const initial = policy?.initialBackoffMs ?? 1_000;
	const max = policy?.maxBackoffMs ?? 60_000;
	const multiplier = policy?.backoffMultiplier ?? 2;
	return Math.min(max, initial * multiplier ** Math.max(attempt - 1, 0));
}

export const queueActor = actor({
	state: {
		pending: [],
		idempotencyKeys: {},
		inFlight: {},
	} as QueueActorState,
	actions: {
		/**
		 * Enqueue a message. Returns its stable message id.
		 */
		enqueue: (
			c,
			queueName: ValidQueueName,
			payload: unknown,
			opts?: {
				idempotencyKey?: string;
				delay?: number;
				retryPolicy?: QueueRetryPolicy;
			},
		): { messageId: string } => {
			c.state.queueName ??= queueName;

			if (opts?.idempotencyKey) {
				const existing = c.state.idempotencyKeys[opts.idempotencyKey];
				if (existing) {
					return { messageId: existing };
				}
			}

			const now = Date.now();
			const msg: PendingMessage = {
				id: uuidv4(),
				queueName,
				payload,
				attempt: 0,
				createdAt: now,
				deliverAfter: now + (opts?.delay ?? 0),
				idempotencyKey: opts?.idempotencyKey,
				retryPolicy: opts?.retryPolicy,
			};

			c.state.pending.push(msg);
			if (opts?.idempotencyKey) {
				c.state.idempotencyKeys[opts.idempotencyKey] = msg.id;
			}
			return { messageId: msg.id };
		},

		/**
		 * Pull the next deliverable message. Returns null if none are ready.
		 * The caller is expected to process the message then call `ack` or
		 * `nack`. This powers the HTTP `createQueueHandler` dispatcher loop.
		 */
		claimNext: (c): PendingMessage | null => {
			const now = Date.now();
			const idx = c.state.pending.findIndex(
				(m) => m.deliverAfter <= now,
			);
			if (idx === -1) return null;
			const [msg] = c.state.pending.splice(idx, 1);
			msg.attempt += 1;
			c.state.inFlight[msg.id] = msg;
			return msg;
		},

		/** Mark a message as successfully processed. */
		ack: (c, messageId: string) => {
			const msg = c.state.inFlight[messageId];
			if (!msg) return;
			delete c.state.inFlight[messageId];
			if (msg.idempotencyKey) {
				// Keep idempotency mapping for dedup window. In practice this
				// should age out; we keep it indefinitely for now.
			}
		},

		/** Mark a message as failed and schedule a retry (or drop). */
		nack: (c, messageId: string, err?: unknown) => {
			const msg = c.state.inFlight[messageId];
			if (!msg) return;
			delete c.state.inFlight[messageId];
			const maxAttempts = msg.retryPolicy?.maxAttempts ?? 5;
			if (msg.attempt >= maxAttempts) {
				c.log.warn({
					msg: "queue message exceeded max attempts",
					messageId,
					queueName: msg.queueName,
					attempt: msg.attempt,
					err: err ?? null,
				});
				return;
			}
			msg.deliverAfter =
				Date.now() + computeBackoffMs(msg.attempt, msg.retryPolicy);
			c.state.pending.push(msg);
		},

		stats: (c) => ({
			queueName: c.state.queueName,
			pending: c.state.pending.length,
			inFlight: Object.keys(c.state.inFlight).length,
		}),
	},
});
