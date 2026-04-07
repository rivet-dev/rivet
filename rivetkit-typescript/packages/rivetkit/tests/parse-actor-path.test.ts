// Keep this test suite in sync with the Rust equivalent at
// engine/packages/guard/tests/parse_actor_path.rs
import * as cbor from "cbor-x";
import { describe, expect, test } from "vitest";
import { InvalidRequest } from "@/actor/errors";
import { parseActorPath } from "@/actor-gateway/gateway";
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
		});
	});

	describe("rvt-* query actor paths", () => {
		test("parses a get query path with multi-component keys", () => {
			const result = parseActorPath(
				"/gateway/chat-room/ws?rvt-namespace=prod&rvt-method=get&rvt-key=region-west%2F1,shard-2,member%40a&rvt-token=query%2Ftoken&debug=true",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.query).toEqual({
				getForKey: {
					name: "chat-room",
					key: ["region-west/1", "shard-2", "member@a"],
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
				`/gateway/worker/action?rvt-namespace=default&rvt-method=getOrCreate&rvt-runner=my-pool&rvt-key=tenant,job&rvt-input=${encodedInput}&rvt-region=iad&rvt-crash-policy=restart`,
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

		test("parses rvt-key= as empty key array", () => {
			const result = parseActorPath(
				"/gateway/builder?rvt-namespace=default&rvt-method=get&rvt-key=",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.query).toEqual({
				getForKey: {
					name: "builder",
					key: [],
				},
			});
			expect(result.namespace).toBe("default");
		});

		test("parses comma-separated multi-component keys", () => {
			const result = parseActorPath(
				"/gateway/lobby?rvt-namespace=default&rvt-method=get&rvt-key=a,b,c",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.query).toEqual({
				getForKey: {
					name: "lobby",
					key: ["a", "b", "c"],
				},
			});
		});

		test("strips rvt-* params from remaining path", () => {
			const result = parseActorPath(
				"/gateway/lobby/api/v1?rvt-namespace=prod&rvt-method=get&foo=bar&baz=qux",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.remainingPath).toBe("/api/v1?foo=bar&baz=qux");
		});

		test("strips all rvt-* params leaving no query string", () => {
			const result = parseActorPath(
				"/gateway/lobby/ws?rvt-namespace=prod&rvt-method=get",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.remainingPath).toBe("/ws");
		});
	});

	describe("encoding preservation", () => {
		test("preserves percent-encoding in actor query params", () => {
			const result = parseActorPath(
				"/gateway/lobby/api?rvt-namespace=default&rvt-method=get&callback=https%3A%2F%2Fexample.com&name=hello%20world",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.remainingPath).toBe(
				"/api?callback=https%3A%2F%2Fexample.com&name=hello%20world",
			);
		});

		test("preserves plus signs in actor query params", () => {
			const result = parseActorPath(
				"/gateway/lobby/api?rvt-namespace=default&rvt-method=get&search=hello+world&tag=c%2B%2B",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.remainingPath).toBe(
				"/api?search=hello+world&tag=c%2B%2B",
			);
		});

		test("handles interleaved rvt-* and actor params", () => {
			const result = parseActorPath(
				"/gateway/lobby/ws?foo=1&rvt-namespace=default&bar=2&rvt-method=get&baz=3",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.remainingPath).toBe("/ws?foo=1&bar=2&baz=3");
			expect(result.query).toEqual({
				getForKey: {
					name: "lobby",
					key: [],
				},
			});
		});

		test("decodes plus as space in rvt-* values", () => {
			const result = parseActorPath(
				"/gateway/lobby/api?rvt-namespace=my+ns&rvt-method=get&rvt-key=hello+world&q=search+term",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.query).toEqual({
				getForKey: {
					name: "lobby",
					key: ["hello world"],
				},
			});
			expect(result.namespace).toBe("my ns");
			// Actor param + is preserved literally.
			expect(result.remainingPath).toBe("/api?q=search+term");
		});

		test("preserves uppercase and lowercase percent-encoding", () => {
			const result = parseActorPath(
				"/gateway/lobby/api?rvt-namespace=default&rvt-method=get&lower=%2f&upper=%2F",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.remainingPath).toBe("/api?lower=%2f&upper=%2F");
		});

		test("strips empty parts from consecutive ampersands", () => {
			const result = parseActorPath(
				"/gateway/lobby/api?rvt-namespace=default&&rvt-method=get&&foo=bar&&baz=qux",
			);

			expect(result).not.toBeNull();
			expect(result?.type).toBe("query");
			if (!result || result.type !== "query") {
				throw new Error("expected a query actor path");
			}

			expect(result.remainingPath).toBe("/api?foo=bar&baz=qux");
		});
	});

	describe("invalid rvt-* query actor paths", () => {
		test("rejects a missing namespace", () => {
			expect(() =>
				parseActorPath("/gateway/chat-room?rvt-method=get"),
			).toThrowError(InvalidRequest);
		});

		test("rejects unknown params", () => {
			expect(() =>
				parseActorPath(
					"/gateway/chat-room?rvt-namespace=default&rvt-method=get&rvt-extra=value",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects duplicate params", () => {
			expect(() =>
				parseActorPath(
					"/gateway/chat-room?rvt-namespace=default&rvt-method=get&rvt-method=getOrCreate",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects invalid query methods", () => {
			expect(() =>
				parseActorPath(
					"/gateway/chat-room?rvt-namespace=default&rvt-method=create",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects @token syntax on query paths", () => {
			expect(() =>
				parseActorPath(
					"/gateway/chat-room@token/ws?rvt-namespace=default&rvt-method=get",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects input and region for get queries", () => {
			const encodedInput = toBase64Url(cbor.encode({ ok: true }));

			expect(() =>
				parseActorPath(
					`/gateway/chat-room?rvt-namespace=default&rvt-method=get&rvt-input=${encodedInput}`,
				),
			).toThrowError(InvalidRequest);

			expect(() =>
				parseActorPath(
					"/gateway/chat-room?rvt-namespace=default&rvt-method=get&rvt-region=iad",
				),
			).toThrowError(InvalidRequest);

			expect(() =>
				parseActorPath(
					"/gateway/chat-room?rvt-namespace=default&rvt-method=get&rvt-crash-policy=restart",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects runner for get queries", () => {
			expect(() =>
				parseActorPath(
					"/gateway/chat-room?rvt-namespace=default&rvt-method=get&rvt-runner=default",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects missing runner for getOrCreate queries", () => {
			expect(() =>
				parseActorPath(
					"/gateway/worker?rvt-namespace=default&rvt-method=getOrCreate",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects invalid base64url input", () => {
			expect(() =>
				parseActorPath(
					"/gateway/worker?rvt-namespace=default&rvt-method=getOrCreate&rvt-runner=default&rvt-input=***",
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects invalid CBOR input", () => {
			const invalidCbor = toBase64Url(new Uint8Array([0x1c]));

			expect(() =>
				parseActorPath(
					`/gateway/worker?rvt-namespace=default&rvt-method=getOrCreate&rvt-runner=default&rvt-input=${invalidCbor}`,
				),
			).toThrowError(InvalidRequest);
		});

		test("rejects an empty actor name", () => {
			expect(() =>
				parseActorPath(
					"/gateway/?rvt-namespace=default&rvt-method=get",
				),
			).toThrowError(InvalidRequest);
		});
	});
});
