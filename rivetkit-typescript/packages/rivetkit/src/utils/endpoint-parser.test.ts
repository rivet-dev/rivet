import { describe, expect, test } from "vitest";
import type { z } from "zod";
import { tryParseEndpoint } from "./endpoint-parser";

// Helper to create a mock Zod refinement context for testing
function createMockCtx(): { ctx: z.RefinementCtx; issues: z.ZodIssue[] } {
	const issues: z.ZodIssue[] = [];
	const ctx = {
		addIssue: (issue: z.IssueData) => {
			issues.push(issue as z.ZodIssue);
		},
		path: [],
		value: undefined,
		issues: [],
	} as unknown as z.RefinementCtx;
	return { ctx, issues };
}

describe("tryParseEndpoint", () => {
	describe("basic parsing", () => {
		test("parses endpoint with full auth", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, {
				endpoint: "https://foo:bar@api.rivet.dev",
			});
			expect(issues).toHaveLength(0);
			expect(result).toEqual({
				endpoint: "https://api.rivet.dev/",
				namespace: "foo",
				token: "bar",
			});
		});

		test("parses endpoint with namespace only", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, {
				endpoint: "https://foo@api.rivet.dev",
			});
			expect(issues).toHaveLength(0);
			expect(result).toEqual({
				endpoint: "https://api.rivet.dev/",
				namespace: "foo",
				token: undefined,
			});
		});

		test("parses endpoint without auth", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, {
				endpoint: "https://api.rivet.dev",
			});
			expect(issues).toHaveLength(0);
			expect(result).toEqual({
				endpoint: "https://api.rivet.dev/",
				namespace: undefined,
				token: undefined,
			});
		});

		test("preserves path", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, {
				endpoint: "https://foo:bar@api.rivet.dev/v1/actors",
			});
			expect(issues).toHaveLength(0);
			expect(result).toEqual({
				endpoint: "https://api.rivet.dev/v1/actors",
				namespace: "foo",
				token: "bar",
			});
		});
	});

	describe("validation errors", () => {
		test("adds issue for invalid URL", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, { endpoint: "not-a-url" });
			expect(result).toBeUndefined();
			expect(issues).toHaveLength(1);
			expect(issues[0]?.message).toContain("invalid URL");
		});

		test("adds issue for query string", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, {
				endpoint: "https://foo:bar@api.rivet.dev?region=us",
			});
			expect(result).toBeUndefined();
			expect(issues).toHaveLength(1);
			expect(issues[0]?.message).toBe("endpoint cannot contain a query string");
		});

		test("adds issue for fragment", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, {
				endpoint: "https://foo:bar@api.rivet.dev#section",
			});
			expect(result).toBeUndefined();
			expect(issues).toHaveLength(1);
			expect(issues[0]?.message).toBe("endpoint cannot contain a fragment");
		});

		test("adds issue for token without namespace", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, {
				endpoint: "https://:token@api.rivet.dev",
			});
			expect(result).toBeUndefined();
			expect(issues).toHaveLength(1);
			expect(issues[0]?.message).toBe(
				"endpoint cannot have a token without a namespace",
			);
		});
	});

	describe("duplicate credential checking", () => {
		test("adds issue when namespace in URL and config", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, {
				endpoint: "https://url-ns@api.rivet.dev",
				path: ["endpoint"],
				namespace: "config-ns",
			});
			// Still returns result, but adds issue
			expect(result).toEqual({
				endpoint: "https://api.rivet.dev/",
				namespace: "url-ns",
				token: undefined,
			});
			expect(issues).toHaveLength(1);
			expect(issues[0]?.message).toContain(
				"cannot specify namespace both in endpoint URL and as a separate config option",
			);
			expect(issues[0]?.path).toEqual(["namespace"]);
		});

		test("adds issue when token in URL and config", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, {
				endpoint: "https://ns:url-token@api.rivet.dev",
				path: ["endpoint"],
				token: "config-token",
			});
			// Still returns result, but adds issue
			expect(result).toEqual({
				endpoint: "https://api.rivet.dev/",
				namespace: "ns",
				token: "url-token",
			});
			expect(issues).toHaveLength(1);
			expect(issues[0]?.message).toContain(
				"cannot specify token both in endpoint URL and as a separate config option",
			);
			expect(issues[0]?.path).toEqual(["token"]);
		});

		test("adds issues for both namespace and token duplicates", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, {
				endpoint: "https://url-ns:url-token@api.rivet.dev",
				path: ["endpoint"],
				namespace: "config-ns",
				token: "config-token",
			});
			expect(result).toBeDefined();
			expect(issues).toHaveLength(2);
		});

		test("no issue when namespace only in URL", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, {
				endpoint: "https://url-ns@api.rivet.dev",
				path: ["endpoint"],
				token: "config-token",
			});
			expect(result).toBeDefined();
			expect(issues).toHaveLength(0);
		});

		test("no issue when namespace only in config", () => {
			const { ctx, issues } = createMockCtx();
			const result = tryParseEndpoint(ctx, {
				endpoint: "https://api.rivet.dev",
				path: ["endpoint"],
				namespace: "config-ns",
			});
			expect(result).toBeDefined();
			expect(issues).toHaveLength(0);
		});
	});

	describe("custom path", () => {
		test("uses custom path in error issues", () => {
			const { ctx, issues } = createMockCtx();
			tryParseEndpoint(ctx, {
				endpoint: "not-a-url",
				path: ["serverless", "publicEndpoint"],
			});
			expect(issues[0]?.path).toEqual(["serverless", "publicEndpoint"]);
		});
	});
});
