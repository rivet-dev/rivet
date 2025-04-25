import { ErrorComponent } from "@/components/error-component";
import { ProjectBuildsTableActions } from "@/domains/project/components/project-builds-table-actions";
import { TagsSelect } from "@/domains/project/components/tags-select";
import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/servers-layout";
import {
	projectBuildsQueryOptions,
	projectCurrentBuildsQueryOptions,
	usePatchActorBuildTagsMutation,
	useUpgradeAllActorsMutation,
} from "@/domains/project/queries";
import type { Rivet } from "@rivet-gg/api";
import {
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	DiscreteCopyButton,
	Flex,
	Skeleton,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
	Text,
	WithTooltip,
} from "@rivet-gg/components";
import { ActorTags } from "@rivet-gg/components/actors";
import { Icon, faCheckCircle, faInfoCircle, faRefresh } from "@rivet-gg/icons";
import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import {
	createFileRoute,
	type ErrorComponentProps,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";

function ProjectBuildsRoute() {
	const { gameId: projectId, nameId: projectNameId } = useProject();
	const { namespaceId: environmentId, nameId: environmentNameId } =
		useEnvironment();

	const search = Route.useSearch();
	const tags = "tags" in search ? Object.fromEntries(search.tags || []) : {};
	const {
		data: builds,
		isRefetching,
		isLoading,
		refetch,
	} = useQuery(projectBuildsQueryOptions({ projectId, environmentId, tags }));

	const navigate = Route.useNavigate();

	return (
		<div className="p-4">
			<Card className="max-w-5xl mx-auto">
				<CardHeader>
					<Flex items="center" gap="4" justify="between">
						<CardTitle>Versions</CardTitle>
						<div className="flex gap-2">
							<TagsSelect
								value={tags}
								projectId={projectId}
								environmentId={environmentId}
								onValueChange={(newTags) => {
									navigate({
										search: {
											tags: Object.entries(newTags).map(
												([key, value]) =>
													[key, value] as [
														string,
														string,
													],
											),
										},
									});
								}}
							/>
							<Button
								size="icon"
								isLoading={isRefetching}
								variant="outline"
								onClick={() => refetch()}
							>
								<Icon icon={faRefresh} />
							</Button>
						</div>
					</Flex>
				</CardHeader>
				<CardContent>
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>ID</TableHead>
								<TableHead>Name</TableHead>
								<TableHead>Tags</TableHead>
								<TableHead>
									<WithTooltip
										content="Actors will be created with this build if a version is not explicitly specified."
										trigger={
											<span>
												Current{" "}
												<Icon icon={faInfoCircle} />
											</span>
										}
									/>
								</TableHead>
								<TableHead>Created</TableHead>
								<TableHead />
							</TableRow>
						</TableHeader>
						<TableBody>
							{!isLoading && builds?.length === 0 ? (
								<TableRow>
									<TableCell colSpan={6}>
										<Text className="text-center">
											There's no versions matching
											criteria.
										</Text>
									</TableCell>
								</TableRow>
							) : null}
							{isLoading ? (
								<>
									<RowSkeleton />
									<RowSkeleton />
									<RowSkeleton />
									<RowSkeleton />
									<RowSkeleton />
									<RowSkeleton />
									<RowSkeleton />
									<RowSkeleton />
								</>
							) : null}
							{builds?.map((build) => (
								<TableRow key={build.id}>
									<TableCell>
										<WithTooltip
											content={build.id}
											trigger={
												<DiscreteCopyButton
													value={build.id}
												>
													{build.id.split("-")[0]}
												</DiscreteCopyButton>
											}
										/>
									</TableCell>
									<TableCell>
										<DiscreteCopyButton
											value={build.tags.name}
										>
											{build.tags.name}
										</DiscreteCopyButton>
									</TableCell>
									<TableCell>
										<ActorTags
											{...build}
											excludeBuiltIn="builds"
										/>
									</TableCell>
									<TableCell>
										<ProjectBuildLatestButton
											projectNameId={projectNameId}
											environmentNameId={
												environmentNameId
											}
											projectId={projectId}
											environmentId={environmentId}
											{...build}
										/>
									</TableCell>
									<TableCell>
										{build.createdAt.toLocaleString()}
									</TableCell>
									<TableCell>
										<ProjectBuildsTableActions
											buildId={build.id}
										/>
									</TableCell>
								</TableRow>
							))}
						</TableBody>
					</Table>
				</CardContent>
			</Card>
		</div>
	);
}

function RowSkeleton() {
	return (
		<TableRow>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
		</TableRow>
	);
}

interface ProjectBuildLatestButtonProps extends Rivet.builds.Build {
	projectNameId: string;
	environmentNameId: string;
	projectId: string;
	environmentId: string;
}

function ProjectBuildLatestButton({
	tags,
	id,
	projectId,
	environmentId,
	projectNameId,
	environmentNameId,
}: ProjectBuildLatestButtonProps) {
	const { mutateAsync: mutateBuildTagsAsync } =
		usePatchActorBuildTagsMutation();
	const { mutate: mutateUpgradeActors, isPending } =
		useUpgradeAllActorsMutation();
	const { data: latestBuilds } = useSuspenseQuery(
		projectCurrentBuildsQueryOptions({ projectId, environmentId }),
	);

	if (tags.current !== "true") {
		return (
			<Button
				variant="outline"
				size="sm"
				isLoading={isPending}
				onClick={async () => {
					await mutateBuildTagsAsync({
						projectNameId,
						environmentNameId,
						buildId: id,
						tags: { current: "true" },
					});
					if (latestBuilds.length > 0) {
						mutateUpgradeActors({
							projectNameId,
							environmentNameId,
							buildTags: { current: "true" },
							tags: { name: latestBuilds[0].name },
						});
					}
				}}
			>
				Make current
			</Button>
		);
	}

	return <Icon icon={faCheckCircle} className="fill-primary text-primary" />;
}

const searchSchema = z.object({
	tags: z.array(z.tuple([z.string(), z.string()])).optional(),
});

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/_v2/actor-versions",
)({
	validateSearch: zodValidator(searchSchema),
	component: ProjectBuildsRoute,
	staticData: {
		layout: "v2",
	},
	pendingComponent: () => (
		<div className="flex flex-col gap-4 p-4">
			<Layout.Content.Skeleton />
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
