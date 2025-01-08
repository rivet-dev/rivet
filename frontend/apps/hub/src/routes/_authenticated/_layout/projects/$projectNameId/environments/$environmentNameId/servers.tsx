import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/servers",
)({
	beforeLoad: ({ params }) => {
		throw redirect({
			to: "/projects/$projectNameId/environments/$environmentNameId/actors",
			params,
		});
	},
});
