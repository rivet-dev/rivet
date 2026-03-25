import * as cbor from "cbor-x";
import { Hono } from "hono";
import { describe, expect, test } from "vitest";
import { toBase64Url } from "./test-utils";
import type { Encoding } from "@/actor/mod";
import type {
	ActorOutput,
	GetForIdInput,
	GetOrCreateWithKeyInput,
	GetWithKeyInput,
	ListActorsInput,
	ManagerDisplayInformation,
	ManagerDriver,
} from "@/driver-helpers/mod";
import { actorGateway } from "@/manager/gateway";
import { RegistryConfigSchema } from "@/registry";
import type { GetUpgradeWebSocket } from "@/utils";

describe("actorGateway query path routing", () => {
	test("resolves get query paths before proxying http requests", async () => {
		const getWithKeyCalls: GetWithKeyInput[] = [];
		const proxiedRequests: Array<{ actorId: string; request: Request }> =
			[];
		const managerDriver = createManagerDriver({
			async getWithKey(input) {
				getWithKeyCalls.push(input);
				return {
					actorId: "resolved-get-actor",
					name: input.name,
					key: input.key,
				};
			},
			async proxyRequest(_c, actorRequest, actorId) {
				proxiedRequests.push({ actorId, request: actorRequest });
				return new Response("proxied", { status: 202 });
			},
		});
		const app = createGatewayApp(managerDriver);

		const response = await app.request(
			"http://example.com/gateway/chat-room;namespace=default;method=get;key=tenant,room/messages?watch=true",
			{
				method: "POST",
				headers: {
					"x-test-header": "present",
				},
				body: "payload",
			},
		);

		expect(response.status).toBe(202);
		expect(getWithKeyCalls).toHaveLength(1);
		expect(getWithKeyCalls[0]).toMatchObject({
			name: "chat-room",
			key: ["tenant", "room"],
		});
		expect(proxiedRequests).toHaveLength(1);
		expect(proxiedRequests[0]?.actorId).toBe("resolved-get-actor");
		expect(proxiedRequests[0]?.request.url).toBe(
			"http://actor/messages?watch=true",
		);
		expect(proxiedRequests[0]?.request.headers.get("x-test-header")).toBe(
			"present",
		);
	});

	test("resolves getOrCreate query paths before proxying http requests", async () => {
		const input = { job: "sync", attempts: 2 };
		const encodedInput = toBase64Url(cbor.encode(input));
		const getOrCreateCalls: GetOrCreateWithKeyInput[] = [];
		const proxiedActorIds: string[] = [];
		const managerDriver = createManagerDriver({
			async getOrCreateWithKey(input) {
				getOrCreateCalls.push(input);
				return {
					actorId: "resolved-get-or-create-actor",
					name: input.name,
					key: input.key,
				};
			},
			async proxyRequest(_c, _actorRequest, actorId) {
				proxiedActorIds.push(actorId);
				return new Response("proxied");
			},
		});
		const app = createGatewayApp(managerDriver);

		const response = await app.request(
			`http://example.com/gateway/worker;namespace=default;method=getOrCreate;runnerName=default;key=tenant,job;input=${encodedInput};region=us-west-2;crashPolicy=restart/input`,
		);

		expect(response.status).toBe(200);
		expect(getOrCreateCalls).toEqual([
			expect.objectContaining({
				name: "worker",
				key: ["tenant", "job"],
				input,
				region: "us-west-2",
				crashPolicy: "restart",
			}),
		]);
		expect(proxiedActorIds).toEqual(["resolved-get-or-create-actor"]);
	});

	test("resolves getOrCreate query paths before proxying websocket requests", async () => {
		const input = { source: "gateway-test" };
		const encodedInput = toBase64Url(cbor.encode(input));
		const getOrCreateCalls: GetOrCreateWithKeyInput[] = [];
		const proxiedSockets: Array<{
			actorId: string;
			path: string;
			encoding: Encoding;
			params: unknown;
		}> = [];
		const managerDriver = createManagerDriver({
			async getOrCreateWithKey(input) {
				getOrCreateCalls.push(input);
				return {
					actorId: "resolved-get-or-create-actor",
					name: input.name,
					key: input.key,
				};
			},
			async proxyWebSocket(_c, path, actorId, encoding, params) {
				proxiedSockets.push({ actorId, path, encoding, params });
				return new Response("ws proxied", { status: 201 });
			},
		});
		const app = createGatewayApp(
			managerDriver,
			() => (_createEvents) => async () =>
				new Response(null, { status: 101 }),
		);

		const response = await app.request(
			`http://example.com/gateway/builder;namespace=default;method=getOrCreate;runnerName=default;input=${encodedInput};region=iad;crashPolicy=restart/connect`,
			{
				headers: {
					upgrade: "websocket",
					"sec-websocket-protocol": "json",
				},
			},
		);

		expect(response.status).toBe(201);
		expect(getOrCreateCalls).toEqual([
			expect.objectContaining({
				name: "builder",
				key: [],
				input,
				region: "iad",
				crashPolicy: "restart",
			}),
		]);
		expect(proxiedSockets).toEqual([
			{
				actorId: "resolved-get-or-create-actor",
				path: "/connect",
				encoding: "json",
				params: undefined,
			},
		]);
	});

	test("returns 500 when getWithKey throws actor not found", async () => {
		const managerDriver = createManagerDriver({
			async getWithKey(_input) {
				return undefined;
			},
		});
		const app = createGatewayApp(managerDriver);

		const response = await app.request(
			"http://example.com/gateway/missing;namespace=default;method=get;key=nope/action",
		);

		expect(response.status).toBe(500);
	});

	test("returns 500 when getOrCreateWithKey driver method throws", async () => {
		const managerDriver = createManagerDriver({
			async getOrCreateWithKey(_input) {
				throw new Error("runner unavailable");
			},
		});
		const app = createGatewayApp(
			managerDriver,
			() => (_createEvents) => async () =>
				new Response(null, { status: 101 }),
		);

		const response = await app.request(
			"http://example.com/gateway/worker;namespace=default;method=getOrCreate;runnerName=default/connect",
			{
				headers: {
					upgrade: "websocket",
					"sec-websocket-protocol": "json",
				},
			},
		);

		expect(response.status).toBe(500);
	});

	test("preserves query string through query path resolution", async () => {
		const proxiedRequests: Array<{ request: Request }> = [];
		const managerDriver = createManagerDriver({
			async getOrCreateWithKey(input) {
				return {
					actorId: "qs-actor",
					name: input.name,
					key: input.key,
				};
			},
			async proxyRequest(_c, actorRequest, _actorId) {
				proxiedRequests.push({ request: actorRequest });
				return new Response("ok");
			},
		});
		const app = createGatewayApp(managerDriver);

		await app.request(
			"http://example.com/gateway/svc;namespace=default;method=getOrCreate;runnerName=default;key=a/data?format=json&page=2",
		);

		expect(proxiedRequests).toHaveLength(1);
		expect(proxiedRequests[0]?.request.url).toBe(
			"http://actor/data?format=json&page=2",
		);
	});

	test("keeps direct actor path routing unchanged", async () => {
		const getWithKeyCalls: GetWithKeyInput[] = [];
		const proxiedActorIds: string[] = [];
		const managerDriver = createManagerDriver({
			async getWithKey(input) {
				getWithKeyCalls.push(input);
				return {
					actorId: "should-not-be-used",
					name: input.name,
					key: input.key,
				};
			},
			async proxyRequest(_c, _actorRequest, actorId) {
				proxiedActorIds.push(actorId);
				return new Response("proxied");
			},
		});
		const app = createGatewayApp(managerDriver);

		const response = await app.request(
			"http://example.com/gateway/direct-actor-id/status",
		);

		expect(response.status).toBe(200);
		expect(getWithKeyCalls).toEqual([]);
		expect(proxiedActorIds).toEqual(["direct-actor-id"]);
	});
});


function createGatewayApp(
	managerDriver: ManagerDriver,
	getUpgradeWebSocket?: GetUpgradeWebSocket,
) {
	const app = new Hono();
	const config = RegistryConfigSchema.parse({
		use: {},
		inspector: {},
	});

	app.use(
		"*",
		actorGateway.bind(
			undefined,
			config,
			managerDriver,
			getUpgradeWebSocket,
		),
	);
	app.all("*", (c) => c.text("next", 418));

	return app;
}

function createManagerDriver(
	overrides: Partial<ManagerDriver> = {},
): ManagerDriver {
	return {
		async getForId(
			_input: GetForIdInput,
		): Promise<ActorOutput | undefined> {
			throw new Error("getForId not implemented in test");
		},
		async getWithKey(
			_input: GetWithKeyInput,
		): Promise<ActorOutput | undefined> {
			throw new Error("getWithKey not implemented in test");
		},
		async getOrCreateWithKey(
			_input: GetOrCreateWithKeyInput,
		): Promise<ActorOutput> {
			throw new Error("getOrCreateWithKey not implemented in test");
		},
		async createActor(_input): Promise<ActorOutput> {
			throw new Error("createActor not implemented in test");
		},
		async listActors(_input: ListActorsInput): Promise<ActorOutput[]> {
			throw new Error("listActors not implemented in test");
		},
		async sendRequest(_target, _actorRequest): Promise<Response> {
			throw new Error("sendRequest not implemented in test");
		},
		async openWebSocket(_path, _target, _encoding, _params) {
			throw new Error("openWebSocket not implemented in test");
		},
		async proxyRequest(_c, _actorRequest, _actorId): Promise<Response> {
			throw new Error("proxyRequest not implemented in test");
		},
		async proxyWebSocket(
			_c,
			_path,
			_actorId,
			_encoding,
			_params,
		): Promise<Response> {
			throw new Error("proxyWebSocket not implemented in test");
		},
		async buildGatewayUrl(_target): Promise<string> {
			throw new Error("buildGatewayUrl not implemented in test");
		},
		displayInformation(): ManagerDisplayInformation {
			return { properties: {} };
		},
		setGetUpgradeWebSocket() {},
		async kvGet(
			_actorId: string,
			_key: Uint8Array,
		): Promise<string | null> {
			throw new Error("kvGet not implemented in test");
		},
		...overrides,
	};
}
