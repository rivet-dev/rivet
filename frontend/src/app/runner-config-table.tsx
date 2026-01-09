import {
	faCog,
	faCogs,
	faEllipsisVertical,
	faNextjs,
	faPencil,
	faRailway,
	faTrash,
	faTriangleExclamation,
	faVercel,
	Icon,
} from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import { Link, useNavigate } from "@tanstack/react-router";
import { useMemo } from "react";
import {
	Button,
	CopyArea,
	DiscreteCopyButton,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
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
					<TableHead className="text-center">Provider</TableHead>
					<TableHead className="text-center">Endpoint</TableHead>
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

function Row({
	name,
	...value
}: { name: string } & Rivet.RunnerConfigsListResponseRunnerConfigsValue) {
	const navigate = useNavigate();

	const datacenters = Object.entries(value.datacenters);

	return (
		<TableRow className="hover:bg-muted/50">
			<StatusCell datacenters={value.datacenters} />
			<TableCell>
				<DiscreteCopyButton value={name}>{name}</DiscreteCopyButton>
			</TableCell>
			<TableCell className="text-center">
				<Providers datacenters={datacenters} />
			</TableCell>
			<TableCell className="text-center">
				<Endpoints datacenters={datacenters} />
			</TableCell>
			<TableCell className="text-center">
				<Regions regions={Object.keys(value.datacenters)} />
			</TableCell>
			<TableCell className="text-right">
				<DropdownMenu>
					<DropdownMenuTrigger asChild>
						<Button variant="ghost" size="icon">
							<Icon icon={faEllipsisVertical} />
						</Button>
					</DropdownMenuTrigger>
					<DropdownMenuContent align="end">
						<DropdownMenuItem
							indicator={<Icon icon={faPencil} />}
							onSelect={() =>
								navigate({
									to: ".",
									search: {
										modal: "edit-provider-config",
										config: name,
									},
								})
							}
						>
							Edit
						</DropdownMenuItem>
						<DropdownMenuItem
							indicator={<Icon icon={faTrash} />}
							onSelect={() =>
								navigate({
									to: ".",
									search: {
										modal: "delete-provider-config",
										config: name,
									},
								})
							}
						>
							Delete
						</DropdownMenuItem>
					</DropdownMenuContent>
				</DropdownMenu>
			</TableCell>
		</TableRow>
	);
}

function StatusCell({
	datacenters,
}: {
	datacenters: Record<string, Rivet.RunnerConfigResponse>;
}) {
	const errors = useMemo(() => {
		const errorMap: Record<string, Rivet.RunnerPoolError | undefined> = {};
		let hasErrors = false;
		for (const [dc, config] of Object.entries(datacenters)) {
			if (config.runnerPoolError) {
				errorMap[dc] = config.runnerPoolError;
				hasErrors = true;
			}
		}
		return hasErrors ? errorMap : null;
	}, [datacenters]);

	if (!errors) {
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
					<>
						<p>Some providers are experiencing errors:</p>
						<ul className="max-w-xs whitespace-pre-wrap text-left my-0 space-y-1">
							{typeof errors !== "string"
								? Object.entries(errors).map(([dc, error]) => {
										if (!error) return null;
										return (
											<li
												key={dc}
												className="border-t pt-2 pb-2"
											>
												<ActorRegion
													className="w-full justify-start items-center mr-2 mt-3 mb-2"
													regionId={dc}
													showLabel
												/>
												<div className="text-xs">
													<RunnerPoolError
														error={error}
													/>
												</div>
											</li>
										);
									})
								: errors}
						</ul>
					</>
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

function Providers({
	datacenters,
}: {
	datacenters: [string, Rivet.RunnerConfigResponse][];
}) {
	const providers = useMemo(() => {
		const providerSet = new Set<string>();
		for (const [, config] of datacenters) {
			const providerName = getProviderName(config.metadata);
			providerSet.add(providerName);
		}
		return Array.from(providerSet);
	}, [datacenters]);

	if (providers.length === 1) {
		return <Provider metadata={datacenters[0][1].metadata} />;
	}

	return (
		<WithTooltip
			content={providers.join(" and ")}
			trigger={<span>Multiple providers</span>}
		/>
	);
}

function Endpoints({
	datacenters,
}: {
	datacenters: [string, Rivet.RunnerConfigResponse][];
}) {
	const endpoints = useMemo(() => {
		const endpointSet = new Set<string>();
		for (const [, config] of datacenters) {
			if (config.serverless?.url) {
				endpointSet.add(config.serverless.url);
			}
		}
		return Array.from(endpointSet);
	}, [datacenters]);

	if (endpoints.length === 1) {
		const endpoint = endpoints[0];
		return (
			<DiscreteCopyButton value={endpoint}>
				{endpoint && endpoint.length > 32
					? `${endpoint.slice(0, 16)}...${endpoint.slice(-16)}`
					: endpoint || "-"}
			</DiscreteCopyButton>
		);
	}

	return (
		<WithTooltip
			content={
				<>
					<p className="my-2">Endpoints:</p>
					<ul className="list-disc list-inside">
						{endpoints.map((endpoint) => (
							<li key={endpoint}>
								<DiscreteCopyButton
									tooltip={false}
									value={endpoint}
									className="px-2 -mx-2"
								>
									<span>
										{endpoint && endpoint.length > 32
											? `${endpoint.slice(0, 16)}...${endpoint.slice(-16)}`
											: endpoint || "-"}
									</span>
								</DiscreteCopyButton>
							</li>
						))}
					</ul>
				</>
			}
			trigger={<span>Multiple endpoints</span>}
		/>
	);
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
				<div className="whitespace-nowrap">
					<Icon icon={faVercel} className="mr-1" /> Vercel
				</div>
			);
		}
		if (metadata.provider === "next-js") {
			return (
				<div className="whitespace-nowrap">
					<Icon icon={faNextjs} className="mr-1" /> Next.js
				</div>
			);
		}
		if (metadata.provider === "railway") {
			return (
				<div className="whitespace-nowrap">
					<Icon icon={faRailway} className="mr-1" /> Railway
				</div>
			);
		}
		if (metadata.provider === "hetzner") {
			return (
				<div className="whitespace-nowrap">
					<Icon icon={faCogs} className="mr-1" /> Hetzner
				</div>
			);
		}
		if (metadata.provider === "gcp") {
			return (
				<div className="whitespace-nowrap">
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
				<span className="w-full cursor-pointer">Multiple regions</span>
			}
		/>
	);
}
