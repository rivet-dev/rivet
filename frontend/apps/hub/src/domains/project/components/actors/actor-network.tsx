import { Rivet } from "@rivet-gg/api";
import {
	Button,
	Dd,
	DiscreteCopyButton,
	Dl,
	DocsSheet,
	Dt,
	Flex,
} from "@rivet-gg/components";
import { Icon, faBooks } from "@rivet-gg/icons";
import { ActorObjectInspector } from "./console/actor-inspector";
import { Fragment } from "react";

export interface ActorNetworkProps
	extends Pick<Rivet.actors.Actor, "network"> {}

const NETWORK_MODE_LABELS: Record<Rivet.actors.NetworkMode, string> = {
	bridge: "Bridge",
	host: "Host",
};

const WEB_ACCESIBLE_PROTOCOLS: Rivet.actors.PortProtocol[] = [
	Rivet.actors.PortProtocol.Http,
	Rivet.actors.PortProtocol.Https,
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
				<Flex gap="2" direction="col" className="text-xs">
					<Dl className="items-start">
						<Dt>Ports</Dt>
						<Dd>
							{Object.entries(network.ports).map(
								([name, port]) => (
									<Fragment key={name}>
										{name}{" "}
										<Dl className="mb-2 mt-2">
											<Dt>Port</Dt>
											<Dd>
												<DiscreteCopyButton
													size="xs"
													value={String(
														port.port || "",
													)}
												>
													{port.port}
												</DiscreteCopyButton>
											</Dd>
											<Dt>Hostname</Dt>
											<Dd>
												<DiscreteCopyButton
													size="xs"
													className="max-w-full min-w-0"
													value={port.hostname || ""}
												>
													{port.hostname}
												</DiscreteCopyButton>
											</Dd>
											{port.url ? (
												<>
													{" "}
													<Dt>URL</Dt>
													<Dd>
														<DiscreteCopyButton
															size="xs"
															className="max-w-full min-w-0"
															value={
																port.url || ""
															}
														>
															{port.url}
														</DiscreteCopyButton>
													</Dd>
												</>
											) : null}
										</Dl>
										<ActorObjectInspector data={port} />
									</Fragment>
								),
							)}
						</Dd>
					</Dl>
				</Flex>
			</div>
		</div>
	);
}
