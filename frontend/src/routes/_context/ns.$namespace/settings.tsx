import { createFileRoute } from "@tanstack/react-router";
import { RouteComponent } from "../_cloud/orgs.$organization/projects.$project/ns.$namespace/settings";

export const Route = createFileRoute("/_context/ns/$namespace/settings")({
	component: RouteComponent,
});
