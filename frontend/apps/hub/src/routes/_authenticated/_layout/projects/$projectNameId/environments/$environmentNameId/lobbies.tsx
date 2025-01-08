import { ErrorComponent } from "@/components/error-component";
import * as Layout from "@/domains/project/layouts/matchmaker-layout";
import {
	type ErrorComponentProps,
	Outlet,
	createFileRoute,
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
});
