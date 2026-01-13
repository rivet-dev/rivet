import {
	faHourglassEnd,
	faPlus,
	faSignalAlt,
	faSignalAlt2,
	faSignalAlt3,
	faSignalAlt4,
	Icon,
} from "@rivet-gg/icons";

import type { Rivet } from "@rivetkit/engine-api-full";
import { useInfiniteQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
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
import { ActorRegion, useEngineCompatDataProvider } from "@/components/actors";

interface RunnersTableProps {
	isLoading?: boolean;
	isError?: boolean;
	hasNextPage?: boolean;
	fetchNextPage?: () => void;
	runners: Rivet.Runner[];
}

export function RunnersTable({
	isLoading,
	isError,
	hasNextPage,
	fetchNextPage,
	runners,
}: RunnersTableProps) {
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead />
					<TableHead className="pl-8">ID</TableHead>
					<TableHead className="pl-8">Name</TableHead>
					<TableHead>Datacenter</TableHead>
					<TableHead>Slots</TableHead>
					<TableHead>Created</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{!isLoading && !isError && runners?.length === 0 ? (
					<EmptyState />
				) : null}
				{isError ? (
					<TableRow>
						<TableCell colSpan={7}>
							<Text className="text-center">
								An error occurred while fetching runners.
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

export function Row(runner: Rivet.Runner) {
	return (
		<TableRow key={runner.runnerId}>
			<TableCell className="size-8">
				<RunnerStatusBadge {...runner} />
			</TableCell>
			<TableCell>
				<WithTooltip
					content={runner.runnerId}
					trigger={
						<DiscreteCopyButton value={runner.runnerId}>
							{runner.runnerId.slice(0, 8)}
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
				<ActorRegion regionId={runner.datacenter} showLabel />
			</TableCell>

			<TableCell>
				{runner.remainingSlots}/{runner.totalSlots}
			</TableCell>

			<TableCell>
				<CreateTs createTs={runner.createTs} />
			</TableCell>
		</TableRow>
	);
}

function CreateTs({ createTs }: { createTs: number }) {
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

function RunnerStatusBadge(runner: Rivet.Runner) {
	// check if the last ping ts was higher than 15 seconds ago
	const now = Date.now();
	if (now - runner.lastPingTs > 15000) {
		return (
			<WithTooltip
				content={`Offline (last seen ${formatDistance(
					runner.lastPingTs,
					now,
					{ addSuffix: true },
				)})`}
				trigger={
					<div className="text-center relative size-8">
						<Icon
							icon={faHourglassEnd}
							className="text-red-500 absolute inset-1/2 -translate-x-1/2 -translate-y-1/2"
						/>
					</div>
				}
			/>
		);
	}
	if (runner.lastRtt <= 50) {
		return (
			<WithTooltip
				content={`${runner.lastRtt}ms`}
				trigger={
					<div className="text-center relative size-8">
						<Icon
							icon={faSignalAlt4}
							className="text-green-500 absolute inset-1/2 -translate-x-1/2 -translate-y-1/2"
						/>
					</div>
				}
			/>
		);
	}
	if (runner.lastRtt > 50 && runner.lastRtt <= 200) {
		return (
			<WithTooltip
				content={`${runner.lastRtt}ms`}
				trigger={
					<div className="text-center relative size-8">
						<Icon
							icon={faSignalAlt4}
							className="text-muted-foreground/20 absolute inset-1/2 -translate-x-1/2 -translate-y-1/2"
						/>
						<Icon
							icon={faSignalAlt3}
							className="text-primary/70 absolute inset-1/2 -translate-x-1/2 -translate-y-1/2"
						/>
					</div>
				}
			/>
		);
	}

	return (
		<WithTooltip
			content={`${runner.lastRtt}ms`}
			trigger={
				<div className="text-center relative size-8">
					<Icon
						icon={faSignalAlt}
						className="text-muted-foreground/20 absolute inset-1/2 -translate-x-1/2 -translate-y-1/2"
					/>
					<Icon
						icon={faSignalAlt2}
						className="text-red-500 absolute inset-1/2 -translate-x-1/2 -translate-y-1/2"
					/>
				</div>
			}
		/>
	);
}

function EmptyState() {
	const { data: serverlessConfig } = useInfiniteQuery({
		...useEngineCompatDataProvider().runnerConfigsQueryOptions(),
		select(data) {
			for (const page of data.pages) {
				for (const rc of Object.values(page.runnerConfigs)) {
					for (const [dc, config] of Object.entries(rc.datacenters)) {
						if (config.serverless) {
							return config;
						}
					}
				}
			}
			return null;
		},
	});

	const { data: actorNames } = useInfiniteQuery({
		...useEngineCompatDataProvider().buildsQueryOptions(),
		select(data) {
			return data.pages[0].builds.length > 0;
		},
	});

	return (
		<TableRow>
			<TableCell colSpan={7}>
				{serverlessConfig ? (
					<>
						<Text className="text-center">
							Runners will be created when an actor is created.
						</Text>
						{actorNames ? (
							<div className="text-center mt-2">
								<Button
									asChild
									size="sm"
									startIcon={<Icon icon={faPlus} />}
								>
									<Link
										to="."
										search={{ modal: "create-actor" }}
									>
										Create Actor
									</Link>
								</Button>
							</div>
						) : null}
					</>
				) : (
					<Text className="text-center">
						There are no runners connected. You will not be able to
						run actors until a runner appears here.
					</Text>
				)}
			</TableCell>
		</TableRow>
	);
}
