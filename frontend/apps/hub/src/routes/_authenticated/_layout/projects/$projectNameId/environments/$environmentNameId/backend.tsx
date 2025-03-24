import { ErrorComponent } from "@/components/error-component";
import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/backend-layout";
import { guardEnterprise } from "@/lib/guards";
import {
	type ErrorComponentProps,
	Outlet,
	createFileRoute,
} from "@tanstack/react-router";

function BackendLayoutErrorComponent(props: ErrorComponentProps) {
	const { namespaceId: environmentId, nameId: environmentNameId } =
		useEnvironment();
	const { gameId: projectId, nameId: projectNameId } = useProject();

	return (
		<Layout.Root
			environmentId={environmentId}
			environmentNameId={environmentNameId}
			projectId={projectId}
			projectNameId={projectNameId}
		>
			<ErrorComponent {...props} />
		</Layout.Root>
	);
}

function BackendLayoutView() {
	const { namespaceId: environmentId, nameId: environmentNameId } =
		useEnvironment();
	const { gameId: projectId, nameId: projectNameId } = useProject();

	return (
		<Layout.Root
			environmentId={environmentId}
			environmentNameId={environmentNameId}
			projectId={projectId}
			projectNameId={projectNameId}
		>
			<Outlet />
		</Layout.Root>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/backend",
)({
	component: BackendLayoutView,
	errorComponent: BackendLayoutErrorComponent,
	pendingComponent: Layout.Root.Skeleton,
	beforeLoad: async ({ context: { queryClient } }) => {
		await guardEnterprise({ queryClient });
	},
});
