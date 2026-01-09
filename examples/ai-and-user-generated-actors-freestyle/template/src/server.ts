import { Hono } from "hono";
import { serveStatic } from "hono/deno";
import { registry } from "./registry";

globalThis.addEventListener("unhandledrejection", (event) => {
	console.error("Unhandled promise rejection:", event.reason);
	event.preventDefault();
});

const app = new Hono();

app.use("/api/rivet/*", async (c) => {
	return await registry.fetch(c.req.raw);
});

app.use("*", serveStatic({ root: "./public" }));

// @ts-expect-error
Deno.serve(app.fetch);
