import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_authenticated/devices/link_/$token")({
	beforeLoad: async ({ params: { token } }) => {
		throw redirect({
			to: "/devices/link",
			search: { token },
		});
	},
});
