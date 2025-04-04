import { createFileRoute, Outlet } from "@tanstack/react-router";

export const Route = createFileRoute("/_layout")({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<div className="size-full bg-card">
			<Outlet />
		</div>
	);
}
