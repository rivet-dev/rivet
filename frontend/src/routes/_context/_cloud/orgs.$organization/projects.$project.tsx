import { createFileRoute, Outlet } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { createProjectContext } from "@/app/data-providers/cloud-data-provider";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project",
)({
	component: RouteComponent,
	context: ({ context, params }) => {
		return match(context)
			.with({ __type: "cloud" }, (context) => ({
				dataProvider: {
					...context.dataProvider,
					...createProjectContext({
						...context.dataProvider,
						organization: params.organization,
						project: params.project,
					}),
				},
			}))
			.otherwise(() => {
				throw new Error("Invalid context type for this route");
			});
	},
});

function RouteComponent() {
	return <Outlet />;
}
