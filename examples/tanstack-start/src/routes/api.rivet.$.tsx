import { createFileRoute } from "@tanstack/react-router";
import { registry } from "@/actors";

export const Route = createFileRoute("/api/rivet/$")({
	server: {
		handlers: {
			ANY: (ctx) => registry.handler(ctx.request),
		},
	},
});
