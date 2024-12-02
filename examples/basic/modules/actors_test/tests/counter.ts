import { test, TestContext } from "../module.gen.ts";
import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { delay } from "jsr:@std/async";
import { TICK_INTERVAL } from "../actors/counter.ts";

test("counter", async (ctx: TestContext) => {
  const key = `${Math.floor(Math.random() * 100000)}`;

  for (let i = 0; i < 5; i++) {
    const res = await ctx.modules.actorsTest.fetchCounter({ key });
    assertEquals(res.count, i);

    await delay(TICK_INTERVAL);
  }
});

