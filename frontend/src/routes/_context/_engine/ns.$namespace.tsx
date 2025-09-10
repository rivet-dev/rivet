import { createFileRoute } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { createNamespaceContext } from "@/app/data-providers/engine-data-provider";
import { RouteLayout } from "@/app/route-layout";

export const Route = createFileRoute("/_context/_engine/ns/$namespace")({
	context: ({ context, params }) => {
		return match(context)
			.with({ __type: "engine" }, (ctx) => ({
				dataProvider: {
					...ctx.dataProvider,
					...createNamespaceContext({
						...ctx.dataProvider,
						namespace: params.namespace,
					}),
				},
			}))
			.otherwise(() => {
				throw new Error("Invalid context type for this route");
			});
	},
	component: RouteComponent,
});

function RouteComponent() {
	return <RouteLayout />;
}
