import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_authenticated/devices/link/$token")({
	beforeLoad: async ({ params: { token } }) => {
		throw redirect({
			to: "/devices/link",
			search: { token },
		});
	},
});
