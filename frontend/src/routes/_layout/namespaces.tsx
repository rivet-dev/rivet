import { createFileRoute, notFound } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { NamespacesPage } from "@/app/namespaces-page";

export const Route = createFileRoute("/_layout/namespaces")({
	component: match(__APP_TYPE__)
		.with("engine", () => RouteComponent)
		.otherwise(() => () => {
			throw notFound();
		}),
});

function RouteComponent() {
	return <NamespacesPage from="/ns/$namespace" />;
}
