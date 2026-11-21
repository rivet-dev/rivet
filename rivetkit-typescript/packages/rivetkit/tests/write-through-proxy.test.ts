import { types } from "node:util";
import onChange from "@rivetkit/on-change";
import { describe, expect, test, vi } from "vitest";
import {
	createWriteThroughProxy,
	unwrapWriteThroughProxy,
} from "../src/registry/write-through-proxy";

/**
 * Counts how many `@rivetkit/on-change` proxy layers wrap a value. A raw value
 * has depth 0.
 */
function proxyDepth(value: unknown): number {
	let depth = 0;
	let current = value;
	while (current !== null && typeof current === "object") {
		const target = onChange.target(current as Record<string, any>);
		if (target === current) {
			break;
		}
		depth++;
		current = target;
	}
	return depth;
}

describe("createWriteThroughProxy", () => {
	test("tracks mutations on basic types and calls commit", () => {
		const commit = vi.fn();
		const state = createWriteThroughProxy(
			{
				str: "hello",
				num: 42,
				arr: [1, 2],
				nested: { a: "old" },
			},
			commit,
		);

		state.str = "world";
		expect(commit).toHaveBeenCalledTimes(1);
		expect(state.str).toBe("world");

		commit.mockClear();
		state.nested.a = "new";
		expect(commit).toHaveBeenCalledTimes(1);
		expect(state.nested.a).toBe("new");

		commit.mockClear();
		state.arr.push(3);
		expect(commit).toHaveBeenCalled();
		expect(state.arr).toEqual([1, 2, 3]);
	});

	test("returns primitives as-is without proxying", () => {
		const commit = vi.fn();
		expect(createWriteThroughProxy(42, commit)).toBe(42);
		expect(createWriteThroughProxy("hello", commit)).toBe("hello");
		expect(createWriteThroughProxy(null, commit)).toBe(null);
		expect(createWriteThroughProxy(undefined, commit)).toBe(undefined);
	});

	test("beforeChange receives new value and rejecting prevents mutation", () => {
		const commit = vi.fn();
		const state = createWriteThroughProxy({ x: 1 }, commit, (newValue) => {
			if (typeof newValue === "string") {
				throw new TypeError("strings not allowed");
			}
		});

		state.x = 2;
		expect(commit).toHaveBeenCalledTimes(1);
		expect(state.x).toBe(2);

		commit.mockClear();
		expect(() => {
			(state as Record<string, unknown>).x = "bad";
		}).toThrow("strings not allowed");
		expect(commit).not.toHaveBeenCalled();
		expect(state.x).toBe(2);
	});
});

describe("unwrapWriteThroughProxy", () => {
	test("strips proxies nested in objects, arrays, maps, and sets", () => {
		const raw = {
			nested: { inner: { deep: true } },
			arr: [{ item: 1 }],
			map: new Map<string, { value: number }>([["a", { value: 1 }]]),
			set: new Set([{ value: 2 }]),
		};
		const proxied = createWriteThroughProxy(raw, () => {});

		const spread = {
			nested: proxied.nested,
			arr: proxied.arr,
			map: proxied.map,
			set: proxied.set,
		};
		expect(proxyDepth(spread.nested)).toBe(1);

		const unwrapped = unwrapWriteThroughProxy(spread);
		expect(proxyDepth(unwrapped.nested)).toBe(0);
		expect(proxyDepth(unwrapped.nested.inner)).toBe(0);
		expect(proxyDepth(unwrapped.arr)).toBe(0);
		expect(proxyDepth(unwrapped.arr[0])).toBe(0);
		expect(proxyDepth(unwrapped.map)).toBe(0);
		expect(proxyDepth(unwrapped.map.get("a"))).toBe(0);
		expect(proxyDepth([...unwrapped.set][0])).toBe(0);

		expect(unwrapped.nested).toBe(raw.nested);
		expect(unwrapped.map.get("a")).toEqual({ value: 1 });
		expect([...unwrapped.set]).toEqual([{ value: 2 }]);
	});

	test("returns primitives unchanged and tolerates circular references", () => {
		expect(unwrapWriteThroughProxy(null)).toBe(null);
		expect(unwrapWriteThroughProxy(undefined)).toBe(undefined);
		expect(unwrapWriteThroughProxy(100)).toBe(100);
		expect(unwrapWriteThroughProxy("foo")).toBe("foo");

		const cyclic: Record<string, unknown> = { name: "cycle" };
		cyclic.self = cyclic;
		expect(unwrapWriteThroughProxy(cyclic)).toBe(cyclic);
	});
});

describe("nested state assignments (#5961)", () => {
	test("unwraps proxies when spreading into a nested array", () => {
		const rawState = { items: [{ id: 0, text: "first" }] };
		const proxy = createWriteThroughProxy(rawState, () => {});

		// Rebuild nested array from state
		proxy.items = [...proxy.items, { id: 1, text: "second" }];

		// Underlyling raw state items must NOT be Proxies
		expect(types.isProxy(rawState.items[0])).toBe(false);
		expect(types.isProxy(rawState.items[1])).toBe(false);
		expect(proxyDepth(rawState.items[0])).toBe(0);
		expect(proxyDepth(rawState.items[1])).toBe(0);

		// structuredClone must succeed without DataCloneError (e.g. for database transactions)
		expect(() => structuredClone(rawState)).not.toThrow();
		const cloned = structuredClone(rawState);
		expect(cloned.items).toEqual([
			{ id: 0, text: "first" },
			{ id: 1, text: "second" },
		]);
	});

	test("unwraps proxies when filtering a nested array", () => {
		const rawState = {
			items: [
				{ id: 0, active: true },
				{ id: 1, active: false },
				{ id: 2, active: true },
			],
		};
		const proxy = createWriteThroughProxy(rawState, () => {});

		// Rebuild nested array by filtering
		proxy.items = proxy.items.filter((item) => item.active);

		expect(rawState.items).toHaveLength(2);
		expect(types.isProxy(rawState.items[0])).toBe(false);
		expect(types.isProxy(rawState.items[1])).toBe(false);
		expect(proxyDepth(rawState.items[0])).toBe(0);
		expect(proxyDepth(rawState.items[1])).toBe(0);
		expect(() => structuredClone(rawState)).not.toThrow();
	});

	test("unwraps proxies when assigning a nested object", () => {
		const rawState = {
			user: {
				profile: { name: "Alice", role: "admin" },
				settings: { theme: "dark" },
			},
		};
		const proxy = createWriteThroughProxy(rawState, () => {});

		// Spread update nested object
		proxy.user = {
			...proxy.user,
			profile: { ...proxy.user.profile, name: "Bob" },
		};

		expect(types.isProxy(rawState.user)).toBe(false);
		expect(types.isProxy(rawState.user.profile)).toBe(false);
		expect(types.isProxy(rawState.user.settings)).toBe(false);
		expect(proxyDepth(rawState.user.profile)).toBe(0);
		expect(rawState.user.profile.name).toBe("Bob");
		expect(() => structuredClone(rawState)).not.toThrow();
	});

	test("unwraps a proxy assigned directly to a property", () => {
		const foreignRaw = { config: { retries: 3 } };
		const foreignProxy = createWriteThroughProxy(foreignRaw, () => {});

		const rawState = { activeConfig: null as unknown };
		const proxy = createWriteThroughProxy(rawState, () => {});

		proxy.activeConfig = foreignProxy.config;

		expect(types.isProxy(rawState.activeConfig)).toBe(false);
		expect(proxyDepth(rawState.activeConfig)).toBe(0);
		expect(rawState.activeConfig).toEqual({ retries: 3 });
		expect(() => structuredClone(rawState)).not.toThrow();
	});

	test("prevents proxy layer compounding across consecutive actions", () => {
		let rawState: { items: Array<{ id: number }> } = {
			items: [{ id: 0 }],
		};

		const start = performance.now();
		// Simulate 100 consecutive actions rebuilding state on the same actor
		for (let i = 1; i <= 100; i++) {
			const proxy = createWriteThroughProxy(rawState, (next) => {
				rawState = next;
			});
			proxy.items = [...proxy.items, { id: i }];
		}
		const elapsed = performance.now() - start;

		expect(rawState.items).toHaveLength(101);
		// Proxy depth must remain 0 for all items in raw state
		expect(proxyDepth(rawState.items[0])).toBe(0);
		expect(proxyDepth(rawState.items[50])).toBe(0);
		expect(proxyDepth(rawState.items[100])).toBe(0);

		expect(types.isProxy(rawState.items[0])).toBe(false);
		expect(types.isProxy(rawState.items[100])).toBe(false);

		// Without the fix, 100 appends took several seconds; with unwrapping it takes < 50ms
		expect(elapsed).toBeLessThan(1000);
		expect(() => structuredClone(rawState)).not.toThrow();
	});

	test("handles maps and sets assigned to nested keys", () => {
		const rawState = {
			map: new Map<string, { count: number }>(),
			set: new Set<{ tag: string }>(),
		};
		const proxy = createWriteThroughProxy(rawState, () => {});

		const childProxy = createWriteThroughProxy({ count: 1 }, () => {});
		proxy.map.set("counter", childProxy);

		const tagProxy = createWriteThroughProxy({ tag: "v1" }, () => {});
		proxy.set.add(tagProxy);

		const mapVal = rawState.map.get("counter");
		expect(types.isProxy(mapVal)).toBe(false);
		expect(proxyDepth(mapVal)).toBe(0);

		const setVal = [...rawState.set][0];
		expect(types.isProxy(setVal)).toBe(false);
		expect(proxyDepth(setVal)).toBe(0);

		expect(() => structuredClone(rawState)).not.toThrow();
	});
});
