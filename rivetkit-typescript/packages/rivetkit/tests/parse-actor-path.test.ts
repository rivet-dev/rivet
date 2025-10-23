import { describe, expect, test } from "vitest";
import { parseActorPath } from "@/manager/gateway";

describe("parseActorPath", () => {
	describe("Valid paths with token", () => {
		test("should parse basic path with token and route", () => {
			const path =
				"/gateway/actors/actor-123/tokens/my-token/route/api/v1/endpoint";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBe("my-token");
			expect(result?.remainingPath).toBe("/api/v1/endpoint");
		});

		test("should parse path with UUID as actor ID", () => {
			const path =
				"/gateway/actors/12345678-1234-1234-1234-123456789abc/tokens/my-token/route/status";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe(
				"12345678-1234-1234-1234-123456789abc",
			);
			expect(result?.token).toBe("my-token");
			expect(result?.remainingPath).toBe("/status");
		});

		test("should parse path with token and query parameters", () => {
			const path =
				"/gateway/actors/actor-456/tokens/token123/route/api?key=value";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-456");
			expect(result?.token).toBe("token123");
			expect(result?.remainingPath).toBe("/api?key=value");
		});

		test("should parse path with token and no remaining path", () => {
			const path = "/gateway/actors/actor-000/tokens/tok/route";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-000");
			expect(result?.token).toBe("tok");
			expect(result?.remainingPath).toBe("/");
		});

		test("should parse complex path with token and multiple segments", () => {
			const path =
				"/gateway/actors/actor-complex/tokens/secure-token/route/api/v2/users/123/profile/settings";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-complex");
			expect(result?.token).toBe("secure-token");
			expect(result?.remainingPath).toBe(
				"/api/v2/users/123/profile/settings",
			);
		});
	});

	describe("Valid paths without token", () => {
		test("should parse basic path without token", () => {
			const path = "/gateway/actors/actor-123/route/api/v1/endpoint";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/api/v1/endpoint");
		});

		test("should parse path with UUID without token", () => {
			const path =
				"/gateway/actors/12345678-1234-1234-1234-123456789abc/route/status";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe(
				"12345678-1234-1234-1234-123456789abc",
			);
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/status");
		});

		test("should parse path without token and with query params", () => {
			const path =
				"/gateway/actors/actor-456/route/api/endpoint?foo=bar&baz=qux";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-456");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/api/endpoint?foo=bar&baz=qux");
		});

		test("should parse path without token and no remaining path", () => {
			const path = "/gateway/actors/actor-000/route";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-000");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/");
		});
	});

	describe("Query parameters and fragments", () => {
		test("should preserve query parameters", () => {
			const path =
				"/gateway/actors/actor-456/route/api/endpoint?foo=bar&baz=qux";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.remainingPath).toBe("/api/endpoint?foo=bar&baz=qux");
		});

		test("should strip fragment from path", () => {
			const path = "/gateway/actors/actor-789/route/page#section";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-789");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/page");
		});

		test("should preserve query but strip fragment", () => {
			const path = "/gateway/actors/actor-123/route/api?query=1#section";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/api?query=1");
		});

		test("should handle path ending with route but having query string", () => {
			const path = "/gateway/actors/actor-123/route?direct=true";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/?direct=true");
		});
	});

	describe("Trailing slashes", () => {
		test("should preserve trailing slash in remaining path", () => {
			const path = "/gateway/actors/actor-111/route/api/";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-111");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/api/");
		});
	});

	describe("Special characters", () => {
		test("should handle actor ID with allowed special characters", () => {
			const path = "/gateway/actors/actor_id-123.test/route/endpoint";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor_id-123.test");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/endpoint");
		});

		test("should handle URL encoded characters in remaining path", () => {
			const path =
				"/gateway/actors/actor-123/route/api%20endpoint/test%2Fpath";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/api%20endpoint/test%2Fpath");
		});
	});

	describe("Invalid paths - wrong prefix", () => {
		test("should reject path with wrong prefix", () => {
			expect(parseActorPath("/api/actors/123/route/endpoint")).toBeNull();
		});

		test("should reject path with wrong actor keyword", () => {
			expect(
				parseActorPath("/gateway/actor/123/route/endpoint"),
			).toBeNull();
		});

		test("should reject path missing gateway prefix", () => {
			expect(parseActorPath("/actors/123/route/endpoint")).toBeNull();
		});
	});

	describe("Invalid paths - missing route", () => {
		test("should reject path without route keyword", () => {
			expect(parseActorPath("/gateway/actors/123")).toBeNull();
		});

		test("should reject path with endpoint but no route keyword", () => {
			expect(parseActorPath("/gateway/actors/123/endpoint")).toBeNull();
		});

		test("should reject path with tokens but no route keyword", () => {
			expect(parseActorPath("/gateway/actors/123/tokens/tok")).toBeNull();
		});
	});

	describe("Invalid paths - too short", () => {
		test("should reject path with only gateway", () => {
			expect(parseActorPath("/gateway")).toBeNull();
		});

		test("should reject path with only gateway and actors", () => {
			expect(parseActorPath("/gateway/actors")).toBeNull();
		});

		test("should reject path with only gateway, actors, and actor ID", () => {
			expect(parseActorPath("/gateway/actors/123")).toBeNull();
		});
	});

	describe("Invalid paths - malformed token path", () => {
		test("should reject token path missing route keyword", () => {
			expect(
				parseActorPath("/gateway/actors/123/tokens/tok/api"),
			).toBeNull();
		});

		test("should reject path with empty token", () => {
			expect(
				parseActorPath("/gateway/actors/123/tokens//route/api"),
			).toBeNull();
		});
	});

	describe("Invalid paths - wrong segment positions", () => {
		test("should reject segments in wrong order", () => {
			expect(
				parseActorPath("/actors/gateway/123/route/endpoint"),
			).toBeNull();
		});

		test("should reject route keyword in wrong position", () => {
			expect(
				parseActorPath("/gateway/route/actors/123/endpoint"),
			).toBeNull();
		});
	});

	describe("Invalid paths - empty values", () => {
		test("should reject path with empty actor ID", () => {
			expect(
				parseActorPath("/gateway/actors//route/endpoint"),
			).toBeNull();
		});

		test("should reject path with empty actor ID in token path", () => {
			expect(
				parseActorPath("/gateway/actors//tokens/tok/route/endpoint"),
			).toBeNull();
		});
	});

	describe("Invalid paths - double slash", () => {
		test("should reject path with double slashes", () => {
			const path = "/gateway/actors//actor-123/route/endpoint";
			expect(parseActorPath(path)).toBeNull();
		});
	});

	describe("Invalid paths - case sensitive", () => {
		test("should reject path with capitalized Gateway", () => {
			expect(
				parseActorPath("/Gateway/actors/123/route/endpoint"),
			).toBeNull();
		});

		test("should reject path with capitalized Actors", () => {
			expect(
				parseActorPath("/gateway/Actors/123/route/endpoint"),
			).toBeNull();
		});

		test("should reject path with capitalized Route", () => {
			expect(
				parseActorPath("/gateway/actors/123/Route/endpoint"),
			).toBeNull();
		});

		test("should reject token path with capitalized Route", () => {
			expect(
				parseActorPath("/gateway/actors/123/tokens/tok/Route/endpoint"),
			).toBeNull();
		});
	});
});
