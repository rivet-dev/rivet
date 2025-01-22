import { ErrorComponent } from "@/components/error-component";
import * as Layout from "@/domains/project/layouts/project-layout";
import { projectQueryOptions } from "@/domains/project/queries";
import { useDialog } from "@/hooks/use-dialog";
import { guardUuids } from "@/lib/guards";
import {
	type ErrorComponentProps,
	Outlet,
	createFileRoute,
	notFound,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";

function Modals() {
	const navigate = Route.useNavigate();
	const {
		project: { gameId: projectId, nameId: projectNameId },
		environment: { namespaceId: environmentId, nameId: environmentNameId },
	} = Route.useRouteContext();
	const { modal, buildId } = Route.useSearch();

	const ConfirmOuterbaseConnectionDialog =
		useDialog.ConfirmOuterbaseConnection.Dialog;

	const EditBuildTagsDialog = useDialog.EditBuildTags.Dialog;
	const CreateActorDialog = useDialog.CreateActor.Dialog;

	const handleOpenChange = (value: boolean) => {
		if (!value) {
			navigate({ search: (old) => ({ ...old, modal: undefined }) });
		}
	};

	return (
		<>
			<ConfirmOuterbaseConnectionDialog
				environmentId={environmentId}
				projectId={projectId}
				dialogProps={{
					open: modal === "database",
					onOpenChange: handleOpenChange,
				}}
			/>
			<EditBuildTagsDialog
				// biome-ignore lint/style/noNonNullAssertion: at this point we know buildId is defined
				buildId={buildId!}
				environmentNameId={environmentNameId}
				projectNameId={projectNameId}
				dialogProps={{
					open: modal === "edit-tags",
					onOpenChange: handleOpenChange,
				}}
			/>
			<CreateActorDialog
				environmentNameId={environmentNameId}
				projectNameId={projectNameId}
				dialogProps={{
					open: modal === "create-actor",
					onOpenChange: handleOpenChange,
				}}
			/>
		</>
	);
}

function EnvironmentErrorComponent(props: ErrorComponentProps) {
	return <ErrorComponent {...props} />;
}

function environmentIdRoute() {
	return (
		<>
			<Outlet />
			<Modals />
		</>
	);
}
const searchSchema = z.object({
	modal: z
		.enum(["database", "edit-tags", "create-actor"])
		.or(z.string())
		.optional()
		.catch(undefined),
	buildId: z.string().optional().catch(undefined),
});
export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId",
)({
	validateSearch: zodValidator(searchSchema),
	beforeLoad: async ({
		matches,
		location,
		params: { projectNameId, environmentNameId },
		context: {
			queryClient,
			project: { gameId: projectId },
		},
	}) => {
		await guardUuids({
			queryClient,
			location,
			projectNameId,
			environmentNameId,
		});

		const { game: project } = await queryClient.ensureQueryData(
			projectQueryOptions(projectId),
		);
		const environment = project.namespaces.find(
			(ns) => ns.nameId === environmentNameId,
		);

		if (!environment || !project) {
			throw notFound();
		}

		return { environment };
	},
	loader: async ({ context: { project, environment } }) => {
		if (!environment || !project) {
			throw notFound();
		}

		return { environment };
	},
	component: environmentIdRoute,
	errorComponent: EnvironmentErrorComponent,
	pendingComponent: Layout.Root.Skeleton,
});
