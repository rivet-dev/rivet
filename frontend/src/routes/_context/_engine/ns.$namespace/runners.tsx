import { faRefresh, Icon } from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import { useInfiniteQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import {
	Button,
	DiscreteCopyButton,
	H1,
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
import { useEngineCompatDataProvider } from "@/components/actors";

export const Route = createFileRoute("/_context/_engine/ns/$namespace/runners")(
	{
		component: RouteComponent,
	},
);

function RouteComponent() {
	const { namespace } = Route.useParams();
	const {
		data: runners,
		isRefetching,
		hasNextPage,
		fetchNextPage,
		isLoading,
		isError,
		refetch,
	} = useInfiniteQuery(
		useEngineCompatDataProvider().runnersQueryOptions({
			namespace: namespace,
		}),
	);

	return (
		<div className="bg-card h-full border my-2 mr-2 rounded-lg">
			<div className="max-w-5xl mx-auto my-2 flex justify-between items-center px-6 py-4">
				<H1>Runners</H1>
				<div className="flex items-center gap-2">
					<WithTooltip
						content="Refresh"
						trigger={
							<Button
								size="icon"
								isLoading={isRefetching}
								variant="outline"
								onClick={() => refetch()}
							>
								<Icon icon={faRefresh} />
							</Button>
						}
					/>
				</div>
			</div>

			<hr className="mb-4" />

			<div className="p-4">
				<div className="max-w-5xl mx-auto p-2">
					<div className="border rounded-md">
						<Table>
							<TableHeader>
								<TableRow>
									<TableHead>ID</TableHead>
									<TableHead>Name</TableHead>
									<TableHead>Slots</TableHead>
									<TableHead>Last ping</TableHead>
									<TableHead>Created</TableHead>
								</TableRow>
							</TableHeader>
							<TableBody>
								{!isLoading && runners?.length === 0 ? (
									<TableRow>
										<TableCell colSpan={8}>
											<Text className="text-center">
												There's no runners matching
												criteria.
											</Text>
										</TableCell>
									</TableRow>
								) : null}
								{isError ? (
									<TableRow>
										<TableCell colSpan={8}>
											<Text className="text-center">
												An error occurred while fetching
												runners.
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
								{runners?.map((runner) => (
									<Row {...runner} key={runner.runnerId} />
								))}

								{!isLoading && hasNextPage ? (
									<TableRow>
										<TableCell colSpan={6}>
											<Button
												variant="outline"
												isLoading={isLoading}
												onClick={() => fetchNextPage()}
												disabled={!hasNextPage}
											>
												Load more
											</Button>
										</TableCell>
									</TableRow>
								) : null}
							</TableBody>
						</Table>
					</div>
				</div>
			</div>
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
		</TableRow>
	);
}

function Row(runner: Rivet.Runner) {
	return (
		<TableRow key={runner.runnerId}>
			<TableCell>
				<WithTooltip
					content={runner.runnerId}
					trigger={
						<DiscreteCopyButton value={runner.name}>
							{runner.name}
						</DiscreteCopyButton>
					}
				/>
			</TableCell>
			<TableCell>
				<DiscreteCopyButton value={runner.name}>
					{runner.name}
				</DiscreteCopyButton>
			</TableCell>

			<TableCell>
				{runner.remainingSlots}/{runner.totalSlots}
			</TableCell>

			<TableCell>
				{new Date(runner.lastPingTs).toLocaleString()}
			</TableCell>

			<TableCell>{new Date(runner.createTs).toLocaleString()}</TableCell>
		</TableRow>
	);
}
