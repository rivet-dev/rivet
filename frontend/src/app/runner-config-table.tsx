import {
	faChevronDown,
	faChevronRight,
	faCog,
	faCogs,
	faNextjs,
	faRailway,
	faTrash,
	faTriangleExclamation,
	faVercel,
	Icon,
} from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import { Link } from "@tanstack/react-router";
import { useMemo, useState } from "react";
import { match, P } from "ts-pattern";
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
import { ActorRegion, RunnerPoolError } from "@/components/actors";
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
					<TableHead className="w-8"></TableHead>
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
						<TableCell colSpan={6}>
							<Text className="text-center">
								There's no providers matching criteria.
							</Text>
						</TableCell>
					</TableRow>
				) : null}
				{isError ? (
					<TableRow>
						<TableCell colSpan={6}>
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
			<TableCell className="w-8"></TableCell>
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

type DatacenterConfig = {
	datacenterId: string;
	config: Rivet.RunnerConfigResponse;
};

type GroupedConfig = {
	provider: string;
	endpoint: string;
	metadata: unknown;
	datacenters: string[];
	runnerPoolErrors: Record<string, Rivet.RunnerPoolError | undefined>;
};

function Row({
	name,
	...value
}: { name: string } & Rivet.RunnerConfigsListResponseRunnerConfigsValue) {
	const [isExpanded, setIsExpanded] = useState(true);

	const groupedConfigs = useMemo(() => {
		const datacenterEntries: DatacenterConfig[] = Object.entries(
			value.datacenters,
		).map(([datacenterId, config]) => ({ datacenterId, config }));

		const groupedConfigs: GroupedConfig[] = [];

		for (const { datacenterId, config } of datacenterEntries) {
			const provider = getProviderName(config.metadata);
			const endpoint = config.serverless?.url || "";
			const runnerPoolError = config.runnerPoolError;

			const existingGroup = groupedConfigs.find(
				(g) => g.provider === provider && g.endpoint === endpoint,
			);

			if (existingGroup) {
				existingGroup.datacenters.push(datacenterId);
				existingGroup.runnerPoolErrors[datacenterId] = runnerPoolError;
			} else {
				groupedConfigs.push({
					provider,
					endpoint,
					metadata: config.metadata,
					datacenters: [datacenterId],
					runnerPoolErrors: { [datacenterId]: runnerPoolError },
				});
			}
		}
		return groupedConfigs;
	}, [value.datacenters]);

	const hasMultipleConfigs = groupedConfigs.length > 1;

	if (!hasMultipleConfigs) {
		return <ProviderRow {...groupedConfigs[0]} name={name} />;
	}

	const hasAtLeastOneError = groupedConfigs.some(
		(g) => Object.keys(g.runnerPoolErrors).length > 0,
	);

	return (
		<>
			<TableRow
				className="cursor-pointer hover:bg-muted/50"
				onClick={() => setIsExpanded(!isExpanded)}
			>
				<StatusCell
					errors={hasAtLeastOneError ? "One or more providers	have errors" : ""}
				/>
				<TableCell>
					<div className="flex items-center gap-2">
						<Icon
							icon={isExpanded ? faChevronDown : faChevronRight}
							className="text-muted-foreground"
						/>
						<DiscreteCopyButton value={name}>
							{name}
						</DiscreteCopyButton>
					</div>
				</TableCell>
				<TableCell colSpan={2}>
					<Text className="text-muted-foreground">
						{groupedConfigs.length}{" "}
						{groupedConfigs.length === 1 ? "provider" : "providers"}
					</Text>
				</TableCell>
				<TableCell className="text-center">
					<Regions regions={Object.keys(value.datacenters)} />
				</TableCell>
				<TableCell>
					<div className="flex gap-2 justify-end">
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

			{isExpanded &&
				groupedConfigs.map((groupedConfig, idx) => (
					<ProviderRow key={idx} name={name} {...groupedConfig} />
				))}
		</>
	);
}



function StatusCell({
	errors,
}: {
	errors: Record<string, Rivet.RunnerPoolError | undefined> | string;
}) {
	const hasErrors = typeof errors !== "string" ? Object.values(errors).some((err) => err !== undefined) : !!errors;

	if (!hasErrors) {
		return (
			<TableCell className="w-8">
				<Ping variant="success" className="relative" />
			</TableCell>
		);
	}

	return (
		<TableCell className="w-8 text-center">
			<WithTooltip
				content={
					<div className="max-w-xs whitespace-pre-wrap text-left space-y-1">
						{typeof errors !== "string" ? Object.entries(errors).map(([dc, error]) => {
							if (!error) return null;
							return (
								<div key={dc}>
									<ActorRegion
										className="w-full items-center mr-2"
										regionId={dc}
										showLabel
									/>
									<RunnerPoolError error={error} />
								</div>
							);
						}) : errors}
					</div>
				}
				trigger={
					<span className="inline-flex items-center justify-center text-destructive">
						<Icon icon={faTriangleExclamation} />
					</span>
				}
			/>
		</TableCell>
	);
}

function ProviderRow({
	name,
	metadata,
	endpoint,
	runnerPoolErrors,
	datacenters,
}: GroupedConfig & { name: string }) {
	return (
		<TableRow>
			<StatusCell errors={runnerPoolErrors} />
			<TableCell>
				<DiscreteCopyButton value={name}>{name}</DiscreteCopyButton>
			</TableCell>
			<TableCell>
				<Provider metadata={metadata} />
			</TableCell>
			<TableCell>
				<WithTooltip
					content={endpoint || "-"}
					trigger={
						<DiscreteCopyButton value={endpoint || ""}>
							<span>
								{endpoint && endpoint.length > 32
									? `${endpoint.slice(0, 16)}...${endpoint.slice(-16)}`
									: endpoint || "-"}
							</span>
						</DiscreteCopyButton>
					}
				/>
			</TableCell>
			<TableCell className="text-center">
				<Regions regions={datacenters} />
			</TableCell>
			<TableCell>
				<div className="flex gap-2 justify-end">
					{endpoint && hasMetadataProvider(metadata) ? (
						<WithTooltip
							content="Edit provider settings"
							trigger={
								<Button variant="outline" size="icon" asChild>
									<Link
										to="."
										search={{
											modal: getModal(metadata),
											config: name,
											dc: datacenters[0],
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

function getModal(metadata: unknown) {
	return "edit-provider-config";
}

function getProviderName(metadata: unknown): string {
	if (!metadata || typeof metadata !== "object") {
		return "unknown";
	}
	if ("provider" in metadata && typeof metadata.provider === "string") {
		return metadata.provider;
	}
	return "unknown";
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
