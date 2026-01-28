import type { RunContext } from "@/actor/contexts/run";
import type { AnyDatabaseProvider } from "@/actor/database";
import type { WorkflowContextInterface } from "@rivetkit/workflow-engine";
import type {
	BranchConfig,
	BranchOutput,
	EntryKindType,
	LoopConfig,
	LoopResult,
	StepConfig,
} from "@rivetkit/workflow-engine";
import { WORKFLOW_GUARD_KV_KEY } from "./constants";

export class ActorWorkflowContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> implements WorkflowContextInterface
{
	#inner: WorkflowContextInterface;
	#runCtx: RunContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>;
	#actorAccessDepth = 0;
	#allowActorAccess = false;
	#guardViolation = false;

	constructor(
		inner: WorkflowContextInterface,
		runCtx: RunContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
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

	async step<T>(
		nameOrConfig: string | Parameters<WorkflowContextInterface["step"]>[0],
		run?: () => Promise<T>,
	): Promise<T> {
		if (typeof nameOrConfig === "string") {
			if (!run) {
				throw new Error("Step run function missing");
			}
			return await this.#wrapActive(() =>
				this.#inner.step(nameOrConfig, () =>
					this.#withActorAccess(run),
				),
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
			ctx: WorkflowContextInterface,
		) => Promise<LoopResult<undefined, T>>,
	): Promise<T>;
	async loop<S, T>(config: LoopConfig<S, T>): Promise<T>;
	async loop(
		nameOrConfig: string | LoopConfig<unknown, unknown>,
		run?: (
			ctx: WorkflowContextInterface,
		) => Promise<LoopResult<undefined, unknown>>,
	): Promise<unknown> {
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
		const wrapped: LoopConfig<unknown, unknown> = {
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

	listen<T>(name: string, messageName: string): Promise<T> {
		return this.#inner.listen(name, messageName);
	}

	listenN<T>(
		name: string,
		messageName: string,
		limit: number,
	): Promise<T[]> {
		return this.#inner.listenN(name, messageName, limit);
	}

	listenWithTimeout<T>(
		name: string,
		messageName: string,
		timeoutMs: number,
	): Promise<T | null> {
		return this.#inner.listenWithTimeout(name, messageName, timeoutMs);
	}

	listenUntil<T>(
		name: string,
		messageName: string,
		timestampMs: number,
	): Promise<T | null> {
		return this.#inner.listenUntil(name, messageName, timestampMs);
	}

	listenNWithTimeout<T>(
		name: string,
		messageName: string,
		limit: number,
		timeoutMs: number,
	): Promise<T[]> {
		return this.#inner.listenNWithTimeout(
			name,
			messageName,
			limit,
			timeoutMs,
		);
	}

	listenNUntil<T>(
		name: string,
		messageName: string,
		limit: number,
		timestampMs: number,
	): Promise<T[]> {
		return this.#inner.listenNUntil(name, messageName, limit, timestampMs);
	}

	async rollbackCheckpoint(name: string): Promise<void> {
		await this.#wrapActive(() => this.#inner.rollbackCheckpoint(name));
	}

	async join<T extends Record<string, BranchConfig<unknown>>>(
		name: string,
		branches: T,
	): Promise<{ [K in keyof T]: BranchOutput<T[K]> }> {
		const wrappedBranches = Object.fromEntries(
			Object.entries(branches).map(([key, branch]) => [
				key,
				{
					run: async (ctx: WorkflowContextInterface) =>
						branch.run(this.#createChildContext(ctx)),
				},
			]),
		) as T;

		return (await this.#wrapActive(() =>
			this.#inner.join(name, wrappedBranches),
		)) as { [K in keyof T]: BranchOutput<T[K]> };
	}

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
		TDatabase
	> {
		return new ActorWorkflowContext(ctx, this.#runCtx);
	}
}
