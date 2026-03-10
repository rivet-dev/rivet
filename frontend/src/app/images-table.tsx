import { useSuspenseInfiniteQuery } from "@tanstack/react-query";
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
import { VisibilitySensor } from "@/components/visibility-sensor";

interface ImagesTableProps {
	isLoading?: boolean;
	isError?: boolean;
	hasNextPage?: boolean;
	fetchNextPage?: () => void;
	images: Image[];
	deployments: Deployment[];
}

interface Image {
	repository: string;
	tag: string;
	createdAt: string;
}

interface Deployment {
	repository?: string;
	namespace?: string;
	tag?: string;
}

export function ImagesTable({
	isLoading,
	isError,
	hasNextPage,
	fetchNextPage,
	images,
	deployments,
}: ImagesTableProps) {
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead className="pl-8">Tag</TableHead>
					<TableHead className="pl-8">Deployed To</TableHead>
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
				<Skeleton className="w-full size-4" />
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

export function ImageRow({
	repository,
	tag,
	createdAt,
	deployments,
}: Image & {
	deployments: Deployment[];
}) {
	return (
		<TagRow
			key={tag}
			repository={repository}
			tag={tag}
			createTs={createdAt}
			deployments={deployments}
		/>
	);
}

function TagRow({
	repository,
	tag,
	createTs,
	deployments,
}: {
	repository: string;
	tag: string;
	createTs: string;
	deployments: Deployment[];
}) {
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
						<div key={deployment.namespace}>
							{deployment.namespace}
						</div>
					))
				) : (
					<Text className="text-muted-foreground">-</Text>
				)}
			</TableCell>
			<TableCell>
				<CreateTs createTs={createTs} />
			</TableCell>
			<TableCell></TableCell>
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
