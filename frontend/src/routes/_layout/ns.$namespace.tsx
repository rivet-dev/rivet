import { createFileRoute, notFound, Outlet } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { match } from "ts-pattern";
import z from "zod";

export const Route = createFileRoute("/_layout/ns/$namespace")({
	component: match(__APP_TYPE__)
		.with("engine", () => RouteComponent)
		.otherwise(() => () => {
			throw notFound();
		}),
	validateSearch: zodValidator(
		z.object({
			n: z.array(z.string()).optional(),
		}),
	),
});

function RouteComponent() {
	return <Outlet />;
}
