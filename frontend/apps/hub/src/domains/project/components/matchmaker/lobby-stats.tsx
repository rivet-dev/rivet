import type { Rivet } from "@rivet-gg/api";
import { Flex, Progress, ScrollArea, Strong } from "@rivet-gg/components";
import { filesize } from "filesize";
import { LobbyCPUStats } from "./lobby-cpu-stats";
import { LobbyMemoryStats } from "./lobby-memory-stats";

interface LobbyStatsProps extends Rivet.cloud.SvcMetrics {
	lobbyId: string;
	metricsAt: number;
}

export function LobbyStats({
	cpu,
	memory,
	metricsAt,
	allocatedMemory,
	lobbyId,
}: LobbyStatsProps) {
	const syncId = `stats-${lobbyId}`;

	const memoryPercentage =
		(memory[memory.length - 1] / (allocatedMemory || 1)) * 100;

	const cpuPercentage = cpu[cpu.length - 1];

	return (
		<ScrollArea className=" h-full w-full overflow-auto  p-4">
			<div className="grid grid-cols-1 gap-4 @3xl:grid-cols-2 gap-y-8 ">
				<div>
					<p className="mb-4 font-bold ml-5">Memory Usage</p>
					<LobbyMemoryStats
						memory={memory}
						metricsAt={metricsAt}
						allocatedMemory={allocatedMemory}
						syncId={syncId}
					/>
					<Flex
						items="center"
						justify="center"
						py="4"
						gap="4"
						className="ml-16"
					>
						<Progress value={memoryPercentage} className="h-2" />
						<Flex direction="col" gap="2">
							<Strong className="tabular-nums">
								{memoryPercentage.toFixed(2)}%
							</Strong>
						</Flex>
					</Flex>
					<p className="ml-16">
						<Strong className="tabular-nums">
							{filesize(memory[memory.length - 1])}
						</Strong>{" "}
						/{" "}
						<span className="tabular-nums">
							{filesize(allocatedMemory || 1)}
						</span>{" "}
						Total
					</p>
				</div>
				<div>
					<p className="mb-4 font-bold ml-5">CPU Usage</p>
					<LobbyCPUStats
						cpu={cpu}
						metricsAt={metricsAt}
						syncId={syncId}
					/>

					<Flex
						items="center"
						justify="center"
						py="4"
						gap="4"
						className="ml-16"
					>
						<Progress value={cpuPercentage} className="h-2" />
						<Flex direction="col" gap="2">
							<Strong className="tabular-nums">
								{cpuPercentage.toFixed(2)}%
							</Strong>
						</Flex>
					</Flex>
				</div>
			</div>
		</ScrollArea>
	);
}
