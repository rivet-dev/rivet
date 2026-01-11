// import { describe, expect, test } from "vitest";
// import { serializeKey } from "../src/util";
//
// // Access internal KEYS directly
// // Since KEYS is a private constant in manager_driver.ts, we'll redefine it here for testing
// const KEYS = {
// 	ACTOR: {
// 		metadata: (actorId: string) => `actor:${actorId}:metadata`,
// 		keyIndex: (name: string, key: string[] = []) => {
// 			// Use serializeKey for consistent handling of all keys
// 			return `actor_key:${serializeKey(key)}`;
// 		},
// 	},
// };
//
// describe("Key index functions", () => {
// 	test("keyIndex handles empty key array", () => {
// 		expect(KEYS.ACTOR.keyIndex("test-actor")).toBe("actor_key:(none)");
// 		expect(KEYS.ACTOR.keyIndex("actor:with:colons")).toBe(
// 			"actor_key:(none)",
// 		);
// 	});
//
// 	test("keyIndex handles single-item key arrays", () => {
// 		// Note: keyIndex ignores the name parameter
// 		expect(KEYS.ACTOR.keyIndex("test-actor", ["key1"])).toBe(
// 			"actor_key:key1",
// 		);
// 		expect(
// 			KEYS.ACTOR.keyIndex("actor:with:colons", ["key:with:colons"]),
// 		).toBe("actor_key:key:with:colons");
// 	});
//
// 	test("keyIndex handles multi-item array keys", () => {
// 		// Note: keyIndex ignores the name parameter
// 		expect(KEYS.ACTOR.keyIndex("test-actor", ["key1", "key2"])).toBe(
// 			`actor_key:key1,key2`,
// 		);
//
// 		// Test with special characters
// 		expect(KEYS.ACTOR.keyIndex("test-actor", ["key,with,commas"])).toBe(
// 			"actor_key:key\\,with\\,commas",
// 		);
// 	});
//
// 	test("metadata key creates proper pattern", () => {
// 		expect(KEYS.ACTOR.metadata("123-456")).toBe("actor:123-456:metadata");
// 	});
// });
