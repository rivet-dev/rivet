import { trpcServer } from "@hono/trpc-server";
import { initTRPC } from "@trpc/server";
import { Hono } from "hono";
import { createClient } from "rivetkit/client";
import { z } from "zod";
import { registry } from "./actors.ts";

const client = createClient<typeof registry>();

// Initialize tRPC
const t = initTRPC.create();

// Create tRPC router with RivetKit integration
const appRouter = t.router({
	// Increment a named counter
	increment: t.procedure
		.input(z.object({ name: z.string() }))
		.mutation(async ({ input }) => {
			const counter = client.counter.getOrCreate(input.name);
			const newCount = await counter.increment(1);
			return newCount;
		}),
});

// Export type for client
export type AppRouter = typeof appRouter;

const app = new Hono();

app.use("/trpc/*", trpcServer({ router: appRouter }));

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

export default app;
