import { describe, expect, it } from "vitest";
import {
	formatServerlessMetadataError,
	HEALTH_CHECK_FALLBACK_ERROR,
} from "./serverless-health-error";

// Mirrors the `{message, details, metadata}` envelopes emitted by
// `ServerlessMetadataError` in
// `engine/packages/pegboard/src/ops/serverless_metadata/fetch.rs`.
describe("formatServerlessMetadataError", () => {
	it("surfaces the server message for simple variants", () => {
		expect(
			formatServerlessMetadataError({
				message: "failed to reach serverless endpoint",
				metadata: { kind: "request_failed" },
			}),
		).toBe("failed to reach serverless endpoint");
	});

	it("appends the response body for non_success_status", () => {
		expect(
			formatServerlessMetadataError({
				message: "serverless metadata request returned status 502",
				metadata: {
					kind: "non_success_status",
					status_code: 502,
					body: "Bad Gateway",
				},
			}),
		).toBe("serverless metadata request returned status 502: Bad Gateway");
	});

	it("appends the parse error for invalid_response_json", () => {
		expect(
			formatServerlessMetadataError({
				message: "serverless metadata response is not valid JSON",
				metadata: {
					kind: "invalid_response_json",
					body: "<html>",
					parse_error: "expected value at line 1 column 1",
				},
			}),
		).toBe(
			"serverless metadata response is not valid JSON: expected value at line 1 column 1",
		);
	});

	it("surfaces the full message for invalid_envoy_protocol_version (the reported bug)", () => {
		expect(
			formatServerlessMetadataError({
				message:
					"envoy protocol version 5 is not supported (max supported: 4)",
				metadata: {
					kind: "invalid_envoy_protocol_version",
					envoy_protocol_version: 5,
					max_supported_envoy_protocol_version: 4,
				},
			}),
		).toBe("envoy protocol version 5 is not supported (max supported: 4)");
	});

	it("appends details when present", () => {
		expect(
			formatServerlessMetadataError({
				message: "something failed",
				details: "extra context",
				metadata: { kind: "request_failed" },
			}),
		).toBe("something failed (extra context)");
	});

	it("still surfaces the message for an unknown future kind", () => {
		expect(
			formatServerlessMetadataError({
				message: "something new went wrong",
				metadata: { kind: "some_future_variant" },
			}),
		).toBe("something new went wrong");
	});

	it("derives a readable string from kind when message is absent", () => {
		expect(
			formatServerlessMetadataError({
				metadata: { kind: "request_timed_out" },
			}),
		).toBe("request timed out");
	});

	it("falls back when given an unparseable value", () => {
		expect(formatServerlessMetadataError(undefined)).toBe(
			HEALTH_CHECK_FALLBACK_ERROR,
		);
		expect(formatServerlessMetadataError(null)).toBe(
			HEALTH_CHECK_FALLBACK_ERROR,
		);
		expect(formatServerlessMetadataError("a string")).toBe(
			HEALTH_CHECK_FALLBACK_ERROR,
		);
	});
});
