import { useAtomValue } from "jotai";
import { selectAtom } from "jotai/utils";
import equal from "fast-deep-equal";
import { useState, useEffect } from "react";
import { Dd, Dl, Dt, Flex, Button } from "@rivet-gg/components";
import type { Actor, ActorAtom } from "./actor-context";

const selector = (a: Actor) => ({
	metrics: a.metrics,
	status: a.status,
});

export interface ActorMetricsProps {
	actor: ActorAtom;
}

export function ActorMetrics({ actor }: ActorMetricsProps) {
	const { metrics, status } = useAtomValue(selectAtom(actor, selector, equal));
	const metricsData = useAtomValue(metrics);
	const [showAdvanced, setShowAdvanced] = useState(false);
	const [cpuPercentage, setCpuPercentage] = useState<string>("n/a");

	const isActorRunning = status === "running";

	const formatBytes = (bytes: number | null | undefined) => {
		if (!isActorRunning || bytes === null || bytes === undefined) return "n/a";
		const mb = bytes / 1024 / 1024;
		if (mb < 1024) {
			return `${mb.toFixed(1)} MB`;
		} else {
			return `${(mb / 1024).toFixed(1)} GB`;
		}
	};

	const formatCpuUsage = (cpu: number | null | undefined) => {
		if (!isActorRunning || cpu === null || cpu === undefined) return "n/a";
		return `${(cpu * 100).toFixed(2)}%`;
	};

	const formatNumber = (value: number | null | undefined) => {
		if (!isActorRunning || value === null || value === undefined) return "n/a";
		return value.toLocaleString();
	};

	const formatTimestamp = (timestamp: number | null | undefined) => {
		if (!isActorRunning || timestamp === null || timestamp === undefined) return "n/a";
		return new Date(timestamp * 1000).toLocaleString();
	};

	// Calculate CPU percentage using time series data points
	useEffect(() => {
		if (!isActorRunning) {
			setCpuPercentage("n/a");
			return;
		}

		const data = metricsData;
		if (!data || !data.rawData || !data.interval) {
			setCpuPercentage("n/a");
			return;
		}

		const cpuValues = data.rawData.cpu_usage_seconds_total;
		if (!cpuValues || cpuValues.length < 2) {
			setCpuPercentage("n/a");
			return;
		}

		// Find two non-zero consecutive data points to calculate rate
		let cpuRate = 0;
		for (let i = cpuValues.length - 1; i > 0; i--) {
			const currentCpu = cpuValues[i];
			const previousCpu = cpuValues[i - 1];
			
			if (currentCpu !== 0 && previousCpu !== 0 && currentCpu >= previousCpu) {
				const cpuDelta = currentCpu - previousCpu;
				const timeDelta = data.interval / 1000; // Convert ms to seconds
				
				// Rate calculation: CPU seconds used per second of real time
				// This gives the fraction of available CPU used (0-1)
				cpuRate = (cpuDelta / timeDelta) * 100;
				break;
			}
		}

		setCpuPercentage(`${Math.min(cpuRate, 100).toFixed(2)}%`);
	}, [metricsData, isActorRunning]);

	const calculateMemoryPercentage = (usage: number | null | undefined, limit: number | null | undefined) => {
		if (!isActorRunning || usage === null || usage === undefined || limit === null || limit === undefined || limit === 0) {
			return null;
		}
		return (usage / limit) * 100;
	};

	const isLoading = metricsData.status === "pending";
	const hasError = metricsData.status === "error";
	const data = metricsData.metrics || {};

	if (isLoading) {
		return (
			<div className="px-4 my-8">
				<h3 className="mb-2 font-semibold">Metrics</h3>
				<div className="text-xs text-muted-foreground">Loading...</div>
			</div>
		);
	}

	if (hasError) {
		return (
			<div className="px-4 my-8">
				<h3 className="mb-2 font-semibold">Metrics</h3>
				<div className="text-xs text-destructive">Error loading metrics</div>
			</div>
		);
	}

	const memoryPercentage = calculateMemoryPercentage(data.memory_usage_bytes, data.spec_memory_limit_bytes);

	return (
		<div className="px-4 my-8">
			<h3 className="mb-4 font-semibold">Container Metrics</h3>
			
			{/* Main Metrics */}
			<div className="mb-6">
				<Dl className="grid grid-cols-2 gap-4">
					<div>
						<Dt className="text-sm font-medium">CPU Usage</Dt>
						<Dd className="text-lg font-semibold">
							{cpuPercentage}
						</Dd>
					</div>
					<div>
						<Dt className="text-sm font-medium">Memory Usage</Dt>
						<Dd className="text-lg font-semibold">
							{formatBytes(data.memory_usage_bytes)}
							{memoryPercentage !== null && (
								<span className="text-sm text-muted-foreground ml-1">
									({memoryPercentage.toFixed(1)}%)
								</span>
							)}
						</Dd>
					</div>
				</Dl>
			</div>

			{/* Advanced Section Toggle */}
			<div className="mb-4">
				<Button
					variant="outline"
					size="sm"
					onClick={() => setShowAdvanced(!showAdvanced)}
					className="text-xs"
				>
					{showAdvanced ? "Hide" : "Show"} Advanced (Experimental)
				</Button>
			</div>

			{/* Advanced Metrics */}
			{showAdvanced && (
				<Flex gap="4" direction="col" className="text-xs">
					{/* CPU & Performance */}
					<div>
						<h4 className="font-medium mb-2">CPU & Performance</h4>
						<Dl>
							<Dt>CPU Load Average (10s)</Dt>
							<Dd>{formatCpuUsage(data.cpu_load_average_10s)}</Dd>
							<Dt>CPU Usage Seconds Total</Dt>
							<Dd>{formatNumber(data.cpu_usage_seconds_total)}</Dd>
							<Dt>CPU User Seconds Total</Dt>
							<Dd>{formatNumber(data.cpu_user_seconds_total)}</Dd>
							<Dt>CPU System Seconds Total</Dt>
							<Dd>{formatNumber(data.cpu_system_seconds_total)}</Dd>
							<Dt>CPU Schedstat Run Periods</Dt>
							<Dd>{formatNumber(data.cpu_schedstat_run_periods_total)}</Dd>
							<Dt>CPU Schedstat Run Seconds</Dt>
							<Dd>{formatNumber(data.cpu_schedstat_run_seconds_total)}</Dd>
							<Dt>CPU Schedstat Runqueue Seconds</Dt>
							<Dd>{formatNumber(data.cpu_schedstat_runqueue_seconds_total)}</Dd>
						</Dl>
					</div>

					{/* Memory */}
					<div>
						<h4 className="font-medium mb-2">Memory</h4>
						<Dl>
							<Dt>Memory Usage</Dt>
							<Dd>{formatBytes(data.memory_usage_bytes)}</Dd>
							<Dt>Memory Working Set</Dt>
							<Dd>{formatBytes(data.memory_working_set_bytes)}</Dd>
							<Dt>Memory RSS</Dt>
							<Dd>{formatBytes(data.memory_rss)}</Dd>
							<Dt>Memory Cache</Dt>
							<Dd>{formatBytes(data.memory_cache)}</Dd>
							<Dt>Memory Swap</Dt>
							<Dd>{formatBytes(data.memory_swap)}</Dd>
							<Dt>Memory Max Usage</Dt>
							<Dd>{formatBytes(data.memory_max_usage_bytes)}</Dd>
							<Dt>Memory Mapped File</Dt>
							<Dd>{formatBytes(data.memory_mapped_file)}</Dd>
							<Dt>Memory Failcnt</Dt>
							<Dd>{formatNumber(data.memory_failcnt)}</Dd>
						</Dl>
					</div>

					{/* Memory Failures */}
					<div>
						<h4 className="font-medium mb-2">Memory Failures</h4>
						<Dl>
							<Dt>Page Fault (Container)</Dt>
							<Dd>{formatNumber(data.memory_failures_pgfault_container)}</Dd>
							<Dt>Page Fault (Hierarchy)</Dt>
							<Dd>{formatNumber(data.memory_failures_pgfault_hierarchy)}</Dd>
							<Dt>Major Page Fault (Container)</Dt>
							<Dd>{formatNumber(data.memory_failures_pgmajfault_container)}</Dd>
							<Dt>Major Page Fault (Hierarchy)</Dt>
							<Dd>{formatNumber(data.memory_failures_pgmajfault_hierarchy)}</Dd>
						</Dl>
					</div>

					{/* Memory Limits */}
					<div>
						<h4 className="font-medium mb-2">Memory Limits</h4>
						<Dl>
							<Dt>Memory Limit</Dt>
							<Dd>{formatBytes(data.spec_memory_limit_bytes)}</Dd>
							<Dt>Memory Reservation Limit</Dt>
							<Dd>{formatBytes(data.spec_memory_reservation_limit_bytes)}</Dd>
							<Dt>Memory Swap Limit</Dt>
							<Dd>{formatBytes(data.spec_memory_swap_limit_bytes)}</Dd>
						</Dl>
					</div>

					{/* Processes & Threads */}
					<div>
						<h4 className="font-medium mb-2">Processes & Threads</h4>
						<Dl>
							<Dt>Processes</Dt>
							<Dd>{formatNumber(data.processes)}</Dd>
							<Dt>Threads</Dt>
							<Dd>{formatNumber(data.threads)}</Dd>
							<Dt>Max Threads</Dt>
							<Dd>{formatNumber(data.threads_max)}</Dd>
							<Dt>Tasks Running</Dt>
							<Dd>{formatNumber(data.tasks_state_running)}</Dd>
							<Dt>Tasks Sleeping</Dt>
							<Dd>{formatNumber(data.tasks_state_sleeping)}</Dd>
							<Dt>Tasks Stopped</Dt>
							<Dd>{formatNumber(data.tasks_state_stopped)}</Dd>
							<Dt>Tasks IO Waiting</Dt>
							<Dd>{formatNumber(data.tasks_state_iowaiting)}</Dd>
							<Dt>Tasks Uninterruptible</Dt>
							<Dd>{formatNumber(data.tasks_state_uninterruptible)}</Dd>
						</Dl>
					</div>

					{/* Filesystem */}
					<div>
						<h4 className="font-medium mb-2">Filesystem</h4>
						<Dl>
							<Dt>Reads Bytes Total (sda)</Dt>
							<Dd>{formatBytes(data.fs_reads_bytes_total_sda)}</Dd>
							<Dt>Writes Bytes Total (sda)</Dt>
							<Dd>{formatBytes(data.fs_writes_bytes_total_sda)}</Dd>
						</Dl>
					</div>

					{/* Network - Receive */}
					<div>
						<h4 className="font-medium mb-2">Network - Receive</h4>
						<Dl>
							<Dt>Bytes Total (eth0)</Dt>
							<Dd>{formatBytes(data.network_receive_bytes_total_eth0)}</Dd>
							<Dt>Bytes Total (eth1)</Dt>
							<Dd>{formatBytes(data.network_receive_bytes_total_eth1)}</Dd>
							<Dt>Errors Total (eth0)</Dt>
							<Dd>{formatNumber(data.network_receive_errors_total_eth0)}</Dd>
							<Dt>Errors Total (eth1)</Dt>
							<Dd>{formatNumber(data.network_receive_errors_total_eth1)}</Dd>
							<Dt>Packets Dropped (eth0)</Dt>
							<Dd>{formatNumber(data.network_receive_packets_dropped_total_eth0)}</Dd>
							<Dt>Packets Dropped (eth1)</Dt>
							<Dd>{formatNumber(data.network_receive_packets_dropped_total_eth1)}</Dd>
							<Dt>Packets Total (eth0)</Dt>
							<Dd>{formatNumber(data.network_receive_packets_total_eth0)}</Dd>
							<Dt>Packets Total (eth1)</Dt>
							<Dd>{formatNumber(data.network_receive_packets_total_eth1)}</Dd>
						</Dl>
					</div>

					{/* Network - Transmit */}
					<div>
						<h4 className="font-medium mb-2">Network - Transmit</h4>
						<Dl>
							<Dt>Bytes Total (eth0)</Dt>
							<Dd>{formatBytes(data.network_transmit_bytes_total_eth0)}</Dd>
							<Dt>Bytes Total (eth1)</Dt>
							<Dd>{formatBytes(data.network_transmit_bytes_total_eth1)}</Dd>
							<Dt>Errors Total (eth0)</Dt>
							<Dd>{formatNumber(data.network_transmit_errors_total_eth0)}</Dd>
							<Dt>Errors Total (eth1)</Dt>
							<Dd>{formatNumber(data.network_transmit_errors_total_eth1)}</Dd>
							<Dt>Packets Dropped (eth0)</Dt>
							<Dd>{formatNumber(data.network_transmit_packets_dropped_total_eth0)}</Dd>
							<Dt>Packets Dropped (eth1)</Dt>
							<Dd>{formatNumber(data.network_transmit_packets_dropped_total_eth1)}</Dd>
							<Dt>Packets Total (eth0)</Dt>
							<Dd>{formatNumber(data.network_transmit_packets_total_eth0)}</Dd>
							<Dt>Packets Total (eth1)</Dt>
							<Dd>{formatNumber(data.network_transmit_packets_total_eth1)}</Dd>
						</Dl>
					</div>

					{/* TCP Connections */}
					<div>
						<h4 className="font-medium mb-2">TCP Connections</h4>
						<Dl>
							<Dt>Close</Dt>
							<Dd>{formatNumber(data.network_tcp_usage_close)}</Dd>
							<Dt>Close Wait</Dt>
							<Dd>{formatNumber(data.network_tcp_usage_closewait)}</Dd>
							<Dt>Closing</Dt>
							<Dd>{formatNumber(data.network_tcp_usage_closing)}</Dd>
							<Dt>Established</Dt>
							<Dd>{formatNumber(data.network_tcp_usage_established)}</Dd>
							<Dt>Fin Wait 1</Dt>
							<Dd>{formatNumber(data.network_tcp_usage_finwait1)}</Dd>
							<Dt>Fin Wait 2</Dt>
							<Dd>{formatNumber(data.network_tcp_usage_finwait2)}</Dd>
							<Dt>Last Ack</Dt>
							<Dd>{formatNumber(data.network_tcp_usage_lastack)}</Dd>
							<Dt>Listen</Dt>
							<Dd>{formatNumber(data.network_tcp_usage_listen)}</Dd>
							<Dt>Syn Recv</Dt>
							<Dd>{formatNumber(data.network_tcp_usage_synrecv)}</Dd>
							<Dt>Syn Sent</Dt>
							<Dd>{formatNumber(data.network_tcp_usage_synsent)}</Dd>
							<Dt>Time Wait</Dt>
							<Dd>{formatNumber(data.network_tcp_usage_timewait)}</Dd>
						</Dl>
					</div>

					{/* TCP6 Connections */}
					<div>
						<h4 className="font-medium mb-2">TCP6 Connections</h4>
						<Dl>
							<Dt>Close</Dt>
							<Dd>{formatNumber(data.network_tcp6_usage_close)}</Dd>
							<Dt>Close Wait</Dt>
							<Dd>{formatNumber(data.network_tcp6_usage_closewait)}</Dd>
							<Dt>Closing</Dt>
							<Dd>{formatNumber(data.network_tcp6_usage_closing)}</Dd>
							<Dt>Established</Dt>
							<Dd>{formatNumber(data.network_tcp6_usage_established)}</Dd>
							<Dt>Fin Wait 1</Dt>
							<Dd>{formatNumber(data.network_tcp6_usage_finwait1)}</Dd>
							<Dt>Fin Wait 2</Dt>
							<Dd>{formatNumber(data.network_tcp6_usage_finwait2)}</Dd>
							<Dt>Last Ack</Dt>
							<Dd>{formatNumber(data.network_tcp6_usage_lastack)}</Dd>
							<Dt>Listen</Dt>
							<Dd>{formatNumber(data.network_tcp6_usage_listen)}</Dd>
							<Dt>Syn Recv</Dt>
							<Dd>{formatNumber(data.network_tcp6_usage_synrecv)}</Dd>
							<Dt>Syn Sent</Dt>
							<Dd>{formatNumber(data.network_tcp6_usage_synsent)}</Dd>
							<Dt>Time Wait</Dt>
							<Dd>{formatNumber(data.network_tcp6_usage_timewait)}</Dd>
						</Dl>
					</div>

					{/* UDP Connections */}
					<div>
						<h4 className="font-medium mb-2">UDP Connections</h4>
						<Dl>
							<Dt>Dropped</Dt>
							<Dd>{formatNumber(data.network_udp_usage_dropped)}</Dd>
							<Dt>Listen</Dt>
							<Dd>{formatNumber(data.network_udp_usage_listen)}</Dd>
							<Dt>RX Queued</Dt>
							<Dd>{formatNumber(data.network_udp_usage_rxqueued)}</Dd>
							<Dt>TX Queued</Dt>
							<Dd>{formatNumber(data.network_udp_usage_txqueued)}</Dd>
						</Dl>
					</div>

					{/* UDP6 Connections */}
					<div>
						<h4 className="font-medium mb-2">UDP6 Connections</h4>
						<Dl>
							<Dt>Dropped</Dt>
							<Dd>{formatNumber(data.network_udp6_usage_dropped)}</Dd>
							<Dt>Listen</Dt>
							<Dd>{formatNumber(data.network_udp6_usage_listen)}</Dd>
							<Dt>RX Queued</Dt>
							<Dd>{formatNumber(data.network_udp6_usage_rxqueued)}</Dd>
							<Dt>TX Queued</Dt>
							<Dd>{formatNumber(data.network_udp6_usage_txqueued)}</Dd>
						</Dl>
					</div>

					{/* System */}
					<div>
						<h4 className="font-medium mb-2">System</h4>
						<Dl>
							<Dt>File Descriptors</Dt>
							<Dd>{formatNumber(data.file_descriptors)}</Dd>
							<Dt>Sockets</Dt>
							<Dd>{formatNumber(data.sockets)}</Dd>
							<Dt>Last Seen</Dt>
							<Dd>{formatTimestamp(data.last_seen)}</Dd>
							<Dt>Start Time</Dt>
							<Dd>{formatTimestamp(data.start_time_seconds)}</Dd>
							<Dt>CPU Shares</Dt>
							<Dd>{formatNumber(data.spec_cpu_shares)}</Dd>
							<Dt>CPU Period</Dt>
							<Dd>{formatNumber(data.spec_cpu_period)}</Dd>
						</Dl>
					</div>
				</Flex>
			)}
		</div>
	);
}
