import * as Layout from "@/domains/user/layouts/profile-layout";
import { Outlet, createFileRoute } from "@tanstack/react-router";

function MyProfileRoute() {
	return (
		<Layout.Root>
			<Outlet />
		</Layout.Root>
	);
}

export const Route = createFileRoute("/_authenticated/_layout/my-profile")({
	component: MyProfileRoute,
});
