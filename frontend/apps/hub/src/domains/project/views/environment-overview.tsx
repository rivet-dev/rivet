import {
	ActionCard,
	Badge,
	Grid,
	Text,
	WithTooltip,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { BackendDeploymentLink } from "../components/backend/backend-deployment-link";
import { projectBackendQueryOptions } from "../queries";

interface OverviewCardProps {
	environmentId: string;
	projectId: string;
}

function OverviewCard({ environmentId, projectId }: OverviewCardProps) {
	const { data } = useSuspenseQuery(
		projectBackendQueryOptions({ projectId, environmentId }),
	);

	return (
		<ActionCard title={"Overview"}>
			<Text>Created {data.createdAt.toLocaleString()}</Text>
			<Text>
				URL: <BackendDeploymentLink url={data.endpoint} />
			</Text>
			<Text asChild>
				<div className="flex gap-2 items-center ">
					Tier{" "}
					<WithTooltip
						content="This can't be changed. Please contact support to upgrade."
						trigger={<Badge>{data.tier}</Badge>}
					/>
				</div>
			</Text>
		</ActionCard>
	);
}

interface ProjectBackendEnvironmentOverviewProps {
	environmentId: string;
	projectId: string;
}

export function ProjectBackendEnvironmentOverview({
	environmentId,
	projectId,
}: ProjectBackendEnvironmentOverviewProps) {
	return (
		<Grid columns={{ initial: "1", md: "2" }} gap="4" items="start">
			<OverviewCard environmentId={environmentId} projectId={projectId} />
		</Grid>
	);
}
