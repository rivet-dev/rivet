import {
	faBoxArchive,
	faCube,
	faHardDrive,
	faNetworkWired,
	Icon,
} from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import {
	ScrollArea,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components";
import type { ActorId } from "../queries";
import { AgentOsEmpty, formatBytes, SectionHeader, StatusDot } from "./common";
import type { MountInfo, MountKind } from "./types";
import { useAgentOsInspector } from "./use-agent-os-inspector";

const MOUNT_ICON: Record<MountKind, typeof faHardDrive> = {
	persistent: faHardDrive,
	s3: faBoxArchive,
	sandbox: faCube,
	gdrive: faNetworkWired,
};

export function MountsTab({ mounts }: { mounts: MountInfo[] }) {
	return (
		<div className="flex h-full flex-col">
			<SectionHeader
				title="Mounted backends"
				description="Storage and sandbox backends mounted into the VM filesystem."
			/>
			{mounts.length === 0 ? (
				<AgentOsEmpty>No mounts configured.</AgentOsEmpty>
			) : (
				<ScrollArea className="min-h-0 flex-1">
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>Mount</TableHead>
								<TableHead>Kind</TableHead>
								<TableHead>Provider</TableHead>
								<TableHead className="text-right">
									Size
								</TableHead>
								<TableHead>Status</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{mounts.map((mount) => (
								<TableRow key={mount.path}>
									<TableCell className="py-2 font-mono">
										{mount.path}
									</TableCell>
									<TableCell className="py-2">
										<span className="inline-flex items-center gap-1.5 text-muted-foreground">
											<Icon
												icon={MOUNT_ICON[mount.kind]}
												className="size-3"
											/>
											{mount.kind}
										</span>
									</TableCell>
									<TableCell className="py-2 font-mono text-xs text-muted-foreground">
										{mount.provider}
									</TableCell>
									<TableCell className="py-2 text-right tabular-nums text-muted-foreground">
										{formatBytes(mount.sizeBytes)}
									</TableCell>
									<TableCell className="py-2">
										<span className="inline-flex items-center gap-1.5 text-xs">
											<StatusDot
												color={
													mount.status === "online"
														? "green"
														: "amber"
												}
											/>
											{mount.status}
										</span>
									</TableCell>
								</TableRow>
							))}
						</TableBody>
					</Table>
				</ScrollArea>
			)}
		</div>
	);
}

export function MountsTabConnected({ actorId }: { actorId: ActorId }) {
	const inspector = useAgentOsInspector();
	const { data = [] } = useQuery(inspector.mountsQueryOptions(actorId));
	return <MountsTab mounts={data} />;
}
