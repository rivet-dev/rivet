import { describe, expect, test } from "vitest";
import { zodParseEndpoint, EndpointSchema } from "./endpoint-parser";

describe("zodParseEndpoint", () => {
	describe("full auth syntax", () => {
		test("parses namespace and token from endpoint", () => {
			const result = zodParseEndpoint("https://foo:bar@api.rivet.dev");
			expect(result.endpoint).toBe("https://api.rivet.dev/");
			expect(result.namespace).toBe("foo");
			expect(result.token).toBe("bar");
		});

		test("parses with port", () => {
			const result = zodParseEndpoint("https://foo:bar@api.rivet.dev:8080");
			expect(result.endpoint).toBe("https://api.rivet.dev:8080/");
			expect(result.namespace).toBe("foo");
			expect(result.token).toBe("bar");
		});

		test("parses with path", () => {
			const result = zodParseEndpoint("https://foo:bar@api.rivet.dev/v1/actors");
			expect(result.endpoint).toBe("https://api.rivet.dev/v1/actors");
			expect(result.namespace).toBe("foo");
			expect(result.token).toBe("bar");
		});

		test("throws on query string", () => {
			expect(() =>
				zodParseEndpoint("https://foo:bar@api.rivet.dev?region=us-east"),
			).toThrow("endpoint cannot contain a query string");
		});

		test("throws on fragment", () => {
			expect(() =>
				zodParseEndpoint("https://foo:bar@api.rivet.dev#section"),
			).toThrow("endpoint cannot contain a fragment");
		});

		test("handles percent-encoded characters in namespace", () => {
			const result = zodParseEndpoint("https://foo%40bar:token@api.rivet.dev");
			expect(result.endpoint).toBe("https://api.rivet.dev/");
			expect(result.namespace).toBe("foo@bar");
			expect(result.token).toBe("token");
		});

		test("handles percent-encoded characters in token", () => {
			const result = zodParseEndpoint("https://foo:bar%3Abaz@api.rivet.dev");
			expect(result.endpoint).toBe("https://api.rivet.dev/");
			expect(result.namespace).toBe("foo");
			expect(result.token).toBe("bar:baz");
		});
	});

	describe("namespace only (no token)", () => {
		test("parses namespace without token", () => {
			const result = zodParseEndpoint("https://foo@api.rivet.dev");
			expect(result.endpoint).toBe("https://api.rivet.dev/");
			expect(result.namespace).toBe("foo");
			expect(result.token).toBeUndefined();
		});

		test("parses namespace without token with path", () => {
			const result = zodParseEndpoint("https://foo@api.rivet.dev/v1/actors");
			expect(result.endpoint).toBe("https://api.rivet.dev/v1/actors");
			expect(result.namespace).toBe("foo");
			expect(result.token).toBeUndefined();
		});
	});

	describe("no auth", () => {
		test("parses endpoint without auth", () => {
			const result = zodParseEndpoint("https://api.rivet.dev");
			expect(result.endpoint).toBe("https://api.rivet.dev/");
			expect(result.namespace).toBeUndefined();
			expect(result.token).toBeUndefined();
		});

		test("parses endpoint without auth with path", () => {
			const result = zodParseEndpoint("https://api.rivet.dev/v1/actors");
			expect(result.endpoint).toBe("https://api.rivet.dev/v1/actors");
			expect(result.namespace).toBeUndefined();
			expect(result.token).toBeUndefined();
		});

		test("throws on query string without auth", () => {
			expect(() =>
				zodParseEndpoint("https://api.rivet.dev?region=us-east"),
			).toThrow("endpoint cannot contain a query string");
		});
	});

	describe("http protocol", () => {
		test("parses http endpoint with auth", () => {
			const result = zodParseEndpoint("http://foo:bar@localhost:6420");
			expect(result.endpoint).toBe("http://localhost:6420/");
			expect(result.namespace).toBe("foo");
			expect(result.token).toBe("bar");
		});

		test("parses http endpoint without auth", () => {
			const result = zodParseEndpoint("http://localhost:6420");
			expect(result.endpoint).toBe("http://localhost:6420/");
			expect(result.namespace).toBeUndefined();
			expect(result.token).toBeUndefined();
		});
	});

	describe("error handling", () => {
		test("throws on invalid URL", () => {
			expect(() => zodParseEndpoint("not-a-url")).toThrow();
		});

		test("throws on empty string", () => {
			expect(() => zodParseEndpoint("")).toThrow();
		});

		test("throws on token without namespace", () => {
			expect(() => zodParseEndpoint("https://:token@api.rivet.dev")).toThrow(
				"endpoint cannot have a token without a namespace",
			);
		});
	});
});

describe("EndpointSchema", () => {
	test("parses endpoint with full auth", () => {
		const result = EndpointSchema.parse("https://foo:bar@api.rivet.dev");
		expect(result).toEqual({
			endpoint: "https://api.rivet.dev/",
			namespace: "foo",
			token: "bar",
		});
	});

	test("parses endpoint with namespace only", () => {
		const result = EndpointSchema.parse("https://foo@api.rivet.dev");
		expect(result).toEqual({
			endpoint: "https://api.rivet.dev/",
			namespace: "foo",
			token: undefined,
		});
	});

	test("parses endpoint without auth", () => {
		const result = EndpointSchema.parse("https://api.rivet.dev");
		expect(result).toEqual({
			endpoint: "https://api.rivet.dev/",
			namespace: undefined,
			token: undefined,
		});
	});

	test("preserves path", () => {
		const result = EndpointSchema.parse(
			"https://foo:bar@api.rivet.dev/v1/actors",
		);
		expect(result).toEqual({
			endpoint: "https://api.rivet.dev/v1/actors",
			namespace: "foo",
			token: "bar",
		});
	});

	test("throws on query string", () => {
		expect(() =>
			EndpointSchema.parse("https://foo:bar@api.rivet.dev?region=us"),
		).toThrow();
	});

	test("throws on fragment", () => {
		expect(() =>
			EndpointSchema.parse("https://foo:bar@api.rivet.dev#section"),
		).toThrow();
	});

	test("throws on invalid URL", () => {
		expect(() => EndpointSchema.parse("not-a-url")).toThrow();
	});

	test("works with optional()", () => {
		const schema = EndpointSchema.optional();
		expect(schema.parse(undefined)).toBeUndefined();
		expect(schema.parse("https://foo:bar@api.rivet.dev")).toEqual({
			endpoint: "https://api.rivet.dev/",
			namespace: "foo",
			token: "bar",
		});
	});
});
