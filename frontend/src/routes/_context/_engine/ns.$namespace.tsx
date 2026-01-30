import { createFileRoute } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { NotFoundCard } from "@/app/not-found-card";
import { RouteLayout } from "@/app/route-layout";

export const Route = createFileRoute("/_context/_engine/ns/$namespace")({
	context: ({ context, params }) => {
		return match(context)
			.with({ __type: "engine" }, (ctx) => {
				return {
					dataProvider: context.getOrCreateEngineNamespaceContext(
						ctx.dataProvider,
						params.namespace,
					),
				};
			})
			.otherwise(() => {
				throw new Error("Invalid context type for this route");
			});
	},
	component: RouteComponent,
	notFoundComponent: () => <NotFoundCard />,
});

function RouteComponent() {
	return <RouteLayout />;
}
