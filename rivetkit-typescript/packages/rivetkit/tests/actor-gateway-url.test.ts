import * as cbor from "cbor-x";
import { describe, expect, test } from "vitest";
import {
	ClientConfigSchema,
	DEFAULT_MAX_QUERY_INPUT_SIZE,
} from "@/client/config";
import { parseActorPath } from "@/manager/gateway";
import {
	buildActorGatewayUrl,
	buildActorQueryGatewayUrl,
} from "@/remote-manager-driver/actor-websocket-client";
import { toBase64Url } from "./test-utils";

describe("gateway URL builders", () => {
	test("defaults maxInputSize to 4 KiB", () => {
		const config = ClientConfigSchema.parse({
			endpoint: "https://api.rivet.dev",
		});

		expect(config.maxInputSize).toBe(DEFAULT_MAX_QUERY_INPUT_SIZE);
	});

	test("preserves direct actor ID paths", () => {
		const url = buildActorGatewayUrl(
			"https://api.rivet.dev/manager",
			"actor/123",
			"tok/en",
			"/status?watch=true",
		);

		expect(url).toBe(
			"https://api.rivet.dev/manager/gateway/actor%2F123@tok%2Fen/status?watch=true",
		);
	});

	test("serializes get queries with comma-separated key encoding", () => {
		const url = buildActorQueryGatewayUrl(
			"https://api.rivet.dev/manager",
			"prod",
			{
				getForKey: {
					name: "alpha team",
					key: ["part/one", "shard-2", "100%"],
				},
			},
			"tok/en",
			"/status",
		);

		const urlObj = new URL(url);
		const params = urlObj.searchParams;
		expect(params.get("rvt-namespace")).toBe("prod");
		expect(params.get("rvt-method")).toBe("get");
		expect(params.get("rvt-key")).toBe("part/one,shard-2,100%");
		expect(params.get("rvt-token")).toBe("tok/en");
		expect(urlObj.pathname).toContain("/gateway/alpha%20team/status");
		expect(url).not.toContain("@");
	});

	test("serializes getOrCreate queries with rvt-* params", () => {
		const input = { hello: "world" };
		const url = buildActorQueryGatewayUrl(
			"https://api.rivet.dev/manager",
			"default",
			{
				getOrCreateForKey: {
					name: "room",
					key: ["user", ""],
					input,
					region: "local/us-west",
				},
			},
			"tok/en",
			"/connect",
			undefined,
			undefined,
			"my-pool",
		);

		const urlObj = new URL(url);
		const params = urlObj.searchParams;
		expect(params.get("rvt-namespace")).toBe("default");
		expect(params.get("rvt-method")).toBe("getOrCreate");
		expect(params.get("rvt-runner")).toBe("my-pool");
		expect(params.get("rvt-key")).toBe("user,");
		expect(params.get("rvt-input")).toBe(toBase64Url(cbor.encode(input)));
		expect(params.get("rvt-region")).toBe("local/us-west");
		expect(params.get("rvt-crash-policy")).toBe("sleep");
		expect(params.get("rvt-token")).toBe("tok/en");
		expect(urlObj.pathname).toContain("/gateway/room/connect");
	});

	test("omits rvt-key param for empty key arrays", () => {
		const url = buildActorQueryGatewayUrl(
			"https://api.rivet.dev/manager",
			"default",
			{
				getOrCreateForKey: {
					name: "room",
					key: [],
					input: { ready: true },
					region: "iad",
				},
			},
			undefined,
			"",
			undefined,
			undefined,
			"default",
		);

		expect(new URL(url).searchParams.has("rvt-key")).toBe(false);
	});

	test("rejects oversized query input before base64url encoding", () => {
		const input = {
			message:
				"query-backed inputs should be checked before base64url encoding",
		};
		const encodedSize = cbor.encode(input).byteLength;

		expect(() =>
			buildActorQueryGatewayUrl(
				"https://api.rivet.dev/manager",
				"default",
				{
					getOrCreateForKey: {
						name: "room",
						key: ["oversized"],
						input,
					},
				},
				undefined,
				"",
				encodedSize - 1,
				undefined,
				"default",
			),
		).toThrowError(
			`Actor query input exceeds maxInputSize (${encodedSize} > ${encodedSize - 1} bytes). Increase client maxInputSize to allow larger query payloads.`,
		);
	});

	test("allows larger query input when maxInputSize is increased", () => {
		const input = {
			message:
				"query-backed inputs should be checked before base64url encoding",
		};
		const encodedSize = cbor.encode(input).byteLength;

		const url = buildActorQueryGatewayUrl(
			"https://api.rivet.dev/manager",
			"default",
			{
				getOrCreateForKey: {
					name: "room",
					key: ["room"],
					input,
				},
			},
			undefined,
			"",
			encodedSize,
			undefined,
			"default",
		);

		expect(new URL(url).searchParams.has("rvt-input")).toBe(true);
	});

	test("rejects create queries for gateway URLs", () => {
		expect(() =>
			buildActorQueryGatewayUrl(
				"https://api.rivet.dev/manager",
				"default",
				{
					create: {
						name: "creator",
						key: ["room"],
					},
				} as never,
				undefined,
			),
		).toThrowError(
			"Actor query gateway URLs only support get and getOrCreate.",
		);
	});

	test("rejects crashPolicy for get queries", () => {
		expect(() =>
			buildActorQueryGatewayUrl(
				"https://api.rivet.dev/manager",
				"default",
				{
					getForKey: {
						name: "room",
						key: ["a"],
					},
				},
				undefined,
				"",
				DEFAULT_MAX_QUERY_INPUT_SIZE,
				"restart",
			),
		).toThrowError("Actor query method=get does not support crashPolicy.");
	});

	test("handles path ending with ? without producing extraneous separator", () => {
		const url = buildActorQueryGatewayUrl(
			"https://api.rivet.dev/manager",
			"default",
			{
				getForKey: {
					name: "lobby",
					key: ["room"],
				},
			},
			undefined,
			"/status?",
		);

		const urlObj = new URL(url);
		// Should not have ?& or ?? in the URL.
		expect(url).not.toContain("?&");
		expect(url).not.toContain("??");
		expect(urlObj.searchParams.get("rvt-namespace")).toBe("default");
		expect(urlObj.searchParams.get("rvt-method")).toBe("get");
	});

	test("handles path ending with & without producing extraneous separator", () => {
		const url = buildActorQueryGatewayUrl(
			"https://api.rivet.dev/manager",
			"default",
			{
				getForKey: {
					name: "lobby",
					key: ["room"],
				},
			},
			undefined,
			"/status?existing=true&",
		);

		const urlObj = new URL(url);
		// Should not have && in the URL.
		expect(url).not.toContain("&&");
		expect(urlObj.searchParams.get("existing")).toBe("true");
		expect(urlObj.searchParams.get("rvt-namespace")).toBe("default");
		expect(urlObj.searchParams.get("rvt-method")).toBe("get");
	});

	test("round-trips query gateway urls through parseActorPath", () => {
		const builtUrl = buildActorQueryGatewayUrl(
			"https://api.rivet.dev/manager",
			"prod",
			{
				getOrCreateForKey: {
					name: "builder",
					key: ["tenant", "room/1"],
					input: { ready: true },
					region: "iad",
				},
			},
			"tok/en",
			"/connect?watch=true",
			DEFAULT_MAX_QUERY_INPUT_SIZE,
			"restart",
			"my-pool",
		);

		const parsedUrl = new URL(builtUrl);
		const pathForParsing = `${parsedUrl.pathname.replace(/^\/manager/, "")}${parsedUrl.search}`;
		const parsed = parseActorPath(pathForParsing);

		expect(parsed).not.toBeNull();
		expect(parsed?.type).toBe("query");
		if (!parsed || parsed.type !== "query") {
			throw new Error("expected a query actor path");
		}

		expect(parsed.namespace).toBe("prod");
		expect(parsed.runnerName).toBe("my-pool");
		expect(parsed.crashPolicy).toBe("restart");
		expect(parsed.token).toBe("tok/en");

		// Verify the query contents are correct.
		expect(parsed.query).toEqual({
			getOrCreateForKey: {
				name: "builder",
				key: ["tenant", "room/1"],
				input: { ready: true },
				region: "iad",
			},
		});

		// The remaining path should contain the user's query params but not rvt-* params.
		expect(parsed.remainingPath).toContain("/connect");
		expect(parsed.remainingPath).toContain("watch=true");
		expect(parsed.remainingPath).not.toContain("rvt-");
	});
});
