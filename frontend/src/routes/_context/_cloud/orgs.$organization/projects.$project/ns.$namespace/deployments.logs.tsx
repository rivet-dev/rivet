import type { RivetSse } from "@rivet-gg/cloud";
import {
	faCopy,
	faDownload,
	faPause,
	faPlay,
	faQuestionCircle,
	Icon,
} from "@rivet-gg/icons";
import { useInfiniteQuery, useSuspenseQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { startTransition, useCallback, useMemo, useRef, useState } from "react";
import { HelpDropdown } from "@/app/help-dropdown";
import { Content } from "@/app/layout";
import { SidebarToggle } from "@/app/sidebar-toggle";
import { Button, H1, Skeleton } from "@/components";
import {
	useCloudNamespaceDataProvider,
	useDataProvider,
} from "@/components/actors";
import { RegionSelect } from "@/components/actors/region-select";
import { DeploymentLogs } from "@/components/deployment-logs";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/deployments/logs",
)({
	component: RouteComponent,
	loader: async ({ context }) => {
		const dataProvider = context.dataProvider;
		await context.queryClient.prefetchQuery(
			dataProvider.currentNamespaceManagedPoolQueryOptions({
				pool: "default",
				safe: true,
			}),
		);
	},
	pendingComponent: DataLoadingPlaceholder,
});

function RouteComponent() {
	const { namespace } = Route.useParams();
	const dataProvider = useCloudNamespaceDataProvider();

	const { data: pool } = useSuspenseQuery(
		dataProvider.currentNamespaceManagedPoolQueryOptions({
			pool: "default",
			safe: true,
		}),
	);

	const { data: datacenters = [] } = useInfiniteQuery(
		useDataProvider().datacentersQueryOptions(),
	);

	const [search, setSearch] = useState("");
	const [isPaused, setIsPaused] = useState(false);
	const [region, setRegion] = useState<string>("all");
	const logsRef = useRef<RivetSse.LogEntry[]>([]);

	const getLogsText = useCallback(
		() =>
			logsRef.current
				.map((e) => `${e.timestamp}\t${e.region}\t${e.message}`)
				.join("\n"),
		[],
	);

	const handleDownload = useCallback(() => {
		const blob = new Blob([getLogsText()], { type: "text/plain" });
		const url = URL.createObjectURL(blob);
		const a = document.createElement("a");
		a.href = url;
		a.download = `deployment-logs-${namespace}.txt`;
		a.click();
		URL.revokeObjectURL(url);
	}, [getLogsText, namespace]);

	const handleCopy = useCallback(() => {
		navigator.clipboard.writeText(getLogsText());
	}, [getLogsText]);

	return (
		<Content>
			<div className="flex flex-col h-full">
				<div className="pt-2 px-6 mx-auto w-full">
					<div className="flex justify-between items-center px-0 py-4">
						<SidebarToggle className="absolute left-4" />
						<H1>Logs</H1>

						<HelpDropdown>
							<Button
								variant="outline"
								startIcon={<Icon icon={faQuestionCircle} />}
							>
								Need help?
							</Button>
						</HelpDropdown>
					</div>

					<p className="mb-6 text-muted-foreground">
						Monitor real-time logs from your deployments here.
					</p>
				</div>

				<div className="w-full border-t flex-1 flex flex-col min-h-0">
					<div className="flex items-stretch border-b	px-6 shrink-0">
						<div className="border-r flex flex-1">
							<input
								type="text"
								className="bg-transparent outline-none text-xs placeholder:text-muted-foreground font-sans flex-1 py-2"
								placeholder="Search logs..."
								spellCheck={false}
								onChange={(e) =>
									startTransition(() =>
										setSearch(e.target.value),
									)
								}
							/>
						</div>
						<RegionSelect
							onValueChange={setRegion}
							value={region}
							showAuto={false}
							showAllRegions={true}
							className="bg-transparent max-w-64 border-0 rounded-none"
						/>
						<div className="border-l flex items-center pl-4" />
						<Button
							variant="ghost"
							size="icon-sm"
							className="h-full rounded-none"
							onClick={() => setIsPaused((p) => !p)}
						>
							<Icon icon={isPaused ? faPlay : faPause} />
						</Button>
						<DropdownMenu>
							<DropdownMenuTrigger asChild>
								<Button
									variant="ghost"
									size="sm"
									className="mx-1 h-full rounded-none"
								>
									Export
								</Button>
							</DropdownMenuTrigger>
							<DropdownMenuContent align="end">
								<DropdownMenuItem
									indicator={<Icon icon={faDownload} />}
									onClick={handleDownload}
								>
									Download
								</DropdownMenuItem>
								<DropdownMenuItem
									indicator={<Icon icon={faCopy} />}
									onClick={handleCopy}
								>
									Copy
								</DropdownMenuItem>
							</DropdownMenuContent>
						</DropdownMenu>
					</div>

					{pool ? (
						<div className="flex-1 min-h-0 overflow-hidden">
							<DeploymentLogs
								namespace={namespace}
								pool="default"
								filter={search || undefined}
								region={region === "all" ? undefined : region}
								paused={isPaused}
								logsRef={logsRef}
							/>
						</div>
					) : (
						<div className="h-full flex flex-1 flex-col items-center justify-center">
							<p>No logs available.</p>
							<p className="text-muted-foreground text-xs mt-1">
								No active runner pool found. Logs will appear
								here once a deployment is active and running.
							</p>
							<p className="text-muted-foreground text-xs mt-1">
								If you just started a deployment, please allow a
								few moments for the logs to become available.
							</p>
						</div>
					)}
				</div>
			</div>
		</Content>
	);
}

function DataLoadingPlaceholder() {
	return (
		<div className="bg-card h-full border my-2 mr-2 rounded-lg">
			<div className="mt-2 flex justify-between items-center px-6 py-4 max-w-5xl mx-auto">
				<Skeleton className="w-48 h-8" />
			</div>
			<hr className="mb-4" />
			<div className="p-4 px-6 max-w-5xl mx-auto">
				<Skeleton className="w-full h-96 rounded-md" />
			</div>
		</div>
	);
}
