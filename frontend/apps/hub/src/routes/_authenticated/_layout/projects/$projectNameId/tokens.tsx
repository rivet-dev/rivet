import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/tokens",
)({
	loader: ({ params }) => {
		throw redirect({
			to: "/projects/$projectNameId/settings",
			params,
		});
	},
});
