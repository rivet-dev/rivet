import { ErrorComponent } from "@/components/error-component";
import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/project-layout";
import { useDialog } from "@/hooks/use-dialog";
import { ActionCard, Button, H1, Text } from "@rivet-gg/components";
import {
	type ErrorComponentProps,
	Link,
	createFileRoute,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";

function EnvironmentSettingsRoute() {
	return (
		<>
			<div className="max-w-5xl mx-auto my-8 flex justify-between items-center">
				<H1>Settings</H1>
			</div>

			<hr />
			<div className="p-4">
				<div className="max-w-5xl mx-auto flex flex-col gap-8">
					<ServiceTokenCard />
					<Modals />
				</div>
			</div>
		</>
	);
}

function ServiceTokenCard() {
	return (
		<>
			<ActionCard
				className="bg-transparent"
				title="Service token"
				footer={
					<Button asChild variant="secondary">
						<Link to="." search={{ modal: "service-token" }}>
							Generate
						</Link>
					</Button>
				}
			>
				<Text>
					Service tokens are used from private API servers. These
					should never be shared.
				</Text>
			</ActionCard>
		</>
	);
}

function Modals() {
	const navigate = Route.useNavigate();
	const { gameId: projectId, nameId: projectNameId } = useProject();
	const { namespaceId: environmentId, nameId: environmentNameId } =
		useEnvironment();

	const { modal } = Route.useSearch();

	const GenerateProjectEnvServiceTokenDialog =
		useDialog.GenerateProjectEnvServiceToken.Dialog;

	const handleOnOpenChange = (value: boolean) => {
		if (!value) {
			navigate({ search: { modal: undefined } });
		}
	};

	return (
		<>
			<GenerateProjectEnvServiceTokenDialog
				projectId={projectId}
				environmentId={environmentId}
				dialogProps={{
					open: modal === "service-token",
					onOpenChange: handleOnOpenChange,
				}}
			/>
		</>
	);
}

const searchSchema = z.object({
	modal: z.enum(["service-token"]).or(z.string()).optional(),
});

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/_v2/settings",
)({
	staticData: {
		layout: "v2",
	},
	validateSearch: zodValidator(searchSchema),
	component: EnvironmentSettingsRoute,
	pendingComponent: () => (
		<div className="flex flex-col gap-4 p-4">
			<Layout.Root.Skeleton />
		</div>
	),
	errorComponent(props: ErrorComponentProps) {
		return (
			<div className="p-4">
				<div className="max-w-5xl mx-auto">
					<ErrorComponent {...props} />
				</div>
			</div>
		);
	},
});
