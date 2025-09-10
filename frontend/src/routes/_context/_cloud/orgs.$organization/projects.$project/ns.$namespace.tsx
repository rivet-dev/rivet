import { createFileRoute } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { createNamespaceContext } from "@/app/data-providers/cloud-data-provider";
import { RouteLayout } from "@/app/route-layout";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
)({
	component: RouteComponent,
	context: ({ context, params }) => {
		return match(context)
			.with({ __type: "cloud" }, (ctx) => ({
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
});

function RouteComponent() {
	return <RouteLayout />;
}
