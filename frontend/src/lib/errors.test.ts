import { describe, expect, it } from "vitest";

import { isAuthError } from "./errors";

function apiError(statusCode: number, group: string, code: string) {
	return {
		statusCode,
		message: `${group}.${code}`,
		body: { group, code, message: `${group}.${code}` },
	};
}

describe("isAuthError", () => {
	it.each([
		"invalid_token",
		"token_expired",
	])("recognizes auth.%s as invalid credentials", (code) => {
		expect(isAuthError(apiError(401, "auth", code))).toBe(true);
	});

	it("recognizes missing credentials", () => {
		expect(isAuthError(apiError(403, "api", "forbidden"))).toBe(true);
	});

	it.each([
		[403, "auth", "insufficient_permissions"],
		[503, "auth", "verification_unavailable"],
		[401, "api", "unauthorized"],
	])("does not prompt for %s %s.%s", (statusCode, group, code) => {
		expect(isAuthError(apiError(statusCode, group, code))).toBe(false);
	});
});
