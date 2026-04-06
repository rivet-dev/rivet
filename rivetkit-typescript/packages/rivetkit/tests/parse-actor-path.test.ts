// Keep this test suite in sync with the Rust equivalent at
// engine/packages/guard/tests/parse_actor_path.rs
import * as cbor from "cbor-x";
import { describe, expect, test } from "vitest";
import { InvalidRequest } from "@/actor/errors";
import { parseActorPath } from "@/manager/gateway";
import { toBase64Url } from "./test-utils";

describe("parseActorPath", () => {
	describe("direct actor paths", () => {
		test("parses a direct actor path with token", () => {
			const result = parseActorPath(
				"/gateway/actor-123@my-token/api/v1/endpoint",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("direct");
			if (!result || result.type !== "direct") {
				throw new Error("expected a direct actor path");
			}

			expect(result.actorId).toBe("actor-123");
			expect(result.token).toBe("my-token");
			expect(result.remainingPath).toBe("/api/v1/endpoint");
		});

		test("parses a direct actor path without token and preserves the query string", () => {
			const result = parseActorPath(
				"/gateway/actor-456/api/endpoint?foo=bar&baz=qux",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("direct");
			if (!result || result.type !== "direct") {
				throw new Error("expected a direct actor path");
			}

			expect(result.actorId).toBe("actor-456");
			expect(result.token).toBeUndefined();
			expect(result.remainingPath).toBe("/api/endpoint?foo=bar&baz=qux");
		});

		test("strips fragments and preserves a root remaining path", () => {
			const result = parseActorPath(
				"/gateway/actor-123?direct=true#frag",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("direct");
			if (!result || result.type !== "direct") {
				throw new Error("expected a direct actor path");
			}

			expect(result.actorId).toBe("actor-123");
			expect(result.remainingPath).toBe("/?direct=true");
		});

		test("decodes URL-encoded actor IDs and tokens", () => {
			const result = parseActorPath(
				"/gateway/actor%2D123@token%40value/endpoint",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("direct");
			if (!result || result.type !== "direct") {
				throw new Error("expected a direct actor path");
			}

			expect(result.actorId).toBe("actor-123");
			expect(result.token).toBe("token@value");
			expect(result.remainingPath).toBe("/endpoint");
		});

		test("rejects malformed direct actor paths", () => {
			expect(parseActorPath("/api/123/endpoint")).toBeNull();
			expect(parseActorPath("/gateway")).toBeNull();
			expect(parseActorPath("/gateway/@token/endpoint")).toBeNull();
			expect(parseActorPath("/gateway/actor-123@/endpoint")).toBeNull();
			expect(parseActorPath("/gateway//endpoint")).toBeNull();
			expect(parseActorPath("/gateway/actor%ZZ123/endpoint")).toBeNull();
		});
	});

	describe("matrix query actor paths", () => {
		test("parses a get query path with special-character keys and preserves empty key components", () => {
			const result = parseActorPath(
				"/gateway/chat-room;namespace=prod;method=get;key=room%2C1%2Fwest,,member%40a;token=query%2Ftoken/ws?debug=true",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.query).toEqual({
				getForKey: {
					name: "chat-room",
					key: ["room,1/west", "", "member@a"],
				},
			});
			expect(result.namespace).toBe("prod");
			expect(result.crashPolicy).toBeUndefined();
			expect(result.token).toBe("query/token");
			expect(result.remainingPath).toBe("/ws?debug=true");
		});

		test("parses getOrCreate input from base64url CBOR", () => {
			const input = { message: "hello", count: 2 };
			const encodedInput = toBase64Url(cbor.encode(input));

			const result = parseActorPath(
				`/gateway/worker;namespace=default;method=getOrCreate;runnerName=my-pool;key=tenant,job;input=${encodedInput};region=iad;crashPolicy=restart/action`,
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.query).toEqual({
				getOrCreateForKey: {
					name: "worker",
					key: ["tenant", "job"],
					input,
					region: "iad",
				},
			});
			expect(result.namespace).toBe("default");
			expect(result.runnerName).toBe("my-pool");
			expect(result.crashPolicy).toBe("restart");
			expect(result.remainingPath).toBe("/action");
		});

		test("parses key= as a single empty-string key component", () => {
			const result = parseActorPath(
				"/gateway/builder;namespace=default;method=get;key=",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.query).toEqual({
				getForKey: {
					name: "builder",
					key: [""],
				},
			});
			expect(result.namespace).toBe("default");
		});
	});

	describe("invalid matrix query actor paths", () => {
		test("rejects a missing namespace", () => {
			expect(() =>
				parseActorPath("/gateway/chat-room;method=get"),
			).toThrowError(InvalidRequest);
		});

		test("rejects unknown params", () => {
			expect(() =>
				parseActorPath(
					"/gateway/chat-room;namespace=default;method=get;extra=value",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects duplicate params", () => {
			expect(() =>
				parseActorPath(
					"/gateway/chat-room;namespace=default;method=get;name=other-room",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects create query methods", () => {
			expect(() =>
				parseActorPath(
					"/gateway/chat-room;namespace=default;method=create",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects params missing '='", () => {
			expect(() =>
				parseActorPath("/gateway/chat-room;namespace=default;method/get"),
			).toThrowError(InvalidRequest);
		});

		test("rejects invalid percent-encoding", () => {
			expect(() =>
				parseActorPath("/gateway/chat%ZZroom;namespace=default;method=get"),
			).toThrowError(InvalidRequest);
		});

		test("rejects @token syntax on query paths", () => {
			expect(() =>
				parseActorPath(
					"/gateway/chat-room;namespace=default;method=get@token/ws",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects input and region for get queries", () => {
			const encodedInput = toBase64Url(cbor.encode({ ok: true }));

			expect(() =>
				parseActorPath(
					`/gateway/chat-room;namespace=default;method=get;input=${encodedInput}`,
				),
			).toThrowError(InvalidRequest);

			expect(() =>
				parseActorPath(
					"/gateway/chat-room;namespace=default;method=get;region=iad",
				),
			).toThrowError(InvalidRequest);

			expect(() =>
				parseActorPath(
					"/gateway/chat-room;namespace=default;method=get;crashPolicy=restart",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects runnerName for get queries", () => {
			expect(() =>
				parseActorPath(
					"/gateway/chat-room;namespace=default;method=get;runnerName=default",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects missing runnerName for getOrCreate queries", () => {
			expect(() =>
				parseActorPath(
					"/gateway/worker;namespace=default;method=getOrCreate",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects invalid base64url input", () => {
			expect(() =>
				parseActorPath(
					"/gateway/worker;namespace=default;method=getOrCreate;runnerName=default;input=***",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects invalid CBOR input", () => {
			const invalidCbor = toBase64Url(new Uint8Array([0x1c]));

			expect(() =>
				parseActorPath(
					`/gateway/worker;namespace=default;method=getOrCreate;runnerName=default;input=${invalidCbor}`,
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects an empty actor name", () => {
			expect(() =>
				parseActorPath("/gateway/;namespace=default;method=get"),
			).toThrowError(InvalidRequest);
		});
	});
});
