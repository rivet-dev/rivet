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
import { startTransition, useCallback, useState } from "react";
import { HelpDropdown } from "@/app/help-dropdown";
import { Content } from "@/app/layout";
import { SidebarToggle } from "@/app/sidebar-toggle";
import { Button, H1, Skeleton } from "@/components";
import {
	useCloudNamespaceDataProvider,
	useDataProvider,
} from "@/components/actors";
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
	const [logs, setLogs] = useState<RivetSse.LogEntry[]>([]);

	const logsText = useCallback(
		() =>
			logs
				.map((e) => `${e.timestamp}\t${e.region}\t${e.message}`)
				.join("\n"),
		[logs],
	);

	const handleDownload = useCallback(() => {
		const blob = new Blob([logsText()], { type: "text/plain" });
		const url = URL.createObjectURL(blob);
		const a = document.createElement("a");
		a.href = url;
		a.download = `deployment-logs-${namespace}.txt`;
		a.click();
		URL.revokeObjectURL(url);
	}, [logsText, namespace]);

	const handleCopy = useCallback(() => {
		navigator.clipboard.writeText(logsText());
	}, [logsText]);

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

				<div className="w-full border-t flex-1 flex flex-col">
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
						<Select value={region} onValueChange={setRegion}>
							<SelectTrigger className="border-0 border-r bg-transparent rounded-none text-xs h-auto max-w-32 py-0 gap-1.5 px-2">
								<SelectValue placeholder="All regions" />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value="all">All regions</SelectItem>
								{datacenters.map((dc) => (
									<SelectItem key={dc.name} value={dc.name}>
										{dc.name}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
						<Button
							variant="ghost"
							size="icon-sm"
							className="m-1"
							onClick={() => setIsPaused((p) => !p)}
						>
							<Icon icon={isPaused ? faPlay : faPause} />
						</Button>
						<DropdownMenu>
							<DropdownMenuTrigger asChild>
								<Button
									variant="ghost"
									size="sm"
									className="m-1 px-0"
									disabled={logs.length === 0}
								>
									Export
								</Button>
							</DropdownMenuTrigger>
							<DropdownMenuContent align="end">
								<DropdownMenuItem onClick={handleDownload}>
									<Icon icon={faDownload} />
									Download
								</DropdownMenuItem>
								<DropdownMenuItem onClick={handleCopy}>
									<Icon icon={faCopy} />
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
								onLogsChange={setLogs}
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
