import { describe, expect, test } from "vitest";
import { parseActorPath } from "@/manager/gateway";

describe("parseActorPath", () => {
	describe("Valid paths with token", () => {
		test("should parse basic path with token", () => {
			const path = "/gateway/actor-123@my-token/api/v1/endpoint";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBe("my-token");
			expect(result?.remainingPath).toBe("/api/v1/endpoint");
		});

		test("should parse path with UUID as actor ID", () => {
			const path =
				"/gateway/12345678-1234-1234-1234-123456789abc@my-token/status";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe(
				"12345678-1234-1234-1234-123456789abc",
			);
			expect(result?.token).toBe("my-token");
			expect(result?.remainingPath).toBe("/status");
		});

		test("should parse path with token and query parameters", () => {
			const path = "/gateway/actor-456@token123/api?key=value";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-456");
			expect(result?.token).toBe("token123");
			expect(result?.remainingPath).toBe("/api?key=value");
		});

		test("should parse path with token and no remaining path", () => {
			const path = "/gateway/actor-000@tok";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-000");
			expect(result?.token).toBe("tok");
			expect(result?.remainingPath).toBe("/");
		});

		test("should parse complex path with token and multiple segments", () => {
			const path =
				"/gateway/actor-complex@secure-token/api/v2/users/123/profile/settings";
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
			const path = "/gateway/actor-123/api/v1/endpoint";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/api/v1/endpoint");
		});

		test("should parse path with UUID without token", () => {
			const path = "/gateway/12345678-1234-1234-1234-123456789abc/status";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe(
				"12345678-1234-1234-1234-123456789abc",
			);
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/status");
		});

		test("should parse path without token and with query params", () => {
			const path = "/gateway/actor-456/api/endpoint?foo=bar&baz=qux";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-456");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/api/endpoint?foo=bar&baz=qux");
		});

		test("should parse path without token and no remaining path", () => {
			const path = "/gateway/actor-000";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-000");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/");
		});
	});

	describe("Query parameters and fragments", () => {
		test("should preserve query parameters", () => {
			const path = "/gateway/actor-456/api/endpoint?foo=bar&baz=qux";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.remainingPath).toBe("/api/endpoint?foo=bar&baz=qux");
		});

		test("should strip fragment from path", () => {
			const path = "/gateway/actor-789/page#section";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-789");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/page");
		});

		test("should preserve query but strip fragment", () => {
			const path = "/gateway/actor-123/api?query=1#section";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/api?query=1");
		});

		test("should handle path with only actor ID and query string", () => {
			const path = "/gateway/actor-123?direct=true";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/?direct=true");
		});
	});

	describe("Trailing slashes", () => {
		test("should preserve trailing slash in remaining path", () => {
			const path = "/gateway/actor-111/api/";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-111");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/api/");
		});
	});

	describe("Special characters", () => {
		test("should handle actor ID with allowed special characters", () => {
			const path = "/gateway/actor_id-123.test/endpoint";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor_id-123.test");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/endpoint");
		});

		test("should handle URL encoded characters in remaining path", () => {
			const path = "/gateway/actor-123/api%20endpoint/test%2Fpath";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/api%20endpoint/test%2Fpath");
		});
	});

	describe("URL-encoded actor_id and token", () => {
		test("should decode URL-encoded characters in actor_id", () => {
			const path = "/gateway/actor%2D123/endpoint";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/endpoint");
		});

		test("should decode URL-encoded characters in token", () => {
			const path = "/gateway/actor-123@tok%40en/endpoint";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBe("tok@en");
			expect(result?.remainingPath).toBe("/endpoint");
		});

		test("should decode URL-encoded characters in both actor_id and token", () => {
			const path = "/gateway/actor%2D123@token%2Dwith%2Dencoded/endpoint";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBe("token-with-encoded");
			expect(result?.remainingPath).toBe("/endpoint");
		});

		test("should decode URL-encoded spaces in actor_id", () => {
			const path = "/gateway/actor%20with%20spaces/endpoint";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor with spaces");
			expect(result?.token).toBeUndefined();
			expect(result?.remainingPath).toBe("/endpoint");
		});

		test("should reject invalid URL encoding in actor_id", () => {
			// %ZZ is invalid hex
			const path = "/gateway/actor%ZZ123/endpoint";
			const result = parseActorPath(path);

			expect(result).toBeNull();
		});

		test("should reject invalid URL encoding in token", () => {
			// %GG is invalid hex
			const path = "/gateway/actor-123@token%GG/endpoint";
			const result = parseActorPath(path);

			expect(result).toBeNull();
		});
	});

	describe("Invalid paths - wrong prefix", () => {
		test("should reject path with wrong prefix", () => {
			expect(parseActorPath("/api/123/endpoint")).toBeNull();
		});

		test("should reject path missing gateway prefix", () => {
			expect(parseActorPath("/123/endpoint")).toBeNull();
		});
	});

	describe("Invalid paths - too short", () => {
		test("should reject path with only gateway", () => {
			expect(parseActorPath("/gateway")).toBeNull();
		});
	});

	describe("Invalid paths - malformed token", () => {
		test("should reject path with empty actor ID before @", () => {
			expect(parseActorPath("/gateway/@token/endpoint")).toBeNull();
		});

		test("should reject path with empty token after @", () => {
			expect(parseActorPath("/gateway/actor-123@/endpoint")).toBeNull();
		});
	});

	describe("Invalid paths - empty values", () => {
		test("should reject path with empty actor segment", () => {
			expect(parseActorPath("/gateway//endpoint")).toBeNull();
		});
	});

	describe("Invalid paths - double slash", () => {
		test("should reject path with double slashes", () => {
			const path = "/gateway//actor-123/endpoint";
			expect(parseActorPath(path)).toBeNull();
		});
	});

	describe("Invalid paths - case sensitive", () => {
		test("should reject path with capitalized Gateway", () => {
			expect(parseActorPath("/Gateway/123/endpoint")).toBeNull();
		});
	});

	describe("Token edge cases", () => {
		test("should handle token with special characters", () => {
			const path =
				"/gateway/actor-123@token-with-dashes_and_underscores/api";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBe("token-with-dashes_and_underscores");
			expect(result?.remainingPath).toBe("/api");
		});

		test("should handle multiple @ symbols (only first is used)", () => {
			const path = "/gateway/actor-123@token@extra/api";
			const result = parseActorPath(path);

			expect(result).not.toBeNull();
			expect(result?.actorId).toBe("actor-123");
			expect(result?.token).toBe("token@extra");
			expect(result?.remainingPath).toBe("/api");
		});
	});
});
