import { createFileRoute, Outlet } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { RouteError } from "@/app/route-error";
import { FullscreenLoading } from "@/components";
import {
	RECENT_PROJECTS_KEY,
	recordRecentVisit,
} from "@/lib/recently-visited";

export const Route = createFileRoute(
	"/_context/orgs/$organization/projects/$project",
)({
	component: RouteComponent,
	beforeLoad: ({ params, context }) => {
		recordRecentVisit(RECENT_PROJECTS_KEY, params.project);
		return  match(context)
			.with({ __type: "cloud" }, (context) => ({
				dataProvider: context.getOrCreateProjectContext(
					context.dataProvider,
					params.organization,
					params.project,
				),
			}))
			.otherwise(() => {
				throw new Error("Invalid context type for this route");
			});
	},
	loader: ({ context }) => ({ dataProvider: context.dataProvider }),
	errorComponent: RouteError,
	pendingMinMs: 0,
	pendingMs: 0,
	pendingComponent: FullscreenLoading,
});

function RouteComponent() {
	return <Outlet />;
}
