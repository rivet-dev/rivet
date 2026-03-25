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

	test("serializes get queries with per-component key encoding", () => {
		const url = buildActorQueryGatewayUrl(
			"https://api.rivet.dev/manager",
			"prod",
			{
				getForKey: {
					name: "alpha team",
					key: ["part/one", "", "100%"],
				},
			},
			"tok/en",
			"/status",
		);

		expect(url).toBe(
			"https://api.rivet.dev/manager/gateway/alpha%20team;namespace=prod;method=get;key=part%2Fone,,100%25;token=tok%2Fen/status",
		);
		expect(url).not.toContain("@");
	});

	test("serializes getOrCreate queries in canonical field order", () => {
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
		const expectedInput = encodeURIComponent(
			toBase64Url(cbor.encode(input)),
		);

		expect(url).toBe(
			`https://api.rivet.dev/manager/gateway/room;namespace=default;method=getOrCreate;runnerName=my-pool;key=user,;input=${expectedInput};region=local%2Fus-west;crashPolicy=sleep;token=tok%2Fen/connect`,
		);
	});

	test("omits key for empty key arrays and preserves empty string keys", () => {
		const getOrCreateUrl = buildActorQueryGatewayUrl(
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
		const emptyStringKeyUrl = buildActorQueryGatewayUrl(
			"https://api.rivet.dev/manager",
			"default",
			{
				getOrCreateForKey: {
					name: "room",
					key: [""],
				},
			},
			undefined,
			"",
			undefined,
			undefined,
			"default",
		);

		expect(getOrCreateUrl).not.toContain(";key=");
		expect(emptyStringKeyUrl).toContain(";key=");
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

		expect(url).toContain(";input=");
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
		const parsed = parseActorPath(
			`${new URL(builtUrl).pathname.replace(/^\/manager/, "")}${new URL(builtUrl).search}`,
		);

		expect(parsed).not.toBeNull();
		expect(parsed?.type).toBe("query");
		if (!parsed || parsed.type !== "query") {
			throw new Error("expected a query actor path");
		}

		const rebuiltUrl = buildActorQueryGatewayUrl(
			"https://api.rivet.dev/manager",
			parsed.namespace,
			parsed.query,
			parsed.token,
			parsed.remainingPath,
			DEFAULT_MAX_QUERY_INPUT_SIZE,
			parsed.crashPolicy,
			parsed.runnerName,
		);

		expect(rebuiltUrl).toBe(builtUrl);
	});
});
