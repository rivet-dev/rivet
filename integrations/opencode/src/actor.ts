import { createHash } from "node:crypto";
import type { OpenCode } from "@opencode/sdk/effect";
import type {
	Sandbox,
	SandboxActorContext,
	SandboxAdapter,
	SandboxBinding,
} from "@rivet-dev/sandbox-adapter";
import { Effect, Scope, Stream } from "effect";
import {
	actor,
	event,
	type Actions,
	type ActorConfigInput,
	type ActorContext,
	type ActorDefinition,
	type Type,
} from "rivetkit";
import { db, type DatabaseProvider, type RawAccess } from "rivetkit/db";
import {
	invokeNative,
	nativeActions,
	type OpenCodeNativeActions,
	type OpenCodeInput,
} from "./actions.js";
import {
	createHost,
	type OpenCodeEvent,
	type OpenCodeHost,
	type OpenCodeOptions,
	type OpenCodePlugin,
} from "./host.js";

type ActorDb = DatabaseProvider<RawAccess>;
export type OpenCodeContext = ActorContext<
	any,
	any,
	any,
	any,
	any,
	ActorDb,
	any,
	any
> &
	SandboxActorContext;
type BuiltInEvents = { event: Type<OpenCodeEvent> };
type PromptInput = Parameters<OpenCode.Interface["session"]["prompt"]>[0];
type SessionID = PromptInput["sessionID"];
type SessionCreateInput = NonNullable<
	Parameters<OpenCode.Interface["session"]["create"]>[0]
>;
type Directory = NonNullable<SessionCreateInput["location"]>["directory"];
export type OpenCodePromptOptions = Omit<PromptInput, "sessionID" | "text">;

export interface OpenCodeActorOptions {
	/** Embedded SDK configuration; Rivet owns its database and event persistence. */
	opencode?: OpenCodeOptions;
	/** Trusted, application-provided OpenCode plugins. */
	plugins?: readonly OpenCodePlugin[];
	cwd?: string;
	sandbox?: SandboxAdapter<OpenCodeContext>;
	/** Defaults for the lazily created convenience session. */
	session?: OpenCodeInput<Omit<SessionCreateInput, "id" | "location">>;
	onSessionEvent?: (
		context: OpenCodeContext,
		event: OpenCodeEvent,
	) => void | Promise<void>;
}

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown
	? Omit<T, K>
	: never;
export type OpenCodeActorConfigInput<
	S = undefined,
	CP = undefined,
	CS = undefined,
	V = undefined,
	I = undefined,
	E extends Record<string, any> = {},
	Q extends Record<string, any> = {},
	A extends Actions<S, CP, CS, V, I, ActorDb, E, Q> = {},
> = DistributiveOmit<
	ActorConfigInput<S, CP, CS, V, I, ActorDb, E, Q, A>,
	"db"
> &
	OpenCodeActorOptions;

interface Runtime {
	host: OpenCodeHost;
	sandbox?: Sandbox;
	binding?: SandboxBinding;
	cwd: string;
	defaultSession?: Promise<
		Effect.Success<ReturnType<OpenCode.Interface["session"]["create"]>>
	>;
	waits: Map<string, Promise<void>>;
}

export function opencode<
	S = undefined,
	CP = undefined,
	CS = undefined,
	V = undefined,
	I = undefined,
	E extends Record<string, any> = {},
	Q extends Record<string, any> = {},
	A extends Actions<S, CP, CS, V, I, ActorDb, E, Q> = {},
>(
	config: OpenCodeActorConfigInput<
		S,
		CP,
		CS,
		V,
		I,
		E,
		Q,
		A
	> = {} as OpenCodeActorConfigInput<S, CP, CS, V, I, E, Q, A>,
): ActorDefinition<
	S,
	CP,
	CS,
	V,
	I,
	ActorDb,
	E & BuiltInEvents,
	Q,
	A & OpenCodeActions
> {
	const {
		opencode: sdkOptions = {},
		plugins,
		cwd,
		sandbox: adapter,
		session: defaults,
		onSessionEvent,
		...actorConfig
	} = config;
	const runtimes = new Map<string, Promise<Runtime>>();
	const stopping = new Set<string>();
	const logError = (c: OpenCodeContext, error: unknown) =>
		c.log.error({
			msg: "OpenCode background operation failed",
			error: String(error),
		});
	const track = (
		c: OpenCodeContext,
		runtime: Runtime,
		sessionID: SessionID,
	) => {
		if (runtime.waits.has(sessionID) || stopping.has(c.actorId)) return;
		const waiting = c
			.keepAwake(
				Effect.runPromise(runtime.host.client.session.wait({ sessionID })),
			)
			.catch((error) => {
				if (!stopping.has(c.actorId)) logError(c, error);
			})
			.finally(() => runtime.waits.delete(sessionID));
		runtime.waits.set(sessionID, waiting);
	};
	const loadBinding = async (c: OpenCodeContext) => {
		const rows = await c.db.execute<{ binding: string | null; cwd: string }>(
			"SELECT binding, cwd FROM _rivet_opencode WHERE id = 1",
		);
		return rows[0]
			? {
					binding:
						rows[0].binding === null ? undefined : JSON.parse(rows[0].binding),
					cwd: rows[0].cwd,
				}
			: undefined;
	};
	const ensure = (c: OpenCodeContext): Promise<Runtime> => {
		if (stopping.has(c.actorId))
			return Promise.reject(new Error("OpenCode actor is stopping"));
		const existing = runtimes.get(c.actorId);
		if (existing) return existing;
		const ready = (async () => {
			const stored = await loadBinding(c);
			const sandbox = await adapter?.connect(c, {
				id: c.actorId,
				binding: stored?.binding,
				cwd: stored?.cwd ?? cwd,
			});
			const directory = sandbox?.cwd ?? stored?.cwd ?? cwd ?? process.cwd();
			let host: OpenCodeHost | undefined;
			try {
				await c.db.execute(
					"INSERT INTO _rivet_opencode (id, binding, cwd) VALUES (1, ?, ?) ON CONFLICT(id) DO UPDATE SET binding = excluded.binding, cwd = excluded.cwd",
					sandbox ? JSON.stringify(sandbox.binding) : null,
					directory,
				);
				host = await createHost(c.db, sdkOptions, sandbox, plugins);
				const runtime: Runtime = {
					host,
					sandbox,
					binding: sandbox?.binding,
					cwd: directory,
					waits: new Map(),
				};
				await Effect.runPromise(
					host.client.event.subscribe().pipe(
						Stream.runForEach((value) =>
							Effect.promise(async () => {
								if (value.type === "session.execution.started")
									track(c, runtime, value.data.sessionID);
								c.broadcast("event", value);
								if (onSessionEvent)
									void c
										.keepAwake(
											Promise.resolve().then(() => onSessionEvent(c, value)),
										)
										.catch((error) => logError(c, error));
							}),
						),
						Effect.catchCause((cause) =>
							Effect.sync(() => {
								if (!stopping.has(c.actorId)) logError(c, cause);
							}),
						),
						Effect.forkScoped,
						Scope.provide(host.scope),
					),
				);
				await host.resume();
				for (const id of Object.keys(
					await Effect.runPromise(host.client.session.active()),
				))
					track(c, runtime, id as SessionID);
				return runtime;
			} catch (error) {
				await host?.close();
				if (sandbox)
					await adapter?.suspend?.(c, {
						id: c.actorId,
						sandbox,
						binding: sandbox.binding,
						cwd: directory,
					});
				throw error;
			}
		})();
		runtimes.set(c.actorId, ready);
		void ready.catch(() => {
			if (runtimes.get(c.actorId) === ready) runtimes.delete(c.actorId);
		});
		return ready;
	};
	const getDefault = (c: OpenCodeContext, runtime: Runtime) => {
		if (!runtime.defaultSession) {
			const id =
				`ses_${createHash("sha256").update(c.actorId).digest("hex").slice(0, 26)}` as SessionID;
			runtime.defaultSession = (async () => {
				const found = await Effect.runPromise(
					runtime.host.client.session
						.get({ sessionID: id })
						.pipe(
							Effect.catchTag("SessionNotFoundError", () =>
								Effect.succeed(undefined),
							),
						),
				);
				return (
					found ??
					Effect.runPromise(
						runtime.host.client.session.create({
							...defaults,
							id,
							location: { directory: runtime.cwd as Directory },
						} as SessionCreateInput),
					)
				);
			})();
			void runtime.defaultSession.catch(() => {
				runtime.defaultSession = undefined;
			});
		}
		return runtime.defaultSession.then((session) =>
			Effect.runPromise(
				runtime.host.client.session.get({ sessionID: session.id }),
			),
		);
	};
	const read = <T>(c: OpenCodeContext, fn: (runtime: Runtime) => Promise<T>) =>
		c.keepAwake(ensure(c).then(fn));
	const send = (
		c: OpenCodeContext,
		text: string,
		options?: OpenCodePromptOptions,
	) =>
		read(c, async (runtime) => {
			const session = await getDefault(c, runtime);
			const result = await Effect.runPromise(
				runtime.host.client.session.prompt({
					...options,
					sessionID: session.id,
					text,
				}),
			);
			track(c, runtime, session.id);
			return result;
		});
	const actions = {
		...nativeActions((c, path, args) =>
			read(c, async (runtime) => {
				if (path.join(".") === "session.create") {
					const input = args[0] as SessionCreateInput | undefined;
					args = [
						{
							...input,
							location: input?.location ?? { directory: runtime.cwd },
						},
					];
				}
				const result = await invokeNative(runtime.host, path, args);
				if (path.join(".") === "session.remove")
					runtime.defaultSession = undefined;
				const input = args[0] as { sessionID?: SessionID } | undefined;
				if (input?.sessionID) track(c, runtime, input.sessionID);
				// Covers prompt, shell, queued input, child sessions, and resumed work.
				for (const id of Object.keys(
					await Effect.runPromise(runtime.host.client.session.active()),
				))
					track(c, runtime, id as SessionID);
				return result;
			}),
		),
		prompt: send,
		steer: (
			c: OpenCodeContext,
			text: string,
			options?: OpenCodePromptOptions,
		) => send(c, text, { ...options, delivery: "steer" }),
		followUp: (
			c: OpenCodeContext,
			text: string,
			options?: OpenCodePromptOptions,
		) => send(c, text, { ...options, delivery: "queue" }),
		getSession: (c: OpenCodeContext) =>
			read(c, (runtime) => getDefault(c, runtime)),
		getMessages: (c: OpenCodeContext) =>
			read(c, async (runtime) =>
				runtime.host.client.message
					.list({ sessionID: (await getDefault(c, runtime)).id })
					.pipe(Effect.runPromise),
			),
		abort: (c: OpenCodeContext) =>
			read(c, async (runtime) =>
				Effect.runPromise(
					runtime.host.client.session.interrupt({
						sessionID: (await getDefault(c, runtime)).id,
					}),
				),
			),
		waitForIdle: (c: OpenCodeContext) =>
			read(c, async (runtime) =>
				Effect.runPromise(
					runtime.host.client.session.wait({
						sessionID: (await getDefault(c, runtime)).id,
					}),
				),
			),
		readEvents: (
			c: OpenCodeContext,
			input: { sessionID: string; after?: number; limit?: number },
		) =>
			read(c, async (runtime) => {
				const limit = input.limit ?? 100;
				if (!Number.isInteger(limit) || limit < 1 || limit > 1000)
					throw new Error(
						"readEvents limit must be an integer between 1 and 1000",
					);
				return Effect.runPromise(
					runtime.host.client.session
						.log({
							sessionID: input.sessionID as SessionID,
							after: input.after as Parameters<
								OpenCode.Interface["session"]["log"]
							>[0]["after"],
							follow: false,
						})
						.pipe(
							Stream.takeWhile((entry) => entry.type !== "log.synced"),
							Stream.take(limit),
							Stream.runCollect,
						),
				);
			}),
	};
	for (const key of Object.keys(actorConfig.actions ?? {}))
		if (key in actions)
			throw new Error(`opencode() action name is reserved: ${key}`);
	if ("event" in (actorConfig.events ?? {}))
		throw new Error("opencode() event name is reserved: event");
	const dispose = async (c: OpenCodeContext, destroy: boolean) => {
		stopping.add(c.actorId);
		const pending = runtimes.get(c.actorId);
		try {
			const runtime = await pending?.catch(() => undefined);
			const stored = runtime ?? (await loadBinding(c));
			try {
				await runtime?.host.close();
			} finally {
				if (adapter && stored) {
					const input = {
						id: c.actorId,
						binding: stored.binding,
						cwd: stored.cwd,
						sandbox: runtime?.sandbox,
					};
					if (destroy) await adapter.destroy?.(c, input);
					else await adapter.suspend?.(c, input);
				}
			}
		} finally {
			runtimes.delete(c.actorId);
			stopping.delete(c.actorId);
		}
	};
	return actor({
		...actorConfig,
		options: {
			actionTimeout: 2_147_483_647,
			sleepGracePeriod: 15 * 60_000,
			...actorConfig.options,
		},
		db: db({
			onMigrate: async (database) => {
				await database.execute(
					"CREATE TABLE IF NOT EXISTS _rivet_opencode (id INTEGER PRIMARY KEY CHECK (id = 1), binding TEXT, cwd TEXT NOT NULL)",
				);
			},
		}),
		events: { ...actorConfig.events, event: event<OpenCodeEvent>() },
		actions: { ...actorConfig.actions, ...actions },
		onWake: async (c: OpenCodeContext) => {
			try {
				await ensure(c);
				await actorConfig.onWake?.(c as never);
			} catch (error) {
				await dispose(c, false);
				throw error;
			}
		},
		onSleep: async (c: OpenCodeContext) => {
			try {
				await actorConfig.onSleep?.(c as never);
			} finally {
				await dispose(c, false);
			}
		},
		onDestroy: async (c: OpenCodeContext) => {
			try {
				await actorConfig.onDestroy?.(c as never);
			} finally {
				await dispose(c, true);
			}
		},
	} as any) as ActorDefinition<
		S,
		CP,
		CS,
		V,
		I,
		ActorDb,
		E & BuiltInEvents,
		Q,
		A & OpenCodeActions
	>;
}

type NativeSession = OpenCodeNativeActions["session"];
export type OpenCodeActions = OpenCodeNativeActions & {
	prompt: (
		context: OpenCodeContext,
		text: string,
		options?: OpenCodePromptOptions,
	) => ReturnType<NativeSession["prompt"]>;
	steer: (
		context: OpenCodeContext,
		text: string,
		options?: OpenCodePromptOptions,
	) => ReturnType<NativeSession["prompt"]>;
	followUp: (
		context: OpenCodeContext,
		text: string,
		options?: OpenCodePromptOptions,
	) => ReturnType<NativeSession["prompt"]>;
	getSession: (context: OpenCodeContext) => ReturnType<NativeSession["get"]>;
	getMessages: (
		context: OpenCodeContext,
	) => ReturnType<OpenCodeNativeActions["message"]["list"]>;
	abort: (context: OpenCodeContext) => ReturnType<NativeSession["interrupt"]>;
	waitForIdle: (context: OpenCodeContext) => Promise<void>;
	readEvents: (
		context: OpenCodeContext,
		input: { sessionID: string; after?: number; limit?: number },
	) => Promise<
		Array<Stream.Success<ReturnType<OpenCode.Interface["session"]["log"]>>>
	>;
};
