import { useSuspenseInfiniteQuery } from "@tanstack/react-query";
import { createFileRoute, Navigate } from "@tanstack/react-router";
import { Card, CardContent, CardHeader, CardTitle } from "@/components";
import { namespacesQueryOptions } from "@/queries/manager-engine";

import { RouteComponent as NamespaceRouteComponent } from "./ns.$namespace/index";

export const Route = createFileRoute("/_layout/")({
	component:
		__APP_TYPE__ === "engine" ? RouteComponent : NamespaceRouteComponent,
});

function RouteComponent() {
	const { data: namespaces } = useSuspenseInfiniteQuery(
		namespacesQueryOptions(),
	);

	if (namespaces.length <= 0) {
		return (
			<div className="flex size-full">
				<Card className="max-w-md w-full mb-6 mx-auto self-center">
					<CardHeader>
						<CardTitle>No namespaces found</CardTitle>
					</CardHeader>
					<CardContent>
						<p className="mb-4">
							Please consult the documentation for more
							information.
						</p>
					</CardContent>
				</Card>
			</div>
		);
	}

	return (
		<Navigate
			to="/ns/$namespace"
			params={{ namespace: namespaces[0].name }}
		/>
	);
}
