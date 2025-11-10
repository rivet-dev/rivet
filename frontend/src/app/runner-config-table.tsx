import {
	faCog,
	faCogs,
	faNextjs,
	faRailway,
	faTrash,
	faVercel,
	Icon,
} from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import { Link } from "@tanstack/react-router";
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
import { ActorRegion } from "@/components/actors";
import { REGION_LABEL } from "@/components/matchmaker/lobby-region";
import { hasMetadataProvider } from "./data-providers/engine-data-provider";

interface RunnerConfigsTableProps {
	isLoading?: boolean;
	isError?: boolean;
	hasNextPage?: boolean;
	fetchNextPage?: () => void;
	configs: [string, Rivet.RunnerConfigsListResponseRunnerConfigsValue][];
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
					<TableHead className="pl-8">Endpoint</TableHead>
					<TableHead className="text-center">Datacenter</TableHead>
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
				{configs?.map(([id, config]) => (
					<Row name={id} {...config} key={id} />
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

function Row({
	name,
	...value
}: { name: string } & Rivet.RunnerConfigsListResponseRunnerConfigsValue) {
	const config = Object.values(value.datacenters)[0];

	return (
		<TableRow>
			<TableCell>
				<DiscreteCopyButton value={name}>{name}</DiscreteCopyButton>
			</TableCell>
			<TableCell>
				<Provider metadata={config.metadata} />
			</TableCell>
			<TableCell>
				<WithTooltip
					content={config.serverless?.url || "-"}
					trigger={
						<DiscreteCopyButton
							value={config.serverless?.url || ""}
						>
							<span>
								{config.serverless?.url &&
								config.serverless.url.length > 32
									? `${config.serverless.url.slice(0, 16)}...${config.serverless.url.slice(-16)}`
									: config.serverless?.url}
							</span>
						</DiscreteCopyButton>
					}
				/>
			</TableCell>
			<TableCell className="text-center">
				<Regions regions={Object.keys(value.datacenters)} />
			</TableCell>

			<TableCell>
				<div className="flex gap-2 justify-end">
					{config.serverless &&
					hasMetadataProvider(config.metadata) ? (
						<WithTooltip
							content="Edit provider settings"
							trigger={
								<Button variant="outline" size="icon" asChild>
									<Link
										to="."
										search={{
											modal: getModal(
												config.metadata?.provider,
											),
											config: name,
											dc: Object.keys(
												value.datacenters,
											)[0],
										}}
									>
										<Icon icon={faCog} />
									</Link>
								</Button>
							}
						/>
					) : null}
					<WithTooltip
						content="Delete provider"
						trigger={
							<Button variant="outline" size="icon" asChild>
								<Link
									to="."
									search={{
										modal: "delete-provider-config",
										config: name,
									}}
								>
									<Icon icon={faTrash} />
								</Link>
							</Button>
						}
					/>
				</div>
			</TableCell>
		</TableRow>
	);
}

function getModal(provider: string | undefined) {
	return "edit-provider-config";
}

function Provider({ metadata }: { metadata: unknown }) {
	if (!metadata || typeof metadata !== "object") {
		return <span>Unknown</span>;
	}
	if ("provider" in metadata && typeof metadata.provider === "string") {
		if (metadata.provider === "vercel") {
			return (
				<div>
					<Icon icon={faVercel} className="mr-1" /> Vercel
				</div>
			);
		}
		if (metadata.provider === "next-js") {
			return (
				<div>
					<Icon icon={faNextjs} className="mr-1" /> Next.js
				</div>
			);
		}
		if (metadata.provider === "railway") {
			return (
				<div>
					<Icon icon={faRailway} className="mr-1" /> Railway
				</div>
			);
		}
		if (metadata.provider === "hetzner") {
			return (
				<div>
					<Icon icon={faCogs} className="mr-1" /> Hetzner
				</div>
			);
		}
		if (metadata.provider === "gcp") {
			return (
				<div>
					<Icon icon={faCog} className="mr-1" /> Google Cloud Run
				</div>
			);
		}
		return <span>{metadata.provider || "-"}</span>;
	}
	return <span>Unknown</span>;
}

function Regions({ regions }: { regions: string[] }) {
	if (regions.length === 1) {
		return (
			<ActorRegion
				className="w-full items-center flex-1"
				regionId={regions[0]}
				showLabel
			/>
		);
	}

	return (
		<WithTooltip
			content={regions
				.map((region) => REGION_LABEL[region] ?? REGION_LABEL.unknown)
				.join(" and ")}
			trigger={
				<span className="w-full cursor-pointer">
					{regions.length} regions
				</span>
			}
		/>
	);
}
