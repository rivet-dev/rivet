import { test, TestContext } from "../module.gen.ts";
import { assertEquals, assertExists } from "https://deno.land/std@0.224.0/assert/mod.ts";

test("ping-pong", async (ctx: TestContext) => {
	const { pong } = await ctx.modules.foo.ping({});
	assertEquals("pong", pong);
});

test("call-self", async (ctx: TestContext) => {
	const { response } = await ctx.modules.foo.callSelf({});
	assertEquals("pong", response.pong);
});

test("create-entry", async (ctx: TestContext) => {
	const { id } = await ctx.modules.foo.createEntry({});
	assertExists(id);
});
