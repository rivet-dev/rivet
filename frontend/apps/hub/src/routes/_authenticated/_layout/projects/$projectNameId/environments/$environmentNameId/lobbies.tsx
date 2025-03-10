import { ErrorComponent } from "@/components/error-component";
import * as Layout from "@/domains/project/layouts/matchmaker-layout";
import { projectsByGroupQueryOptions } from "@/domains/project/queries";
import { guardUuids } from "@/lib/guards";
import { ls } from "@/lib/ls";
import { safeAsync } from "@rivet-gg/components";
import {
	type ErrorComponentProps,
	Outlet,
	createFileRoute,
	notFound,
} from "@tanstack/react-router";

function MatchmakerLayoutErrorComponent(props: ErrorComponentProps) {
	const {
		environment: { nameId: environmentNameId },
		project: { nameId: projectNameId },
	} = Route.useRouteContext();

	return (
		<Layout.Root
			environmentNameId={environmentNameId}
			projectNameId={projectNameId}
		>
			<ErrorComponent {...props} />
		</Layout.Root>
	);
}

function MatchmakerLayoutView() {
	const {
		environment: { nameId: environmentNameId },
		project: { nameId: projectNameId },
	} = Route.useRouteContext();

	return (
		<Layout.Root
			environmentNameId={environmentNameId}
			projectNameId={projectNameId}
		>
			<Outlet />
		</Layout.Root>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/lobbies",
)({
	component: MatchmakerLayoutView,
	errorComponent: MatchmakerLayoutErrorComponent,
	pendingComponent: Layout.Root.Skeleton,
	beforeLoad: async ({
		location,
		context: { queryClient, auth, environment },
		params: { projectNameId, environmentNameId },
	}) => {
		await guardUuids({
			location,
			queryClient,
			projectNameId,
			environmentNameId,
		});

		const [response] = await safeAsync(
			queryClient.fetchQuery(projectsByGroupQueryOptions()),
		);
		const project = response?.games.find((p) => p.nameId === projectNameId);

		if (!project) {
			throw notFound();
		}

		ls.recentTeam.set(auth, project.developer.groupId);

		return { project, environment };
	},
});
