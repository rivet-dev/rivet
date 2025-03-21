import { ErrorComponent } from "@/components/error-component";
import { BillingProvider } from "@/domains/project/components/billing/billing-context";
import { BillingOverageWarning } from "@/domains/project/components/billing/billing-overage-warning";
import * as Layout from "@/domains/project/layouts/project-layout";
import { projectsByGroupQueryOptions } from "@/domains/project/queries";
import { useDialog } from "@/hooks/use-dialog";
import { guardUuids } from "@/lib/guards";
import { ls } from "@/lib/ls";
import { safeAsync } from "@rivet-gg/components";

import {
	type ErrorComponentProps,
	Outlet,
	createFileRoute,
	notFound,
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
	const { project } = Route.useRouteContext();
	const { modal } = Route.useSearch();

	const GenerateProjectCloudTokenDialog =
		useDialog.GenerateProjectCloudToken.Dialog;
	const CreateEnvironmentDialog = useDialog.CreateEnvironment.Dialog;

	const handleonOpenChange = (value: boolean) => {
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
					onOpenChange: handleonOpenChange,
				}}
			/>
			<CreateEnvironmentDialog
				projectId={project.gameId}
				dialogProps={{
					open: modal === "create-environment",
					onOpenChange: handleonOpenChange,
				}}
			/>
		</>
	);
}

function ProjectIdRoute() {
	const {
		project: { gameId, developer },
	} = Route.useRouteContext();

	return (
		<Layout.Root>
			<BillingProvider projectId={gameId} groupId={developer.groupId}>
				<BillingOverageWarning />
				<Outlet />
			</BillingProvider>
			<Modals />
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

		const [response] = await safeAsync(
			queryClient.fetchQuery(projectsByGroupQueryOptions()),
		);
		const project = response?.games.find((p) => p.nameId === projectNameId);

		if (!project) {
			throw notFound();
		}

		ls.recentTeam.set(auth, project.developer.groupId);

		return { project };
	},
});
