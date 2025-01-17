import type { Rivet } from "@rivet-gg/api";
import { Badge, Dd, Dl, Dt, SmallText } from "@rivet-gg/components";
import { Fragment } from "react";

export interface ActorNetworkProps extends Pick<Rivet.actor.Actor, "network"> {}

const NETWORK_MODE_LABELS: Record<Rivet.actor.NetworkMode, string> = {
	bridge: "Bridge",
	host: "Host",
};

export function ActorNetwork({ network }: ActorNetworkProps) {
	return (
		<>
			<div className="border mt-4 px-4 py-4 rounded-md relative">
				<p className="inline-block bg-card w-auto absolute -top-0 left-3 font-semibold px-0.5 -translate-y-1/2">
					Network{" "}
					<Badge variant="secondary" className="ml-1">
						{NETWORK_MODE_LABELS[network.mode]}
					</Badge>
				</p>
				<div className="text-xs">
					{Object.keys(network.ports || {}).length === 0 ? (
						<SmallText>No ports configured</SmallText>
					) : (
						Object.entries(network.ports).map(([name, config]) => (
							<Fragment key={name}>
								<div className="border p-2 rounded-md mt-4 relative">
									<div className="inline-block bg-card w-auto absolute -top-0 left-3 font-semibold px-0.5 -translate-y-1/2">
										{name}{" "}
										{config.routing.guard ? (
											<Badge
												variant="outline"
												className="ml-1 text-xs"
											>
												Guard
											</Badge>
										) : null}
										{config.routing.host ? (
											<Badge
												variant="outline"
												className="ml-1 text-xs"
											>
												Host
											</Badge>
										) : null}
									</div>
									<div className="border-t border-card mt-3 mb-1">
										<Dl className="ml-2">
											<Dt>Internal port</Dt>
											<Dd>
												{config.internalPort || "-"}
											</Dd>
											<Dt>Protocol</Dt>
											<Dd>{config.protocol}</Dd>
											<Dt>Hostname</Dt>
											<Dd>{config.hostname || "-"}</Dd>
											<Dt>Path</Dt>
											<Dd>{config.path || "-"}</Dd>
											<Dt>Port</Dt>
											<Dd>{config.port || "-"}</Dd>
										</Dl>
									</div>
								</div>
							</Fragment>
						))
					)}
				</div>
			</div>
		</>
	);
}
