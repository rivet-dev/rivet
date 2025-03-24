import { ErrorComponent } from "@/components/error-component";
import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/matchmaker-layout";
import { guardUuids } from "@/lib/guards";
import {
	type ErrorComponentProps,
	Outlet,
	createFileRoute,
} from "@tanstack/react-router";

function MatchmakerLayoutErrorComponent(props: ErrorComponentProps) {
	const { nameId: environmentNameId } = useEnvironment();
	const { nameId: projectNameId } = useProject();

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
	const { nameId: environmentNameId } = useEnvironment();
	const { nameId: projectNameId } = useProject();

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
		context: { queryClient },
		params: { projectNameId, environmentNameId },
	}) => {
		await guardUuids({
			location,
			queryClient,
			projectNameId,
			environmentNameId,
		});
	},
});
