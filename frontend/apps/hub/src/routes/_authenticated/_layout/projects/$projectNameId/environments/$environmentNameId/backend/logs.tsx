import { ErrorComponent } from "@/components/error-component";
import { BackendListEventsPreview } from "@/domains/project/components/backend/backend-list-events-preview";
import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/backend-layout";
import { projectBackendEnvEventsQueryOptions } from "@/domains/project/queries";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	LiveBadge,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import {
	type ErrorComponentProps,
	createFileRoute,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";

function ProjectBackendEnvironmentIdLogsRoute() {
	const { namespaceId: environmentId } = useEnvironment();
	const { gameId: projectId } = useProject();
	const { eventId } = Route.useSearch();

	const { data } = useSuspenseQuery(
		projectBackendEnvEventsQueryOptions({
			projectId,
			environmentId: environmentId,
		}),
	);

	return (
		<Card className="h-full max-h-full flex flex-col p-0">
			<CardHeader className="border-b">
				<CardTitle className="flex gap-4">
					Logs
					<LiveBadge />
				</CardTitle>
			</CardHeader>
			<CardContent className="flex-1 min-h-0 w-full p-0">
				<BackendListEventsPreview
					events={data}
					eventId={eventId}
					projectId={projectId}
					environmentId={environmentId}
				/>
			</CardContent>
		</Card>
	);
}

const searchSchema = z.object({
	eventId: z.string().optional(),
});

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/backend/logs",
)({
	validateSearch: zodValidator(searchSchema),
	staticData: {
		layout: "full",
	},
	component: ProjectBackendEnvironmentIdLogsRoute,
	errorComponent: (props: ErrorComponentProps) => {
		return <ErrorComponent {...props} />;
	},
	pendingComponent: Layout.Content.Skeleton,
});
