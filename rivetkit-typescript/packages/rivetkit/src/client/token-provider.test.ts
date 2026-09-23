import { describe, expect, test, vi } from "vitest";
import {
	isInvalidToken,
	isInvalidTokenResponse,
	TokenProvider,
} from "./token-provider";

describe("renewable client tokens", () => {
	test("single-flights initial issuance and concurrent rejection refreshes", async () => {
		const getToken = vi.fn(
			async ({ forceRefresh }: { forceRefresh: boolean }) =>
				forceRefresh ? "renewed" : "initial",
		);
		const provider = new TokenProvider(getToken);
		expect(
			await Promise.all([provider.current(), provider.current()]),
		).toEqual(["initial", "initial"]);
		expect(getToken).toHaveBeenCalledTimes(1);
		expect(
			await Promise.all([
				provider.refreshIfCurrent("initial"),
				provider.refreshIfCurrent("initial"),
			]),
		).toEqual(["renewed", "renewed"]);
		expect(getToken).toHaveBeenCalledTimes(2);
		expect(await provider.refreshIfCurrent("initial")).toBe("renewed");
		expect(getToken).toHaveBeenCalledTimes(2);
	});

	test("retries failed issuance and refuses empty credentials", async () => {
		const getToken = vi
			.fn()
			.mockRejectedValueOnce(new Error("issuer down"))
			.mockResolvedValueOnce("ok");
		const provider = new TokenProvider(getToken);
		await expect(provider.current()).rejects.toThrow("issuer down");
		expect(await provider.current()).toBe("ok");
		await expect(
			new TokenProvider(async () => "").current(),
		).rejects.toThrow("nonempty");
	});

	test("refreshes before a known JWT expiry and treats opaque tokens as unexpiring hints", async () => {
		const payload = btoa(
			JSON.stringify({ exp: Math.floor(Date.now() / 1_000) + 1 }),
		).replaceAll("=", "");
		const getToken = vi.fn(async () => `header.${payload}.signature`);
		const provider = new TokenProvider(getToken);
		await provider.current();
		await provider.current();
		expect(getToken).toHaveBeenCalledTimes(2);
		const opaque = vi.fn(async () => "opaque");
		const opaqueProvider = new TokenProvider(opaque);
		await opaqueProvider.current();
		await opaqueProvider.current();
		expect(opaque).toHaveBeenCalledTimes(1);
	});

	test("only confirmed 401 invalid/expired errors permit renewal", () => {
		expect(isInvalidToken("auth", "invalid_token")).toBe(true);
		expect(isInvalidToken("auth", "token_expired")).toBe(true);
		for (const code of [
			"insufficient_permissions",
			"verification_unavailable",
			"issuance_unavailable",
		]) {
			expect(isInvalidToken("auth", code)).toBe(false);
		}
		expect(
			isInvalidTokenResponse(
				new Response(null, {
					status: 401,
					headers: { "x-rivet-error": "auth.token_expired" },
				}),
			),
		).toBe(true);
		expect(
			isInvalidTokenResponse(
				new Response(null, {
					status: 503,
					headers: { "x-rivet-error": "auth.token_expired" },
				}),
			),
		).toBe(false);
	});
});
