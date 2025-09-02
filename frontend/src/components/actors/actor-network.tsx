import { faBooks, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { Fragment } from "react";
import {
	Button,
	cn,
	Dd,
	DiscreteCopyButton,
	Dl,
	DocsSheet,
	Dt,
	Flex,
} from "@/components";
import { ActorObjectInspector } from "./console/actor-inspector";
import { useManager } from "./manager-context";
import type { ActorId } from "./queries";

export interface ActorNetworkProps {
	actorId: ActorId;
}

export function ActorNetwork({ actorId }: ActorNetworkProps) {
	const { data: ports } = useQuery(
		useManager().actorNetworkPortsQueryOptions(actorId),
	);
	if (!ports) {
		return null;
	}

	return (
		<div className="px-4 mt-8 ">
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
							{Object.entries(ports).map(
								([name, port], index) => (
									<Fragment key={name}>
										<span
											className={cn(
												index !== 0 && "mt-8 block",
											)}
										>
											{name}
										</span>
										<Dl className="mb-2 mt-2 border-l pl-4">
											<Dt>Protocol</Dt>
											<Dd>
												<DiscreteCopyButton
													size="xs"
													value={port.protocol || ""}
												>
													{port.protocol}
												</DiscreteCopyButton>
											</Dd>
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
													className="max-w-full"
													value={port.hostname || ""}
												>
													<span className=" min-w-0 truncate flex-1">
														{port.hostname}
													</span>
												</DiscreteCopyButton>
											</Dd>
											{port.url ? (
												<>
													<Dt>URL</Dt>
													<Dd>
														<DiscreteCopyButton
															size="xs"
															className="max-w-full"
															value={
																port.url || ""
															}
														>
															<span className=" min-w-0 truncate flex-1">
																{port.url}
															</span>
														</DiscreteCopyButton>
													</Dd>
												</>
											) : null}

											{port.routing?.host ? (
												<>
													<Dt>Host Routing</Dt>
													<Dd>
														<DiscreteCopyButton
															size="xs"
															className="max-w-full min-w-0"
															value={JSON.stringify(
																port.routing
																	.host,
															)}
														>
															<ActorObjectInspector
																data={
																	port.routing
																		.host
																}
															/>
														</DiscreteCopyButton>
													</Dd>
												</>
											) : null}
										</Dl>
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
