import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/connect",
)({
	component: () => null,
	beforeLoad: ({ params }) => {
		throw redirect({
			to: "/orgs/$organization/projects/$project/ns/$namespace/settings",
			params,
			search: true,
		});
	},
});
