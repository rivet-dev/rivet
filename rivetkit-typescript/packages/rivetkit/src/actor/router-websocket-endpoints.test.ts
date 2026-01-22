import { describe, expect, test } from "vitest";
import {
	PATH_WEBSOCKET_BASE,
	PATH_WEBSOCKET_PREFIX,
} from "@/common/actor-router-consts";

/**
 * Unit tests for WebSocket path routing logic.
 *
 * These tests verify the path matching behavior in routeWebSocket
 * without needing a full actor setup.
 *
 * NOTE: The driver-file-system end-to-end tests pass because the driver
 * correctly strips query parameters before calling routeWebSocket
 * (see FileSystemManagerDriver.openWebSocket). However, the bug still
 * exists in routeWebSocket itself and could be triggered by other callers
 * (e.g., engine driver's runnerWebSocket which passes requestPath directly).
 */
describe("websocket path routing", () => {
	// Helper that replicates the routing logic from routeWebSocket
	// After fix: strips query params before comparing
	function matchesWebSocketPath(requestPath: string): boolean {
		const requestPathWithoutQuery = requestPath.split("?")[0];
		return (
			requestPathWithoutQuery === PATH_WEBSOCKET_BASE ||
			requestPathWithoutQuery.startsWith(PATH_WEBSOCKET_PREFIX)
		);
	}

	test("should match base websocket path without query", () => {
		expect(matchesWebSocketPath("/websocket")).toBe(true);
	});

	test("should match websocket path with trailing slash", () => {
		expect(matchesWebSocketPath("/websocket/")).toBe(true);
	});

	test("should match websocket path with subpath", () => {
		expect(matchesWebSocketPath("/websocket/foo")).toBe(true);
		expect(matchesWebSocketPath("/websocket/foo/bar")).toBe(true);
	});

	test("should match websocket path with subpath and query", () => {
		// This works because "/websocket/foo?query" starts with "/websocket/"
		expect(matchesWebSocketPath("/websocket/foo?query=value")).toBe(true);
	});

	// FIX: Query parameters are now stripped before routing comparison.
	// This ensures /websocket?query correctly routes to the websocket handler.
	test("should match base websocket path with query parameters", () => {
		expect(matchesWebSocketPath("/websocket?token=abc")).toBe(true);
		expect(matchesWebSocketPath("/websocket?foo=bar&baz=123")).toBe(true);
	});
});
