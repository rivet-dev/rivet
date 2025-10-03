import { createFileRoute } from "@tanstack/react-router";
import { RouteComponent } from "../../_cloud/orgs.$organization/projects.$project/ns.$namespace/connect";

export const Route = createFileRoute("/_context/_engine/ns/$namespace/connect")(
	{
		component: RouteComponent,
	},
);
