import { describe, expect, test } from "vitest";
import { normalizeHandlerUrl } from "../src/serverless/configure";

describe("local serverless handler ownership", () => {
	test("normalizes equivalent hot-reload handler URLs", () => {
		expect(normalizeHandlerUrl("http://LOCALHOST:3000/api/rivet/")).toBe(
			normalizeHandlerUrl("http://localhost:3000/api/rivet"),
		);
	});

	test("preserves distinct handler ports", () => {
		expect(normalizeHandlerUrl("http://localhost:3000/api/rivet")).not.toBe(
			normalizeHandlerUrl("http://localhost:3001/api/rivet"),
		);
	});
});
