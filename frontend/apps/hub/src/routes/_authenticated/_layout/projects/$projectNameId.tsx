import { ErrorComponent } from "@/components/error-component";
import { BillingProvider } from "@/domains/project/components/billing/billing-context";
import { BillingOverageWarning } from "@/domains/project/components/billing/billing-overage-warning";
import {
	ProjectContextProvider,
	useProject,
} from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/project-layout";
import { useDialog } from "@/hooks/use-dialog";
import { guardUuids } from "@/lib/guards";

import {
	type ErrorComponentProps,
	Outlet,
	createFileRoute,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";

function ProjectIdErrorComponent(props: ErrorComponentProps) {
	return (
		<Layout.EmptyRoot>
			<ErrorComponent {...props} />
		</Layout.EmptyRoot>
	);
}

function Modals() {
	const navigate = Route.useNavigate();
	const project = useProject();
	const { modal } = Route.useSearch();

	const GenerateProjectCloudTokenDialog =
		useDialog.GenerateProjectCloudToken.Dialog;
	const CreateEnvironmentDialog = useDialog.CreateEnvironment.Dialog;

	const handleOnOpenChange = (value: boolean) => {
		if (!value) {
			navigate({ search: { modal: undefined } });
		}
	};

	return (
		<>
			<GenerateProjectCloudTokenDialog
				projectId={project.gameId}
				dialogProps={{
					open: modal === "cloud-token",
					onOpenChange: handleOnOpenChange,
				}}
			/>
			<CreateEnvironmentDialog
				projectId={project.gameId}
				dialogProps={{
					open: modal === "create-environment",
					onOpenChange: handleOnOpenChange,
				}}
			/>
		</>
	);
}

function ProjectIdRoute() {
	const { projectNameId } = Route.useParams();
	return (
		<Layout.Root>
			<ProjectContextProvider projectNameId={projectNameId}>
				<BillingProvider>
					<BillingOverageWarning />
					<Outlet />
				</BillingProvider>
				<Modals />
			</ProjectContextProvider>
		</Layout.Root>
	);
}

const searchSchema = z.object({
	modal: z
		.enum(["cloud-token", "service-token", "create-environment"])
		.or(z.string())
		.optional(),
});

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId",
)({
	validateSearch: zodValidator(searchSchema),
	component: ProjectIdRoute,
	errorComponent: ProjectIdErrorComponent,
	pendingComponent: Layout.Root.Skeleton,
	beforeLoad: async ({
		matches,
		location,
		context: { queryClient, auth },
		params: { projectNameId },
	}) => {
		await guardUuids({
			location,
			queryClient,
			projectNameId,
			environmentNameId: undefined,
		});
	},
});
