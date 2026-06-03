import { useQuery } from "@tanstack/react-query";
import {
	Badge,
	DiscreteCopyButton,
	RelativeTime,
	ScrollArea,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components";
import type { ActorId } from "../queries";
import { AgentOsEmpty, SectionHeader } from "./common";
import type { ConnInfo } from "./types";
import { useAgentOsInspector } from "./use-agent-os-inspector";

export function ConnectionsTab({ connections }: { connections: ConnInfo[] }) {
	return (
		<div className="flex h-full flex-col">
			<SectionHeader
				title="Connections"
				description="Clients subscribed to sessionEvent streams on this actor."
			/>
			{connections.length === 0 ? (
				<AgentOsEmpty>No active connections.</AgentOsEmpty>
			) : (
				<ScrollArea className="min-h-0 flex-1">
					<Table>
						<TableHeader>
							<TableRow>
								<TableHead>Conn ID</TableHead>
								<TableHead>Stream</TableHead>
								<TableHead>Connected</TableHead>
								<TableHead>Origin</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{connections.map((conn) => (
								<TableRow key={conn.connId}>
									<TableCell className="py-2 font-mono">
										<DiscreteCopyButton
											size="xs"
											value={conn.connId}
											className="-mx-2 h-auto"
										>
											{conn.connId}
										</DiscreteCopyButton>
									</TableCell>
									<TableCell className="py-2 font-mono text-xs text-muted-foreground">
										{conn.stream}
									</TableCell>
									<TableCell className="py-2 text-muted-foreground">
										<RelativeTime
											time={new Date(conn.connectedAt)}
										/>
									</TableCell>
									<TableCell className="py-2">
										<Badge variant="outline">
											{conn.origin}
										</Badge>
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

export function ConnectionsTabConnected({ actorId }: { actorId: ActorId }) {
	const inspector = useAgentOsInspector();
	const { data = [] } = useQuery(inspector.connectionsQueryOptions(actorId));
	return <ConnectionsTab connections={data} />;
}
