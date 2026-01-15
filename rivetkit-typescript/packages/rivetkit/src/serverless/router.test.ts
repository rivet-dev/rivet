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

	test("normalizes IPv6 loopback [::1] to localhost", () => {
		expect(normalizeEndpointUrl("http://[::1]:6420")).toBe(
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

	describe("regional endpoint normalization", () => {
		test("normalizes api-us-west-1.rivet.dev to api.rivet.dev", () => {
			expect(normalizeEndpointUrl("https://api-us-west-1.rivet.dev")).toBe(
				"https://api.rivet.dev/",
			);
		});

		test("normalizes api-lax.staging.rivet.dev to api.staging.rivet.dev", () => {
			expect(
				normalizeEndpointUrl("https://api-lax.staging.rivet.dev"),
			).toBe("https://api.staging.rivet.dev/");
		});

		test("preserves api.rivet.dev unchanged", () => {
			expect(normalizeEndpointUrl("https://api.rivet.dev")).toBe(
				"https://api.rivet.dev/",
			);
		});

		test("does not normalize non-api prefixed hostnames", () => {
			expect(normalizeEndpointUrl("https://foo-bar.rivet.dev")).toBe(
				"https://foo-bar.rivet.dev/",
			);
		});

		test("does not normalize non-rivet.dev domains", () => {
			expect(normalizeEndpointUrl("https://api-us-west-1.example.com")).toBe(
				"https://api-us-west-1.example.com/",
			);
		});

		test("preserves path when normalizing regional endpoint", () => {
			expect(
				normalizeEndpointUrl("https://api-us-west-1.rivet.dev/v1/actors"),
			).toBe("https://api.rivet.dev/v1/actors");
		});

		test("preserves port when normalizing regional endpoint", () => {
			expect(
				normalizeEndpointUrl("https://api-us-west-1.rivet.dev:8080"),
			).toBe("https://api.rivet.dev:8080/");
		});
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

	test("matches localhost and IPv6 loopback [::1]", () => {
		expect(
			endpointsMatch("http://localhost:6420", "http://[::1]:6420"),
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

	describe("regional endpoint matching", () => {
		test("matches api.rivet.dev with api-us-west-1.rivet.dev", () => {
			expect(
				endpointsMatch(
					"https://api.rivet.dev",
					"https://api-us-west-1.rivet.dev",
				),
			).toBe(true);
		});

		test("matches api-us-west-1.rivet.dev with api.rivet.dev (reverse order)", () => {
			expect(
				endpointsMatch(
					"https://api-us-west-1.rivet.dev",
					"https://api.rivet.dev",
				),
			).toBe(true);
		});

		test("matches api.staging.rivet.dev with api-lax.staging.rivet.dev", () => {
			expect(
				endpointsMatch(
					"https://api.staging.rivet.dev",
					"https://api-lax.staging.rivet.dev",
				),
			).toBe(true);
		});

		test("matches api-lax.staging.rivet.dev with api.staging.rivet.dev (reverse order)", () => {
			expect(
				endpointsMatch(
					"https://api-lax.staging.rivet.dev",
					"https://api.staging.rivet.dev",
				),
			).toBe(true);
		});

		test("matches with paths", () => {
			expect(
				endpointsMatch(
					"https://api.rivet.dev/v1/actors",
					"https://api-us-west-1.rivet.dev/v1/actors",
				),
			).toBe(true);
		});

		test("does not match different domains", () => {
			expect(
				endpointsMatch(
					"https://api.rivet.dev",
					"https://api-us-west-1.example.com",
				),
			).toBe(false);
		});

		test("does not match different protocols", () => {
			expect(
				endpointsMatch(
					"http://api.rivet.dev",
					"https://api-us-west-1.rivet.dev",
				),
			).toBe(false);
		});

		test("does not match different paths", () => {
			expect(
				endpointsMatch(
					"https://api.rivet.dev/v1",
					"https://api-us-west-1.rivet.dev/v2",
				),
			).toBe(false);
		});

		test("does not match different ports", () => {
			expect(
				endpointsMatch(
					"https://api.rivet.dev:8080",
					"https://api-us-west-1.rivet.dev:9090",
				),
			).toBe(false);
		});

		test("matches with same port", () => {
			expect(
				endpointsMatch(
					"https://api.rivet.dev:8080",
					"https://api-us-west-1.rivet.dev:8080",
				),
			).toBe(true);
		});

		test("does not match non-api prefixed hosts", () => {
			expect(
				endpointsMatch(
					"https://foo.rivet.dev",
					"https://foo-us-west-1.rivet.dev",
				),
			).toBe(false);
		});

		test("does not match api.staging.rivet.dev with api-us-west-1.rivet.dev (different base domains)", () => {
			expect(
				endpointsMatch(
					"https://api.staging.rivet.dev",
					"https://api-us-west-1.rivet.dev",
				),
			).toBe(false);
		});
	});
});
