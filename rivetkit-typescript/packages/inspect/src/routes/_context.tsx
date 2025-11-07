import { createFileRoute, Outlet } from "@tanstack/react-router";
import { createGlobalContext } from "../app/data-providers/inspector-data-provider";

export const Route = createFileRoute("/_context")({
	component: RouteComponent,
	context: ({ location: { search }, context }) => {
		return {
			dataProvider: createGlobalContext({
				url: "http://127.0.0.1:6420",
				token: "lNtOy9TZxmt2yGukMJREcJcEp78WDzuj",
			}),
			__type: "inspector" as const,
		};
	},
});

function RouteComponent() {
	return <Outlet />;
}
