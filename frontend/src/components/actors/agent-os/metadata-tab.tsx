import { useQuery } from "@tanstack/react-query";
import { Dd, DiscreteCopyButton, Dl, Dt, Flex, ScrollArea } from "@/components";
import type { ActorId } from "../queries";
import { AgentOsEmpty, formatUtc } from "./common";
import type { AgentOsMetadata } from "./types";
import { useAgentOsInspector } from "./use-agent-os-inspector";

export function MetadataTab({ metadata }: { metadata: AgentOsMetadata }) {
	return (
		<ScrollArea className="h-full w-full">
			<div className="px-6 py-5">
				<header className="mb-4">
					<h3 className="mb-1 text-sm font-semibold">Metadata</h3>
					<p className="text-xs text-muted-foreground">
						VM-level identity and runtime capabilities.
					</p>
				</header>
				<Flex
					direction="col"
					className="text-xs [&_dl]:items-stretch [&_dt]:py-2 [&_dd]:py-2 [&_dt]:border-b [&_dd]:border-b [&_dt]:border-foreground/[0.06] [&_dd]:border-foreground/[0.06] [&_dt:last-of-type]:border-0 [&_dd:last-of-type]:border-0 [&_dt]:text-muted-foreground [&_dt]:font-normal [&_dd]:text-foreground"
				>
					<Dl>
						<Dt>actor.id</Dt>
						<Dd className="font-mono">
							<DiscreteCopyButton
								size="xs"
								value={metadata.actorId}
								className="-mx-2 h-auto"
							>
								{metadata.actorId}
							</DiscreteCopyButton>
						</Dd>
						<Dt>actor.key</Dt>
						<Dd className="font-mono">"{metadata.actorKey}"</Dd>
						<Dt>actor.name</Dt>
						<Dd>{metadata.actorName}</Dd>
						<Dt>actor.kind</Dt>
						<Dd className="font-mono">{metadata.actorKind}</Dd>
						<Dt>runner</Dt>
						<Dd>{metadata.runner}</Dd>
						<Dt>region</Dt>
						<Dd>{metadata.region}</Dd>
						<Dt>agent</Dt>
						<Dd>{metadata.agentVersion}</Dd>
						<Dt>agentos-core</Dt>
						<Dd>{metadata.agentosCore}</Dd>
						<Dt>software</Dt>
						<Dd>{metadata.software.join(", ")}</Dd>
						<Dt>created</Dt>
						<Dd>{formatUtc(metadata.createdAt)}</Dd>
						<Dt>last sleep</Dt>
						<Dd>{formatUtc(metadata.lastSleepAt)}</Dd>
						<Dt>session count</Dt>
						<Dd>
							{metadata.sessionCount} (
							{metadata.activeSessionCount} active)
						</Dd>
					</Dl>
				</Flex>
			</div>
		</ScrollArea>
	);
}

export function MetadataTabConnected({ actorId }: { actorId: ActorId }) {
	const inspector = useAgentOsInspector();
	const { data } = useQuery(inspector.metadataQueryOptions(actorId));
	if (!data) return <AgentOsEmpty>No metadata available.</AgentOsEmpty>;
	return <MetadataTab metadata={data} />;
}
