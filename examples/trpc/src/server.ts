import { initTRPC } from "@trpc/server";
import { createHTTPServer } from "@trpc/server/adapters/standalone";
import { createClient } from "rivetkit/client";
import { z } from "zod";
import { registry } from "./registry.js";

// Start RivetKit
registry.startRunner();
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

// Create HTTP server
const server = createHTTPServer({
	router: appRouter,
});

server.listen(3001);

console.log("tRPC server listening at http://localhost:3001");
