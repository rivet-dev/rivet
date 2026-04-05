import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { ClientConfigSchema } from "@/client/config";
import { HEADER_RIVET_TOKEN } from "@/common/actor-router-consts";
import { RemoteManagerDriver } from "@/remote-manager-driver/mod";

describe.sequential("RemoteManagerDriver public token usage", () => {
	beforeEach(() => {
		vi.restoreAllMocks();
	});

	afterEach(() => {
		vi.unstubAllGlobals();
	});

	test("uses metadata clientToken for actor HTTP gateway requests", async () => {
		const fetchCalls: Request[] = [];
		const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
			const request = normalizeRequest(input);
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
		});

		vi.stubGlobal("fetch", fetchMock);

		const driver = new RemoteManagerDriver(
			ClientConfigSchema.parse({
				endpoint: "https://default:backend-http-token@backend-http.example/manager",
			}),
		);

		const response = await driver.sendRequest(
			"actor/http",
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

	test("uses metadata clientToken for actor websocket gateway requests", async () => {
		const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
			const request = normalizeRequest(input);

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
		});

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

		const driver = new RemoteManagerDriver(
			ClientConfigSchema.parse({
				endpoint: "https://default:backend-ws-token@backend-ws.example/manager",
			}),
		);

		await driver.openWebSocket(
			"/connect",
			"actor/ws",
			"bare",
			{ room: "lobby" },
		);

		expect(fetchMock).toHaveBeenCalledTimes(1);
		expect(sockets).toHaveLength(1);
		expect(sockets[0]?.url).toBe(
			"https://public-ws.example/manager/gateway/actor%2Fws@public-ws-token/connect",
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

function normalizeRequest(input: RequestInfo | URL): Request {
	if (input instanceof Request) {
		return input;
	}

	return new Request(input);
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
