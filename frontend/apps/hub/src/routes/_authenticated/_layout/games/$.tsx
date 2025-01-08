import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_authenticated/_layout/games/$")({
	beforeLoad: ({ params, location }) => {
		throw redirect({
			to: location.pathname.replace("/games/", "/projects/"),
		});
	},
});
