import { createFileRoute, Outlet } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import z from "zod";

export const Route = createFileRoute("/_layout/ns/$namespace")({
	component: RouteComponent,
	validateSearch: zodValidator(
		z.object({
			n: z.array(z.string()).optional(),
		}),
	),
});

function RouteComponent() {
	return <Outlet />;
}
