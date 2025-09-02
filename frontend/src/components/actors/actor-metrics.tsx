import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Button, Dd, Dl, Dt, Flex } from "@/components";
import type { ActorId } from "./queries";

const timeWindowOptions = [
	{ label: "5 minutes", value: "5m", milliseconds: 5 * 60 * 1000 },
	{ label: "15 minutes", value: "15m", milliseconds: 15 * 60 * 1000 },
	{ label: "30 minutes", value: "30m", milliseconds: 30 * 60 * 1000 },
	{ label: "1 hour", value: "1h", milliseconds: 60 * 60 * 1000 },
	{ label: "3 hours", value: "3h", milliseconds: 3 * 60 * 60 * 1000 },
	{ label: "6 hours", value: "6h", milliseconds: 6 * 60 * 60 * 1000 },
	{ label: "12 hours", value: "12h", milliseconds: 12 * 60 * 60 * 1000 },
	{ label: "24 hours", value: "24h", milliseconds: 24 * 60 * 60 * 1000 },
	{ label: "2 days", value: "2d", milliseconds: 2 * 24 * 60 * 60 * 1000 },
];

export interface ActorMetricsProps {
	actorId: ActorId;
}

export function ActorMetrics({ actorId }: ActorMetricsProps) {
	// const { data: status } = useQuery(actorStatusQueryOptions(actorId));
	// const {
	// 	data: metricsData,
	// 	isLoading,
	// 	isError,
	// } = useQuery(actorMetricsQueryOptions(actorId));

	// const [showAdvanced, setShowAdvanced] = useState(false);

	// const timeWindowMs = useAtomValue(actorMetricsTimeWindowAtom);
	// const setTimeWindowMs = useSetAtom(actorMetricsTimeWindowAtom);
	// const environment = useAtomValue(actorEnvironmentAtom);

	// const currentTimeWindow =
	// 	timeWindowOptions.find(
	// 		(option) => option.milliseconds === timeWindowMs,
	// 	) || timeWindowOptions[1];
	// const [timeWindow, setTimeWindow] = useState(currentTimeWindow.value);

	// const isActorRunning = status === "running";

	// // Create a query for time window-specific metrics
	// const { data: customMetricsData, status: customMetricsStatus } = useQuery({
	// 	...actorMetricsQueryOptions(
	// 		{
	// 			projectNameId: environment?.projectNameId || "",
	// 			environmentNameId: environment?.environmentNameId || "",
	// 			actorId: id,
	// 			timeWindowMs: timeWindowMs,
	// 		},
	// 		{ refetchInterval: 5000 },
	// 	),
	// 	enabled: !!environment && !!id,
	// });

	// // Use custom metrics if available, otherwise fall back to default
	// const metricsData = customMetricsData
	// 	? {
	// 			metrics: customMetricsData.metrics,
	// 			rawData: customMetricsData.rawData,
	// 			interval: customMetricsData.interval,
	// 			status: customMetricsStatus,
	// 			updatedAt: Date.now(),
	// 		}
	// 	: defaultMetricsData;

	// const handleTimeWindowChange = (value: string) => {
	// 	setTimeWindow(value);
	// 	const selectedOption = timeWindowOptions.find(
	// 		(option) => option.value === value,
	// 	);
	// 	if (selectedOption) {
	// 		setTimeWindowMs(selectedOption.milliseconds);
	// 	}
	// };

	// const formatBytes = (bytes: number | null | undefined) => {
	// 	if (!isActorRunning || bytes === null || bytes === undefined)
	// 		return "n/a";
	// 	const mb = bytes / 1024 / 1024;
	// 	if (mb < 1024) {
	// 		return `${mb.toFixed(1)} MB`;
	// 	}
	// 	return `${(mb / 1024).toFixed(1)} GB`;
	// };

	// const formatCpuUsage = (cpu: number | null | undefined) => {
	// 	if (!isActorRunning || cpu === null || cpu === undefined) return "n/a";
	// 	return `${(cpu * 100).toFixed(2)}%`;
	// };

	// const formatNumber = (value: number | null | undefined) => {
	// 	if (!isActorRunning || value === null || value === undefined)
	// 		return "n/a";
	// 	return value.toLocaleString();
	// };

	// const formatTimestamp = (timestamp: number | null | undefined) => {
	// 	if (!isActorRunning || timestamp === null || timestamp === undefined)
	// 		return "n/a";
	// 	return new Date(timestamp * 1000).toLocaleString();
	// };

	// // Calculate CPU percentage using time series data points
	// const cpuPercentage = useMemo(() => {
	// 	if (!isActorRunning) {
	// 		return "Stopped";
	// 	}

	// 	const data = metricsData;
	// 	if (!data || !data.rawData || !data.interval) {
	// 		return "n/a";
	// 	}

	// 	const cpuValues = data.rawData.cpu_usage_seconds_total;
	// 	if (!cpuValues || cpuValues.length < 2) {
	// 		return "n/a";
	// 	}

	// 	// Find the last valid CPU rate from the most recent data points
	// 	let cpuRate = 0;
	// 	for (let i = cpuValues.length - 1; i > 0; i--) {
	// 		const currentCpu = cpuValues[i];
	// 		const previousCpu = cpuValues[i - 1];

	// 		if (
	// 			currentCpu !== 0 &&
	// 			previousCpu !== 0 &&
	// 			currentCpu >= previousCpu
	// 		) {
	// 			const cpuDelta = currentCpu - previousCpu;
	// 			const timeDelta = data.interval / 1000; // Convert ms to seconds

	// 			// Rate calculation: CPU seconds used per second of real time
	// 			// This gives the fraction of available CPU used (0-1)
	// 			cpuRate = (cpuDelta / timeDelta) * 100;
	// 			break;
	// 		}
	// 	}

	// 	return `${Math.min(cpuRate, 100).toFixed(2)}%`;
	// }, [metricsData, isActorRunning]);

	// const calculateMemoryPercentage = (usage: number | null | undefined) => {
	// 	if (
	// 		!isActorRunning ||
	// 		usage === null ||
	// 		usage === undefined ||
	// 		!resources ||
	// 		!resources.memory ||
	// 		resources.memory === 0
	// 	) {
	// 		return null;
	// 	}
	// 	// Convert usage from bytes to MB and compare with resources.memory (which is in MB)
	// 	const usageMB = usage / (1024 * 1024);
	// 	return (usageMB / resources.memory) * 100;
	// };

	// const data = metricsData?.metrics || {};

	// if (isLoading) {
	// 	return (
	// 		<div className="px-4 my-8">
	// 			<h3 className="mb-2 font-semibold">Metrics</h3>
	// 			<div className="text-xs text-muted-foreground">Loading...</div>
	// 		</div>
	// 	);
	// }

	// if (isError) {
	// 	return (
	// 		<div className="px-4 my-8">
	// 			<h3 className="mb-2 font-semibold">Metrics</h3>
	// 			<div className="text-xs text-destructive">
	// 				Error loading metrics
	// 			</div>
	// 		</div>
	// 	);
	// }

	// const memoryPercentage = calculateMemoryPercentage(data.memory_usage_bytes);

	// return (
	// 	<div className="px-4 my-8">
	// 		<div className="flex items-center justify-between mb-4">
	// 			<h3 className="font-semibold">Container Metrics</h3>
	// 			<Select
	// 				value={timeWindow}
	// 				onValueChange={handleTimeWindowChange}
	// 			>
	// 				<SelectTrigger className="w-32">
	// 					<SelectValue />
	// 				</SelectTrigger>
	// 				<SelectContent>
	// 					{timeWindowOptions.map((option) => (
	// 						<SelectItem key={option.value} value={option.value}>
	// 							{option.label}
	// 						</SelectItem>
	// 					))}
	// 				</SelectContent>
	// 			</Select>
	// 		</div>

	// 		{/* Main Metrics */}
	// 		<div className="mb-6">
	// 			<Dl className="grid grid-cols-2 md:grid-cols-2 gap-4">
	// 				<div>
	// 					<Dt className="text-sm font-medium">CPU Usage</Dt>
	// 					<Dd className="text-lg font-semibold flex flex-col gap-1">
	// 						<span>{cpuPercentage}</span>
	// 						{metricsData.rawData?.cpu_usage_seconds_total &&
	// 						metricsData.rawData.cpu_usage_seconds_total.length >
	// 							0 ? (
	// 							<ActorCpuStats
	// 								syncId="actor-stats"
	// 								interval={metricsData.interval / 1000}
	// 								metricsAt={metricsData.updatedAt}
	// 								cpu={
	// 									metricsData.rawData
	// 										.cpu_usage_seconds_total ?? []
	// 								}
	// 								isRunning={isActorRunning}
	// 							/>
	// 						) : null}
	// 					</Dd>
	// 				</div>
	// 				<div>
	// 					<Dt className="text-sm font-medium">Memory Usage</Dt>
	// 					<Dd className="text-lg font-semibold flex flex-col gap-1">
	// 						<span>
	// 							{formatBytes(data.memory_usage_bytes)}
	// 							{memoryPercentage !== null && (
	// 								<span className="text-sm text-muted-foreground ml-1">
	// 									({memoryPercentage.toFixed(1)}%)
	// 								</span>
	// 							)}
	// 						</span>
	// 						{metricsData.rawData?.memory_usage_bytes &&
	// 						metricsData.rawData.memory_usage_bytes.length >
	// 							0 ? (
	// 							<ActorMemoryStats
	// 								syncId="actor-stats"
	// 								interval={metricsData.interval / 1000}
	// 								metricsAt={metricsData.updatedAt}
	// 								memory={
	// 									metricsData.rawData
	// 										.memory_usage_bytes ?? []
	// 								}
	// 								allocatedMemory={
	// 									resources?.memory
	// 										? resources.memory * 1024 * 1024
	// 										: 0
	// 								}
	// 								isRunning={isActorRunning}
	// 							/>
	// 						) : null}
	// 					</Dd>
	// 				</div>
	// 			</Dl>
	// 		</div>

	// 		{/* Advanced Metrics */}
	// 		{false && (
	// 			<Flex gap="4" direction="col" className="text-xs">
	// 				{/* CPU & Performance */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">CPU & Performance</h4>
	// 					<Dl>
	// 						<Dt>CPU Load Average (10s)</Dt>
	// 						<Dd>{formatCpuUsage(data.cpu_load_average_10s)}</Dd>
	// 						<Dt>CPU Usage Seconds Total</Dt>
	// 						<Dd>
	// 							{formatNumber(data.cpu_usage_seconds_total)}
	// 						</Dd>
	// 						<Dt>CPU User Seconds Total</Dt>
	// 						<Dd>{formatNumber(data.cpu_user_seconds_total)}</Dd>
	// 						<Dt>CPU System Seconds Total</Dt>
	// 						<Dd>
	// 							{formatNumber(data.cpu_system_seconds_total)}
	// 						</Dd>
	// 						<Dt>CPU Schedstat Run Periods</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.cpu_schedstat_run_periods_total,
	// 							)}
	// 						</Dd>
	// 						<Dt>CPU Schedstat Run Seconds</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.cpu_schedstat_run_seconds_total,
	// 							)}
	// 						</Dd>
	// 						<Dt>CPU Schedstat Runqueue Seconds</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.cpu_schedstat_runqueue_seconds_total,
	// 							)}
	// 						</Dd>
	// 					</Dl>
	// 				</div>

	// 				{/* Memory */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">Memory</h4>
	// 					<Dl>
	// 						<Dt>Memory Usage</Dt>
	// 						<Dd>{formatBytes(data.memory_usage_bytes)}</Dd>
	// 						<Dt>Memory Working Set</Dt>
	// 						<Dd>
	// 							{formatBytes(data.memory_working_set_bytes)}
	// 						</Dd>
	// 						<Dt>Memory RSS</Dt>
	// 						<Dd>{formatBytes(data.memory_rss)}</Dd>
	// 						<Dt>Memory Cache</Dt>
	// 						<Dd>{formatBytes(data.memory_cache)}</Dd>
	// 						<Dt>Memory Swap</Dt>
	// 						<Dd>{formatBytes(data.memory_swap)}</Dd>
	// 						<Dt>Memory Max Usage</Dt>
	// 						<Dd>{formatBytes(data.memory_max_usage_bytes)}</Dd>
	// 						<Dt>Memory Mapped File</Dt>
	// 						<Dd>{formatBytes(data.memory_mapped_file)}</Dd>
	// 						<Dt>Memory Failcnt</Dt>
	// 						<Dd>{formatNumber(data.memory_failcnt)}</Dd>
	// 					</Dl>
	// 				</div>

	// 				{/* Memory Failures */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">Memory Failures</h4>
	// 					<Dl>
	// 						<Dt>Page Fault (Container)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.memory_failures_pgfault_container,
	// 							)}
	// 						</Dd>
	// 						<Dt>Page Fault (Hierarchy)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.memory_failures_pgfault_hierarchy,
	// 							)}
	// 						</Dd>
	// 						<Dt>Major Page Fault (Container)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.memory_failures_pgmajfault_container,
	// 							)}
	// 						</Dd>
	// 						<Dt>Major Page Fault (Hierarchy)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.memory_failures_pgmajfault_hierarchy,
	// 							)}
	// 						</Dd>
	// 					</Dl>
	// 				</div>

	// 				{/* Resource Limits */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">Resource Limits</h4>
	// 					<Dl>
	// 						<Dt>Memory Limit</Dt>
	// 						<Dd>
	// 							{resources?.memory
	// 								? `${resources.memory} MB`
	// 								: "n/a"}
	// 						</Dd>
	// 						<Dt>CPU Limit</Dt>
	// 						<Dd>
	// 							{resources?.cpu
	// 								? `${resources.cpu / 1000} cores`
	// 								: "n/a"}
	// 						</Dd>
	// 					</Dl>
	// 				</div>

	// 				{/* Processes & Threads */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">
	// 						Processes & Threads
	// 					</h4>
	// 					<Dl>
	// 						<Dt>Processes</Dt>
	// 						<Dd>{formatNumber(data.processes)}</Dd>
	// 						<Dt>Threads</Dt>
	// 						<Dd>{formatNumber(data.threads)}</Dd>
	// 						<Dt>Max Threads</Dt>
	// 						<Dd>{formatNumber(data.threads_max)}</Dd>
	// 						<Dt>Tasks Running</Dt>
	// 						<Dd>{formatNumber(data.tasks_state_running)}</Dd>
	// 						<Dt>Tasks Sleeping</Dt>
	// 						<Dd>{formatNumber(data.tasks_state_sleeping)}</Dd>
	// 						<Dt>Tasks Stopped</Dt>
	// 						<Dd>{formatNumber(data.tasks_state_stopped)}</Dd>
	// 						<Dt>Tasks IO Waiting</Dt>
	// 						<Dd>{formatNumber(data.tasks_state_iowaiting)}</Dd>
	// 						<Dt>Tasks Uninterruptible</Dt>
	// 						<Dd>
	// 							{formatNumber(data.tasks_state_uninterruptible)}
	// 						</Dd>
	// 					</Dl>
	// 				</div>

	// 				{/* Filesystem */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">Filesystem</h4>
	// 					<Dl>
	// 						<Dt>Reads Bytes Total (sda)</Dt>
	// 						<Dd>
	// 							{formatBytes(data.fs_reads_bytes_total_sda)}
	// 						</Dd>
	// 						<Dt>Writes Bytes Total (sda)</Dt>
	// 						<Dd>
	// 							{formatBytes(data.fs_writes_bytes_total_sda)}
	// 						</Dd>
	// 					</Dl>
	// 				</div>

	// 				{/* Network - Receive */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">Network - Receive</h4>
	// 					<Dl>
	// 						<Dt>Bytes Total (eth0)</Dt>
	// 						<Dd>
	// 							{formatBytes(
	// 								data.network_receive_bytes_total_eth0,
	// 							)}
	// 						</Dd>
	// 						<Dt>Bytes Total (eth1)</Dt>
	// 						<Dd>
	// 							{formatBytes(
	// 								data.network_receive_bytes_total_eth1,
	// 							)}
	// 						</Dd>
	// 						<Dt>Errors Total (eth0)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_receive_errors_total_eth0,
	// 							)}
	// 						</Dd>
	// 						<Dt>Errors Total (eth1)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_receive_errors_total_eth1,
	// 							)}
	// 						</Dd>
	// 						<Dt>Packets Dropped (eth0)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_receive_packets_dropped_total_eth0,
	// 							)}
	// 						</Dd>
	// 						<Dt>Packets Dropped (eth1)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_receive_packets_dropped_total_eth1,
	// 							)}
	// 						</Dd>
	// 						<Dt>Packets Total (eth0)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_receive_packets_total_eth0,
	// 							)}
	// 						</Dd>
	// 						<Dt>Packets Total (eth1)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_receive_packets_total_eth1,
	// 							)}
	// 						</Dd>
	// 					</Dl>
	// 				</div>

	// 				{/* Network - Transmit */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">Network - Transmit</h4>
	// 					<Dl>
	// 						<Dt>Bytes Total (eth0)</Dt>
	// 						<Dd>
	// 							{formatBytes(
	// 								data.network_transmit_bytes_total_eth0,
	// 							)}
	// 						</Dd>
	// 						<Dt>Bytes Total (eth1)</Dt>
	// 						<Dd>
	// 							{formatBytes(
	// 								data.network_transmit_bytes_total_eth1,
	// 							)}
	// 						</Dd>
	// 						<Dt>Errors Total (eth0)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_transmit_errors_total_eth0,
	// 							)}
	// 						</Dd>
	// 						<Dt>Errors Total (eth1)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_transmit_errors_total_eth1,
	// 							)}
	// 						</Dd>
	// 						<Dt>Packets Dropped (eth0)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_transmit_packets_dropped_total_eth0,
	// 							)}
	// 						</Dd>
	// 						<Dt>Packets Dropped (eth1)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_transmit_packets_dropped_total_eth1,
	// 							)}
	// 						</Dd>
	// 						<Dt>Packets Total (eth0)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_transmit_packets_total_eth0,
	// 							)}
	// 						</Dd>
	// 						<Dt>Packets Total (eth1)</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_transmit_packets_total_eth1,
	// 							)}
	// 						</Dd>
	// 					</Dl>
	// 				</div>

	// 				{/* TCP Connections */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">TCP Connections</h4>
	// 					<Dl>
	// 						<Dt>Close</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp_usage_close)}
	// 						</Dd>
	// 						<Dt>Close Wait</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp_usage_closewait)}
	// 						</Dd>
	// 						<Dt>Closing</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp_usage_closing)}
	// 						</Dd>
	// 						<Dt>Established</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_tcp_usage_established,
	// 							)}
	// 						</Dd>
	// 						<Dt>Fin Wait 1</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp_usage_finwait1)}
	// 						</Dd>
	// 						<Dt>Fin Wait 2</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp_usage_finwait2)}
	// 						</Dd>
	// 						<Dt>Last Ack</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp_usage_lastack)}
	// 						</Dd>
	// 						<Dt>Listen</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp_usage_listen)}
	// 						</Dd>
	// 						<Dt>Syn Recv</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp_usage_synrecv)}
	// 						</Dd>
	// 						<Dt>Syn Sent</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp_usage_synsent)}
	// 						</Dd>
	// 						<Dt>Time Wait</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp_usage_timewait)}
	// 						</Dd>
	// 					</Dl>
	// 				</div>

	// 				{/* TCP6 Connections */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">TCP6 Connections</h4>
	// 					<Dl>
	// 						<Dt>Close</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp6_usage_close)}
	// 						</Dd>
	// 						<Dt>Close Wait</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_tcp6_usage_closewait,
	// 							)}
	// 						</Dd>
	// 						<Dt>Closing</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp6_usage_closing)}
	// 						</Dd>
	// 						<Dt>Established</Dt>
	// 						<Dd>
	// 							{formatNumber(
	// 								data.network_tcp6_usage_established,
	// 							)}
	// 						</Dd>
	// 						<Dt>Fin Wait 1</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp6_usage_finwait1)}
	// 						</Dd>
	// 						<Dt>Fin Wait 2</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp6_usage_finwait2)}
	// 						</Dd>
	// 						<Dt>Last Ack</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp6_usage_lastack)}
	// 						</Dd>
	// 						<Dt>Listen</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp6_usage_listen)}
	// 						</Dd>
	// 						<Dt>Syn Recv</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp6_usage_synrecv)}
	// 						</Dd>
	// 						<Dt>Syn Sent</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp6_usage_synsent)}
	// 						</Dd>
	// 						<Dt>Time Wait</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_tcp6_usage_timewait)}
	// 						</Dd>
	// 					</Dl>
	// 				</div>

	// 				{/* UDP Connections */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">UDP Connections</h4>
	// 					<Dl>
	// 						<Dt>Dropped</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_udp_usage_dropped)}
	// 						</Dd>
	// 						<Dt>Listen</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_udp_usage_listen)}
	// 						</Dd>
	// 						<Dt>RX Queued</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_udp_usage_rxqueued)}
	// 						</Dd>
	// 						<Dt>TX Queued</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_udp_usage_txqueued)}
	// 						</Dd>
	// 					</Dl>
	// 				</div>

	// 				{/* UDP6 Connections */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">UDP6 Connections</h4>
	// 					<Dl>
	// 						<Dt>Dropped</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_udp6_usage_dropped)}
	// 						</Dd>
	// 						<Dt>Listen</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_udp6_usage_listen)}
	// 						</Dd>
	// 						<Dt>RX Queued</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_udp6_usage_rxqueued)}
	// 						</Dd>
	// 						<Dt>TX Queued</Dt>
	// 						<Dd>
	// 							{formatNumber(data.network_udp6_usage_txqueued)}
	// 						</Dd>
	// 					</Dl>
	// 				</div>

	// 				{/* System */}
	// 				<div>
	// 					<h4 className="font-medium mb-2">System</h4>
	// 					<Dl>
	// 						<Dt>File Descriptors</Dt>
	// 						<Dd>{formatNumber(data.file_descriptors)}</Dd>
	// 						<Dt>Sockets</Dt>
	// 						<Dd>{formatNumber(data.sockets)}</Dd>
	// 						<Dt>Last Seen</Dt>
	// 						<Dd>{formatTimestamp(data.last_seen)}</Dd>
	// 						<Dt>Start Time</Dt>
	// 						<Dd>{formatTimestamp(data.start_time_seconds)}</Dd>
	// 					</Dl>
	// 				</div>
	// 			</Flex>
	// 		)}
	// 	</div>
	// );
	return null;
}
