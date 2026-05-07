import { Hono } from "hono";
import { z } from "zod";
import { toRivetError } from "@/actor/errors";
import { ENGINE_ENDPOINT } from "@/common/engine";
import { VERSION } from "@/utils";
import {
	crossPlatformServe,
	loadRuntimeServeStatic,
} from "@/utils/serve";
import {
	type RegistryActors,
	type RegistryConfig,
	type EntrypointConfigInput,
	type RegistryConfigInput,
	RegistryConfigSchema,
} from "./config";
import { EnvoyConfigSchema } from "./config/envoy";
import { logger } from "./log";
import { buildConfiguredRegistry } from "./native";
import type { RuntimeServerlessResponseHead } from "./runtime";

type ShutdownSignal = "SIGINT" | "SIGTERM";

function metadataError(metadata: unknown): string | undefined {
	if (
		typeof metadata === "object" &&
		metadata !== null &&
		"error" in metadata &&
		typeof metadata.error === "string"
	) {
		return metadata.error;
	}

	return undefined;
}

function isEngineReachabilityError(message: string): boolean {
	return /cannot reach Rivet Engine|Connection refused|ECONNREFUSED|tcp connect error|error sending request/i.test(
		message,
	);
}

function logRegistryServeError(config: RegistryConfig, err: unknown): void {
	const rivetError = toRivetError(err);
	const detail = metadataError(rivetError.metadata) ?? rivetError.message;

	if (config.endpoint && isEngineReachabilityError(detail)) {
		logger().warn(
			{
				endpoint: config.endpoint,
				namespace: config.namespace,
				group: rivetError.group,
				code: rivetError.code,
				error: detail,
			},
			"cannot reach Rivet Engine",
		);
		return;
	}

	logger().warn({ err }, "runtime registry serve errored");
}

export type FetchHandler = (
	request: Request,
	...args: any
) => Response | Promise<Response>;

export interface ServerlessHandler {
	fetch: FetchHandler;
}

export interface RegistryDiagnostics {
	mode: string;
	envoyActiveActorCount?: number | null;
}

const FetchHandlerDevConfigSchema = z.object({
	url: z.string().url(),
	startEngine: z.boolean().optional(),
	drainTimeout: z.number().int().positive().optional(),
	requestTimeout: z.number().int().positive().optional(),
});
export type FetchHandlerDevConfig = z.infer<
	typeof FetchHandlerDevConfigSchema
>;
export type FetchHandlerDev = false | string | FetchHandlerDevConfig;

const FetchHandlerOptsSchema = z.object({
	path: z.string().min(1),
	dev: z
		.union([z.literal(false), z.string().url(), FetchHandlerDevConfigSchema])
		.optional(),
	publicEndpoint: z.string().optional(),
	publicToken: z.string().optional(),
	maxStartPayloadBytes: z.number().int().positive().optional(),
});
export type FetchHandlerOpts = z.input<typeof FetchHandlerOptsSchema>;

const StaticConfigSchema = z.union([
	z.boolean(),
	z.string().min(1),
	z.object({ dir: z.string().min(1) }),
]);
export type StaticConfig = z.input<typeof StaticConfigSchema>;

const ListenDevConfigSchema = z.object({
	url: z.string().url().optional(),
	startEngine: z.boolean().optional(),
	drainTimeout: z.number().int().positive().optional(),
	requestTimeout: z.number().int().positive().optional(),
});
export type ListenDevConfig = z.infer<typeof ListenDevConfigSchema>;
export type ListenDev = boolean | ListenDevConfig;

const ListenOptsSchema = z
	.object({
		port: z.number().int().positive(),
		host: z.string().optional(),
		path: z.string().min(1),
		static: StaticConfigSchema.optional(),
		dev: z.union([z.boolean(), ListenDevConfigSchema]).optional(),
	})
	.strict();
export type ListenOpts = z.input<typeof ListenOptsSchema>;

const StartOptsSchema = z
	.object({
		envoy: EnvoyConfigSchema.optional().default(() =>
			EnvoyConfigSchema.parse({}),
		),
	})
	.strict()
	.optional()
	.default(() => ({ envoy: EnvoyConfigSchema.parse({}) }));
export type StartOpts = z.input<typeof StartOptsSchema>;

type EntrypointKind = "start" | "fetchHandler" | "listen";

function isDevelopmentEnv(): boolean {
	return (
		typeof process !== "undefined" &&
		process.env?.NODE_ENV === "development"
	);
}

function normalizeBasePath(path: string): string {
	const trimmed = path.trim();
	if (trimmed === "" || trimmed === "/") return "/";
	return `/${trimmed.replace(/^\/+|\/+$/g, "")}`;
}

function routeForBasePath(path: string): string {
	const normalized = normalizeBasePath(path);
	return normalized === "/" ? "/*" : `${normalized}/*`;
}

function staticDirFromConfig(config: StaticConfig | undefined): string | undefined {
	if (config === false) return undefined;
	if (config === true || config === undefined) return "public";
	if (typeof config === "string") return config;
	return config.dir;
}

export class Registry<A extends RegistryActors> {
	#config: RegistryConfigInput<A>;

	get config(): RegistryConfigInput<A> {
		return this.#config;
	}

	parseConfig(): RegistryConfig {
		return RegistryConfigSchema.parse(this.#config);
	}

	#runtimeServePromise?: Promise<void>;
	#runtimeServeConfiguredPromise?: ReturnType<typeof buildConfiguredRegistry>;
	#runtimeServerlessPromise?: ReturnType<typeof buildConfiguredRegistry>;
	#runtimeHttpServerPromise?: Promise<{ closeServer?: () => void }>;
	#activeEntrypoint?: EntrypointKind;
	#welcomePrinted = false;
	#shutdownInstalled = false;
	#shutdownInFlight: Promise<void> | null = null;
	#signalHandlers: Partial<Record<ShutdownSignal, () => void>> = {};

	constructor(config: RegistryConfigInput<A>) {
		this.#config = config;
	}

	#claimEntrypoint(kind: EntrypointKind): void {
		if (this.#activeEntrypoint !== undefined) {
			throw new Error(
				`registry.${kind}() cannot be used after registry.${this.#activeEntrypoint}() has already selected the runtime entrypoint.`,
			);
		}
		this.#activeEntrypoint = kind;
	}

	#parseConfigWithEntrypoint(
		entrypoint: EntrypointConfigInput,
	): RegistryConfig {
		return RegistryConfigSchema.parse({
			...this.#config,
			entrypoint,
		});
	}

	#createServerlessHandler(
		config: RegistryConfig,
		configuredRegistryPromise: ReturnType<typeof buildConfiguredRegistry>,
		kind: "serverless" | "listen" = "serverless",
	): FetchHandler {
		this.#printWelcome(config, kind);

		return async (request: Request): Promise<Response> => {
			return await this.#handleServerlessRequest(
				request,
				config,
				configuredRegistryPromise,
			);
		};
	}

	async #handleServerlessRequest(
		request: Request,
		config: RegistryConfig,
		configuredRegistryPromise: ReturnType<typeof buildConfiguredRegistry>,
	): Promise<Response> {
		const { runtime, registry, serveConfig } =
			await configuredRegistryPromise;
		const isStartRequest = isServerlessStartRequest(
			request,
			serveConfig.serverlessBasePath ?? "/api/rivet",
		);
		const cancelToken = runtime.createCancellationToken();
		const abort = () => runtime.cancelCancellationToken(cancelToken);
		if (request.signal.aborted) {
			abort();
		} else {
			request.signal.addEventListener("abort", abort, { once: true });
		}

		const requestBody = await request.arrayBuffer();
		if (
			isStartRequest &&
			requestBody.byteLength > serveConfig.serverlessMaxStartPayloadBytes
		) {
			request.signal.removeEventListener("abort", abort);
			runtime.cancelCancellationToken(cancelToken);
			return new Response(
				JSON.stringify({
					group: "message",
					code: "incoming_too_long",
					message: `Incoming message too long. Received ${requestBody.byteLength} bytes, limit is ${serveConfig.serverlessMaxStartPayloadBytes} bytes.`,
					metadata: null,
				}),
				{
					status: 413,
					headers: { "content-type": "application/json" },
				},
			);
		}

		let settled = false;
		let controllerRef:
			| ReadableStreamDefaultController<Uint8Array>
			| undefined;
		const backpressureWaiters: Array<() => void> = [];
		const resolveBackpressure = () => {
			while (
				controllerRef &&
				(controllerRef.desiredSize ?? 1) > 0 &&
				backpressureWaiters.length > 0
			) {
				backpressureWaiters.shift()?.();
			}
		};
		const waitForBackpressure = async () => {
			if (!controllerRef || (controllerRef.desiredSize ?? 1) > 0) return;
			await new Promise<void>((resolve) => {
				backpressureWaiters.push(resolve);
			});
		};
		const stream = new ReadableStream<Uint8Array>({
			start(controller) {
				controllerRef = controller;
			},
			pull() {
				resolveBackpressure();
			},
			cancel() {
				settled = true;
				resolveBackpressure();
				runtime.cancelCancellationToken(cancelToken);
			},
		});

		const headers: Record<string, string> = {};
		request.headers.forEach((value, key) => {
			headers[key] = value;
		});

		let head: RuntimeServerlessResponseHead;
		try {
			head = await runtime.handleServerlessRequest(
				registry,
				{
					method: request.method,
					url: request.url,
					headers,
					body: new Uint8Array(requestBody),
				},
				async (
					error: unknown,
					event?: {
						kind: "chunk" | "end";
						chunk?: Uint8Array;
						error?: {
							group: string;
							code: string;
							message: string;
						};
					},
				) => {
					if (error) throw error;
					if (!event || settled) return;
					if (event.kind === "chunk") {
						await waitForBackpressure();
						if (settled) return;
						if (event.chunk) controllerRef?.enqueue(event.chunk);
						return;
					}

					settled = true;
					resolveBackpressure();
					request.signal.removeEventListener("abort", abort);
					if (event.error) {
						controllerRef?.error(
							new Error(
								`${event.error.group}.${event.error.code}: ${event.error.message}`,
							),
						);
					} else {
						controllerRef?.close();
					}
				},
				cancelToken,
				serveConfig,
			);
		} catch (err) {
			// The runtime call itself rejected (e.g. `registry_shut_down_error`).
			// Clean up the abort listener so it doesn't leak, then propagate.
			request.signal.removeEventListener("abort", abort);
			runtime.cancelCancellationToken(cancelToken);
			throw err;
		}

		return new Response(stream, {
			status: head.status,
			headers: head.headers,
		});
	}

	/**
	 * Returns a fetch handler for serverless deployments.
	 *
	 * @example
	 * ```ts
	 * const fetch = registry.fetchHandler({ path: "/api/rivet" });
	 * export default { fetch };
	 * ```
	 */
	public fetchHandler(opts: FetchHandlerOpts): FetchHandler {
		const parsed = FetchHandlerOptsSchema.parse(opts);
		const dev = this.#resolveFetchHandlerDev(parsed.dev);
		const config = this.#parseConfigWithEntrypoint({
			kind: "serverless",
			startEngine: dev?.startEngine,
			devServerless: dev?.url
				? {
						url: dev.url,
						drainTimeout: dev.drainTimeout,
						requestTimeout: dev.requestTimeout,
					}
				: undefined,
			serverless: {
				basePath: normalizeBasePath(parsed.path),
				publicEndpoint: parsed.publicEndpoint,
				publicToken: parsed.publicToken,
				maxStartPayloadBytes: parsed.maxStartPayloadBytes,
			},
		});

		this.#claimEntrypoint("fetchHandler");
		this.#runtimeServerlessPromise = buildConfiguredRegistry(config);
		return this.#createServerlessHandler(
			config,
			this.#runtimeServerlessPromise,
		);
	}

	public async diagnostics(): Promise<RegistryDiagnostics> {
		const candidates = [
			this.#runtimeServerlessPromise,
			this.#runtimeServeConfiguredPromise,
		].filter((candidate): candidate is ReturnType<typeof buildConfiguredRegistry> =>
			candidate !== undefined
		);

		for (const candidate of candidates) {
			const { runtime, registry } = await candidate;
			const diagnostics = await runtime.registryDiagnostics?.(registry);
			if (diagnostics) return diagnostics;
		}

		return { mode: "not_started", envoyActiveActorCount: null };
	}

	/**
	 * Starts an actor envoy for standalone server deployments.
	 */
	#startPersistentRuntime(config: RegistryConfig, printWelcome: boolean) {
		if (!this.#runtimeServePromise) {
			const configuredRegistryPromise = buildConfiguredRegistry(config);
			this.#runtimeServeConfiguredPromise = configuredRegistryPromise;
			this.#runtimeServePromise = configuredRegistryPromise
				.then(async ({ runtime, registry, serveConfig }) => {
					await runtime.serveRegistry(registry, serveConfig);
				})
				.catch((err) => {
					// Always-attached catch so the stored promise never leaves a
					// rejection unhandled. Downstream awaits (e.g. #runShutdown's
					// Promise.race) attach their own catches and still observe
					// resolution via the race.
					logRegistryServeError(config, err);
				});
			// Install signal handlers once an envoy lifecycle has begun. Only
			// Mode A ever reaches here. Mode B (handler(request)) intentionally
			// does not install handlers because it runs on Workers/Vercel/Deno
			// Deploy where `process.on` is absent or forbidden; those platforms
			// own their own signal policy.
			this.#installSignalHandlers(config, configuredRegistryPromise);
		}
		if (printWelcome) {
			this.#printWelcome(config, "envoy");
		}
	}

	#installSignalHandlers(
		config: RegistryConfig,
		configuredRegistryPromise: ReturnType<typeof buildConfiguredRegistry>,
	): void {
		if (this.#shutdownInstalled) return;
		if (config.shutdown?.disableSignalHandlers) return;
		// Guard against non-Node runtimes (Workers/Edge) where `process` may
		// exist but `process.on` is unavailable or forbidden.
		if (
			typeof process === "undefined" ||
			typeof process.on !== "function" ||
			typeof process.kill !== "function"
		) {
			return;
		}
		this.#shutdownInstalled = true;

		const install = (signal: ShutdownSignal) => {
			const handler = () =>
				this.#onShutdownSignal(
					signal,
					config,
					configuredRegistryPromise,
				);
			this.#signalHandlers[signal] = handler;
			process.on(signal, handler);
		};
		install("SIGINT");
		install("SIGTERM");
	}

	#onShutdownSignal(
		signal: ShutdownSignal,
		config: RegistryConfig,
		configuredRegistryPromise: ReturnType<typeof buildConfiguredRegistry>,
	): void {
		if (this.#shutdownInFlight !== null) {
			// Second delivery of the same (or another) shutdown signal.
			// Remove our handler only (preserving any user-installed listeners)
			// and re-raise so Node proceeds with its default exit path.
			this.#removeSignalHandlers();
			process.kill(process.pid, signal);
			return;
		}
		this.#shutdownInFlight = this.#runShutdown(
			signal,
			config,
			configuredRegistryPromise,
		).catch((err) => {
			logger().warn({ err }, "shutdown error");
		});
	}

	async #runShutdown(
		signal: ShutdownSignal,
		config: RegistryConfig,
		configuredRegistryPromise: ReturnType<typeof buildConfiguredRegistry>,
	): Promise<void> {
		const gracePeriodMs = config.shutdown?.gracePeriodMs ?? 30_000;
		// Race the entire drain sequence (both modes + serve promise) against
		// a single grace ceiling. Without this, each mode's Rust-side drain
		// (20s) could stack sequentially and blow past gracePeriodMs before
		// we re-raise the signal.
		const drain = async () => {
			// Shut down every live `CoreRegistry` we know about. Mode A
			// (`start()`) and Mode B (`handler()`) each build a separate
			// runtime registry, so one signal handler fans out to both to
			// honor the spec invariant "single shutdown tears down both modes".
			const registries: Promise<void>[] = [
				(async () => {
					try {
						const { runtime, registry } =
							await configuredRegistryPromise;
						await runtime.shutdownRegistry(registry);
					} catch (err) {
						logger().warn(
							{ err },
							"runtime registry shutdown errored (mode A)",
						);
					}
				})(),
			];
			const runtimeServerlessPromise = this.#runtimeServerlessPromise;
			if (runtimeServerlessPromise !== undefined) {
				registries.push(
					(async () => {
						try {
							const { runtime, registry } =
								await runtimeServerlessPromise;
							await runtime.shutdownRegistry(registry);
						} catch (err) {
							logger().warn(
								{ err },
								"runtime registry shutdown errored (mode B)",
							);
						}
					})(),
				);
			}
			await Promise.all(registries);

			const runtimeServePromise = this.#runtimeServePromise;
			if (runtimeServePromise !== undefined) {
				// Swallow rejection so the race doesn't itself reject; the
				// always-attached `.catch` at the promise assignment site has
				// already logged any serve-side error.
				await runtimeServePromise.catch(() => undefined);
			}
			const runtimeHttpServerPromise = this.#runtimeHttpServerPromise;
			if (runtimeHttpServerPromise !== undefined) {
				try {
					const server = await runtimeHttpServerPromise;
					server.closeServer?.();
				} catch (err) {
					logger().warn({ err }, "runtime HTTP server shutdown errored");
				}
			}
		};
		await Promise.race([
			drain(),
			new Promise<void>((resolve) =>
				setTimeout(resolve, gracePeriodMs).unref?.(),
			),
		]);
		this.#removeSignalHandlers();
		process.kill(process.pid, signal);
	}

	#removeSignalHandlers(): void {
		for (const [signal, handler] of Object.entries(
			this.#signalHandlers,
		) as [ShutdownSignal, () => void][]) {
			if (handler) process.removeListener(signal, handler);
		}
		this.#signalHandlers = {};
	}

	/**
	 * Starts the actor envoy.
	 *
	 * @example
	 * ```ts
	 * const registry = setup({ use: { counter } });
	 * registry.start();
	 * ```
	 */
	public start(opts?: StartOpts): void {
		const parsed = StartOptsSchema.parse(opts);
		const config = this.#parseConfigWithEntrypoint({
			kind: "envoy",
			envoy: parsed.envoy,
		});
		this.#claimEntrypoint("start");
		this.#startPersistentRuntime(config, true);
	}

	/**
	 * Starts a local server for serverless deployments.
	 */
	public listen(opts: ListenOpts): void {
		const parsed = ListenOptsSchema.parse(opts);
		const dev = this.#resolveListenDev(parsed);
		const basePath = normalizeBasePath(parsed.path);
		const config = this.#parseConfigWithEntrypoint({
			kind: "listen",
			startEngine: dev?.startEngine,
			devServerless: dev?.url
				? {
						url: dev.url,
						drainTimeout: dev.drainTimeout,
						requestTimeout: dev.requestTimeout,
					}
				: undefined,
			serverless: {
				basePath,
			},
			staticDir: staticDirFromConfig(parsed.static),
			httpBasePath: basePath,
			httpPort: parsed.port,
			httpHost: parsed.host,
		});

		this.#claimEntrypoint("listen");
		const configuredRegistryPromise = buildConfiguredRegistry(config);
		this.#runtimeServerlessPromise = configuredRegistryPromise;
		const handler = this.#createServerlessHandler(
			config,
			configuredRegistryPromise,
			"listen",
		);
		this.#runtimeHttpServerPromise = this.#startHttpServer(config, handler);
		this.#installSignalHandlers(config, configuredRegistryPromise);
	}

	async #startHttpServer(
		config: RegistryConfig,
		handler: FetchHandler,
	): Promise<{ closeServer?: () => void }> {
		const app = new Hono();
		const route = routeForBasePath(config.httpBasePath ?? "/api/rivet");
		app.all(route, (c) => handler(c.req.raw));
		if (config.staticDir) {
			const runtime =
				"Deno" in globalThis
					? "deno"
					: "Bun" in globalThis
						? "bun"
						: "node";
			const serveStatic = await loadRuntimeServeStatic(runtime);
			app.use("/*", serveStatic({ root: config.staticDir }));
			app.get("*", serveStatic({ root: config.staticDir, path: "/index.html" }));
		}
		return await crossPlatformServe(config, config.httpPort ?? 6421, app);
	}

	#resolveFetchHandlerDev(
		dev: z.infer<typeof FetchHandlerOptsSchema>["dev"],
	): (FetchHandlerDevConfig & { startEngine: boolean }) | undefined {
		if (dev === undefined || dev === false || !isDevelopmentEnv()) {
			return undefined;
		}
		if (typeof dev === "string") {
			return { url: dev, startEngine: true };
		}
		return { url: dev.url, startEngine: dev.startEngine ?? true };
	}

	#resolveListenDev(
		opts: z.infer<typeof ListenOptsSchema>,
	): (ListenDevConfig & { url: string; startEngine: boolean }) | undefined {
		if (opts.dev === false || !isDevelopmentEnv()) {
			return undefined;
		}
		const inferred = opts.dev === undefined;
		if (inferred) {
			logger().info("RivetKit dev mode enabled via NODE_ENV=development");
		}
		const defaultUrl = `http://localhost:${opts.port}${normalizeBasePath(opts.path)}`;
		if (opts.dev === undefined || opts.dev === true) {
			return { url: defaultUrl, startEngine: true };
		}
		return {
			url: opts.dev.url ?? defaultUrl,
			startEngine: opts.dev.startEngine ?? true,
		};
	}

	#printWelcome(
		config: RegistryConfig,
		kind: "serverless" | "envoy" | "listen",
	): void {
		if (config.noWelcome || this.#welcomePrinted) return;
		this.#welcomePrinted = true;

		const logLine = (label: string, value: string) => {
			const padding = " ".repeat(Math.max(0, 13 - label.length));
			console.log(`  - ${label}:${padding}${value}`);
		};

		console.log();
		console.log(`  RivetKit ${VERSION} (${config.mode})`);
		logLine("Entrypoint", kind);
		logLine("Mode", config.mode);
		logLine(
			"Engine",
			config.startEngine ? "managed local" : "external",
		);

		if (config.namespace !== "default") {
			logLine("Namespace", config.namespace);
		}

		if (config.endpoint) {
			const endpointType =
				config.endpoint === ENGINE_ENDPOINT ? "local native" : "remote";
			logLine("Endpoint", `${config.endpoint} (${endpointType})`);
		}

		if (kind === "serverless" && config.publicEndpoint) {
			logLine("Client", config.publicEndpoint);
		}
		if (kind === "listen") {
			logLine("HTTP", `${config.httpHost ?? "0.0.0.0"}:${config.httpPort}`);
			logLine("Path", config.httpBasePath ?? "/api/rivet");
			logLine("Static", config.staticDir ?? "disabled");
		}

		logLine("Actors", Object.keys(config.use).length.toString());
		console.log();
	}
}

function isServerlessStartRequest(request: Request, basePath: string): boolean {
	if (request.method !== "POST") return false;
	const parsed = new URL(request.url);
	const normalizedBase =
		basePath === "/" ? "" : `/${basePath.replace(/^\/+|\/+$/g, "")}`;
	return parsed.pathname === `${normalizedBase}/start`;
}

export function setup<A extends RegistryActors>(
	input: RegistryConfigInput<A>,
): Registry<A> {
	return new Registry(input);
}

export type { RegistryConfig, RegistryActors };
export { RegistryConfigSchema };
