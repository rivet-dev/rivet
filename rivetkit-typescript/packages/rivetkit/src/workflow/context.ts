import type {
	BranchConfig,
	BranchOutput,
	EntryKindType,
	LoopConfig,
	LoopResult,
	StepConfig,
	TryBlockConfig,
	TryBlockResult,
	TryStepConfig,
	TryStepResult,
	WorkflowContextInterface,
	WorkflowQueueMessage,
} from "@rivetkit/workflow-engine";
import type {
	QueueFilterName,
	QueueNextBatchOptions,
	QueueNextOptions,
	QueueResultMessageForName,
} from "@/actor/config";
import { RAW_STATE_SYMBOL, type RunContext } from "@/actor/config";
import type {
	AnyActorDefinition,
	BaseActorDefinition,
} from "@/actor/definition";
import type {
	EventSchemaConfig,
	InferEventArgs,
	InferSchemaMap,
	QueueSchemaConfig,
} from "@/actor/schema";
import type { Client } from "@/client/client";
import type {
	AnyDatabaseProvider,
	InferDatabaseClient,
} from "@/common/database/config";
import type { Registry } from "@/registry";
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

type WorkflowActorQueueNextBatchOptionsFallback<TCompletable extends boolean> =
	Omit<QueueNextBatchOptions<string, TCompletable>, "signal">;

// Step run callbacks receive a WorkflowStepContext, which is the only place
// actor data (state/db/vars/client) may be touched.
type WorkflowStepRun<
	T,
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
> = (
	step: WorkflowStepContext<
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

// Step rollback callbacks compensate a committed step, so they also run with a
// WorkflowStepContext to mutate actor data.
type WorkflowStepRollback<
	T,
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
> = (
	step: WorkflowStepContext<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>,
	output: T,
) => Promise<void>;

// Orchestration callbacks (try/loop/race/join) receive a WorkflowContext,
// because inside them you sequence further steps rather than touch actor data.
type WorkflowContextRun<
	T,
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
> = (
	ctx: WorkflowContext<
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

export type WorkflowStepConfig<
	T,
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
> = Omit<StepConfig<T>, "run" | "rollback"> & {
	run: WorkflowStepRun<
		T,
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>;
	rollback?: WorkflowStepRollback<
		T,
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>;
};

export type WorkflowTryStepConfig<
	T,
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
> = Omit<TryStepConfig<T>, "run" | "rollback"> & {
	run: WorkflowStepRun<
		T,
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>;
	rollback?: WorkflowStepRollback<
		T,
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>;
};

export type WorkflowTryConfig<
	T,
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
> = Omit<TryBlockConfig<T>, "run"> & {
	run: WorkflowContextRun<
		T,
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>;
};

export type WorkflowLoopConfig<
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
		ctx: WorkflowContext<
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
	) => Promise<
		LoopResult<S, T> | (S extends undefined ? undefined | void : never)
	>;
};

export type WorkflowBranchConfig<
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
	run: WorkflowContextRun<
		TOutput,
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>;
};

// Marks a step context inactive once its step has finished. Module-private so it
// never appears on the public surface.
const DEACTIVATE_STEP = Symbol("workflow.step.deactivate");

/**
 * The context handed to a workflow step (`step` / `tryStep` callbacks). This is
 * the only scope where actor data (state, vars, db, client) and side effects
 * (broadcast, queue.send) are reachable. It is valid only while its step is
 * executing; using it after the step resolves throws.
 */
export class WorkflowStepContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
> {
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
	#active = true;
	#onGuardViolation: () => void;

	constructor(
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
		onGuardViolation: () => void,
	) {
		this.#runCtx = runCtx;
		this.#onGuardViolation = onGuardViolation;
	}

	[DEACTIVATE_STEP](): void {
		this.#active = false;
	}

	#ensureActive(feature: string): void {
		if (!this.#active) {
			this.#onGuardViolation();
			throw new Error(
				`${feature} is only available inside workflow steps`,
			);
		}
	}

	get actorId(): string {
		return this.#runCtx.actorId;
	}

	get name(): string {
		return this.#runCtx.name;
	}

	get key(): string[] {
		return this.#runCtx.key;
	}

	get log() {
		return this.#runCtx.log;
	}

	get abortSignal(): AbortSignal {
		return this.#runCtx.abortSignal;
	}

	get state(): TState extends never ? never : TState {
		this.#ensureActive("state");
		return this.#runCtx.state as TState extends never ? never : TState;
	}

	get vars(): TVars extends never ? never : TVars {
		this.#ensureActive("vars");
		return this.#runCtx.vars as TVars extends never ? never : TVars;
	}

	get db(): TDatabase extends never ? never : InferDatabaseClient<TDatabase> {
		this.#ensureActive("db");
		return this.#runCtx.db as TDatabase extends never
			? never
			: InferDatabaseClient<TDatabase>;
	}

	client<R extends Registry<any> = Registry<any>>(): Client<R> {
		this.#ensureActive("client");
		return this.#runCtx.client<R>();
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
		this.#ensureActive("broadcast");
		this.#runCtx.broadcast(
			name as never,
			...(args as unknown[] as never[]),
		);
	}

	get queue() {
		const self = this;
		function send<K extends keyof TQueues & string>(
			name: K,
			body: InferSchemaMap<TQueues>[K],
		): Promise<void>;
		function send(
			name: keyof TQueues extends never ? string : never,
			body: unknown,
		): Promise<void>;
		async function send(name: string, body: unknown): Promise<void> {
			self.#ensureActive("queue.send");
			await self.#runCtx.queue.send(name as never, body as never);
		}
		return { send };
	}

	/**
	 * Holds the actor awake for the duration of the provided promise. The actor
	 * cannot idle-sleep or finalize the sleep grace period until the promise
	 * settles.
	 */
	keepAwake<T>(promise: Promise<T>): Promise<T> {
		this.#ensureActive("keepAwake");
		return this.#runCtx.keepAwake(promise);
	}

	/**
	 * Registers a promise that the sleep grace period will wait on. Use this for
	 * best-effort flush/cleanup work that may complete inside the grace window.
	 */
	waitUntil(promise: Promise<void>): void {
		this.#ensureActive("waitUntil");
		this.#runCtx.waitUntil(promise);
	}

	destroy(): void {
		this.#ensureActive("destroy");
		this.#runCtx.destroy();
	}
}

/**
 * The context handed to the workflow function and to orchestration callbacks
 * (`try` / `loop` / `race` / `join`). It exposes the deterministic, replayable
 * workflow primitives (step, sleep, queue waits, control flow). It deliberately
 * does NOT expose actor data; reach `state`, `db`, `vars`, `client`, and
 * `broadcast` through the step context passed to `step` / `tryStep`.
 */
export class WorkflowContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
> {
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

	get actorId(): string {
		return this.#runCtx.actorId;
	}

	get name(): string {
		return this.#runCtx.name;
	}

	get key(): string[] {
		return this.#runCtx.key;
	}

	get log() {
		return this.#runCtx.log;
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
		// The implementation signature stays broad so the schema-typed public
		// overloads above remain compatible with it.
		async function next(name: string, opts?: any): Promise<any> {
			const message = await self.#inner.queue.next(name, opts);
			return self.#toActorQueueMessage(message);
		}

		function nextBatch<
			const TName extends QueueFilterName<TQueues>,
			const TCompletable extends boolean = false,
		>(
			name: string,
			opts?: WorkflowActorQueueNextBatchOptions<TName, TCompletable>,
		): Promise<
			Array<QueueResultMessageForName<TQueues, TName, TCompletable>>
		>;
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
		async function nextBatch(name: string, opts?: any): Promise<any> {
			const messages = await self.#inner.queue.nextBatch(name, opts);
			return messages.map((message) =>
				self.#toActorQueueMessage(message),
			);
		}

		return {
			next,
			nextBatch,
		};
	}

	step<T>(
		name: string,
		run: WorkflowStepRun<
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
	step<T>(
		config: WorkflowStepConfig<
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
	async step<T>(
		nameOrConfig:
			| string
			| WorkflowStepConfig<
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
		run?: WorkflowStepRun<
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
	): Promise<T> {
		if (typeof nameOrConfig === "string") {
			if (!run) {
				throw new Error("Step run function missing");
			}
			const stepRun = run;
			return await this.#wrapActive(() =>
				this.#inner.step(nameOrConfig, () => this.#runStep(stepRun)),
			);
		}
		const stepConfig = nameOrConfig;
		const rollback = stepConfig.rollback;
		const config: StepConfig<T> = {
			...stepConfig,
			run: () => this.#runStep(stepConfig.run),
			rollback: rollback
				? (_ctx, output) => this.#runRollback(rollback, output)
				: undefined,
		};
		return await this.#wrapActive(() => this.#inner.step(config));
	}

	tryStep<T>(
		name: string,
		run: WorkflowStepRun<
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
	): Promise<TryStepResult<T>>;
	tryStep<T>(
		config: WorkflowTryStepConfig<
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
	): Promise<TryStepResult<T>>;
	async tryStep<T>(
		nameOrConfig:
			| string
			| WorkflowTryStepConfig<
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
		run?: WorkflowStepRun<
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
	): Promise<TryStepResult<T>> {
		if (typeof nameOrConfig === "string") {
			if (!run) {
				throw new Error("Step run function missing");
			}
			const stepRun = run;
			return await this.#wrapActive(() =>
				this.#inner.tryStep(nameOrConfig, () => this.#runStep(stepRun)),
			);
		}
		const stepConfig = nameOrConfig;
		const rollback = stepConfig.rollback;
		const config: TryStepConfig<T> = {
			...stepConfig,
			run: () => this.#runStep(stepConfig.run),
			rollback: rollback
				? (_ctx, output) => this.#runRollback(rollback, output)
				: undefined,
		};
		return await this.#wrapActive(() => this.#inner.tryStep(config));
	}

	try<T>(
		name: string,
		run: WorkflowContextRun<
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
	): Promise<TryBlockResult<T>>;
	try<T>(
		config: WorkflowTryConfig<
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
	): Promise<TryBlockResult<T>>;
	async try<T>(
		nameOrConfig:
			| string
			| WorkflowTryConfig<
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
		run?: WorkflowContextRun<
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
	): Promise<TryBlockResult<T>> {
		if (typeof nameOrConfig === "string") {
			if (!run) {
				throw new Error("Try run function missing");
			}
			const tryRun = run;
			return await this.#wrapActive(() =>
				this.#inner.try(nameOrConfig, async (ctx) =>
					tryRun(this.#createChildContext(ctx)),
				),
			);
		}
		const tryConfig = nameOrConfig;
		const config: TryBlockConfig<T> = {
			...tryConfig,
			run: async (ctx) => tryConfig.run(this.#createChildContext(ctx)),
		};
		return await this.#wrapActive(() => this.#inner.try(config));
	}

	loop<T>(
		name: string,
		run: (
			ctx: WorkflowContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase,
				TEvents,
				TQueues
			>,
		) => Promise<LoopResult<undefined, T> | undefined | void>,
	): Promise<T>;
	loop<S, T>(
		config: WorkflowLoopConfig<
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
	async loop(
		nameOrConfig:
			| string
			| WorkflowLoopConfig<
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
			ctx: WorkflowContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase,
				TEvents,
				TQueues
			>,
		) => Promise<LoopResult<undefined, any> | undefined | void>,
	): Promise<any> {
		if (typeof nameOrConfig === "string") {
			if (!run) {
				throw new Error("Loop run function missing");
			}
			const loopRun = run;
			return await this.#wrapActive(() =>
				this.#inner.loop(
					nameOrConfig,
					// A void return (no explicit Loop result) is undefined at
					// runtime, which the engine treats as continue.
					async (
						ctx,
					): Promise<LoopResult<undefined, any> | undefined> =>
						(await loopRun(this.#createChildContext(ctx))) ??
						undefined,
				),
			);
		}
		const loopConfig = nameOrConfig;
		const wrapped: LoopConfig<any, any> = {
			...loopConfig,
			run: (async (ctx, state) =>
				(await loopConfig.run(this.#createChildContext(ctx), state)) ??
				undefined) as LoopConfig<any, any>["run"],
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
			WorkflowBranchConfig<
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
	async join(
		name: string,
		branches: Record<string, { run: (ctx: any) => Promise<unknown> }>,
	) {
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
			run: WorkflowContextRun<
				T,
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase,
				TEvents,
				TQueues
			>;
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

	getVersion(name: string, latest: number): Promise<number> {
		return this.#wrapActive(() => this.#inner.getVersion(name, latest));
	}

	isEvicted(): boolean {
		return this.#inner.isEvicted();
	}

	// Runs a user step body inside a fresh step context, snapshotting actor
	// state/vars so a thrown step rolls back its mutations, and deactivating the
	// step context once the body settles so it cannot be used after the step.
	async #runStep<T>(
		run: WorkflowStepRun<
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
	): Promise<T> {
		const stepCtx = new WorkflowStepContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>(this.#runCtx, () => this.#markGuardTriggered());

		let stateSnapshot: { state: TState } | null = null;
		try {
			stateSnapshot = { state: this.#runCtx[RAW_STATE_SYMBOL]() };
		} catch (error) {
			this.#runCtx.log.debug({
				msg: "failed to get state, likely due to being stateless workflow",
				error,
			});
		}
		if (stateSnapshot) {
			stateSnapshot.state = structuredClone(stateSnapshot.state);
		}
		const varsSnapshot = structuredClone(this.#runCtx.vars);

		try {
			return await run(stepCtx);
		} catch (error) {
			if (stateSnapshot) {
				this.#runCtx.state = stateSnapshot.state;
			}
			this.#runCtx.vars = varsSnapshot;
			throw error;
		} finally {
			stepCtx[DEACTIVATE_STEP]();
		}
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

	// Runs a step rollback compensation with an active step context. Rollbacks
	// intentionally mutate actor state, so their writes are not snapshotted.
	async #runRollback<T>(
		rollback: WorkflowStepRollback<
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
		output: T,
	): Promise<void> {
		const stepCtx = new WorkflowStepContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>(this.#runCtx, () => this.#markGuardTriggered());
		try {
			await rollback(stepCtx, output);
		} finally {
			stepCtx[DEACTIVATE_STEP]();
		}
	}

	async #wrapActive<T>(run: () => Promise<T>): Promise<T> {
		return await this.#runCtx.internalKeepAwake(run);
	}

	// Records that a step context was used outside its step. Mirrors the value
	// onto actor state and a KV flag so callers can observe the violation.
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
	): WorkflowContext<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> {
		return new WorkflowContext(ctx, this.#runCtx);
	}
}

export type WorkflowContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
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
		? WorkflowContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type WorkflowLoopContextOf<AD extends AnyActorDefinition> =
	WorkflowContextOf<AD>;

export type WorkflowBranchContextOf<AD extends AnyActorDefinition> =
	WorkflowContextOf<AD>;

export type WorkflowStepContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
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
		? WorkflowStepContext<S, CP, CS, V, I, DB, E, Q>
		: never;
