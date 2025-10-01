import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_context/_inspector/inspect")({
	component: RouteComponent,
});

function RouteComponent() {
	return <div>Hello "/_context/_inspector/inspect"!</div>;
}
