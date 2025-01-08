import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/servers/$",
)({
	beforeLoad: ({ params, location }) => {
		if (location.href.endsWith("/builds")) {
			throw redirect({
				to: "/projects/$projectNameId/environments/$environmentNameId/builds",
				params,
			});
		}
	},
});
