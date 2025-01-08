import type { Rivet } from "@rivet-gg/api";
import {
	Badge,
	Code,
	Dd,
	Dl,
	Dt,
	Flex,
	ScrollArea,
	SmallText,
} from "@rivet-gg/components";
import { Fragment } from "react/jsx-runtime";

interface ActorNetworkTabProps extends Pick<Rivet.actor.Actor, "network"> {}

export function ActorNetworkTab({ network }: ActorNetworkTabProps) {
	return (
		<ScrollArea className="overflow-auto h-full px-4 my-2">
			<Flex gap="2" direction="col">
				<Dl>
					<Dt>Mode</Dt>
					<Dd>
						<Code>{network.mode}</Code>
					</Dd>
				</Dl>
				<Dt asChild>
					<p>Ports</p>
				</Dt>
				<Dl className="ml-5" asChild>
					<div>
						{Object.keys(network.ports || {}).length === 0 ? (
							<SmallText>No ports configured</SmallText>
						) : (
							Object.entries(network.ports).map(
								([name, config]) => (
									<Fragment key={name}>
										<Dt>{name}</Dt>
										<Dd>
											{config.routing.guard ? (
												<Badge variant="secondary">
													Guard
												</Badge>
											) : null}
											{config.routing.host ? (
												<Badge variant="secondary">
													Host
												</Badge>
											) : null}
										</Dd>
										<Dl className="col-span-2 ml-5">
											<Dt>Internal port</Dt>
											<Dd>
												{config.internalPort || "-"}
											</Dd>
										</Dl>
										<Dl className="col-span-2 ml-5">
											<Dt>Protocol</Dt>
											<Dd>{config.protocol}</Dd>
										</Dl>
										<Dl className="col-span-2 ml-5">
											<Dt>Hostname</Dt>
											<Dd>{config.hostname || "-"}</Dd>
										</Dl>
										<Dl className="col-span-2 ml-5">
											<Dt>Path</Dt>
											<Dd>{config.path || "-"}</Dd>
										</Dl>
										<Dl className="col-span-2 ml-5">
											<Dt>Port</Dt>
											<Dd>{config.port || "-"}</Dd>
										</Dl>
									</Fragment>
								),
							)
						)}
					</div>
				</Dl>
			</Flex>
		</ScrollArea>
	);
}
