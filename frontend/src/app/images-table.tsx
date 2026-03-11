import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { formatDistance } from "date-fns";
import {
	Button,
	DiscreteCopyButton,
	Skeleton,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
	Text,
	WithTooltip,
} from "@/components";
import { useCloudNamespaceDataProvider } from "@/components/actors";

interface ImagesTableProps {
	isLoading?: boolean;
	isError?: boolean;
	hasNextPage?: boolean;
	fetchNextPage?: () => void;
	images: Image[];
	deployments: Deployment[];
	namespace: string;
}

interface Image {
	repository: string;
	tag: string;
	createdAt: string;
}

interface Deployment {
	repository?: string;
	namespace: string;
	tag?: string;
}

export function ImagesTable({
	isLoading,
	isError,
	hasNextPage,
	fetchNextPage,
	images,
	deployments,
	namespace,
}: ImagesTableProps) {
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead className="pl-8">Tag</TableHead>
					<TableHead>Deployed To</TableHead>
					<TableHead>Date</TableHead>
					<TableHead />
				</TableRow>
			</TableHeader>
			<TableBody>
				{!isLoading && !isError && images?.length === 0 ? (
					<TableRow>
						<TableCell colSpan={5}>
							<Text className="text-center">
								No images found.
							</Text>
						</TableCell>
					</TableRow>
				) : null}
				{isError ? (
					<TableRow>
						<TableCell colSpan={7}>
							<Text className="text-center">
								An error occurred while fetching images.
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
				{images?.map((image) => (
					<ImageRow
						{...image}
						deployments={deployments.filter(
							(deployment) =>
								deployment.repository === image.repository &&
								deployment.tag === image.tag,
						)}
						namespace={namespace}
						key={`${image.repository}:${image.tag}`}
					/>
				))}

				{!isLoading && hasNextPage ? (
					<TableRow>
						<TableCell colSpan={7}>
							<Button
								variant="outline"
								isLoading={isLoading}
								onClick={() => fetchNextPage?.()}
								disabled={!hasNextPage}
							>
								Load more
							</Button>
						</TableCell>
					</TableRow>
				) : null}
			</TableBody>
		</Table>
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
		</TableRow>
	);
}

export function ImageRow({
	repository,
	tag,
	createdAt,
	deployments,
	namespace,
}: Image & {
	deployments: Deployment[];
	namespace: string;
}) {
	return (
		<TagRow
			key={tag}
			repository={repository}
			tag={tag}
			createTs={createdAt}
			deployments={deployments}
			namespace={namespace}
		/>
	);
}

function TagRow({
	repository,
	tag,
	createTs,
	deployments,
	namespace,
}: {
	repository: string;
	tag: string;
	createTs: string;
	deployments: Deployment[];
	namespace: string;
}) {
	const navigate = useNavigate();
	const isDeployedToCurrentNamespace = deployments.some(
		(d) => d.namespace === namespace,
	);

	return (
		<TableRow>
			<TableCell className="size-8">
				<DiscreteCopyButton value={`${repository}:${tag}`}>
					{repository}:{tag}
				</DiscreteCopyButton>
			</TableCell>
			<TableCell>
				{deployments.length > 0 ? (
					deployments.map((deployment) => (
						<DeploymentNamespace
							key={deployment.namespace}
							namespace={deployment.namespace}
						/>
					))
				) : (
					<Text className="text-muted-foreground">-</Text>
				)}
			</TableCell>
			<TableCell>
				<CreateTs createTs={createTs} />
			</TableCell>
			<TableCell>
				{!isDeployedToCurrentNamespace ? (
					<Button
						variant="outline"
						size="sm"
						onClick={() =>
							navigate({
								to: ".",
								search: (old) => ({
									...old,
									modal: "upsert-deployment",
									namespace,
									repository,
									tag,
								}),
							})
						}
					>
						Deploy
					</Button>
				) : null}
			</TableCell>
		</TableRow>
	);
}

function CreateTs({ createTs }: { createTs: string }) {
	return (
		<WithTooltip
			content={new Date(createTs).toLocaleString()}
			trigger={
				<div>
					{formatDistance(createTs, new Date(), {
						addSuffix: true,
					})}
				</div>
			}
		/>
	);
}

function DeploymentNamespace({ namespace }: { namespace: string }) {
	const provider = useCloudNamespaceDataProvider();
	const { data } = useQuery(
		provider.currentProjectNamespaceQueryOptions({ namespace }),
	);
	return <div>{data?.displayName || <Skeleton className="w-6 h-4" />}</div>;
}
