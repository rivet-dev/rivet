import {
	type RegistryActors,
	type RegistryConfig,
	type RegistryConfigInput,
	RegistryConfigSchema,
} from "./config";
import { ENGINE_ENDPOINT } from "@/common/engine";
import { buildNativeRegistry } from "./native";
import { configureServerlessPool } from "@/serverless/configure";
import { VERSION } from "@/utils";

export type FetchHandler = (
	request: Request,
	...args: any
) => Response | Promise<Response>;

export interface ServerlessHandler {
	fetch: FetchHandler;
}

export class Registry<A extends RegistryActors> {
	#config: RegistryConfigInput<A>;

	get config(): RegistryConfigInput<A> {
		return this.#config;
	}

	parseConfig(): RegistryConfig {
		return RegistryConfigSchema.parse(this.#config);
	}

	#nativeServePromise?: Promise<void>;
	#nativeServerlessPromise?: ReturnType<typeof buildNativeRegistry>;
	#configureServerlessPoolPromise?: Promise<void>;
	#welcomePrinted = false;

	constructor(config: RegistryConfigInput<A>) {
		this.#config = config;
	}

	/**
	 * Handle an incoming HTTP request for serverless deployments.
	 *
	 * @example
	 * ```ts
	 * const app = new Hono();
	 * app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));
	 * export default app;
	 * ```
	 */
	public async handler(request: Request): Promise<Response> {
		const config = this.parseConfig();
		this.#printWelcome(config, "serverless");

		if (config.configurePool && !this.#configureServerlessPoolPromise) {
			this.#configureServerlessPoolPromise =
				configureServerlessPool(config);
		}

		if (!this.#nativeServerlessPromise) {
			this.#nativeServerlessPromise = buildNativeRegistry(config);
		}

		const { bindings, registry, serveConfig } =
			await this.#nativeServerlessPromise;
		const cancelToken = new bindings.CancellationToken();
		const abort = () => cancelToken.cancel();
		if (request.signal.aborted) {
			abort();
		} else {
			request.signal.addEventListener("abort", abort, { once: true });
		}

		const requestBody = await request.arrayBuffer();
		if (
			isServerlessStartRequest(
				request,
				serveConfig.serverlessBasePath ?? "/api/rivet",
			) &&
			requestBody.byteLength > serveConfig.serverlessMaxStartPayloadBytes
		) {
			request.signal.removeEventListener("abort", abort);
			cancelToken.cancel();
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
		let controllerRef: ReadableStreamDefaultController<Uint8Array> | undefined;
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
				cancelToken.cancel();
			},
		});

		const headers: Record<string, string> = {};
		request.headers.forEach((value, key) => {
			headers[key] = value;
		});

		const head = await registry.handleServerlessRequest(
			{
				method: request.method,
				url: request.url,
				headers,
				body: Buffer.from(requestBody),
			},
			async (
				error: unknown,
				event?: {
					kind: "chunk" | "end";
					chunk?: Buffer;
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
	 * export default registry.serve();
	 * ```
	 */
	public serve(): ServerlessHandler {
		return {
			fetch: (request) => this.handler(request),
		};
	}

	/**
	 * Starts an actor envoy for standalone server deployments.
	 */
	#startEnvoy(config: RegistryConfig, printWelcome: boolean) {
		if (!this.#nativeServePromise) {
			this.#nativeServePromise = buildNativeRegistry(
				config,
			).then(async ({ registry, serveConfig }) => {
				await registry.serve(serveConfig);
			});
		}
		if (printWelcome) {
			this.#printWelcome(config, "serverful");
		}
	}

	public startEnvoy() {
		this.#startEnvoy(this.parseConfig(), true);
	}

	/**
	 * Starts the native actor envoy for standalone server deployments.
	 *
	 * @example
	 * ```ts
	 * const registry = setup({ use: { counter } });
	 * registry.start();
	 * ```
	 */
	public start() {
		const config = this.parseConfig();
		this.#startEnvoy(config, true);
	}

	#printWelcome(
		config: RegistryConfig,
		kind: "serverless" | "serverful",
	): void {
		if (config.noWelcome || this.#welcomePrinted) return;
		this.#welcomePrinted = true;

		const logLine = (label: string, value: string) => {
			const padding = " ".repeat(Math.max(0, 13 - label.length));
			console.log(`  - ${label}:${padding}${value}`);
		};

		console.log();
		console.log(
			`  RivetKit ${VERSION} (Engine - ${kind === "serverless" ? "Serverless" : "Serverful"})`,
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
