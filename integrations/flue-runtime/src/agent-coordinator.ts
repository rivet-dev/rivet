import type {
	AgentExecutionStore,
	AgentSubmission,
	AgentSubmissionObserverRegistry,
	AgentSubmissionStore,
	EventStreamStore,
	FlueContextInternal,
} from '@flue/runtime/adapter-kit';
import type { AttachedAgentEvent, CreatedAgent, PromptImage } from '@flue/runtime';
import {
	agentStreamPath,
	assertAgentDispatchAdmissionInput,
	createAgentSubmissionObserverRegistry,
	createDirectAgentSubmissionInput,
	deleteSessionTree,
	handleAgentRequest,
	handleStreamHead,
	handleStreamRead,
	isStreamExcludedEvent,
	processSubmission,
	reconcileInterruptedSubmission,
	submissionSyntheticRequest,
} from '@flue/runtime/adapter-kit';
import { runWithRivetContext, type RivetActorIdentity, type RivetContext } from './context.js';
import { createAsyncSqlStores, ensureAsyncSqlSchema, type AsyncSqlDb } from './stores/index.js';

export const RIVET_AGENT_INTERNAL_DISPATCH_PATH = '/__flue/internal/dispatch';

const FLUE_AGENT_SUBMISSION_WAKE_ACTION = '__flueWakeAgentSubmissions';
const FLUE_AGENT_SUBMISSION_WAKE_SECONDS = 30;

export interface RivetAgentActorContext {
	readonly actorId: string;
	readonly name: string;
	readonly key: readonly string[];
	readonly region: string;
	readonly env?: Record<string, unknown>;
	readonly request?: Request;
	readonly abortSignal: AbortSignal;
	readonly db: AsyncSqlDb;
	readonly schedule?: {
		after(duration: number, action: string, ...args: unknown[]): Promise<void>;
	};
	keepAwake<T>(promise: Promise<T>): Promise<T>;
}

interface RivetAgentPreparedCoordinator {
	readonly agentName: string;
	readonly executionStore: AgentExecutionStore;
}

export interface RivetAgentRuntimeOptions {
	readonly createdAgents: Record<string, CreatedAgent>;
	readonly createContext: (options: {
		readonly executionStore: AgentExecutionStore;
		readonly actor: RivetAgentActorContext;
		readonly payload: unknown;
		readonly request: Request;
		readonly initialEventIndex?: number;
		readonly dispatchId?: string;
	}) => FlueContextInternal;
	readonly createEventStreamStore: (actor: RivetAgentActorContext) => EventStreamStore;
	readonly resolveInstanceId?: (actor: RivetAgentActorContext) => string;
}

export interface RivetAgentRuntime {
	prepare(options: {
		readonly db: AsyncSqlDb;
		readonly agentName: string;
	}): Promise<RivetAgentPreparedCoordinator>;
	attach(
		actor: RivetAgentActorContext,
		prepared: RivetAgentPreparedCoordinator,
	): RivetAgentCoordinator;
	onWake(actor: RivetAgentActorContext, inherited?: () => Promise<unknown> | unknown): Promise<void>;
	wakeSubmissions(actor: RivetAgentActorContext): Promise<void>;
	onRequest(actor: RivetAgentActorContext, request: Request): Promise<Response | null>;
}

type DirectAgentPayload = {
	message: string;
	images?: PromptImage[];
};

export function createRivetAgentRuntime(options: RivetAgentRuntimeOptions): RivetAgentRuntime {
	const coordinators = new WeakMap<RivetAgentActorContext, RivetAgentCoordinator>();
	const observers = createAgentSubmissionObserverRegistry();
	const activeAttempts = new Set<string>();

	const getCoordinator = (actor: RivetAgentActorContext): RivetAgentCoordinator => {
		const coordinator = coordinators.get(actor);
		if (!coordinator) {
			throw new Error('[flue] Generated Rivet agent coordinator was not initialized.');
		}
		return coordinator;
	};

	return {
		async prepare({ db, agentName }) {
			await ensureAsyncSqlSchema(db);
			return {
				agentName,
				executionStore: createAsyncSqlStores(db).executionStore,
			};
		},
		attach(actor, prepared) {
			const coordinator = new RivetAgentCoordinator(
				actor,
				prepared,
				options,
				options.createEventStreamStore(actor),
				observers,
				activeAttempts,
			);
			coordinators.set(actor, coordinator);
			return coordinator;
		},
		onWake(actor, inherited = () => undefined) {
			return getCoordinator(actor).onWake(inherited);
		},
		wakeSubmissions(actor) {
			return getCoordinator(actor).wakeSubmissions();
		},
		onRequest(actor, request) {
			return getCoordinator(actor).onRequest(request);
		},
	};
}

export class RivetAgentCoordinator {
	private readonly actor: RivetAgentActorContext;
	private readonly prepared: RivetAgentPreparedCoordinator;
	private readonly options: RivetAgentRuntimeOptions;
	private readonly eventStreamStore: EventStreamStore;
	private readonly observers: AgentSubmissionObserverRegistry;
	private readonly activeAttempts: Set<string>;
	private readonly pendingEventWrites = new Set<Promise<void>>();

	constructor(
		actor: RivetAgentActorContext,
		prepared: RivetAgentPreparedCoordinator,
		options: RivetAgentRuntimeOptions,
		eventStreamStore: EventStreamStore,
		observers: AgentSubmissionObserverRegistry,
		activeAttempts: Set<string>,
	) {
		this.actor = actor;
		this.prepared = prepared;
		this.options = options;
		this.eventStreamStore = eventStreamStore;
		this.observers = observers;
		this.activeAttempts = activeAttempts;
	}

	async onWake(inherited: () => Promise<unknown> | unknown): Promise<void> {
		await this.restoreSubmissionWake();
		await this.runWithActorContext(() => inherited());
		await this.resumePendingSessionDeletions();
		await this.reconcileSubmissions({ driverAlreadyArmed: true });
	}

	async wakeSubmissions(): Promise<void> {
		if (!(await this.submissions.hasUnsettledSubmissions())) return;
		await this.armSubmissionWake();
		await this.reconcileSubmissions({ driverAlreadyArmed: true });
	}

	async onRequest(request: Request): Promise<Response | null> {
		if (isInternalDispatchRequest(request)) return this.admitDispatch(request);

		if (request.method === 'GET' || request.method === 'HEAD') {
			const streamPath = agentStreamPath(this.agentName, this.instanceId);
			if (request.method === 'HEAD') return handleStreamHead(this.eventStreamStore, streamPath);
			return handleStreamRead({ store: this.eventStreamStore, path: streamPath, request });
		}

		return this.runWithActorContext(() =>
			handleAgentRequest({
				request,
				id: this.instanceId,
				agentName: this.agentName,
				eventStreamStore: this.eventStreamStore,
				admitAttachedSubmission: (payload, onEvent, waitForResult) =>
					this.admitAttachedSubmission(payload, onEvent, waitForResult),
			}),
		);
	}

	private get agentName(): string {
		return this.prepared.agentName;
	}

	private get executionStore(): AgentExecutionStore {
		return this.prepared.executionStore;
	}

	private get submissions(): AgentSubmissionStore {
		return this.executionStore.submissions;
	}

	private get instanceId(): string {
		return this.options.resolveInstanceId?.(this.actor) ?? this.actor.key[0] ?? this.actor.actorId;
	}

	private runWithActorContext<T>(callback: () => T): T {
		const context: RivetContext = {
			identity: createIdentity(this.actor),
			env: this.actor.env ?? {},
		};
		return runWithRivetContext(context, callback);
	}

	private createContext(
		payload: unknown,
		request: Request,
		initialEventIndex?: number,
		dispatchId?: string,
	): FlueContextInternal {
		return this.options.createContext({
			executionStore: this.executionStore,
			actor: this.actor,
			payload,
			request,
			initialEventIndex,
			dispatchId,
		});
	}

	private createDurableContext(
		payload: unknown,
		request: Request,
		dispatchId?: string,
	): FlueContextInternal {
		const ctx = this.createContext(payload, request, undefined, dispatchId);
		const streamPath = agentStreamPath(this.agentName, this.instanceId);
		ctx.subscribeEvent((event) => {
			if (isStreamExcludedEvent(event)) return;
			let write!: Promise<void>;
			write = this.eventStreamStore
				.appendEvent(streamPath, event)
				.then(
					() => {},
					(error) => {
						console.error('[flue:event-stream] appendEvent failed:', error);
					},
				)
				.finally(() => this.pendingEventWrites.delete(write));
			this.pendingEventWrites.add(write);
		});
		return ctx;
	}

	private async flushEventWrites(): Promise<void> {
		while (this.pendingEventWrites.size > 0) {
			await Promise.all([...this.pendingEventWrites]);
		}
	}

	private async resumePendingSessionDeletions(): Promise<void> {
		for (const sessionKey of await this.submissions.listPendingSessionDeletions()) {
			try {
				await this.submissions.deleteSession(sessionKey, () =>
					deleteSessionTree(this.executionStore.sessions, sessionKey),
				);
			} catch (error) {
				console.error(
					'[flue:session-deletion]',
					{
						agentName: this.agentName,
						instanceId: this.instanceId,
						sessionKey,
						operation: 'resume_session_deletion',
						outcome: 'failed',
					},
					error,
				);
			}
		}
	}

	private async armSubmissionWake(): Promise<void> {
		await this.actor.schedule?.after(
			FLUE_AGENT_SUBMISSION_WAKE_SECONDS,
			FLUE_AGENT_SUBMISSION_WAKE_ACTION,
		);
	}

	private async restoreSubmissionWake(): Promise<boolean> {
		if (!(await this.submissions.hasUnsettledSubmissions())) return false;
		await this.armSubmissionWake();
		return true;
	}

	private async reconcileSubmissions(
		options: { driverAlreadyArmed?: boolean } = {},
	): Promise<boolean> {
		if (!(await this.submissions.hasUnsettledSubmissions())) return false;
		if (!options.driverAlreadyArmed) await this.restoreSubmissionWake();
		// Unlike the Cloudflare adapter, we intentionally do NOT consult persisted
		// attempt markers here, only the in-process `activeAttempts` set. Those
		// markers exist on Cloudflare to coordinate with its durable resumable
		// fibers (`runFiber`/`onFiberRecovered`): after a DO eviction the SDK
		// auto-resumes an in-flight attempt, and a still-fresh marker tells reconcile
		// "the fiber is being resumed — don't terminalize it as interrupted." Rivet
		// has no fiber resume — an actor restart permanently drops the in-flight
		// `keepAwake` work — so every `running` submission found after a restart IS
		// genuinely interrupted and must be reconciled. `activeAttempts` (shared
		// across this process, added synchronously before the turn starts) suppresses
		// reconcile for attempts still live in THIS process; the `claimSubmission`
		// CAS + the attempt-id ownership re-check in `processSubmission` guard against
		// double-processing regardless. Wiring markers in here would wrongly suppress
		// legitimate post-restart recovery. The store still implements the marker
		// methods for `AgentSubmissionStore` contract compliance; this coordinator
		// just never needs them.
		for (const submission of await this.submissions.listRunningSubmissions()) {
			if (this.activeAttempts.has(this.submissionAttemptLocalKey(submission))) continue;
			try {
				await this.reconcileInterruptedSubmission(submission);
			} catch (error) {
				this.logSubmissionReconciliationFailure(submission, 'reconcile_submission', error);
			}
		}
		for (const submission of await this.submissions.listRunnableSubmissions()) {
			const claimed = await this.submissions.claimSubmission({
				submissionId: submission.submissionId,
				attemptId: crypto.randomUUID(),
				ownerId: this.actor.actorId,
				leaseExpiresAt: 0,
			});
			if (!claimed) continue;
			try {
				await this.startSubmissionAttempt(claimed);
			} catch (error) {
				this.logSubmissionReconciliationFailure(claimed, 'start_submission', error);
			}
		}
		return await this.submissions.hasUnsettledSubmissions();
	}

	private logSubmissionReconciliationFailure(
		submission: AgentSubmission,
		operation: 'reconcile_submission' | 'start_submission',
		error: unknown,
	): void {
		console.error(
			'[flue:submission-reconciliation]',
			{
				agentName: this.agentName,
				instanceId: this.instanceId,
				submissionId: submission.submissionId,
				sessionKey: submission.sessionKey,
				attemptId: submission.attemptId,
				operation,
				outcome: 'deferred_to_wake',
			},
			error,
		);
	}

	private async reconcileInterruptedSubmission(submission: AgentSubmission): Promise<void> {
		const agent = this.options.createdAgents[this.agentName];
		if (!agent) throw new Error('[flue] Agent target unavailable during durable reconciliation.');
		await this.eventStreamStore.createStream(agentStreamPath(this.agentName, this.instanceId)).catch(
			(error) => {
				console.error('[flue:event-stream] createStream failed:', error);
			},
		);
		const reconciled = await this.runWithActorContext(() =>
			reconcileInterruptedSubmission(
				this.submissions,
				submission,
				agent,
				(payload, dispatchId) =>
					this.createDurableContext(
						payload,
						submissionSyntheticRequest(submission.input),
						dispatchId,
					),
				{ ownerId: this.actor.actorId, leaseExpiresAt: 0 },
			),
		);
		await this.flushEventWrites();
		if (reconciled.disposition === 'replacement') {
			await this.startSubmissionAttempt(reconciled.submission);
		} else if (submission.kind === 'direct') {
			if (reconciled.disposition === 'completed') {
				this.observers.complete(submission.submissionId, reconciled.result);
			} else if (reconciled.disposition === 'failed') {
				this.observers.fail(submission.submissionId, reconciled.error);
			}
		}
	}

	private async startSubmissionAttempt(submission: AgentSubmission): Promise<void> {
		if (submission.status !== 'running' || !submission.attemptId) return;
		const attemptKey = this.submissionAttemptLocalKey(submission);
		if (this.activeAttempts.has(attemptKey)) return;
		this.activeAttempts.add(attemptKey);
		const running = this.actor.keepAwake(
			this.processSubmissionEntry(submission).finally(() => {
				this.activeAttempts.delete(attemptKey);
			}),
		);
		try {
			await running;
		} catch (error) {
			console.error(
				'[flue:submission-processing]',
				{
					agentName: this.agentName,
					instanceId: this.instanceId,
					submissionId: submission.submissionId,
					operation: 'process',
					outcome: 'failed',
				},
				error,
			);
			throw error;
		}
	}

	private async processSubmissionEntry(submission: AgentSubmission): Promise<void> {
		await this.eventStreamStore.createStream(agentStreamPath(this.agentName, this.instanceId));
		await processSubmission({
			submissions: this.submissions,
			submission,
			resolveAgent: (name) => {
				const agent = this.options.createdAgents[name];
				if (!agent) throw new Error('[flue] Agent target unavailable during durable processing.');
				return agent;
			},
			createContext: (payload, dispatchId) =>
				this.createDurableContext(
					payload,
					submissionSyntheticRequest(submission.input),
					dispatchId,
				),
			observers: this.observers,
			// Deliberately do NOT pass `this.actor.abortSignal` here. It is tied to
			// one dispatch request, while an admitted Flue submission is durable work
			// that must settle or remain `running` for onWake recovery. The enclosing
			// actor action awaits this turn so its action-scoped database stays open.
			wrapExecution: (fn) => this.runWithActorContext(fn),
		});
		await this.flushEventWrites();
	}

	private submissionAttemptLocalKey(submission: AgentSubmission): string {
		return `${this.actor.actorId}:${submission.attemptId}`;
	}

	private async admitAttachedSubmission(
		payload: DirectAgentPayload,
		onEvent?: (event: AttachedAgentEvent) => Promise<void> | void,
		waitForResult = true,
	) {
		const input = createDirectAgentSubmissionInput({
			agent: this.agentName,
			id: this.instanceId,
			payload,
		});
		const attachment = this.observers.attach(input.submissionId, { onEvent });
		try {
			await this.armSubmissionWake();
			await this.submissions.admitDirect(input);
			await this.reconcileSubmissions({ driverAlreadyArmed: true });
			if (!waitForResult) return { submissionId: input.submissionId };
			return { submissionId: input.submissionId, result: await attachment.completion };
		} catch (error) {
			this.observers.fail(input.submissionId, error);
			throw error;
		} finally {
			attachment.detach();
		}
	}

	private async admitDispatch(request: Request): Promise<Response> {
		const input: unknown = await request.json();
		assertAgentDispatchAdmissionInput(input);
		if (input.agent !== this.agentName || input.id !== this.instanceId) {
			return new Response('Invalid internal dispatch target.', { status: 400 });
		}
		if (!this.options.createdAgents[this.agentName]) {
			return new Response('Dispatch target unavailable.', { status: 404 });
		}
		await this.armSubmissionWake();
		const admission = await this.submissions.admitDispatch(input);
		if (admission.kind === 'retained_receipt') {
			return Response.json({
				dispatchId: admission.receipt.submissionId,
				acceptedAt: new Date(admission.receipt.acceptedAt).toISOString(),
			});
		}
		if (admission.kind === 'conflict') {
			return new Response('Conflicting internal dispatch replay.', { status: 409 });
		}
		await this.reconcileSubmissions({ driverAlreadyArmed: true });
		return Response.json({
			dispatchId: admission.submission.submissionId,
			acceptedAt: input.acceptedAt,
		});
	}
}

function createIdentity(actor: RivetAgentActorContext): RivetActorIdentity {
	return {
		actorId: actor.actorId,
		key: actor.key,
		name: actor.name,
		region: actor.region,
	};
}

function isInternalDispatchRequest(request: Request): boolean {
	return (
		request.method === 'POST' && new URL(request.url).pathname === RIVET_AGENT_INTERNAL_DISPATCH_PATH
	);
}
