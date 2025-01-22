import { Rivet } from "@rivet-gg/api";
import { Button, DocsSheet } from "@rivet-gg/components";
import { Icon, faBooks } from "@rivet-gg/icons";
import { ActorObjectInspector } from "./console/actor-inspector";

export interface ActorNetworkProps extends Pick<Rivet.actor.Actor, "network"> {}

const NETWORK_MODE_LABELS: Record<Rivet.actor.NetworkMode, string> = {
	bridge: "Bridge",
	host: "Host",
};

const WEB_ACCESIBLE_PROTOCOLS: Rivet.actor.PortProtocol[] = [
	Rivet.actor.PortProtocol.Http,
	Rivet.actor.PortProtocol.Https,
];

export function ActorNetwork({ network }: ActorNetworkProps) {
	return (
		<div className="px-4 mt-4 ">
			<div className="flex gap-1 items-center mb-2">
				<h3 className=" font-semibold">Network</h3>
				<DocsSheet title="Networking" path="docs/networking">
					<Button
						variant="outline"
						size="sm"
						startIcon={<Icon icon={faBooks} />}
					>
						Documentation
					</Button>
				</DocsSheet>
			</div>
			<div className="text-xs">
				<ActorObjectInspector
					data={{ mode: network?.mode, ports: network?.ports }}
				/>
			</div>
		</div>
	);
}
