import type { Rivet } from "@rivetkit/engine-api-full";
import { formatRelative } from "date-fns";
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

interface RunnerConfigsTableProps {
	isLoading?: boolean;
	isError?: boolean;
	hasNextPage?: boolean;
	fetchNextPage?: () => void;
	configs: Rivet.RunnerConfig[];
}

export function RunnerConfigsTable({
	isLoading,
	isError,
	hasNextPage,
	fetchNextPage,
	configs,
}: RunnerConfigsTableProps) {
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead className="pl-8">Name</TableHead>
					<TableHead>Provider</TableHead>
					<TableHead>Endpoint</TableHead>
					<TableHead></TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{!isLoading && !isError && configs?.length === 0 ? (
					<TableRow>
						<TableCell colSpan={4}>
							<Text className="text-center">
								There's no providers matching criteria.
							</Text>
						</TableCell>
					</TableRow>
				) : null}
				{isError ? (
					<TableRow>
						<TableCell colSpan={4}>
							<Text className="text-center">
								An error occurred while fetching providers.
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
				{configs?.map((config) => (
					<Row {...config} key={config.metadata} />
				))}

				{!isLoading && hasNextPage ? (
					<TableRow>
						<TableCell colSpan={4}>
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
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
		</TableRow>
	);
}

function Row(config: Rivet.RunnerConfig) {
	return (
		<TableRow key={runner.runnerId}>
			<TableCell>
				<WithTooltip
					content={config}
					trigger={
						<DiscreteCopyButton value={runner.runnerId}>
							{runner.runnerId.slice(0, 8)}
						</DiscreteCopyButton>
					}
				/>
			</TableCell>
			<TableCell>
				{config.metadata &&
				typeof config.metadata === "object" &&
				"provider" in config.metadata
					? (config.metadata.provider as string)
					: "unknown"}
			</TableCell>
			<TableCell>-</TableCell>
			<TableCell>{runner.datacenter}</TableCell>

			<TableCell>
				{runner.remainingSlots}/{runner.totalSlots}
			</TableCell>

			<TableCell>{formatRelative(runner.createTs, new Date())}</TableCell>
		</TableRow>
	);
}
