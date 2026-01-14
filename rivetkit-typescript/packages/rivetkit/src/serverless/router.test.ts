import { describe, expect, test } from "vitest";
import { endpointsMatch, normalizeEndpointUrl } from "./router";

describe("normalizeEndpointUrl", () => {
	test("normalizes URL without trailing slash", () => {
		expect(normalizeEndpointUrl("http://localhost:6420")).toBe(
			"http://localhost:6420/",
		);
	});

	test("normalizes URL with trailing slash", () => {
		expect(normalizeEndpointUrl("http://localhost:6420/")).toBe(
			"http://localhost:6420/",
		);
	});

	test("normalizes 127.0.0.1 to localhost", () => {
		expect(normalizeEndpointUrl("http://127.0.0.1:6420")).toBe(
			"http://localhost:6420/",
		);
	});

	test("normalizes 0.0.0.0 to localhost", () => {
		expect(normalizeEndpointUrl("http://0.0.0.0:6420")).toBe(
			"http://localhost:6420/",
		);
	});

	test("preserves path without trailing slash", () => {
		expect(normalizeEndpointUrl("http://example.com/api/v1")).toBe(
			"http://example.com/api/v1",
		);
	});

	test("removes trailing slash from path", () => {
		expect(normalizeEndpointUrl("http://example.com/api/v1/")).toBe(
			"http://example.com/api/v1",
		);
	});

	test("removes multiple trailing slashes", () => {
		expect(normalizeEndpointUrl("http://example.com/api///")).toBe(
			"http://example.com/api",
		);
	});

	test("preserves port", () => {
		expect(normalizeEndpointUrl("https://localhost:3000/api")).toBe(
			"https://localhost:3000/api",
		);
	});

	test("strips query string", () => {
		expect(normalizeEndpointUrl("http://example.com/api?foo=bar")).toBe(
			"http://example.com/api",
		);
	});

	test("strips fragment", () => {
		expect(normalizeEndpointUrl("http://example.com/api#section")).toBe(
			"http://example.com/api",
		);
	});

	test("returns null for invalid URL", () => {
		expect(normalizeEndpointUrl("not-a-url")).toBeNull();
	});

	test("returns null for empty string", () => {
		expect(normalizeEndpointUrl("")).toBeNull();
	});
});

describe("endpointsMatch", () => {
	test("matches identical URLs", () => {
		expect(
			endpointsMatch("http://127.0.0.1:6420", "http://127.0.0.1:6420"),
		).toBe(true);
	});

	test("matches URL with and without trailing slash", () => {
		expect(
			endpointsMatch("http://127.0.0.1:6420", "http://127.0.0.1:6420/"),
		).toBe(true);
	});

	test("matches URLs with paths ignoring trailing slash", () => {
		expect(
			endpointsMatch("http://example.com/api/v1", "http://example.com/api/v1/"),
		).toBe(true);
	});

	test("matches localhost and 127.0.0.1", () => {
		expect(
			endpointsMatch("http://localhost:6420", "http://127.0.0.1:6420"),
		).toBe(true);
	});

	test("matches localhost and 0.0.0.0", () => {
		expect(
			endpointsMatch("http://localhost:6420", "http://0.0.0.0:6420"),
		).toBe(true);
	});

	test("does not match different hosts", () => {
		expect(
			endpointsMatch("http://localhost:6420", "http://example.com:6420"),
		).toBe(false);
	});

	test("does not match different ports", () => {
		expect(
			endpointsMatch("http://localhost:6420", "http://localhost:3000"),
		).toBe(false);
	});

	test("does not match different protocols", () => {
		expect(
			endpointsMatch("http://localhost:6420", "https://localhost:6420"),
		).toBe(false);
	});

	test("does not match different paths", () => {
		expect(
			endpointsMatch("http://example.com/api/v1", "http://example.com/api/v2"),
		).toBe(false);
	});

	test("falls back to string comparison for invalid URLs", () => {
		expect(endpointsMatch("not-a-url", "not-a-url")).toBe(true);
		expect(endpointsMatch("not-a-url", "different")).toBe(false);
	});
});
