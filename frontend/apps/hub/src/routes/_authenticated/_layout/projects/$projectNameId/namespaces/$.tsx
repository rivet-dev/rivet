import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/namespaces/$",
)({
	beforeLoad: ({ params, location }) => {
		throw redirect({
			to: location.pathname.replace("/namespaces/", "/environments/"),
			params,
		});
	},
});
