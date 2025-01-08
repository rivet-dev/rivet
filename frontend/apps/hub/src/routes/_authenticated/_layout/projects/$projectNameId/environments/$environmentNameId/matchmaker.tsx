import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/matchmaker",
)({
	loader: ({ params }) => {
		throw redirect({
			to: "/projects/$projectNameId/environments/$environmentNameId/lobbies",
			params,
		});
	},
});
