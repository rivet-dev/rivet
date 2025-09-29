import { faHourglassClock, Icon } from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import {
	Button,
	DiscreteCopyButton,
	Ping,
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
						<TableCell colSpan={9}>
							<Text className="text-center">
								There's no runners matching criteria.
							</Text>
						</TableCell>
					</TableRow>
				) : null}
				{isError ? (
					<TableRow>
						<TableCell colSpan={9}>
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
						<TableCell colSpan={6}>
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

function Row(runner: Rivet.Runner) {
	return (
		<TableRow key={runner.runnerId}>
			<TableCell>
				<RunnerStatusBadge {...runner} />
			</TableCell>
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

function RunnerStatusBadge(runner: Rivet.Runner) {
	if (runner.drainTs) {
		return (
			<WithTooltip
				content="Draining"
				trigger={
					<Icon icon={faHourglassClock} className="text-warning" />
				}
			/>
		);
	}
	if (runner.stopTs) {
		return (
			<WithTooltip
				content="Stopped"
				trigger={
					<span className={"size-2 rounded-full, bg-foreground/10"} />
				}
			/>
		);
	}

	return <Ping variant="success" className="relative right-auto" />;
}
