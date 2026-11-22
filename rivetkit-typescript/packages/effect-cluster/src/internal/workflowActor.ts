/**
 * The Rivet Actor definition that hosts the executions of one workflow, one
 * actor per execution id.
 *
 * @internal
 */
import { type AnyActorDefinition, actor, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { makeActorSql } from "./sql.ts";
import {
	makeScheduleTimer,
	type TimerCron,
	type TimerKind,
	type TimerSchedule,
	timerAction,
} from "./timer.ts";
import type {
	WorkflowRegistry,
	WorkflowRunOptions,
	WorkflowStub,
} from "./workflowRegistry.ts";
import {
	type DeferredState,
	makeWorkflowRuntime,
	type WorkflowRuntime,
} from "./workflowRuntime.ts";
import type { WakeMode } from "./workflowStorage.ts";
import { workflowMigrations } from "./workflowStorage.ts";

/** The actor name prefix of every workflow's actor. */
export const workflowActorPrefix = "Workflow/";

/** The actor name that hosts the executions of `workflowName`. */
export const workflowActorName = (workflowName: string): string =>
	`${workflowActorPrefix}${workflowName}`;

/** The error code a stopping workflow actor returns. Clients retry it. */
export const workflowStoppingCode = "cluster_workflow_stopping";

/** What a workflow actor needs from the cluster that hosts it. */
export interface WorkflowActorHost {
	readonly registry: WorkflowRegistry;
	readonly deferredState: DeferredState;
	readonly heartbeatMillis: number;
	/** Resolves once the Effect layer has registered every workflow. */
	readonly ready: Promise<void>;
	readonly getStub: (
		workflowName: string,
		executionId: string,
	) => WorkflowStub;
}

interface WorkflowVars {
	runtime: WorkflowRuntime | undefined;
}

interface WorkflowContext {
	readonly name: string;
	readonly key: ReadonlyArray<string>;
	readonly db: RawAccess;
	readonly schedule: TimerSchedule;
	readonly cron: TimerCron;
	readonly vars: WorkflowVars;
	readonly abortSignal: AbortSignal;
	readonly log: { error(fields: Record<string, unknown>): void };
	keepAwake<T>(promise: Promise<T>): Promise<T>;
}

const getRuntime = (c: { readonly vars: WorkflowVars }): WorkflowRuntime => {
	const runtime = c.vars.runtime;
	if (runtime === undefined) {
		throw new Error("Workflow actor received a request before it started");
	}
	return runtime;
};

/** Runs a workflow action, turning failures while stopping into a retryable error. */
const runAction = <A>(
	runtime: WorkflowRuntime,
	call: () => Promise<A>,
): Promise<A> => {
	const stopping = () =>
		new UserError("Workflow actor is stopping", {
			code: workflowStoppingCode,
		});
	if (runtime.isStopping()) return Promise.reject(stopping());
	return call().catch((error) => {
		throw runtime.isStopping() ? stopping() : error;
	});
};

/** Builds the actor definition for one workflow's executions. */
export const makeWorkflowActor = (
	host: WorkflowActorHost,
	options: { readonly actionTimeout: number },
): AnyActorDefinition =>
	actor({
		db: db({
			onMigrate: async (raw) => {
				for (const statement of workflowMigrations)
					await raw.execute(statement);
			},
		}),
		createVars: (): WorkflowVars => ({ runtime: undefined }),
		onWake: async (rawContext) => {
			const c = rawContext as unknown as WorkflowContext;
			await host.ready;
			const executionId = c.key[0];
			if (executionId === undefined) {
				throw new Error(
					"Workflow actor requires its execution id as the actor key",
				);
			}
			const runtime = makeWorkflowRuntime({
				workflowName: c.name.slice(workflowActorPrefix.length),
				executionId,
				registry: host.registry,
				deferredState: host.deferredState,
				sql: makeActorSql(c.db),
				timer: makeScheduleTimer({
					schedule: c.schedule,
					cron: c.cron,
					heartbeatMillis: host.heartbeatMillis,
				}),
				keepAwake: (promise) => {
					c.keepAwake(promise).catch(() => undefined);
				},
				getStub: host.getStub,
			});
			c.vars.runtime = runtime;
			// Stop the running attempt as soon as shutdown starts. Its pending
			// resume stays recorded, so it replays on the next start.
			c.abortSignal.addEventListener(
				"abort",
				() => {
					runtime.shutdown().catch(() => undefined);
				},
				{ once: true },
			);
			c.keepAwake(runtime.wake()).catch((error) => {
				c.log.error({ msg: "workflow wake replay failed", error });
			});
		},
		actions: {
			run: (
				c,
				payload: string,
				options: {
					readonly discard: boolean;
					readonly parent: WorkflowRunOptions["parent"] | null;
				},
			): Promise<string> =>
				runAction(getRuntime(c), () =>
					getRuntime(c).run(payload, {
						discard: options.discard,
						parent: options.parent ?? undefined,
					}),
				),
			poll: (c): Promise<string | null> =>
				getRuntime(c)
					.poll()
					.then((result) => result ?? null),
			resume: (c, mode: WakeMode | null): Promise<void> =>
				runAction(getRuntime(c), () =>
					getRuntime(c).resume(mode ?? undefined),
				),
			interrupt: (c): Promise<void> =>
				runAction(getRuntime(c), () => getRuntime(c).interrupt()),
			interruptUnsafe: (c): Promise<void> =>
				runAction(getRuntime(c), () => getRuntime(c).interruptUnsafe()),
			deferredDone: (c, name: string, exit: string): Promise<void> =>
				runAction(getRuntime(c), () =>
					getRuntime(c).deferredDone(name, exit),
				),
			scheduleClock: (
				c,
				name: string,
				deferredName: string,
				wakeUp: number,
			): Promise<void> =>
				runAction(getRuntime(c), () =>
					getRuntime(c).scheduleClock(name, deferredName, wakeUp),
				),
			[timerAction]: (c, _kind: TimerKind): Promise<void> =>
				getRuntime(c).runTimer(),
		},
		options: { actionTimeout: options.actionTimeout },
	}) as unknown as AnyActorDefinition;
