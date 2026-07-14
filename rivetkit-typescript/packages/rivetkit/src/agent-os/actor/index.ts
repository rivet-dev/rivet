import type { AgentOsOptions, MountConfig } from "@rivet-dev/agent-os-core";
import { AgentOs, createInMemoryFileSystem } from "@rivet-dev/agent-os-core";
import { type ActorDefinition, actor, event } from "@/actor/mod";
import type { DatabaseProvider, RawAccess } from "@/common/database/config";
import { db } from "@/common/database/mod";
import {
	type AgentOsActorConfig,
	type AgentOsActorConfigInput,
	agentOsActorConfigSchema,
} from "../config";
import type {
	AgentOsActionContext,
	AgentOsActorState,
	AgentOsActorVars,
	CronEventPayload,
	PermissionRequestPayload,
	ProcessExitPayload,
	ProcessOutputPayload,
	SessionEventPayload,
	ShellDataPayload,
	VmBootedPayload,
	VmShutdownPayload,
} from "../types";
import { buildCronActions } from "./cron";
import { migrateAgentOsTables } from "./db";
import { buildFilesystemActions } from "./filesystem";
import { buildNetworkActions } from "./network";
import { buildOnRequestHandler, buildPreviewActions } from "./preview";
import { buildProcessActions } from "./process";
import {
	buildConfigActions,
	buildPromptActions,
	buildSessionActions,
	buildSessionPersistenceActions,
} from "./session";
import { buildShellActions } from "./shell";

// --- VM lifecycle helpers ---

async function ensureVm<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	config: AgentOsActorConfig<TConnParams>,
): Promise<AgentOs> {
	if (c.vars.agentOs) {
		return c.vars.agentOs;
	}

	const start = Date.now();

	// `config.options` is either a static AgentOsOptions object OR a
	// `(c) => AgentOsOptions` factory resolved per actor instance with the
	// live context so callers can derive mount config from the actor identity.
	let options: AgentOsOptions;
	let agentOs: AgentOs;
	try {
		const resolvedUserOptions =
			typeof config.options === "function"
				? await config.options(c)
				: config.options;

		// Build options with in-memory VFS as default working directory mount.
		options = buildVmOptions(resolvedUserOptions);

		agentOs = await AgentOs.create(options);
	} catch (err) {
		// Surface the REAL boot failure. The actor runtime otherwise wraps any
		// throw from the options factory or `AgentOs.create` as an opaque
		// `internal_error` ("An internal error occurred"), masking the cause —
		// which makes a failing VM boot near-impossible to diagnose from logs.
		c.log.error({
			msg: "agent-os: VM boot failed",
			err: err instanceof Error ? err.message : String(err),
			stack: err instanceof Error ? err.stack : undefined,
		});
		throw err;
	}
	c.vars.agentOs = agentOs;

	// agent-os 0.1.2 auto-created each mount's mount-point path in the base VFS
	// when applying mounts; 0.2.4 does not. A nested, writable mount (e.g. the S3
	// workspace at `/root/workspaces/<id>`) is then left without a navigable
	// mount-point directory — the base path to it never materializes — so the
	// in-VM agent's first write fails with "failed to redirect …" and the action
	// throws `internal_error` in a tight retry loop (a brand-new workspace can
	// never create its first file). Only an in-VM `mkdir -p` recreates BOTH the
	// base-fs path AND the mount-point entry: host-side `agentOs.mkdir(path)`
	// resolves into the mount's own VFS (a no-op for the flat S3 driver), a
	// parent-only mkdir isn't enough, and raw S3 seeding doesn't help — the gap is
	// the base-fs path, not the prefix. Restore the old behaviour explicitly by
	// `mkdir -p`-ing every writable mount point once, right after boot. Read-only
	// mounts (the hoisted runtime, dev fixtures) are left to their own materialize;
	// any failure here is logged, never fatal.
	const mountPointsToMaterialize = (options.mounts ?? [])
		.filter((m: MountConfig) => {
			const readOnly = "readOnly" in m ? m.readOnly === true : false;
			return !!m?.path && m.path !== "/" && !readOnly;
		})
		.map((m: MountConfig) => m.path);
	if (mountPointsToMaterialize.length > 0) {
		try {
			const quoted = mountPointsToMaterialize
				.map((p) => `'${p.replace(/'/g, "'\\''")}'`)
				.join(" ");
			const res = await agentOs.exec(`mkdir -p ${quoted}`);
			if (res?.exitCode !== 0) {
				c.log.warn({
					msg: "agent-os: mount-point materialization exited non-zero",
					exitCode: res?.exitCode,
					stderr: res?.stderr,
				});
			}
		} catch (err) {
			c.log.warn({
				msg: "agent-os: failed to materialize mount points",
				paths: mountPointsToMaterialize,
				err: err instanceof Error ? err.message : String(err),
			});
		}
	}

	// Wire cron events to actor events.
	agentOs.onCronEvent((cronEvent) => {
		c.broadcast("cronEvent", { event: cronEvent });
	});

	c.broadcast("vmBooted", {});
	c.log.info({
		msg: "agent-os vm booted",
		bootDurationMs: Date.now() - start,
	});

	return agentOs;
}

function buildVmOptions(userOptions?: AgentOsOptions): AgentOsOptions {
	const userMounts = userOptions?.mounts ?? [];

	// Check if the user already provided a mount at /home/user. If so, respect
	// their override and skip the default in-memory VFS mount.
	const hasWorkdirMount = userMounts.some(
		(m: MountConfig) => m.path === "/home/user",
	);

	if (hasWorkdirMount) {
		return userOptions ?? {};
	}

	// TODO: Reimplement with persistent backend (SQLite-backed metadata and
	// actor storage-backed blocks) so VM filesystem state survives sleep/wake.
	const memMount: MountConfig = {
		path: "/home/user",
		driver: createInMemoryFileSystem(),
	};

	return {
		...userOptions,
		mounts: [memMount, ...userMounts],
	};
}

// --- Keep-awake coordination ---
//
// rivetkit 2.3.0-rc.5+ replaces the boolean `c.setPreventSleep(enabled)`
// API with the promise-based `c.keepAwake(promise)`. The runtime holds an
// internal keep-awake counter that's incremented when `keepAwake(promise)`
// is called and decremented when the promise settles; the actor becomes
// eligible for idle sleep only when the counter hits zero.
//
// agent-os tracks four kinds of activity that must keep the actor awake:
// active Pi sessions, running processes, in-flight hooks, and open shells.
// Each of those is bookkept as a `Set` on `c.vars`. To bridge the
// set-based bookkeeping into the promise-based keepAwake API, we maintain
// a single barrier promise per actor instance:
//
//   - When activity count goes 0 -> >0, we create a new
//     `Promise.withResolvers<void>()`, stash the resolver on
//     `c.vars.keepAwakeResolver`, and hand the promise to `c.keepAwake`.
//     The runtime's keep-awake counter goes from 0 to 1.
//   - When activity count goes >0 -> 0, we resolve the stashed promise.
//     The runtime's keep-awake counter goes from 1 to 0; idle sleep
//     becomes eligible again.
//
// The barrier is one logical keep-awake regardless of how many activities
// pile on, which matches the actor's reality (one VM either alive or
// not). It also survives sub-counter churn — e.g. a session ending and
// another starting in the same tick doesn't bounce the runtime counter.
//
// The function name `syncPreventSleep` is preserved because layerr docs
// reference it; only the implementation changed.

function syncPreventSleep<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
): void {
	const isActive =
		c.vars.activeSessionIds.size > 0 ||
		c.vars.activeProcesses.size > 0 ||
		c.vars.activeHooks.size > 0 ||
		c.vars.activeShells.size > 0;

	if (isActive && !c.vars.keepAwakeResolver) {
		// Transition 0 -> active: open the keep-awake barrier.
		const { promise, resolve } = Promise.withResolvers<void>();
		c.vars.keepAwakeResolver = resolve;
		c.keepAwake(promise);
		c.log.info({
			msg: "agent-os keepAwake barrier opened",
			activeSessions: c.vars.activeSessionIds.size,
			activeProcesses: c.vars.activeProcesses.size,
			activeHooks: c.vars.activeHooks.size,
			activeShells: c.vars.activeShells.size,
		});
	} else if (!isActive && c.vars.keepAwakeResolver) {
		// Transition active -> 0: close the barrier so the runtime
		// counter decrements and idle sleep becomes eligible again.
		c.vars.keepAwakeResolver();
		c.vars.keepAwakeResolver = null;
		c.log.info({ msg: "agent-os keepAwake barrier closed" });
	}
}

// --- Hook tracking ---

function runHook<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	name: string,
	callback: () => void | Promise<void>,
): void {
	const promise = Promise.resolve(callback())
		.catch((error) =>
			c.log.error({ msg: "agent-os hook failed", hookName: name, error }),
		)
		.finally(() => {
			c.vars.activeHooks.delete(promise);
			syncPreventSleep(c);
		});
	c.vars.activeHooks.add(promise);
	syncPreventSleep(c);
	c.waitUntil(promise);
}

// --- Public API ---

export function agentOs<TConnParams = undefined>(
	config: AgentOsActorConfigInput<TConnParams>,
): ActorDefinition<
	AgentOsActorState,
	TConnParams,
	undefined,
	AgentOsActorVars,
	undefined,
	DatabaseProvider<RawAccess>,
	{
		sessionEvent: typeof sessionEventToken;
		permissionRequest: typeof permissionRequestToken;
		vmBooted: typeof vmBootedToken;
		vmShutdown: typeof vmShutdownToken;
		processOutput: typeof processOutputToken;
		processExit: typeof processExitToken;
		shellData: typeof shellDataToken;
		cronEvent: typeof cronEventToken;
	},
	Record<never, never>,
	any
> {
	const parsedConfig = agentOsActorConfigSchema.parse(
		config,
	) as AgentOsActorConfig<TConnParams>;
	const actions = {
		...buildSessionActions(parsedConfig),
		...buildPromptActions(parsedConfig),
		...buildConfigActions(parsedConfig),
		...buildSessionPersistenceActions(parsedConfig),
		...buildProcessActions(parsedConfig),
		...buildFilesystemActions(parsedConfig),
		...buildPreviewActions(parsedConfig),
		...buildShellActions(parsedConfig),
		...buildCronActions(parsedConfig),
		...buildNetworkActions(parsedConfig),
		// Layerr: user-supplied actions merged last so they can extend (or
		// override) the built-in set. Schema-validated as Record<string,
		// Function>; the actor({}) call below enforces full signature shape.
		...(parsedConfig.actions ?? {}),
	};

	return actor<
		AgentOsActorState,
		TConnParams,
		undefined,
		AgentOsActorVars,
		undefined,
		DatabaseProvider<RawAccess>,
		{
			sessionEvent: typeof sessionEventToken;
			permissionRequest: typeof permissionRequestToken;
			vmBooted: typeof vmBootedToken;
			vmShutdown: typeof vmShutdownToken;
			processOutput: typeof processOutputToken;
			processExit: typeof processExitToken;
			shellData: typeof shellDataToken;
			cronEvent: typeof cronEventToken;
		},
		Record<never, never>,
		typeof actions
	>({
		options: {
			sleepGracePeriod: parsedConfig.sleepGracePeriod ?? 900_000,
			actionTimeout: parsedConfig.actionTimeout ?? 900_000,
			...(parsedConfig.noSleep !== undefined
				? { noSleep: parsedConfig.noSleep }
				: {}),
			...(parsedConfig.sleepTimeout !== undefined
				? { sleepTimeout: parsedConfig.sleepTimeout }
				: {}),
		},
		createState: async () => ({}),
		createVars: () => ({
			agentOs: null,
			activeSessionIds: new Set<string>(),
			activeProcesses: new Set<number>(),
			activeHooks: new Set<Promise<void>>(),
			activeShells: new Set<string>(),
			sessions: new Set(),
			keepAwakeResolver: null,
		}),
		db: db({
			onMigrate: migrateAgentOsTables,
		}),
		events: {
			sessionEvent: sessionEventToken,
			permissionRequest: permissionRequestToken,
			vmBooted: vmBootedToken,
			vmShutdown: vmShutdownToken,
			processOutput: processOutputToken,
			processExit: processExitToken,
			shellData: shellDataToken,
			cronEvent: cronEventToken,
		},
		onBeforeConnect: parsedConfig.onBeforeConnect
			? async (ctx, params) => {
					// Skip user auth for preview URL requests. The signed token
					// in onRequest is the credential; browsers navigating preview
					// URLs cannot supply actor connection params.
					if (ctx.request) {
						const url = new URL(ctx.request.url);
						if (url.pathname.startsWith("/fetch/")) {
							return;
						}
					}
					await parsedConfig.onBeforeConnect?.(ctx, params);
				}
			: undefined,
		onRequest: buildOnRequestHandler(parsedConfig),
		onSleep: async (c) => {
			c.log.info({
				msg: "agent-os vm shutdown for sleep",
				activeSessions: c.vars.sessions.size,
				activeProcesses: c.vars.activeProcesses.size,
				activeShells: c.vars.activeShells.size,
			});

			// Defensively close any open keep-awake barrier. If activity
			// is still > 0 here it means a session/process didn't clean
			// up via the normal path; resolving keeps the runtime counter
			// from leaking across sleep/wake cycles.
			if (c.vars.keepAwakeResolver) {
				c.vars.keepAwakeResolver();
				c.vars.keepAwakeResolver = null;
			}

			if (c.vars.agentOs) {
				await c.vars.agentOs.dispose();
				c.vars.agentOs = null;
			}

			c.broadcast("vmShutdown", { reason: "sleep" as const });
		},
		onDestroy: async (c) => {
			c.log.info({
				msg: "agent-os vm shutdown for destroy",
				activeSessions: c.vars.sessions.size,
				activeProcesses: c.vars.activeProcesses.size,
				activeShells: c.vars.activeShells.size,
			});

			if (c.vars.keepAwakeResolver) {
				c.vars.keepAwakeResolver();
				c.vars.keepAwakeResolver = null;
			}

			if (c.vars.agentOs) {
				await c.vars.agentOs.dispose();
				c.vars.agentOs = null;
			}

			c.broadcast("vmShutdown", { reason: "destroy" as const });
		},
		actions,
	});
}

// Event type tokens. Declared at module level so they can be referenced in
// the actor generic type parameters.
const sessionEventToken = event<SessionEventPayload>();
const permissionRequestToken = event<PermissionRequestPayload>();
const vmBootedToken = event<VmBootedPayload>();
const vmShutdownToken = event<VmShutdownPayload>();
const processOutputToken = event<ProcessOutputPayload>();
const processExitToken = event<ProcessExitPayload>();
const shellDataToken = event<ShellDataPayload>();
const cronEventToken = event<CronEventPayload>();

export { ensureVm, syncPreventSleep, runHook };
