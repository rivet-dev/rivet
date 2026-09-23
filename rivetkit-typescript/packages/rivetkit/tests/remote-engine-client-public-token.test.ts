import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { ClientConfigSchema } from "@/client/config";
import { createClient } from "@/client/mod";
import {
	HEADER_RIVET_ACTOR,
	HEADER_RIVET_SKIP_READY_WAIT,
	HEADER_RIVET_TARGET,
	HEADER_RIVET_TOKEN,
	WS_PROTOCOL_ACTOR,
	WS_PROTOCOL_SKIP_READY_WAIT,
	WS_PROTOCOL_TARGET,
	WS_PROTOCOL_TOKEN,
} from "@/common/actor-router-consts";
import { RemoteEngineControlClient } from "@/engine-client/mod";

describe.sequential("RemoteEngineControlClient public token usage", () => {
	beforeEach(() => {
		vi.restoreAllMocks();
	});

	afterEach(() => {
		vi.unstubAllGlobals();
	});

	test("renews a rejected read once while leaving unrelated errors untouched", async () => {
		const getToken = vi.fn(
			async ({ forceRefresh }: { forceRefresh: boolean }) =>
				forceRefresh ? "replacement" : "first",
		);
		const tokens: (string | null)[] = [];
		vi.stubGlobal(
			"fetch",
			vi.fn(async (input: Request) => {
				const token = input.headers.get("authorization");
				tokens.push(token);
				return token === "Bearer replacement"
					? jsonResponse({ actors: [] })
					: new Response(
							JSON.stringify({
								group: "auth",
								code: "token_expired",
								message: "expired",
							}),
							{
								status: 401,
								headers: {
									"content-type": "application/json",
									"x-rivet-error": "auth.token_expired",
								},
							},
						);
			}),
		);
		const driver = new RemoteEngineControlClient(
			ClientConfigSchema.parse({
				endpoint: "https://api.rivet.dev",
				disableMetadataLookup: true,
				getToken,
			}),
		);
		expect(await driver.listActors({ name: "counter" })).toEqual([]);
		expect(tokens).toEqual(["Bearer first", "Bearer replacement"]);
		expect(getToken.mock.calls).toHaveLength(2);
	});

	test("does not replay a rejected mutation, but uses the new token next time", async () => {
		const getToken = vi.fn(
			async ({ forceRefresh }: { forceRefresh: boolean }) =>
				forceRefresh ? "replacement" : "first",
		);
		const calls: string[] = [];
		vi.stubGlobal(
			"fetch",
			vi.fn(async (input: Request) => {
				calls.push(input.headers.get("authorization") ?? "none");
				return new Response(
					JSON.stringify({
						group: "auth",
						code: "invalid_token",
						message: "invalid",
					}),
					{
						status: 401,
						headers: {
							"content-type": "application/json",
							"x-rivet-error": "auth.invalid_token",
						},
					},
				);
			}),
		);
		const driver = new RemoteEngineControlClient(
			ClientConfigSchema.parse({
				endpoint: "https://api.rivet.dev",
				disableMetadataLookup: true,
				getToken,
			}),
		);
		await expect(driver.destroyActor("actor-1")).rejects.toMatchObject({
			group: "auth",
			code: "invalid_token",
		});
		expect(calls).toEqual(["Bearer first"]);
		await expect(driver.destroyActor("actor-1")).rejects.toMatchObject({
			group: "auth",
			code: "invalid_token",
		});
		expect(calls).toEqual(["Bearer first", "Bearer replacement"]);
	});

	test("propagates issuer failure without replaying a rejected mutation", async () => {
		const issuerError = new Error("issuer unavailable");
		const getToken = vi.fn(
			async ({ forceRefresh }: { forceRefresh: boolean }) => {
				if (forceRefresh) throw issuerError;
				return "first";
			},
		);
		const fetchMock = vi.fn(
			async () =>
				new Response(
					JSON.stringify({
						group: "auth",
						code: "invalid_token",
						message: "invalid",
					}),
					{
						status: 401,
						headers: { "content-type": "application/json" },
					},
				),
		);
		vi.stubGlobal("fetch", fetchMock);
		const driver = new RemoteEngineControlClient(
			ClientConfigSchema.parse({
				endpoint: "https://api.rivet.dev",
				disableMetadataLookup: true,
				getToken,
			}),
		);
		await expect(driver.destroyActor("actor-1")).rejects.toBe(issuerError);
		expect(fetchMock).toHaveBeenCalledTimes(1);
		expect(getToken.mock.calls).toEqual([
			[{ forceRefresh: false }],
			[{ forceRefresh: true }],
		]);
	});

	test("does not renew API credentials for a 503 response", async () => {
		const getToken = vi.fn(async () => "first");
		vi.stubGlobal(
			"fetch",
			vi.fn(
				async () =>
					new Response(
						JSON.stringify({
							group: "auth",
							code: "invalid_token",
							message: "temporarily unavailable",
						}),
						{
							status: 503,
							headers: { "content-type": "application/json" },
						},
					),
			),
		);
		const driver = new RemoteEngineControlClient(
			ClientConfigSchema.parse({
				endpoint: "https://api.rivet.dev",
				disableMetadataLookup: true,
				getToken,
			}),
		);
		await expect(
			driver.listActors({ name: "counter" }),
		).rejects.toMatchObject({
			group: "auth",
			code: "invalid_token",
			statusCode: 503,
		});
		expect(getToken).toHaveBeenCalledTimes(1);
	});

	test("refreshes actor HTTP reads but never replays POST or 403", async () => {
		const getToken = vi.fn(
			async ({ forceRefresh }: { forceRefresh: boolean }) =>
				forceRefresh ? "replacement" : "first",
		);
		const calls: { method: string; token: string | null }[] = [];
		vi.stubGlobal(
			"fetch",
			vi.fn(async (input: Request | URL | string, init?: RequestInit) => {
				const request = normalizeRequest(input, init);
				calls.push({
					method: request.method,
					token: request.headers.get(HEADER_RIVET_TOKEN),
				});
				return request.headers.get(HEADER_RIVET_TOKEN) === "replacement"
					? new Response("ok")
					: new Response(null, {
							status: 401,
							headers: { "x-rivet-error": "auth.invalid_token" },
						});
			}),
		);
		const driver = new RemoteEngineControlClient(
			ClientConfigSchema.parse({
				endpoint: "https://api.rivet.dev",
				disableMetadataLookup: true,
				getToken,
			}),
		);
		const response = await driver.sendRequest(
			{ directId: "one" },
			new Request("https://actor.test/request"),
		);
		expect(response.status).toBe(200);
		expect(calls).toEqual([
			{ method: "GET", token: "first" },
			{ method: "GET", token: "replacement" },
		]);
		expect(
			(
				await driver.sendRequest(
					{ directId: "one" },
					new Request("https://actor.test/request", {
						method: "POST",
					}),
				)
			).status,
		).toBe(200);
		expect(calls).toHaveLength(3);
	});

	test("does not refresh a permission denial", async () => {
		const getToken = vi.fn(async () => "first");
		vi.stubGlobal(
			"fetch",
			vi.fn(
				async () =>
					new Response(null, {
						status: 403,
						headers: {
							"x-rivet-error": "auth.insufficient_permissions",
						},
					}),
			),
		);
		const driver = new RemoteEngineControlClient(
			ClientConfigSchema.parse({
				endpoint: "https://api.rivet.dev",
				disableMetadataLookup: true,
				getToken,
			}),
		);
		expect(
			(
				await driver.sendRequest(
					{ directId: "one" },
					new Request("https://actor.test/request"),
				)
			).status,
		).toBe(403);
		expect(getToken).toHaveBeenCalledTimes(1);
	});

	test("propagates issuer failure without replaying actor POST", async () => {
		const issuerError = new Error("issuer unavailable");
		const getToken = vi.fn(
			async ({ forceRefresh }: { forceRefresh: boolean }) => {
				if (forceRefresh) throw issuerError;
				return "first";
			},
		);
		const fetchMock = vi.fn(
			async () =>
				new Response(null, {
					status: 401,
					headers: { "x-rivet-error": "auth.invalid_token" },
				}),
		);
		vi.stubGlobal("fetch", fetchMock);
		const driver = new RemoteEngineControlClient(
			ClientConfigSchema.parse({
				endpoint: "https://api.rivet.dev",
				disableMetadataLookup: true,
				getToken,
			}),
		);
		await expect(
			driver.sendRequest(
				{ directId: "actor-1" },
				new Request("https://actor.test/request", { method: "POST" }),
			),
		).rejects.toBe(issuerError);
		expect(fetchMock).toHaveBeenCalledTimes(1);
		expect(getToken.mock.calls).toEqual([
			[{ forceRefresh: false }],
			[{ forceRefresh: true }],
		]);
	});

	test("uses renewable tokens for query handles ahead of static credentials", async () => {
		const getToken = vi.fn(async () => "dynamic");
		const requests: Request[] = [];
		vi.stubGlobal(
			"fetch",
			vi.fn(async (input: Request | URL | string, init?: RequestInit) => {
				requests.push(normalizeRequest(input, init));
				return new Response("ok");
			}),
		);
		const client = createClient({
			endpoint: "https://api.rivet.dev",
			disableMetadataLookup: true,
			token: "static",
			getToken,
		});
		await client
			.getOrCreate("mockAgenticLoop", ["counter"])
			.fetch("/status");
		expect(requests).toHaveLength(1);
		expect(new URL(requests[0]!.url).pathname).toBe(
			"/gateway/mockAgenticLoop/request/status",
		);
		expect(requests[0]?.headers.get(HEADER_RIVET_TOKEN)).toBe("dynamic");
		expect(getToken).toHaveBeenCalledTimes(1);
	});

	test("does not renew or retry a canceled actor request", async () => {
		const getToken = vi.fn(async () => "first");
		const fetchMock = vi.fn(
			async (input: Request | URL | string, init?: RequestInit) => {
				const request = normalizeRequest(input, init);
				if (request.signal.aborted) {
					throw new DOMException("canceled", "AbortError");
				}
				return new Response("ok");
			},
		);
		vi.stubGlobal("fetch", fetchMock);
		const driver = new RemoteEngineControlClient(
			ClientConfigSchema.parse({
				endpoint: "https://api.rivet.dev",
				disableMetadataLookup: true,
				getToken,
			}),
		);
		const controller = new AbortController();
		controller.abort();
		await expect(
			driver.sendRequest(
				{ directId: "actor-1" },
				new Request("https://actor.test/request", {
					signal: controller.signal,
				}),
			),
		).rejects.toMatchObject({ name: "AbortError" });
		expect(fetchMock).toHaveBeenCalledTimes(1);
		expect(getToken).toHaveBeenCalledTimes(1);
	});

	test("uses metadata clientToken for actor HTTP gateway requests", async () => {
		const fetchCalls: Request[] = [];
		const fetchMock = vi.fn(
			async (input: Request | URL | string, init?: RequestInit) => {
				const request = normalizeRequest(input, init);
				fetchCalls.push(request);

				if (
					request.url ===
					"https://backend-http.example/manager/metadata?namespace=default"
				) {
					return jsonResponse({
						runtime: "rivetkit",
						version: "test",
						runner: { kind: { normal: {} }, version: "test" },
						actorNames: {},
						clientEndpoint: "https://public-http.example/manager",
						clientNamespace: "default",
						clientToken: "public-http-token",
					});
				}

				if (
					request.url ===
					"https://public-http.example/manager/gateway/actor%2Fhttp@public-http-token/status?watch=true"
				) {
					return new Response("ok");
				}

				return new Response("ok");
			},
		);

		vi.stubGlobal("fetch", fetchMock);

		const driver = new RemoteEngineControlClient(
			ClientConfigSchema.parse({
				endpoint:
					"https://default:backend-http-token@backend-http.example/manager",
			}),
		);

		const response = await driver.sendRequest(
			{ directId: "actor/http" },
			new Request("http://actor/status?watch=true", {
				method: "POST",
				headers: {
					"x-user-header": "present",
				},
				body: "payload",
			}),
		);

		expect(response.status).toBe(200);
		expect(fetchCalls).toHaveLength(2);

		const actorRequest = fetchCalls[1];
		expect(actorRequest?.url).toBe(
			"https://public-http.example/manager/gateway/actor%2Fhttp@public-http-token/status?watch=true",
		);
		expect(actorRequest?.headers.get(HEADER_RIVET_TOKEN)).toBe(
			"public-http-token",
		);
		expect(actorRequest?.headers.get("x-user-header")).toBe("present");
	});

	test("sets skip ready wait header for actor HTTP gateway requests", async () => {
		const fetchCalls: Request[] = [];
		const fetchMock = vi.fn(
			async (input: Request | URL | string, init?: RequestInit) => {
				const request = normalizeRequest(input, init);
				fetchCalls.push(request);
				return new Response("ok");
			},
		);
		vi.stubGlobal("fetch", fetchMock);

		const driver = new RemoteEngineControlClient(
			ClientConfigSchema.parse({
				endpoint: "https://api.rivet.dev",
				disableMetadataLookup: true,
			}),
		);

		const response = await driver.sendRequest(
			{ directId: "actor-http-skip-ready-wait" },
			new Request("http://actor/request/skip-ready-wait"),
			{ skipReadyWait: true },
		);

		expect(response.status).toBe(200);
		expect(fetchCalls).toHaveLength(1);

		const actorRequest = fetchCalls[0];
		expect(actorRequest?.url).toBe(
			"https://api.rivet.dev/request/skip-ready-wait",
		);
		expect(actorRequest?.headers.get(HEADER_RIVET_TARGET)).toBe("actor");
		expect(actorRequest?.headers.get(HEADER_RIVET_ACTOR)).toBe(
			"actor-http-skip-ready-wait",
		);
		expect(actorRequest?.headers.get(HEADER_RIVET_SKIP_READY_WAIT)).toBe(
			"1",
		);
	});

	test("handle fetch forwards skip ready wait to browser request", async () => {
		const fetchCalls: Request[] = [];
		const fetchMock = vi.fn(
			async (input: Request | URL | string, init?: RequestInit) => {
				const request = normalizeRequest(input, init);
				fetchCalls.push(request);
				return new Response("ok");
			},
		);
		vi.stubGlobal("fetch", fetchMock);

		const client = createClient({
			endpoint: "https://api.rivet.dev",
			disableMetadataLookup: true,
		});
		const handle = client.getForId(
			"mockAgenticLoop",
			"actor-http-skip-ready-wait",
		);

		const response = await handle.fetch("/skip-ready-wait", {
			skipReadyWait: true,
		});

		expect(response.status).toBe(200);
		expect(fetchCalls).toHaveLength(1);

		const actorRequest = fetchCalls[0];
		expect(actorRequest?.url).toBe(
			"https://api.rivet.dev/request/skip-ready-wait",
		);
		expect(actorRequest?.headers.get(HEADER_RIVET_TARGET)).toBe("actor");
		expect(actorRequest?.headers.get(HEADER_RIVET_ACTOR)).toBe(
			"actor-http-skip-ready-wait",
		);
		expect(actorRequest?.headers.get(HEADER_RIVET_SKIP_READY_WAIT)).toBe(
			"1",
		);
	});

	test("query handle fetch keeps skip ready wait on gateway URL", async () => {
		const fetchCalls: Request[] = [];
		const fetchMock = vi.fn(
			async (input: Request | URL | string, init?: RequestInit) => {
				const request = normalizeRequest(input, init);
				fetchCalls.push(request);
				return new Response("ok");
			},
		);
		vi.stubGlobal("fetch", fetchMock);

		const client = createClient({
			endpoint: "https://api.rivet.dev",
			disableMetadataLookup: true,
			gateway: { skipReadyWait: true },
		});
		const handle = client.getOrCreate("mockAgenticLoop", [
			"query-http-skip-ready-wait",
		]);

		const response = await handle.fetch("/skip-ready-wait");

		expect(response.status).toBe(200);
		expect(fetchCalls).toHaveLength(1);

		const actorRequest = fetchCalls[0];
		expect(actorRequest).toBeDefined();
		if (!actorRequest) throw new Error("missing actor request");
		const url = new URL(actorRequest.url);
		expect(url.pathname).toBe(
			"/gateway/mockAgenticLoop/request/skip-ready-wait",
		);
		expect(url.searchParams.get("rvt-method")).toBe("getOrCreate");
		expect(url.searchParams.get("rvt-key")).toBe(
			"query-http-skip-ready-wait",
		);
		expect(url.searchParams.get("rvt-skip-ready-wait")).toBe("true");
		expect(actorRequest?.headers.get(HEADER_RIVET_TARGET)).toBeNull();
		expect(actorRequest?.headers.get(HEADER_RIVET_ACTOR)).toBeNull();
		expect(actorRequest?.headers.get(HEADER_RIVET_SKIP_READY_WAIT)).toBe(
			"1",
		);
	});

	test("uses metadata clientToken for actor websocket gateway requests", async () => {
		const fetchMock = vi.fn(
			async (input: Request | URL | string, init?: RequestInit) => {
				const request = normalizeRequest(input, init);

				if (
					request.url ===
					"https://backend-ws.example/manager/metadata?namespace=default"
				) {
					return jsonResponse({
						runtime: "rivetkit",
						version: "test",
						runner: { kind: { normal: {} }, version: "test" },
						actorNames: {},
						clientEndpoint: "https://public-ws.example/manager",
						clientNamespace: "default",
						clientToken: "public-ws-token",
					});
				}

				throw new Error(`unexpected fetch: ${request.url}`);
			},
		);

		const sockets: FakeWebSocket[] = [];
		vi.stubGlobal("fetch", fetchMock);
		vi.stubGlobal(
			"WebSocket",
			class extends FakeWebSocket {
				constructor(url: string | URL, protocols?: string | string[]) {
					super(url, protocols);
					sockets.push(this);
				}
			},
		);

		const driver = new RemoteEngineControlClient(
			ClientConfigSchema.parse({
				endpoint:
					"https://default:backend-ws-token@backend-ws.example/manager",
			}),
		);

		await driver.openWebSocket(
			"/connect",
			{ directId: "actor/ws" },
			"bare",
			{ room: "lobby" },
		);

		expect(fetchMock).toHaveBeenCalledTimes(1);
		expect(sockets).toHaveLength(1);
		expect(sockets[0]?.url).toBe(
			"https://public-ws.example/manager/gateway/actor%2Fws@public-ws-token/connect",
		);

		await driver.openWebSocket(
			"/connect",
			{ directId: "actor/ws-skip-ready-wait" },
			"bare",
			{ room: "lobby" },
			{ skipReadyWait: true },
		);

		expect(fetchMock).toHaveBeenCalledTimes(1);
		expect(sockets).toHaveLength(2);
		expect(sockets[1]?.url).toBe(
			"https://public-ws.example/manager/connect",
		);
		expect(sockets[1]?.protocols).toEqual(
			expect.arrayContaining([
				`${WS_PROTOCOL_TARGET}actor`,
				`${WS_PROTOCOL_ACTOR}actor/ws-skip-ready-wait`,
				`${WS_PROTOCOL_TOKEN}public-ws-token`,
				WS_PROTOCOL_SKIP_READY_WAIT,
			]),
		);

		await driver.openWebSocket(
			"/websocket?room=lobby",
			{ directId: "actor/ws-query" },
			"bare",
			undefined,
			{ skipReadyWait: true },
		);

		expect(sockets).toHaveLength(3);
		expect(sockets[2]?.url).toBe(
			"https://public-ws.example/manager/websocket?room=lobby",
		);
		expect(sockets[2]?.protocols).toEqual(
			expect.arrayContaining([
				`${WS_PROTOCOL_TARGET}actor`,
				`${WS_PROTOCOL_ACTOR}actor/ws-query`,
				`${WS_PROTOCOL_TOKEN}public-ws-token`,
				WS_PROTOCOL_SKIP_READY_WAIT,
			]),
		);

		const client = createClient({
			endpoint: "https://api.rivet.dev",
			disableMetadataLookup: true,
			gateway: { skipReadyWait: true },
		});
		const handle = client.getOrCreate("mockAgenticLoop", [
			"query-ws-skip-ready-wait",
		]);

		await handle.webSocket("/skip-ready-wait");

		expect(fetchMock).toHaveBeenCalledTimes(1);
		expect(sockets).toHaveLength(4);
		const querySocket = sockets[3];
		expect(querySocket).toBeDefined();
		if (!querySocket) throw new Error("missing query websocket");
		const url = new URL(querySocket.url);
		expect(url.pathname).toBe(
			"/gateway/mockAgenticLoop/websocket/skip-ready-wait",
		);
		expect(url.searchParams.get("rvt-method")).toBe("getOrCreate");
		expect(url.searchParams.get("rvt-key")).toBe(
			"query-ws-skip-ready-wait",
		);
		expect(url.searchParams.get("rvt-skip-ready-wait")).toBe("true");
		expect(querySocket.protocols).toContain(WS_PROTOCOL_SKIP_READY_WAIT);
		expect(querySocket.protocols).not.toContain(
			`${WS_PROTOCOL_TARGET}actor`,
		);
	});
});

function jsonResponse(body: unknown): Response {
	return new Response(JSON.stringify(body), {
		headers: {
			"content-type": "application/json",
		},
	});
}

function normalizeRequest(
	input: Request | URL | string,
	init?: RequestInit,
): Request {
	if (input instanceof Request) {
		return init ? new Request(input, init) : input;
	}

	return new Request(input, init);
}

class FakeWebSocket {
	static readonly OPEN = 1;
	readonly url: string;
	readonly protocols: string | string[] | undefined;
	readonly readyState = FakeWebSocket.OPEN;
	binaryType = "blob";

	constructor(url: string | URL, protocols?: string | string[]) {
		this.url = String(url);
		this.protocols = protocols;
	}

	addEventListener(): void {}

	removeEventListener(): void {}

	send(): void {}

	close(): void {}
}
