import type { AgentOsOptions, MountConfig } from "@rivet-dev/agent-os-core";
import { AgentOs } from "@rivet-dev/agent-os-core";
// 0.2.8 moved the in-memory JS VFS driver off the main entry; the actor
// still uses it as the default `/home/user` working-directory mount.
import { createInMemoryFileSystem } from "@rivet-dev/agent-os-core/test/runtime";
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

// --- Sidecar-death recovery ---
//
// The shared agent-os sidecar is a single native OS process serving every VM
// on this host process (agentos-core caches the handle in a module-level
// map). When that process dies — observed live 2026-07-02: one VM's mount
// shadow-sync timeout was treated as process-fatal, exit code 1 — agentos-core
// never invalidates the cached handle: every later `AgentOs.create` and every
// call on an existing instance rejects with `SidecarProcessExited` until the
// HOST process restarts. The sanctioned repair is
// `AgentOs.getSharedSidecar().dispose()`, which evicts the poisoned handle so
// the next create spawns a fresh sidecar; nothing upstream calls it.
//
// The epoch counter makes that dispose once-per-crash: every actor on this
// host shares the sidecar, so a crash surfaces as concurrent failures from
// many actors — only the first (whose observed epoch still matches) recycles;
// the rest see a bumped epoch and skip, so the freshly-respawned replacement
// can never be disposed by a stale error handler.

let sidecarEpoch = 0;

/**
 * True when `err` indicates the shared sidecar OS process is dead (spawn
 * failure or process exit) — NOT a routine per-instance dispose. Matched by
 * the @secure-exec/core error class names (not re-exported from
 * agentos-core's root, so name/message matching is the stable surface).
 * Deliberately narrow: a broader match (e.g. "disposed" errors from an
 * action racing onSleep) would recycle — and kill — a healthy sidecar.
 */
function isSidecarProcessDeath(err: unknown): boolean {
	if (!err || typeof err !== "object") return false;
	const name = (err as { name?: unknown }).name;
	if (name === "SidecarProcessExited" || name === "SidecarProcessError") {
		return true;
	}
	const message = err instanceof Error ? err.message : "";
	// Covers all three exit shapes: "with code N", "with signal SIGKILL"
	// (OOM kills), and "with disconnect".
	return /sidecar process exited with |sidecar process error:/i.test(message);
}

/**
 * Cap logged error text. `SidecarProcessExited.message` embeds the sidecar's
 * whole-lifetime accumulated stderr (unbounded upstream), so logging it raw
 * floods the log sink with megabytes per failed action.
 */
function truncateForLog(
	text: string | undefined,
	max = 4096,
): string | undefined {
	if (text === undefined || text.length <= max) return text;
	return `${text.slice(0, max)} … [truncated ${text.length - max} chars]`;
}

async function recycleSharedSidecar<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	observedEpoch: number,
): Promise<void> {
	if (observedEpoch !== sidecarEpoch) {
		// Another actor already recycled the pool since this handle/error was
		// minted — do NOT dispose again or we'd kill the fresh replacement.
		return;
	}
	sidecarEpoch += 1;
	try {
		const shared = await AgentOs.getSharedSidecar();
		await shared.dispose();
		c.log.warn({
			msg: "agent-os: shared sidecar recycled after process death",
			epoch: sidecarEpoch,
		});
	} catch (err) {
		// Disposing an already-dead handle can throw; the eviction from the
		// shared pool still happened, which is what un-wedges the next boot.
		c.log.warn({
			msg: "agent-os: shared sidecar recycle threw (pool entry still evicted)",
			err: truncateForLog(
				err instanceof Error ? err.message : String(err),
			),
		});
	}
}

/**
 * Reset this actor's VM bookkeeping after the shared sidecar process died:
 * the cached AgentOs handle is permanently poisoned, and the dead sidecar
 * will never deliver the process-exit / shell-close events that normally
 * clear the activity sets — leaving the keepAwake barrier pinning a dead VM
 * awake forever. Clears both, recycles the shared pool (epoch-guarded), and
 * lets the next action boot a fresh VM.
 */
async function recoverFromSidecarDeath<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	actionName: string,
	err: unknown,
	instAtDispatch: AgentOs | null,
	epochAtDispatch: number,
): Promise<void> {
	// Staleness gate: a death error can be delivered LATE — the action
	// captured its instance, awaited unrelated work, then failed after other
	// actions already recovered and re-booted this actor. If the live
	// instance is no longer the one this action dispatched against, the
	// crash was already handled; wiping now would destroy a healthy VM.
	if (
		instAtDispatch !== null &&
		c.vars.agentOs !== null &&
		c.vars.agentOs !== instAtDispatch
	) {
		c.log.warn({
			msg: "agent-os: stale sidecar-death error ignored (VM already re-booted)",
			action: actionName,
		});
		return;
	}
	c.log.warn({
		msg: "agent-os: sidecar process died — resetting VM for re-boot",
		action: actionName,
		err: truncateForLog(err instanceof Error ? err.message : String(err)),
	});
	// Min of dispatch-time and current epoch: a stale error must never pass
	// the recycle guard against a fresh pool (bootVm re-stamps the vars
	// epoch on every successful boot). Under-recycling is safe — the next
	// failure dispatched against the dead boot carries the current epoch.
	const observedEpoch = Math.min(epochAtDispatch, c.vars.agentOsEpoch);
	const dead = c.vars.agentOs;
	// The wipe below is deliberately await-free: an await before it (e.g.
	// the dead-instance dispose, which can stall ~5s on a dead sidecar's
	// shell-exit timeouts) would let a concurrent re-boot interleave and
	// then get its fresh bookkeeping clobbered.
	c.vars.agentOs = null;
	c.vars.agentOsBoot = null;
	c.vars.activeSessionIds.clear();
	c.vars.activeProcesses.clear();
	c.vars.activeShells.clear();
	c.vars.sessions.clear();
	syncPreventSleep(c);
	await recycleSharedSidecar(c, observedEpoch);
	c.broadcast("vmShutdown", { reason: "error" });
	if (dead) {
		// Fire-and-forget: lease release against a dead process is expected
		// to throw (or stall); the pool recycle disposes the lease anyway.
		void dead.dispose().catch((disposeErr: unknown) => {
			c.log.debug({
				msg: "agent-os: dead VM dispose threw during recovery (expected for a dead process)",
				err: truncateForLog(
					disposeErr instanceof Error
						? disposeErr.message
						: String(disposeErr),
				),
			});
		});
	}
}

/**
 * Wrap every action so a sidecar-death error triggers recovery before the
 * error propagates. The caller still sees the failure (rethrown — rivetkit
 * reports it as usual); the NEXT action boots a fresh sidecar instead of
 * hitting the permanently-dead cached handle.
 */
function withSidecarRecovery<T extends Record<string, unknown>>(actions: T): T {
	const wrapped: Record<string, unknown> = {};
	for (const [name, fn] of Object.entries(actions)) {
		if (typeof fn !== "function") {
			wrapped[name] = fn;
			continue;
		}
		wrapped[name] = async (...args: unknown[]) => {
			const c = args[0] as AgentOsActionContext<never> | undefined;
			// Snapshot at dispatch: recovery decisions are bound to the
			// instance/epoch this action actually ran against, not whatever
			// is live by the time a (possibly late) error surfaces.
			const instAtDispatch = c?.vars?.agentOs ?? null;
			const epochAtDispatch = c?.vars?.agentOsEpoch ?? 0;
			try {
				return await (fn as (...a: unknown[]) => unknown)(...args);
			} catch (err) {
				if (isSidecarProcessDeath(err) && c?.vars) {
					await recoverFromSidecarDeath(
						c,
						name,
						err,
						instAtDispatch,
						epochAtDispatch,
					).catch((recoveryErr) => {
						c.log.error({
							msg: "agent-os: sidecar recovery failed",
							action: name,
							err: truncateForLog(
								recoveryErr instanceof Error
									? recoveryErr.message
									: String(recoveryErr),
							),
						});
					});
				}
				throw err;
			}
		};
	}
	return wrapped as T;
}

// --- VM lifecycle helpers ---

async function ensureVm<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	config: AgentOsActorConfig<TConnParams>,
): Promise<AgentOs> {
	if (c.vars.agentOs) {
		return c.vars.agentOs;
	}

	// Single-flight: concurrent first actions share one boot. Two racing
	// `AgentOs.create` calls would double-boot the VM and leak the loser's
	// sidecar lease. The in-flight promise is cleared on settle (finally), so
	// a REJECTED boot is never cached — the next action retries fresh. The
	// identity guards (`=== box.boot`) matter: recovery can null the slot
	// while this boot is pending, and a NEWER boot may have registered since
	// — an unconditional clear would clobber the newer boot's registration
	// and an unconditional install would overwrite its instance.
	if (c.vars.agentOsBoot) {
		return c.vars.agentOsBoot;
	}
	const box: BootBox = { boot: null };
	const boot = bootVm(c, config, box);
	box.boot = boot;
	c.vars.agentOsBoot = boot;
	try {
		return await boot;
	} finally {
		if (c.vars.agentOsBoot === boot) {
			c.vars.agentOsBoot = null;
		}
	}
}

/**
 * Identity token shared between ensureVm and its bootVm call so the boot can
 * detect being superseded (recovery cleared the single-flight slot and a
 * newer boot may own it now). `boot` is assigned synchronously right after
 * `bootVm()` returns its promise — before bootVm's first await resumes.
 */
interface BootBox {
	boot: Promise<AgentOs> | null;
}

// agent-os 0.2.8 requires one SQLite descriptor per VM for durable sessions
// (and for the native chunked root FS). Matches upstream `@rivet-dev/agentos`
// actor boot: Actor Runtime Socket UDS + chunked_actor_sqlite root.
const ACTOR_SQLITE_CHUNK_SIZE = 512 * 1024;
const ACTOR_SQLITE_INLINE_THRESHOLD = 64 * 1024;
const ROOT_NAMESPACE = "agentos-root";

async function bootVm<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	config: AgentOsActorConfig<TConnParams>,
	box: BootBox,
): Promise<AgentOs> {
	const start = Date.now();
	const bootEpoch = sidecarEpoch;

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

		// HOC owns database + rootFilesystem. Callers configure mounts/software
		// only — passing either of these would fight the actor-UDS wiring.
		if (resolvedUserOptions?.database) {
			throw new Error(
				"agentOs() owns database and injects the actor SQLite UDS descriptor; standalone AgentOs clients may choose a SQLite file",
			);
		}
		if (resolvedUserOptions?.rootFilesystem) {
			throw new Error(
				"agentOs() owns rootFilesystem so it can persist directly through the actor SQLite UDS; use mounts for additional filesystems",
			);
		}

		// Build options with in-memory VFS as default working directory mount.
		options = buildVmOptions(resolvedUserOptions);

		// Provision the Actor Runtime Socket for this generation, then hand the
		// UDS path to AgentOs as the sole VM SQLite descriptor. Without this,
		// createSession fails closed with session_storage_unavailable.
		const actorRuntimeSocket = (
			c as AgentOsActionContext<TConnParams> & {
				actorRuntimeSocket?: () => Promise<{ path: string }>;
			}
		).actorRuntimeSocket;
		if (typeof actorRuntimeSocket !== "function") {
			throw new Error(
				"AgentOS actors require a RivetKit runtime with Actor Runtime Socket support",
			);
		}
		const { path: actorSqliteUdsPath } = await actorRuntimeSocket.call(c);

		agentOs = await AgentOs.create({
			...options,
			database: { type: "actor_uds", path: actorSqliteUdsPath },
			rootFilesystem: {
				type: "native",
				plugin: {
					id: "chunked_actor_sqlite",
					config: {
						namespace: ROOT_NAMESPACE,
						chunkSize: ACTOR_SQLITE_CHUNK_SIZE,
						inlineThreshold: ACTOR_SQLITE_INLINE_THRESHOLD,
						uid: options?.user?.euid ?? options?.user?.uid ?? 1000,
						gid: options?.user?.egid ?? options?.user?.gid ?? 1000,
					},
				},
			},
		});
	} catch (err) {
		// Surface the REAL boot failure. The actor runtime otherwise wraps any
		// throw from the options factory or `AgentOs.create` as an opaque
		// `internal_error` ("An internal error occurred"), masking the cause —
		// which makes a failing VM boot near-impossible to diagnose from logs.
		c.log.error({
			msg: "agent-os: VM boot failed",
			err: truncateForLog(
				err instanceof Error ? err.message : String(err),
			),
			stack: truncateForLog(err instanceof Error ? err.stack : undefined),
		});
		if (isSidecarProcessDeath(err)) {
			await recycleSharedSidecar(c, bootEpoch);
		}
		throw err;
	}
	// Install-or-dispose: if recovery cleared our single-flight registration
	// while `create` was in flight, a newer boot may own the slot — installing
	// would overwrite its (healthy) instance and leak ours. Dispose and bail;
	// awaiters see a retryable failure.
	if (c.vars.agentOsBoot !== box.boot) {
		c.log.warn({
			msg: "agent-os: VM boot superseded during recovery — disposing orphan instance",
		});
		await agentOs.dispose().catch(() => {});
		throw new Error("agent-os: VM boot superseded during recovery — retry");
	}
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
			if (c.abortSignal.aborted) {
				await agentOs.dispose().catch(() => {});
				throw new DOMException(
					"Actor stopped during VM boot",
					"AbortError",
				);
			}
			if (res?.exitCode !== 0) {
				c.log.warn({
					msg: "agent-os: mount-point materialization exited non-zero",
					exitCode: res?.exitCode,
					stderr: res?.stderr,
				});
			}
		} catch (err) {
			if (c.abortSignal.aborted) {
				await agentOs.dispose().catch(() => {});
				throw err;
			}
			c.log.warn({
				msg: "agent-os: failed to materialize mount points",
				paths: mountPointsToMaterialize,
				err: err instanceof Error ? err.message : String(err),
			});
		}
	}
	// Publish only after mount materialization settles. onSleep disposes the
	// published instance; exposing it earlier lets teardown close the VM while
	// the boot's `mkdir -p` is still running, producing the misleading I/O error.
	if (c.abortSignal.aborted || c.vars.agentOsBoot !== box.boot) {
		await agentOs.dispose().catch(() => {});
		throw new DOMException("Actor stopped during VM boot", "AbortError");
	}
	c.vars.agentOs = agentOs;
	// Epoch at success time: if a recycle happened while `create` was in
	// flight, this instance is attached to the replacement sidecar and must
	// carry the replacement's epoch, or its own death could never recycle.
	c.vars.agentOsEpoch = sidecarEpoch;

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
	// Every action (built-in AND user-supplied) is wrapped so a sidecar
	// process death self-heals instead of bricking the actor until restart.
	const actions = withSidecarRecovery({
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
	});

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
			// Required so bootVm can provision an Actor Runtime Socket UDS for
			// agent-os 0.2.8 session/VFS SQLite (database: { type: "actor_uds" }).
			enableActorRuntimeSocket: true,
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
			agentOsBoot: null,
			agentOsEpoch: 0,
			activeSessionIds: new Set<string>(),
			activeProcesses: new Set<number>(),
			activeHooks: new Set<Promise<void>>(),
			activeShells: new Set<string>(),
			sessions: new Set(),
			// Highest durable stream sequence seen per session — used by
			// subscribeToSession to drop 0.2.8 live re-deliveries.
			sessionEventSequences: new Map<string, number>(),
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
				// Dispose throws if the sidecar process already died out from
				// under us — never let that block the sleep transition.
				await c.vars.agentOs.dispose().catch((err: unknown) => {
					c.log.warn({
						msg: "agent-os: dispose on sleep failed",
						err: truncateForLog(
							err instanceof Error ? err.message : String(err),
						),
					});
				});
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
				// Dispose throws if the sidecar process already died out from
				// under us — never let that block the destroy transition.
				await c.vars.agentOs.dispose().catch((err: unknown) => {
					c.log.warn({
						msg: "agent-os: dispose on destroy failed",
						err: truncateForLog(
							err instanceof Error ? err.message : String(err),
						),
					});
				});
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

export {
	ensureVm,
	syncPreventSleep,
	runHook,
	isSidecarProcessDeath,
	withSidecarRecovery,
	truncateForLog,
};
