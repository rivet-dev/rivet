import type { RunContext } from "@/actor/contexts/run";
import type { Client } from "@/client/client";
import type { Registry } from "@/registry";
import type { ActorDefinition, AnyActorDefinition } from "@/actor/definition";
import type { AnyDatabaseProvider, InferDatabaseClient } from "@/actor/database";
import type {
	QueueFilterName,
	QueueNextBatchOptions,
	QueueNextOptions,
	QueueResultMessageForName,
} from "@/actor/instance/queue";
import type {
	EventSchemaConfig,
	InferEventArgs,
	InferSchemaMap,
	QueueSchemaConfig,
} from "@/actor/schema";
import type { WorkflowContextInterface } from "@rivetkit/workflow-engine";
import type {
	BranchConfig,
	BranchOutput,
	EntryKindType,
	LoopConfig,
	LoopResult,
	StepConfig,
	WorkflowQueueMessage,
} from "@rivetkit/workflow-engine";
import { WORKFLOW_GUARD_KV_KEY } from "./constants";

type WorkflowActorQueueNextOptions<
	TName extends string,
	TCompletable extends boolean,
> = Omit<QueueNextOptions<TName, TCompletable>, "signal">;

type WorkflowActorQueueNextOptionsFallback<TCompletable extends boolean> = Omit<
	QueueNextOptions<string, TCompletable>,
	"signal"
>;

type WorkflowActorQueueNextBatchOptions<
	TName extends string,
	TCompletable extends boolean,
> = Omit<QueueNextBatchOptions<TName, TCompletable>, "signal">;

type WorkflowActorQueueNextBatchOptionsFallback<
	TCompletable extends boolean,
> = Omit<QueueNextBatchOptions<string, TCompletable>, "signal">;

type ActorWorkflowLoopConfig<
	S,
	T,
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
> = Omit<LoopConfig<S, T>, "run"> & {
	run: (
		ctx: ActorWorkflowContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		state: S,
	) => Promise<LoopResult<S, T> | (S extends undefined ? void : never)>;
};

type ActorWorkflowBranchConfig<
	TOutput,
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
> = {
	run: (
		ctx: ActorWorkflowContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	) => Promise<TOutput>;
};

export class ActorWorkflowContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
> implements WorkflowContextInterface
{
	#inner: WorkflowContextInterface;
	#runCtx: RunContext<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>;
	#actorAccessDepth = 0;
	#allowActorAccess = false;
	#guardViolation = false;

	constructor(
		inner: WorkflowContextInterface,
		runCtx: RunContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	) {
		this.#inner = inner;
		this.#runCtx = runCtx;
	}

	get workflowId(): string {
		return this.#inner.workflowId;
	}

	get abortSignal(): AbortSignal {
		return this.#inner.abortSignal;
	}

	get queue() {
		const self = this;
		function next<
			const TName extends QueueFilterName<TQueues>,
			const TCompletable extends boolean = false,
		>(
			name: string,
			opts?: WorkflowActorQueueNextOptions<TName, TCompletable>,
		): Promise<QueueResultMessageForName<TQueues, TName, TCompletable>>;
		function next<const TCompletable extends boolean = false>(
			name: string,
			opts?: WorkflowActorQueueNextOptionsFallback<TCompletable>,
		): Promise<
			QueueResultMessageForName<
				TQueues,
				QueueFilterName<TQueues>,
				TCompletable
			>
		>;
		async function next(
			name: string,
			opts?: WorkflowActorQueueNextOptions<string, boolean>,
		): Promise<WorkflowQueueMessage<unknown>> {
			const message = await self.#inner.queue.next(name, opts);
			return self.#toActorQueueMessage(message);
		}

		function nextBatch<
			const TName extends QueueFilterName<TQueues>,
			const TCompletable extends boolean = false,
		>(
			name: string,
			opts?: WorkflowActorQueueNextBatchOptions<TName, TCompletable>,
		): Promise<Array<QueueResultMessageForName<TQueues, TName, TCompletable>>>;
		function nextBatch<const TCompletable extends boolean = false>(
			name: string,
			opts?: WorkflowActorQueueNextBatchOptionsFallback<TCompletable>,
		): Promise<
			Array<
				QueueResultMessageForName<
					TQueues,
					QueueFilterName<TQueues>,
					TCompletable
				>
			>
		>;
		async function nextBatch(
			name: string,
			opts?: WorkflowActorQueueNextBatchOptions<string, boolean>,
		): Promise<Array<WorkflowQueueMessage<unknown>>> {
			const messages = await self.#inner.queue.nextBatch(name, opts);
			return messages.map((message) => self.#toActorQueueMessage(message));
		}

		function send<K extends keyof TQueues & string>(
			name: K,
			body: InferSchemaMap<TQueues>[K],
		): Promise<void>;
		function send(
			name: keyof TQueues extends never ? string : never,
			body: unknown,
		): Promise<void>;
		async function send(name: string, body: unknown): Promise<void> {
			await self.#runCtx.queue.send(name as never, body as never);
		}

		return {
			next,
			nextBatch,
			send,
		};
	}

	async step<T>(
		nameOrConfig: string | Parameters<WorkflowContextInterface["step"]>[0],
		run?: () => Promise<T>,
	): Promise<T> {
		if (typeof nameOrConfig === "string") {
			if (!run) {
				throw new Error("Step run function missing");
			}
			return await this.#wrapActive(() =>
				this.#inner.step(nameOrConfig, () => this.#withActorAccess(run)),
			);
		}
		const stepConfig = nameOrConfig as StepConfig<T>;
		const config: StepConfig<T> = {
			...stepConfig,
			run: () => this.#withActorAccess(stepConfig.run),
		};
		return await this.#wrapActive(() => this.#inner.step(config));
	}

	async loop<T>(
		name: string,
		run: (
			ctx: ActorWorkflowContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase,
				TEvents,
				TQueues
			>,
		) => Promise<LoopResult<undefined, T> | void>,
	): Promise<T>;
	async loop<T>(
		name: string,
		run: (
			ctx: WorkflowContextInterface,
		) => Promise<LoopResult<undefined, T> | void>,
	): Promise<T>;
	async loop<S, T>(
		config: ActorWorkflowLoopConfig<
			S,
			T,
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	): Promise<T>;
	async loop<S, T>(config: LoopConfig<S, T>): Promise<T>;
	async loop(
		nameOrConfig:
			| string
			| LoopConfig<any, any>
			| ActorWorkflowLoopConfig<
					any,
					any,
					TState,
					TConnParams,
					TConnState,
					TVars,
					TInput,
					TDatabase,
					TEvents,
					TQueues
			  >,
		run?: (
			ctx: ActorWorkflowContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase,
				TEvents,
				TQueues
			>,
		) => Promise<LoopResult<undefined, any> | void>,
	): Promise<any> {
		if (typeof nameOrConfig === "string") {
			if (!run) {
				throw new Error("Loop run function missing");
			}
			return await this.#wrapActive(() =>
				this.#inner.loop(nameOrConfig, async (ctx) =>
					run(this.#createChildContext(ctx)),
				),
			);
		}
		const wrapped: LoopConfig<any, any> = {
			...nameOrConfig,
			run: async (ctx, state) =>
				nameOrConfig.run(this.#createChildContext(ctx), state),
		};
		return await this.#wrapActive(() => this.#inner.loop(wrapped));
	}

	sleep(name: string, durationMs: number): Promise<void> {
		return this.#inner.sleep(name, durationMs);
	}

	sleepUntil(name: string, timestampMs: number): Promise<void> {
		return this.#inner.sleepUntil(name, timestampMs);
	}

	async rollbackCheckpoint(name: string): Promise<void> {
		await this.#wrapActive(() => this.#inner.rollbackCheckpoint(name));
	}

	async join<
		T extends Record<
			string,
			ActorWorkflowBranchConfig<
				unknown,
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase,
				TEvents,
				TQueues
			>
		>,
	>(
		name: string,
		branches: T,
	): Promise<{ [K in keyof T]: Awaited<ReturnType<T[K]["run"]>> }>;
	async join<T extends Record<string, BranchConfig<unknown>>>(
		name: string,
		branches: T,
	): Promise<{ [K in keyof T]: BranchOutput<T[K]> }>;
	async join(name: string, branches: Record<string, BranchConfig<unknown>>) {
		const wrappedBranches = Object.fromEntries(
			Object.entries(branches).map(([key, branch]) => [
				key,
				{
					run: async (ctx: WorkflowContextInterface) =>
						branch.run(this.#createChildContext(ctx)),
				},
			]),
		) as Record<string, BranchConfig<unknown>>;
		return await this.#wrapActive(() =>
			this.#inner.join(name, wrappedBranches),
		);
	}

	async race<T>(
		name: string,
		branches: Array<{
			name: string;
			run: (
				ctx: ActorWorkflowContext<
					TState,
					TConnParams,
					TConnState,
					TVars,
					TInput,
					TDatabase,
					TEvents,
					TQueues
				>,
			) => Promise<T>;
		}>,
	): Promise<{ winner: string; value: T }>;
	async race<T>(
		name: string,
		branches: Array<{
			name: string;
			run: (ctx: WorkflowContextInterface) => Promise<T>;
		}>,
	): Promise<{ winner: string; value: T }> {
		const wrappedBranches = branches.map((branch) => ({
			name: branch.name,
			run: (ctx: WorkflowContextInterface) =>
				branch.run(this.#createChildContext(ctx)),
		}));
		return (await this.#wrapActive(() =>
			this.#inner.race(name, wrappedBranches),
		)) as { winner: string; value: T };
	}

	async removed(name: string, originalType: EntryKindType): Promise<void> {
		await this.#wrapActive(() => this.#inner.removed(name, originalType));
	}

	isEvicted(): boolean {
		return this.#inner.isEvicted();
	}

	get state(): TState extends never ? never : TState {
		this.#ensureActorAccess("state");
		return this.#runCtx.state as TState extends never ? never : TState;
	}

	get vars(): TVars extends never ? never : TVars {
		this.#ensureActorAccess("vars");
		return this.#runCtx.vars as TVars extends never ? never : TVars;
	}

	client<R extends Registry<any>>(): Client<R> {
		this.#ensureActorAccess("client");
		return this.#runCtx.client<R>();
	}

	get db(): TDatabase extends never ? never : InferDatabaseClient<TDatabase> {
		this.#ensureActorAccess("db");
		return this.#runCtx.db as TDatabase extends never
			? never
			: InferDatabaseClient<TDatabase>;
	}

	get log() {
		return this.#runCtx.log;
	}

	keepAwake<T>(promise: Promise<T>): Promise<T> {
		return this.#runCtx.keepAwake(promise);
	}

	waitUntil(promise: Promise<void>): void {
		this.#runCtx.waitUntil(promise);
	}

	get actorId(): string {
		return this.#runCtx.actorId;
	}

	broadcast<K extends keyof TEvents & string>(
		name: K,
		...args: InferEventArgs<InferSchemaMap<TEvents>[K]>
	): void;
	broadcast(
		name: keyof TEvents extends never ? string : never,
		...args: Array<unknown>
	): void;
	broadcast(name: string, ...args: Array<unknown>): void {
		this.#runCtx.broadcast(
			name as never,
			...((args as unknown[]) as never[]),
		);
	}

	#toActorQueueMessage<T>(
		message: WorkflowQueueMessage<T>,
	): WorkflowQueueMessage<T> & { id: bigint } {
		let id: bigint;
		try {
			id = BigInt(message.id);
		} catch {
			throw new Error(`Invalid queue message id "${message.id}"`);
		}
		return {
			id,
			name: message.name,
			body: message.body,
			createdAt: message.createdAt,
			...(message.complete ? { complete: message.complete } : {}),
		};
	}

	async #wrapActive<T>(run: () => Promise<T>): Promise<T> {
		return await this.#runCtx.keepAwake(run());
	}

	async #withActorAccess<T>(run: () => Promise<T>): Promise<T> {
		this.#actorAccessDepth++;
		if (this.#actorAccessDepth === 1) {
			this.#allowActorAccess = true;
		}
		try {
			return await run();
		} finally {
			this.#actorAccessDepth--;
			if (this.#actorAccessDepth === 0) {
				this.#allowActorAccess = false;
			}
		}
	}

	#ensureActorAccess(feature: string): void {
		if (!this.#allowActorAccess) {
			this.#guardViolation = true;
			this.#markGuardTriggered();
			throw new Error(
				`${feature} is only available inside workflow steps`,
			);
		}
	}

	consumeGuardViolation(): boolean {
		const violated = this.#guardViolation;
		this.#guardViolation = false;
		return violated;
	}

	#markGuardTriggered(): void {
		try {
			const state = this.#runCtx.state as Record<string, unknown>;
			if (
				state &&
				typeof state === "object" &&
				"guardTriggered" in state
			) {
				(state as Record<string, unknown>).guardTriggered = true;
			}
		} catch {
			// Ignore if state is unavailable
		}

		this.#runCtx.waitUntil(
			(async () => {
				try {
					await this.#runCtx.kv.put(WORKFLOW_GUARD_KV_KEY, "true");
				} catch (error) {
					this.#runCtx.log.error({
						msg: "failed to persist workflow guard flag",
						error,
					});
				}
			})(),
		);
	}

	#createChildContext(
		ctx: WorkflowContextInterface,
	): ActorWorkflowContext<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> {
		return new ActorWorkflowContext(ctx, this.#runCtx);
	}
}

export type WorkflowContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? ActorWorkflowContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type WorkflowLoopContextOf<AD extends AnyActorDefinition> =
	WorkflowContextOf<AD>;

export type WorkflowBranchContextOf<AD extends AnyActorDefinition> =
	WorkflowContextOf<AD>;

export type WorkflowStepContextOf<AD extends AnyActorDefinition> =
	WorkflowContextOf<AD>;
