import { BackendEnvironmentDatabaseLink } from "@/domains/project/components/backend/backend-environment-database-link";
import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/project-layout";
import {
	actorBuildsQueryOptions,
	projectBackendQueryOptions,
	projectEnvironmentQueryOptions,
	projectMetadataQueryOptions,
	projectVersionQueryOptions,
} from "@/domains/project/queries";
import { GuardEnterprise } from "@/lib/guards";
import {
	Button,
	Grid,
	Link as RivetLink,
	ValueCard,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Link, createFileRoute, redirect } from "@tanstack/react-router";

function environmentIdRoute() {
	const { namespaceId: environmentId } = useEnvironment();
	const { gameId: projectId } = useProject();

	const {
		data: { legacyLobbiesEnabled, backendModulesEnabled },
	} = useSuspenseQuery(
		projectMetadataQueryOptions({ projectId, environmentId }),
	);

	return (
		<>
			<Grid columns={{ initial: "1", md: "2", lg: "3" }} gap="4">
				<CurrentBuildCard />
				{backendModulesEnabled ? (
					<GuardEnterprise>
						<BackendEndpointCard />
					</GuardEnterprise>
				) : null}
				{legacyLobbiesEnabled ? <CurrentVersionCard /> : null}
			</Grid>
		</>
	);
}

function BackendEndpointCard() {
	const { namespaceId: environmentId, nameId: environmentNameId } =
		useEnvironment();
	const { gameId: projectId, nameId: projectNameId } = useProject();

	const { data } = useSuspenseQuery(
		projectBackendQueryOptions({ projectId, environmentId }),
	);

	return (
		<ValueCard
			title="Backend"
			value={
				<RivetLink
					href={data.endpoint}
					target="_blank"
					rel="norefferer"
					className="text-base truncate inline-block w-full"
				>
					{data.endpoint}
				</RivetLink>
			}
			footer={
				<>
					<Button asChild variant="outline" className="mr-2">
						<Link
							to="/projects/$projectNameId/environments/$environmentNameId/backend"
							params={{
								projectNameId,
								environmentNameId,
							}}
						>
							Backend Logs
						</Link>
					</Button>
					<BackendEnvironmentDatabaseLink
						variant="outline"
						projectId={projectId}
						environmentId={environmentId}
						startIcon={undefined}
					>
						Database
					</BackendEnvironmentDatabaseLink>
				</>
			}
		/>
	);
}

function CurrentBuildCard() {
	const { nameId: environmentNameId } = useEnvironment();
	const { nameId: projectNameId } = useProject();

	const {
		data: [build],
	} = useSuspenseQuery(
		actorBuildsQueryOptions({
			projectNameId,
			environmentNameId,
			tags: { current: "true" },
		}),
	);

	return (
		<ValueCard
			title="Current Build"
			value={build ? build.name : "n/a"}
			footer={
				<>
					<Button asChild variant="outline" className="mr-2">
						<Link
							to="/projects/$projectNameId/environments/$environmentNameId/servers"
							params={{
								projectNameId,
								environmentNameId,
							}}
						>
							Server Logs
						</Link>
					</Button>
					<Button asChild variant="outline">
						<Link
							to="/projects/$projectNameId/environments/$environmentNameId/builds"
							params={{
								projectNameId,
								environmentNameId,
							}}
						>
							Manage Builds
						</Link>
					</Button>
				</>
			}
		/>
	);
}

function CurrentVersionCard() {
	const { namespaceId: environmentId, nameId: environmentNameId } =
		useEnvironment();
	const { gameId: projectId, nameId: projectNameId } = useProject();

	const {
		data: { namespace: environment },
	} = useSuspenseQuery(
		projectEnvironmentQueryOptions({ projectId, environmentId }),
	);
	const { data: version } = useSuspenseQuery(
		projectVersionQueryOptions({
			projectId,
			versionId: environment.versionId,
		}),
	);

	return (
		<ValueCard
			title="Current Version"
			value={version.displayName}
			footer={
				<Button asChild variant="outline">
					<Link
						to="/projects/$projectNameId/environments/$environmentNameId/versions"
						params={{
							projectNameId,
							environmentNameId,
						}}
					>
						Manage Version
					</Link>
				</Button>
			}
		/>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/",
)({
	beforeLoad: ({ params }) => {
		throw redirect({
			to: "/projects/$projectNameId/environments/$environmentNameId/servers",
			params,
		});
	},
	component: environmentIdRoute,
	pendingComponent: Layout.Root.Skeleton,
});
